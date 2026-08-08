pub use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
    time::Duration,
};

pub use crate::clients::binance::*;
pub use futures_util::StreamExt;
pub use serde_json::{Value, json};
pub use tokio::{
    sync::{Mutex, mpsc, watch},
    task::JoinHandle,
    time::{self, MissedTickBehavior},
};
pub use tracing::{error, info, warn};
pub use uuid::Uuid;

pub use crate::{
    clients::exchanges::{
        ExchangeAccountBalanceUpdate, ExchangeAccountPositionUpdate, ExchangeAccountStreamUpdate,
        ExchangeCredentials, ExchangeMarginMode, ExchangeOrderStreamUpdate, ExchangeOrderType,
        ExchangeSide, ExchangeSymbolConstraints, ExchangeUserStreamEvent,
        ExchangeUserStreamSession, LiveExchangeAdapter, PlaceOrderRequest, PositionSide,
        TimeInForce, create_exchange_adapter, spawn_exchange_user_stream_reader,
    },
    error::AppError,
    realtime::RealtimeHub,
    repositories::{ExchangeRepo, TradingRepo, trading::records::positions::TraderPositionRecord},
    runtime_events::{
        EVENT_BUDGET_CIRCUIT_BREAKER, EVENT_CANCEL_REPLACE_SUCCEEDED,
        EVENT_CANCEL_REPLACE_THROTTLED, EVENT_CANCEL_REPLACE_USED_MARKET_FALLBACK,
        EVENT_LIVE_OPEN_SKIPPED_CONSTRAINTS, EVENT_LIVE_OPEN_SKIPPED_MEDIUM_RISK,
        EVENT_LIVE_OPEN_USED_MARKET_FALLBACK, EVENT_LIVE_ORDER_SUBMITTED, EVENT_LIVE_RISK_SNAPSHOT,
        EVENT_STALE_INTENT_RECONCILE_PENDING, EVENT_STALE_INTENT_RECONCILE_TERMINAL,
    },
    services::llm::LlmMessage,
    state::{RuntimeEngineManager, RuntimeEngineState},
};

pub use super::{
    account_sim::*, ai_decision::*, binance_events::*, config_loaders::*, db_utils::*, engine::*,
    events::*, execution_live::*, execution_live_limit::*, execution_sim::*, market::*,
    market_seed::*, models::*,
};

#[derive(Debug)]
pub struct EngineWorker {
    stop_tx: watch::Sender<bool>,
    handle: JoinHandle<()>,
}

#[derive(Debug)]
pub struct EngineInner {
    pub state: SharedState,
    pub(crate) workers: Mutex<HashMap<String, EngineWorker>>,
}

#[derive(Clone, Debug)]
pub struct TradingRuntimeService {
    pub inner: Arc<EngineInner>,
}

impl TradingRuntimeService {
    pub fn new(
        trading_repo: Arc<TradingRepo>,
        exchange_repo: Arc<ExchangeRepo>,
        live: crate::config::LiveRuntimeConfig,
        runtime_engine_manager: Arc<RwLock<RuntimeEngineManager>>,
        llm_service: Arc<crate::services::llm::LlmService>,
        realtime_hub: RealtimeHub,
    ) -> Self {
        Self {
            inner: Arc::new(EngineInner {
                state: RuntimeState::new(
                    trading_repo,
                    exchange_repo,
                    live,
                    runtime_engine_manager,
                    llm_service,
                    realtime_hub,
                ),
                workers: Mutex::new(HashMap::new()),
            }),
        }
    }

    pub fn state(&self) -> SharedState {
        self.inner.state.clone()
    }

    pub async fn recover_running_traders_from_db(&self) -> Result<Vec<String>, AppError> {
        let state = self.state();
        let rows = state.trading_repo.running_traders().await?;

        let mut recovered = Vec::new();

        for trader_id in rows {
            match self.start_trader(&trader_id).await {
                Ok(_) => {
                    info!("startup recovery resumed trader={}", trader_id);
                    recovered.push(trader_id);
                }
                Err(AppError::AlreadyRunning(_)) => {
                    recovered.push(trader_id);
                }
                Err(err) => {
                    warn!("startup recovery failed trader={} err={}", trader_id, err);

                    if let Err(db_err) = set_trader_running(&state, &trader_id, false).await {
                        warn!(
                            "startup recovery failed to reset is_running trader={} err={}",
                            trader_id, db_err
                        );
                    }

                    let _ = state.set_runtime_engine_running(
                        &trader_id,
                        false,
                        Some(format!("startup recovery failed: {}", err)),
                    );
                }
            }
        }

        Ok(recovered)
    }

