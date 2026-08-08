use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::{
    clients::{
        market_data::normalize_crypto_symbol,
        outbound_http::{OutboundRequestLog, send_text},
    },
    contracts::backtest::{BacktestMessagePayload, BacktestRunActionPayload, BacktestRunsPayload},
    error::{AppError, Result as AppResult},
    realtime::RealtimeHub,
    repositories::{
        backtests::{BacktestRepo, CreateBacktestRunRecord},
        trading::records::traders::CreateTraderRecord,
    },
    services::trading_runtime::{
        self,
        config_loaders::{load_trader_runtime_config, now_i64},
        engine::process_cycle,
        models::{MarketState, RuntimeExecutionContext, RuntimeExecutionMode, TraderRuntimeConfig},
    },
    state::BacktestManager,
};
use reqwest::Method;

// ===== Config =====

/// Configuration for a single backtest run.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BacktestConfig {
    pub run_id: String,
    pub virtual_trader_id: String,
    pub start_ts: i64,
    pub end_ts: i64,
    pub initial_balance: f64,
    pub interval: String,
}

#[derive(Debug, Clone)]
pub struct BacktestService {
    backtest_repo: Arc<BacktestRepo>,
    realtime_hub: RealtimeHub,
    runtime_state: trading_runtime::models::SharedState,
    backtest_manager: Arc<Mutex<BacktestManager>>,
}

impl BacktestService {
    pub fn new(
        backtest_repo: Arc<BacktestRepo>,
        realtime_hub: RealtimeHub,
        runtime_state: trading_runtime::models::SharedState,
        backtest_manager: Arc<Mutex<BacktestManager>>,
    ) -> Self {
        Self {
            backtest_repo,
            realtime_hub,
            runtime_state,
            backtest_manager,
        }
    }

    pub async fn start(
        &self,
        trader_id: &str,
        run_id: Option<String>,
        start_ts: Option<i64>,
        end_ts: Option<i64>,
        initial_balance: Option<f64>,
        interval: Option<String>,
    ) -> AppResult<BacktestRunActionPayload> {
        // Load source trader record (for strategy_id and other DB fields)
        let source_trader = self
            .runtime_state
            .trading_repo
            .get_trader(trader_id)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to load trader: {e}")))?
            .ok_or_else(|| AppError::TraderNotFound(trader_id.to_string()))?;

        // Load source trader runtime config (for AI/exchange/strategy details)
        let source_cfg = load_trader_runtime_config(&self.runtime_state, trader_id)
            .await?
            .ok_or_else(|| AppError::TraderNotFound(trader_id.to_string()))?;

        let run_id = run_id
            .filter(|id| !id.trim().is_empty())
            .unwrap_or_else(|| Uuid::now_v7().to_string());
        let virtual_trader_id = format!("bt-{}", run_id);
        let now_sec = now_i64();

        let cfg = BacktestConfig {
            run_id: run_id.clone(),
            virtual_trader_id: virtual_trader_id.clone(),
            start_ts: start_ts.unwrap_or(now_sec - 7 * 24 * 3600),
            end_ts: end_ts.unwrap_or(now_sec),
            initial_balance: initial_balance.unwrap_or(source_cfg.initial_balance),
            interval: interval.unwrap_or_else(|| "5m".to_string()),
        };

        // Create virtual trader in database (copy source trader's config)
        let now = now_i64();
        self.runtime_state
            .trading_repo
            .create_trader_with_snapshot(CreateTraderRecord {
                id: virtual_trader_id.clone(),
                snapshot_id: Uuid::now_v7().to_string(),
                name: format!("[Backtest] {}", source_cfg.name),
                ai_model_id: source_cfg.ai_model_id.clone(),
                exchange_id: source_cfg.exchange_id.clone(),
                strategy_id: source_trader.strategy_id.clone(),
                initial_balance: cfg.initial_balance,
                scan_interval_minutes: source_cfg.scan_interval_minutes,
                is_cross_margin: source_cfg.is_cross_margin,
                use_ai500: false,
                use_oi_top: false,
                custom_prompt: source_cfg.custom_prompt.clone(),
                override_base_prompt: source_cfg.override_base_prompt,
                system_prompt_template: source_cfg.system_prompt_template.clone(),
                created_at: now,
                updated_at: now,
            })
            .await
            .map_err(|e| AppError::Internal(format!("Failed to create virtual trader: {e}")))?;

        // Load the virtual trader's runtime config (picks up strategy config etc.)
        let runtime_cfg = load_trader_runtime_config(&self.runtime_state, &virtual_trader_id)
            .await?
            .ok_or_else(|| AppError::Internal("Virtual trader not found after creation".into()))?;

        // Persist initial backtest run row
        let config_json = serde_json::to_string(&cfg).unwrap_or_default();
        self.backtest_repo
            .create_run(CreateBacktestRunRecord {
                run_id: run_id.clone(),
                config_json,
                created_at: now,
                updated_at: now,
            })
            .await
            .map_err(|e| AppError::Internal(format!("Failed to create backtest run: {e}")))?;

        // Fetch historical klines per symbol (may take time)
        let klines_by_symbol = load_klines_per_symbol(&cfg, &runtime_cfg).await;
        if klines_by_symbol.is_empty() {
            let _ = write_run_status(
                &self.backtest_repo,
                &run_id,
                "failed",
                "No kline data available",
                &RunMetrics::default(),
            )
            .await;
            // Clean up virtual trader
            let _ = self
                .runtime_state
                .trading_repo
                .delete_trader(&virtual_trader_id)
                .await;
            return Err(AppError::Internal(
                "No kline data available for the requested period".into(),
            ));
        }

        let runner = BacktestRunner {
            cfg,
            runtime_cfg,
            klines_by_symbol,
            runtime_state: self.runtime_state.clone(),
            backtest_repo: self.backtest_repo.clone(),
            realtime_hub: self.realtime_hub.clone(),
            backtest_manager: self.backtest_manager.clone(),
        };

        let (stop_tx, stop_rx) = oneshot::channel::<()>();

        // Register in manager
        {
            let mut mgr = self.backtest_manager.lock().unwrap();
            mgr.insert(run_id.clone(), crate::state::BacktestRunEntry { stop_tx });
        }

        // Spawn the run loop
        tokio::spawn(runner.run(stop_rx));

        Ok(BacktestRunActionPayload {
            run_id,
            message: "Backtest started",
        })
    }

