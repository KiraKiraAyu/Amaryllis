use std::{
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

use axum::extract::FromRef;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

pub use crate::config::AppConfig;
use crate::{
    database,
    repositories::{
        AppSettingsRepo, BacktestRepo, DebateRepo, ExchangeRepo, ModelRepo, StrategyRepo,
        TradingRepo,
    },
    services::{
        llm::LlmService, models::ModelService, strategies::StrategyService,
        trading::service::TradingService, trading_runtime::service::TradingRuntimeService,
    },
};
use crate::{
    realtime::RealtimeHub,
    services::{
        auth::AuthService, backtest::BacktestService, debate::DebateService,
        exchange_config::ExchangeConfigService,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeEngineState {
    pub trader_id: String,
    pub exchange_id: String,
    pub ai_model_id: String,
    pub started_at: u64,
    pub updated_at: u64,
    pub is_running: bool,
    pub last_error: Option<String>,
    /// Unix timestamp (seconds) of the next scheduled scan.
    /// `None` when the trader is stopped or a scan is in progress.
    pub next_scan_at: Option<u64>,
}

#[derive(Debug, Default)]
pub struct RuntimeEngineManager {
    engines: HashMap<String, RuntimeEngineState>, // trader_id -> runtime state
}

impl RuntimeEngineManager {
    pub fn upsert(&mut self, engine: RuntimeEngineState) {
        self.engines.insert(engine.trader_id.clone(), engine);
    }

    pub fn remove(&mut self, trader_id: &str) -> Option<RuntimeEngineState> {
        self.engines.remove(trader_id)
    }

    pub fn get(&self, trader_id: &str) -> Option<RuntimeEngineState> {
        self.engines.get(trader_id).cloned()
    }

    pub fn list(&self) -> Vec<RuntimeEngineState> {
        self.engines.values().cloned().collect()
    }

    pub fn set_running(
        &mut self,
        trader_id: &str,
        is_running: bool,
        updated_at: u64,
        last_error: Option<String>,
    ) -> bool {
        if let Some(v) = self.engines.get_mut(trader_id) {
            v.is_running = is_running;
            v.updated_at = updated_at;
            if let Some(err) = last_error {
                v.last_error = Some(err);
            }
            if !is_running {
                v.next_scan_at = None;
            }
            true
        } else {
            false
        }
    }

    pub fn set_next_scan_at(
        &mut self,
        trader_id: &str,
        next_scan_at: Option<u64>,
        updated_at: u64,
    ) -> bool {
        if let Some(v) = self.engines.get_mut(trader_id) {
            v.next_scan_at = next_scan_at;
            v.updated_at = updated_at;
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Default)]
pub struct BacktestManager {
    runs: HashMap<String, BacktestRunEntry>,
}

#[derive(Debug)]
pub struct BacktestRunEntry {
    pub stop_tx: oneshot::Sender<()>,
}

impl BacktestManager {
    pub fn new() -> Self {
        Self {
            runs: HashMap::new(),
        }
    }

    pub fn insert(&mut self, run_id: String, entry: BacktestRunEntry) {
        self.runs.insert(run_id, entry);
    }

    pub fn remove(&mut self, run_id: &str) -> Option<BacktestRunEntry> {
        self.runs.remove(run_id)
    }

    pub fn stop(&mut self, run_id: &str) -> Result<(), String> {
        if let Some(entry) = self.runs.remove(run_id) {
            let _ = entry.stop_tx.send(());
            Ok(())
        } else {
            Err("run not found or already finished".into())
        }
    }
}

#[derive(Debug, Clone, FromRef)]
pub struct AppState {
    pub config: AppConfig,
    pub db: DatabaseConnection,
    pub boot_unix_ts: u64,
    pub runtime_engine_manager: Arc<RwLock<RuntimeEngineManager>>,
    pub backtest_manager: Arc<Mutex<BacktestManager>>,
    pub realtime_hub: RealtimeHub,
    pub services: Services,
}

#[derive(Debug, Clone)]
pub struct Services {
    pub auth_service: Arc<AuthService>,
    pub backtest_service: Arc<BacktestService>,
    pub debate_service: Arc<DebateService>,
    pub exchange_config_service: Arc<ExchangeConfigService>,
    pub llm_service: Arc<LlmService>,
    pub model_service: Arc<ModelService>,
    pub strategy_service: Arc<StrategyService>,
    pub trading_service: Arc<TradingService>,
    pub trading_runtime_service: Arc<TradingRuntimeService>,
}

impl Services {
    async fn new(
        config: &AppConfig,
        db: &DatabaseConnection,
        runtime_engine_manager: Arc<RwLock<RuntimeEngineManager>>,
        realtime_hub: RealtimeHub,
    ) -> Self {
        let app_settings_repo = Arc::new(AppSettingsRepo::new(db.clone()));
        let auth_service = Arc::new(
            AuthService::new(config.auth.clone(), app_settings_repo)
                .await
                .expect("Failed to initialize auth service"),
        );

        let model_repo = Arc::new(ModelRepo::new(db.clone()));
        let llm_service = Arc::new(LlmService::new(model_repo.clone()));
        let backtest_repo = Arc::new(BacktestRepo::new(db.clone()));

        let exchange_repo = Arc::new(ExchangeRepo::new(db.clone()));
        let exchange_config_service = Arc::new(ExchangeConfigService::new(exchange_repo.clone()));

        let trading_repo = Arc::new(TradingRepo::new(db.clone()));
        let trading_runtime_service = Arc::new(TradingRuntimeService::new(
            trading_repo.clone(),
            exchange_repo.clone(),
            config.live.clone(),
            runtime_engine_manager.clone(),
            llm_service.clone(),
            realtime_hub.clone(),
        ));
        let backtest_manager = Arc::new(Mutex::new(BacktestManager::new()));
        let backtest_service = Arc::new(BacktestService::new(
            backtest_repo,
            realtime_hub.clone(),
            trading_runtime_service.state(),
            backtest_manager.clone(),
        ));

        let debate_repo = Arc::new(DebateRepo::new(db.clone()));
        let debate_service = Arc::new(DebateService::new(
            debate_repo,
            realtime_hub.clone(),
            llm_service.clone(),
        ));
        let model_service = Arc::new(ModelService::new(model_repo.clone(), llm_service.clone()));
        let strategy_repo = Arc::new(StrategyRepo::new(db.clone()));
        let strategy_service = Arc::new(StrategyService::new(strategy_repo, llm_service.clone()));

        let trading_service = Arc::new(TradingService::new(
            trading_repo,
            exchange_repo.clone(),
            config.runtime_alerts.clone(),
            runtime_engine_manager,
            llm_service.clone(),
            trading_runtime_service.clone(),
        ));

        Self {
            auth_service,
            backtest_service,
            debate_service,
            exchange_config_service,
            llm_service,
            model_service,
            strategy_service,
            trading_service,
            trading_runtime_service,
        }
    }
}

impl AppState {
    pub async fn new() -> Self {
        let config = AppConfig::from_env();

        let db = database::init_database(&config.database.url)
            .await
            .expect("Failed to init database");

        let runtime_engine_manager = Arc::new(RwLock::new(RuntimeEngineManager::default()));
        let backtest_manager = Arc::new(Mutex::new(BacktestManager::new()));
        let realtime_hub = RealtimeHub::new();
        let services = Services::new(
            &config,
            &db,
            runtime_engine_manager.clone(),
            realtime_hub.clone(),
        )
        .await;

        Self {
            config,
            db,
            boot_unix_ts: now_unix_ts(),
            runtime_engine_manager,
            backtest_manager,
            realtime_hub,
            services,
        }
    }

    pub fn uptime_secs(&self) -> u64 {
        now_unix_ts().saturating_sub(self.boot_unix_ts)
    }
}

fn now_unix_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