    pub async fn start_trader(&self, trader_id: &str) -> Result<(), AppError> {
        let state = self.state();
        let cfg = load_trader_runtime_config(&state, trader_id)
            .await?
            .ok_or_else(|| AppError::TraderNotFound(trader_id.to_string()))?;

        if cfg.scan_interval_minutes <= 0 {
            return Err(AppError::InvalidConfig(
                "scan_interval_minutes must be > 0".to_string(),
            ));
        }

        // Validate against allowed scan intervals
        const ALLOWED_INTERVALS: [i64; 12] = [1, 5, 10, 15, 20, 30, 60, 120, 240, 480, 720, 1440];
        if !ALLOWED_INTERVALS.contains(&cfg.scan_interval_minutes) {
            return Err(AppError::InvalidConfig(format!(
                "scan_interval_minutes must be one of: {:?}",
                ALLOWED_INTERVALS
            )));
        }

        let _ = load_runtime_execution_context(&state, &cfg).await?;

        {
            let workers = self.inner.workers.lock().await;
            if workers.contains_key(trader_id) {
                return Err(AppError::AlreadyRunning(trader_id.to_string()));
            }
        }

        let now = now_u64();
        state.upsert_runtime_engine(RuntimeEngineState {
            trader_id: cfg.trader_id.clone(),
            exchange_id: cfg.exchange_id.clone(),
            ai_model_id: cfg.ai_model_id.clone(),
            started_at: now,
            updated_at: now,
            is_running: true,
            last_error: None,
            next_scan_at: None,
        })?;

        set_trader_running(&state, &cfg.trader_id, true).await?;

        let (stop_tx, stop_rx) = watch::channel(false);
        let engine = self.clone();
        let cfg_for_task = cfg.clone();
        let handle = tokio::spawn(async move {
            if let Err(err) = run_trader_loop(engine.clone(), cfg_for_task.clone(), stop_rx).await {
                error!("runtime loop failed: {err}");
                if let Err(db_err) =
                    set_trader_running(&engine.inner.state, &cfg_for_task.trader_id, false).await
                {
                    warn!(
                        "failed to clear running flag after runtime error trader={} err={}",
                        cfg_for_task.trader_id, db_err
                    );
                }
                let _ = engine.inner.state.set_runtime_engine_running(
                    &cfg_for_task.trader_id,
                    false,
                    Some(format!("runtime loop failed: {err}")),
                );
            }
        });

        let mut workers = self.inner.workers.lock().await;
        workers.insert(cfg.trader_id.clone(), EngineWorker { stop_tx, handle });

        info!(
            "runtime engine started for trader={} exchange={} model={}",
            cfg.trader_id, cfg.exchange_id, cfg.ai_model_id
        );

        // Notify connected realtime clients that this trader started
        state
            .realtime_hub
            .publish(crate::realtime::RealtimeEvent::EngineStatus {
                trader_id: cfg.trader_id.clone(),
                status: "running".to_string(),
                message: format!(
                    "engine started (exchange={}, model={})",
                    cfg.exchange_id, cfg.ai_model_id
                ),
            });

        Ok(())
    }

    pub async fn stop_trader_for_user(&self, trader_id: &str) -> Result<(), AppError> {
        let state = self.state();
        if state.trading_repo.get_trader(trader_id).await?.is_none() {
            return Err(AppError::TraderNotFound(trader_id.to_string()));
        }

        self.stop_trader(trader_id).await
    }

    pub async fn stop_trader(&self, trader_id: &str) -> Result<(), AppError> {
        let state = self.state();
        let worker = {
            let mut workers = self.inner.workers.lock().await;
            workers.remove(trader_id)
        };

        let Some(worker) = worker else {
            return Err(AppError::NotRunning(trader_id.to_string()));
        };

        let _ = worker.stop_tx.send(true);
        worker.handle.await?;

        if let Err(err) = state.set_runtime_engine_running(trader_id, false, None) {
            warn!("set_runtime_engine_running(false) failed: {err}");
        }

        // Notify realtime clients that the trader stopped
        state
            .realtime_hub
            .publish(crate::realtime::RealtimeEvent::EngineStatus {
                trader_id: trader_id.to_string(),
                status: "stopped".to_string(),
                message: "engine stopped".to_string(),
            });

        Ok(())
    }

    pub async fn shutdown_all(&self) -> Result<(), AppError> {
        let ids = {
            let workers = self.inner.workers.lock().await;
            workers.keys().cloned().collect::<Vec<_>>()
        };

        for id in ids {
            if let Err(err) = self.stop_trader(&id).await {
                warn!("failed to stop trader={} during shutdown: {}", id, err);
            }
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn is_running(&self, trader_id: &str) -> bool {
        let workers = self.inner.workers.lock().await;
        workers.contains_key(trader_id)
    }

    #[allow(dead_code)]
    pub async fn running_traders(&self) -> Vec<String> {
        let workers = self.inner.workers.lock().await;
        workers.keys().cloned().collect()
    }
}

pub fn patch_json_payload(raw: &str, updates: &[(&str, Value)]) -> String {
    let mut payload = match serde_json::from_str::<Value>(raw) {
        Ok(Value::Object(map)) => map,
        _ => serde_json::Map::new(),
    };

    for (key, value) in updates {
        payload.insert((*key).to_string(), value.clone());
    }

    Value::Object(payload).to_string()
}

pub fn position_view_from_record(record: TraderPositionRecord) -> PositionView {
    PositionView {
        id: record.id,
        symbol: record.symbol,
        side: record.side,
        quantity: record.quantity,
        entry_price: record.entry_price,
        mark_price: record.mark_price,
        leverage: record.leverage as i64,
        opened_at: record.opened_at,
    }
}