    pub fn stop(&self, run_id: String) -> AppResult<BacktestRunActionPayload> {
        let mut mgr = self.backtest_manager.lock().unwrap();
        mgr.stop(&run_id)
            .map_err(|err| AppError::BadRequest(err.into()))?;

        Ok(BacktestRunActionPayload {
            run_id,
            message: "Stop sent",
        })
    }

    pub async fn delete(&self, run_id: String) -> AppResult<BacktestMessagePayload> {
        {
            let mut mgr = self.backtest_manager.lock().unwrap();
            let _ = mgr.stop(&run_id);
        }

        // Clean up virtual trader if it still exists
        let virtual_trader_id = format!("bt-{}", run_id);
        let _ = self
            .runtime_state
            .trading_repo
            .delete_trader(&virtual_trader_id)
            .await;

        let rows_affected = self
            .backtest_repo
            .delete_run(&run_id)
            .await
            .map_err(|err| AppError::Internal(format!("Failed to delete run: {err}")))?;

        if rows_affected == 0 {
            return Err(AppError::NotFound("Run not found".into()));
        }

        Ok(BacktestMessagePayload {
            message: "Run deleted",
        })
    }

    pub async fn runs(&self, limit: Option<i64>) -> BacktestRunsPayload {
        let limit = limit.unwrap_or(50).clamp(1, 200);
        let runs = self
            .backtest_repo
            .list_runs(limit)
            .await
            .unwrap_or_default();
        let count = runs.len();
        BacktestRunsPayload { runs, count }
    }
}

// ===== Runner =====

struct BacktestRunner {
    cfg: BacktestConfig,
    runtime_cfg: TraderRuntimeConfig,
    klines_by_symbol: HashMap<String, Vec<KlineBar>>,
    runtime_state: trading_runtime::models::SharedState,
    backtest_repo: Arc<BacktestRepo>,
    realtime_hub: RealtimeHub,
    backtest_manager: Arc<Mutex<BacktestManager>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct RunMetrics {
    total_trades: i64,
    winning_trades: i64,
    total_realized_pnl: f64,
    max_drawdown_pct: f64,
    max_equity: f64,
    final_equity: f64,
    initial_balance: f64,
}

#[derive(Debug, Clone)]
struct KlineBar {
    open_time: i64,
    high: f64,
    low: f64,
    close: f64,
}

impl BacktestRunner {
    async fn run(self, stop_rx: oneshot::Receiver<()>) {
        let mut stop_rx = stop_rx;
        let run_id = self.cfg.run_id.clone();
        let virtual_trader_id = self.cfg.virtual_trader_id.clone();

        // Parse symbols from runtime config
        let symbols: Vec<String> = self
            .runtime_cfg
            .trading_symbols
            .split(',')
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim().to_uppercase())
            .collect();

        if symbols.is_empty() {
            let _ = write_run_status(
                &self.backtest_repo,
                &run_id,
                "failed",
                "No trading symbols configured",
                &RunMetrics::default(),
            )
            .await;
            self.cleanup().await;
            return;
        }

        // Build per-symbol kline lookup maps and collect all unique timestamps
        let mut kline_maps: HashMap<String, BTreeMap<i64, KlineBar>> = HashMap::new();
        let mut all_timestamps: BTreeSet<i64> = BTreeSet::new();

        for (symbol, klines) in &self.klines_by_symbol {
            let mut map = BTreeMap::new();
            for bar in klines {
                map.insert(bar.open_time, bar.clone());
                all_timestamps.insert(bar.open_time);
            }
            kline_maps.insert(symbol.clone(), map);
        }

        let total_bars = all_timestamps.len();
        let sorted_timestamps: Vec<i64> = all_timestamps.into_iter().collect();

        // Initialize market state with first available price for each symbol
        let mut market: HashMap<String, MarketState> = HashMap::new();
        for sym in &symbols {
            let first_close = kline_maps
                .get(sym)
                .and_then(|m| m.values().next())
                .map(|b| b.close)
                .unwrap_or(100.0);
            market.insert(
                sym.clone(),
                MarketState {
                    price: first_close,
                    prev_price: first_close,
                    volatility: 0.01,
                },
            );
        }

        // Simulated execution context (no live exchange adapter)
        let exec_ctx = RuntimeExecutionContext {
            mode: RuntimeExecutionMode::Simulated,
        };

        let mut metrics_cache = RunMetrics::default();
        metrics_cache.initial_balance = self.cfg.initial_balance;
        metrics_cache.max_equity = self.cfg.initial_balance;
        metrics_cache.final_equity = self.cfg.initial_balance;

        let mut decision_cycle = 0usize;
        let mut max_equity = self.cfg.initial_balance;

        // Main loop — iterate through all unique timestamps across symbols
        for ts_ms in &sorted_timestamps {
            // Check stop signal (non-blocking)
            if stop_rx.try_recv().is_ok() {
                let _ = write_run_status(
                    &self.backtest_repo,
                    &run_id,
                    "stopped",
                    "",
                    &metrics_cache,
                )
                .await;
                break;
            }

            let ts_sec = ts_ms / 1000;

            // Update each symbol's market state with the bar at this timestamp
            for sym in &symbols {
                if let Some(bar) = kline_maps.get(sym).and_then(|m| m.get(ts_ms)) {
                    if let Some(state) = market.get_mut(sym) {
                        state.prev_price = state.price;
                        state.price = bar.close;
                        let range = (bar.high - bar.low) / bar.close.max(1e-9);
                        state.volatility = (state.volatility * 0.9 + range * 0.1).clamp(0.001, 0.1);
                    }
                }
            }

            decision_cycle += 1;
            let cycle = decision_cycle;

            // Run process_cycle — reuses live trading logic with virtual time
            let result = process_cycle(
                &self.runtime_state,
                &self.runtime_cfg,
                &symbols,
                &mut market,
                &exec_ctx,
                None, // no live adapter — simulated mode
                ts_sec,
                true, // backtest_mode
            )
            .await;

            if let Err(err) = result {
                tracing::warn!(
                    "backtest {} cycle {} error: {}",
                    run_id,
                    cycle,
                    err
                );
                // Budget circuit breaker — stop the backtest
                if matches!(err, AppError::BudgetExhausted(_)) {
                    let _ = write_run_status(
                        &self.backtest_repo,
                        &run_id,
                        "completed",
                        &format!("Budget exhausted: {}", err),
                        &metrics_cache,
                    )
                    .await;
                    break;
                }
            }

            // Read the latest equity snapshot from the database
            let equity_points = self
                .runtime_state
                .trading_repo
                .equity_history_points(&virtual_trader_id, None, 1)
                .await
                .unwrap_or_default();

            if let Some(point) = equity_points.first() {
                let eq = point.total_equity;

                if eq > max_equity {
                    max_equity = eq;
                }
                let dd = if max_equity > 0.0 {
                    (max_equity - eq) / max_equity * 100.0
                } else {
                    0.0
                };

                metrics_cache.final_equity = eq;
                metrics_cache.max_equity = max_equity;
                if dd > metrics_cache.max_drawdown_pct {
                    metrics_cache.max_drawdown_pct = dd;
                }
            }

            // Push backtest progress to realtime clients
            self.realtime_hub.publish(crate::realtime::RealtimeEvent::BacktestProgress {
                run_id: run_id.clone(),
                state: "running".to_string(),
                bar_index: cycle,
                total_bars,
                equity: metrics_cache.final_equity,
                ts: ts_sec,
            });

            // Write updated status periodically
            let _ = write_run_status(
                &self.backtest_repo,
                &run_id,
                "running",
                "",
                &metrics_cache,
            )
            .await;

            // Small yield so other tasks can run
            tokio::task::yield_now().await;
        }

        // Finalize: collect trade counts, write final status, clean up
        self.finalize(&run_id, &metrics_cache, total_bars).await;
    }

    /// Collect final trade counts from the virtual trader, write final status,
    /// then delete the virtual trader and remove the run from the manager.
    async fn finalize(&self, run_id: &str, metrics_cache: &RunMetrics, total_bars: usize) {
        let virtual_trader_id = &self.cfg.virtual_trader_id;

        // Read trade counts from the virtual trader's live tables before cleanup
        let trades = self
            .runtime_state
            .trading_repo
            .trades(virtual_trader_id, 10_000, 0)
            .await
            .unwrap_or_default();

        let mut total_trades = 0i64;
        let mut winning_trades = 0i64;
        let mut total_realized_pnl = 0.0;

        for t in &trades {
            total_trades += 1;
            if t.realized_pnl > 0.0 {
                winning_trades += 1;
            }
            total_realized_pnl += t.realized_pnl - t.fees;
        }

        let mut final_metrics = metrics_cache.clone();
        final_metrics.total_trades = total_trades;
        final_metrics.winning_trades = winning_trades;
        final_metrics.total_realized_pnl = total_realized_pnl;

        // Write final status
        let _ = write_run_status(
            &self.backtest_repo,
            run_id,
            "completed",
            "",
            &final_metrics,
        )
        .await;

        // Push final status to realtime clients — use actual total_bars to avoid NaN%
        self.realtime_hub.publish(crate::realtime::RealtimeEvent::BacktestProgress {
            run_id: run_id.to_string(),
            state: "completed".to_string(),
            bar_index: total_bars,
            total_bars,
            equity: final_metrics.final_equity,
            ts: now_i64(),
        });

        // Clean up virtual trader from live tables
        let _ = self
            .runtime_state
            .trading_repo
            .delete_trader(virtual_trader_id)
            .await;

        // Remove from backtest manager to prevent memory leak
        {
            let mut mgr = self.backtest_manager.lock().unwrap();
            mgr.remove(run_id);
        }

        tracing::info!(
            "backtest {} finalized: trades={}",
            run_id,
            trades.len()
        );
    }

    /// Emergency cleanup (used when backtest fails before finalize)
    async fn cleanup(&self) {
        let _ = self
            .runtime_state
            .trading_repo
            .delete_trader(&self.cfg.virtual_trader_id)
            .await;

        // Remove from backtest manager to prevent memory leak
        let mut mgr = self.backtest_manager.lock().unwrap();
        mgr.remove(&self.cfg.run_id);
    }
}

// ===== Persistence helpers =====

async fn write_run_status(
    backtest_repo: &BacktestRepo,
    run_id: &str,
    status: &str,
    last_error: &str,
    metrics: &RunMetrics,
) -> Result<(), crate::database::DbErr> {
    let summary = serde_json::to_string(metrics).unwrap_or_default();
    backtest_repo
        .update_run_status(run_id, status, last_error, summary, now_i64())
        .await
}

// ===== Kline fetching =====

async fn fetch_klines_from_binance(
    symbol: &str,
    interval: &str,
    start_ts_ms: i64,
    end_ts_ms: i64,
) -> Vec<KlineBar> {
    let url = format!(
        "https://fapi.binance.com/fapi/v1/klines?symbol={}&interval={}&startTime={}&endTime={}&limit=1500",
        symbol, interval, start_ts_ms, end_ts_ms
    );
    let resp = match send_text(
        reqwest::Client::new().get(&url),
        OutboundRequestLog::new("backtest.binance.klines", Method::GET, &url),
    )
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("fetch klines error: {e}");
            return vec![];
        }
    };
    if !resp.status.is_success() {
        tracing::warn!("fetch klines non-success status={}", resp.status);
        return vec![];
    }
    let rows: Vec<Vec<serde_json::Value>> = match serde_json::from_str(&resp.body) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("parse klines error: {e}");
            return vec![];
        }
    };
    rows.into_iter()
        .filter_map(|r| {
            if r.len() < 5 {
                return None;
            }
            Some(KlineBar {
                open_time: r[0].as_i64()?,
                high: r[2].as_str()?.parse().ok()?,
                low: r[3].as_str()?.parse().ok()?,
                close: r[4].as_str()?.parse().ok()?,
            })
        })
        .collect()
}

/// Fetch klines for a single symbol across the full time range.
async fn fetch_klines_for_symbol(
    symbol: &str,
    interval: &str,
    start_ts: i64,
    end_ts: i64,
) -> Vec<KlineBar> {
    let start_ms = start_ts * 1000;
    let end_ms = end_ts * 1000;
    let mut all: Vec<KlineBar> = Vec::new();
    let mut cursor = start_ms;
    let batch_limit = 1500i64;
    let ms_per_bar = interval_to_ms(interval).unwrap_or(300_000);

    while cursor < end_ms {
        let batch_end_ms = cursor + batch_limit * ms_per_bar;
        let batch_to = batch_end_ms.min(end_ms);
        let bars = fetch_klines_from_binance(symbol, interval, cursor, batch_to).await;
        if bars.is_empty() {
            break;
        }
        cursor = bars.last().map(|b| b.open_time + 1).unwrap_or(batch_to + 1);
        all.extend(bars);
        if cursor >= end_ms {
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
    all.sort_by_key(|b| b.open_time);
    all
}

/// Fetch klines for every configured trading symbol independently.
async fn load_klines_per_symbol(
    cfg: &BacktestConfig,
    runtime_cfg: &TraderRuntimeConfig,
) -> HashMap<String, Vec<KlineBar>> {
    let symbols: Vec<String> = runtime_cfg
        .symbols_config
        .iter()
        .map(|s| normalize_crypto_symbol(&s.symbol))
        .collect();

    let mut result = HashMap::new();
    for symbol in &symbols {
        let klines =
            fetch_klines_for_symbol(symbol, &cfg.interval, cfg.start_ts, cfg.end_ts).await;
        if !klines.is_empty() {
            result.insert(symbol.clone(), klines);
        }
    }
    result
}

fn interval_to_ms(interval: &str) -> Option<i64> {
    match interval {
        "1m" => Some(60_000),
        "3m" => Some(180_000),
        "5m" => Some(300_000),
        "15m" => Some(900_000),
        "30m" => Some(1_800_000),
        "1h" => Some(3_600_000),
        "4h" => Some(14_400_000),
        "1d" => Some(86_400_000),
        _ => None,
    }
}
