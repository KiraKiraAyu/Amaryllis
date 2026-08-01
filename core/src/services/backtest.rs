use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::{
    clients::{
        market_data::{fetch_binance_klines, normalize_crypto_symbol},
        outbound_http::{OutboundRequestLog, send_text},
    },
    contracts::backtest::{
        BacktestDecisionsPayload, BacktestEquityPayload, BacktestExportPayload,
        BacktestLabelRequest, BacktestMessagePayload, BacktestMetricsPayload, BacktestQueryParams,
        BacktestRunActionPayload, BacktestRunIdRequest, BacktestRunsPayload, BacktestStartRequest,
        BacktestStatusPayload, BacktestTracePayload, BacktestTradesPayload, KlinePayload,
        KlinesQuery,
    },
    error::{AppError, Result as AppResult},
    realtime::RealtimeHub,
    repositories::{
        backtests::{
            BacktestDecisionRecord, BacktestEquityPointRecord, BacktestRepo, BacktestTradeRecord,
            CreateBacktestRunRecord,
        },
        models::ResolvedModelRecord,
    },
    services::llm::{LlmMessage, LlmService},
};
use reqwest::Method;

// ===== Constants =====

const MIN_POSITION_SIZE_USD: f64 = 10.0;
const DEFAULT_FEE_BPS: f64 = 4.0;
const DEFAULT_SLIPPAGE_BPS: f64 = 2.0;

// ===== Config =====

/// Configuration for a single backtest run.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BacktestConfig {
    pub run_id: String,
    pub symbols: Vec<String>,
    /// Unix timestamp seconds (start of historical range)
    pub start_ts: i64,
    /// Unix timestamp seconds (end of historical range)
    pub end_ts: i64,
    pub initial_balance: f64,
    /// Fee in basis points (e.g. 4.0 = 0.04%)
    #[serde(default = "default_fee_bps")]
    pub fee_bps: f64,
    /// Slippage in basis points
    #[serde(default = "default_slippage_bps")]
    pub slippage_bps: f64,
    pub ai_model_id: String,
    #[serde(default = "default_prompt_variant")]
    pub prompt_variant: String,
    pub btc_eth_leverage: i64,
    pub altcoin_leverage: i64,
    /// Kline interval string (e.g. "5m", "15m")
    #[serde(default = "default_interval")]
    pub interval: String,
    /// Decision frequency: every N kline bars
    #[serde(default = "default_decision_every")]
    pub decision_every: usize,
    /// Strategy config JSON (optional, used for prompt building)
    #[serde(default)]
    pub strategy_config: Value,
}

fn default_fee_bps() -> f64 {
    DEFAULT_FEE_BPS
}
fn default_slippage_bps() -> f64 {
    DEFAULT_SLIPPAGE_BPS
}
fn default_prompt_variant() -> String {
    "balanced".to_string()
}
fn default_interval() -> String {
    "5m".to_string()
}
fn default_decision_every() -> usize {
    1
}

#[derive(Debug, Clone)]
pub struct BacktestService {
    backtest_repo: Arc<BacktestRepo>,
    realtime_hub: RealtimeHub,
    llm_service: Arc<LlmService>,
}

impl BacktestService {
    pub fn new(
        backtest_repo: Arc<BacktestRepo>,
        realtime_hub: RealtimeHub,
        llm_service: Arc<LlmService>,
    ) -> Self {
        Self {
            backtest_repo,
            realtime_hub,
            llm_service,
        }
    }

    pub async fn start(
        &self,
        req: BacktestStartRequest,
    ) -> AppResult<BacktestRunActionPayload> {
        let resolved_model = self
            .llm_service
            .resolve_for_user(req.ai_model_id.as_deref())
            .await?;

        let run_id = req
            .run_id
            .filter(|id| !id.trim().is_empty())
            .unwrap_or_else(|| Uuid::now_v7().to_string());
        let now_sec = now_ts();
        let cfg = BacktestConfig {
            run_id: run_id.clone(),
            symbols: req.symbols.unwrap_or_else(|| vec!["BTCUSDT".to_string()]),
            start_ts: req.start_ts.unwrap_or(now_sec - 7 * 24 * 3600),
            end_ts: req.end_ts.unwrap_or(now_sec),
            initial_balance: req.initial_balance.unwrap_or(1000.0),
            fee_bps: req.fee_bps.unwrap_or(4.0),
            slippage_bps: req.slippage_bps.unwrap_or(2.0),
            ai_model_id: resolved_model.id.clone(),
            prompt_variant: req.prompt_variant.unwrap_or_else(|| "balanced".to_string()),
            btc_eth_leverage: req.btc_eth_leverage.unwrap_or(5),
            altcoin_leverage: req.altcoin_leverage.unwrap_or(5),
            interval: req.interval.unwrap_or_else(|| "5m".to_string()),
            decision_every: req.decision_every.unwrap_or(1),
            strategy_config: json!({}),
        };

        let started_run_id = start_backtest(
            cfg,
            self.backtest_repo.clone(),
            self.llm_service.clone(),
            resolved_model,
            self.realtime_hub.clone(),
        )
        .await
        .map_err(|err| AppError::Internal(err.into()))?;

        Ok(BacktestRunActionPayload {
            run_id: started_run_id,
            message: "Backtest started",
        })
    }

    pub fn pause(&self, req: BacktestRunIdRequest) -> BacktestRunActionPayload {
        BacktestRunActionPayload {
            run_id: req.run_id,
            message: "Pause requested",
        }
    }

    pub fn resume(&self, req: BacktestRunIdRequest) -> BacktestRunActionPayload {
        BacktestRunActionPayload {
            run_id: req.run_id,
            message: "Resume requested",
        }
    }

    pub fn stop(
        &self,
        req: BacktestRunIdRequest,
    ) -> AppResult<BacktestRunActionPayload> {
        stop_backtest(&req.run_id).map_err(|err| AppError::BadRequest(err.into()))?;

        Ok(BacktestRunActionPayload {
            run_id: req.run_id,
            message: "Stop sent",
        })
    }

    pub async fn label(
        &self,
        req: BacktestLabelRequest,
    ) -> AppResult<BacktestMessagePayload> {
        let rows_affected = self
            .backtest_repo
            .update_label(&req.run_id, req.label, now_ts())
            .await
            .map_err(|err| AppError::Internal(format!("Failed to update run label: {err}")))?;

        if rows_affected == 0 {
            return Err(AppError::NotFound("Run not found".into()));
        }

        Ok(BacktestMessagePayload {
            message: "Label updated",
        })
    }

    pub async fn delete(
        &self,
        req: BacktestRunIdRequest,
    ) -> AppResult<BacktestMessagePayload> {
        let _ = stop_backtest(&req.run_id);

        let rows_affected = self
            .backtest_repo
            .delete_run(&req.run_id)
            .await
            .map_err(|err| AppError::Internal(format!("Failed to delete run: {err}")))?;

        if rows_affected == 0 {
            return Err(AppError::NotFound("Run not found".into()));
        }

        Ok(BacktestMessagePayload {
            message: "Run deleted",
        })
    }

    pub async fn status(
        &self,
        q: BacktestQueryParams,
    ) -> AppResult<BacktestStatusPayload> {
        let run_id = required_run_id(q.run_id)?;
        let status = query_run_status(&self.backtest_repo, &run_id)
            .await
            .ok_or_else(|| AppError::NotFound("Run not found".into()))?;

        Ok(BacktestStatusPayload { status })
    }

    pub async fn runs(&self, q: BacktestQueryParams) -> BacktestRunsPayload {
        let limit = q.limit.unwrap_or(50).clamp(1, 200);
        let runs = list_runs(&self.backtest_repo, limit).await;
        let count = runs.len();
        BacktestRunsPayload { runs, count }
    }

    pub async fn equity(
        &self,
        q: BacktestQueryParams,
    ) -> AppResult<BacktestEquityPayload> {
        let run_id = required_run_id(q.run_id)?;
        let limit = q.limit.unwrap_or(5000).clamp(1, 10_000);
        let points = query_equity_points(&self.backtest_repo, &run_id, limit).await;

        Ok(BacktestEquityPayload {
            count: points.len(),
            points,
        })
    }

    pub async fn trades(
        &self,
        q: BacktestQueryParams,
    ) -> AppResult<BacktestTradesPayload> {
        let run_id = required_run_id(q.run_id)?;
        let limit = q.limit.unwrap_or(1000).clamp(1, 5000);
        let trades = query_trades(&self.backtest_repo, &run_id, limit).await;

        Ok(BacktestTradesPayload {
            count: trades.len(),
            trades,
        })
    }

    pub async fn metrics(
        &self,
        q: BacktestQueryParams,
    ) -> AppResult<BacktestMetricsPayload> {
        let run_id = required_run_id(q.run_id)?;
        let metrics = compute_metrics(&self.backtest_repo, &run_id).await;
        Ok(BacktestMetricsPayload { metrics })
    }

    pub async fn trace(
        &self,
        q: BacktestQueryParams,
    ) -> AppResult<BacktestTracePayload> {
        let run_id = required_run_id(q.run_id)?;
        let limit = q.limit.unwrap_or(50).clamp(1, 200);
        let trace = query_decisions(&self.backtest_repo, &run_id, limit).await;
        Ok(BacktestTracePayload { trace })
    }

    pub async fn decisions(
        &self,
        q: BacktestQueryParams,
    ) -> AppResult<BacktestDecisionsPayload> {
        let run_id = required_run_id(q.run_id)?;
        let limit = q.limit.unwrap_or(1000).clamp(1, 5000);
        let decisions = query_decisions(&self.backtest_repo, &run_id, limit).await;
        let count = decisions.len();
        Ok(BacktestDecisionsPayload { decisions, count })
    }

    pub async fn export(
        &self,
        q: BacktestQueryParams,
    ) -> AppResult<BacktestExportPayload> {
        let run_id = required_run_id(q.run_id)?;
        let trades = query_trades(&self.backtest_repo, &run_id, 10_000).await;
        let equity = query_equity_points(&self.backtest_repo, &run_id, 10_000).await;
        Ok(BacktestExportPayload {
            run_id,
            trades,
            equity,
            exported_at: now_ts(),
        })
    }

    pub async fn klines(&self, q: KlinesQuery) -> AppResult<Vec<KlinePayload>> {
        let symbol = q.symbol.trim().to_uppercase();
        if symbol.is_empty() {
            return Err(AppError::BadRequest("symbol is required".into()));
        }

        let interval = q.interval.unwrap_or_else(|| "5m".to_string());
        let limit = q.limit.unwrap_or(1000).clamp(1, 1500) as usize;
        let symbol = normalize_crypto_symbol(&symbol);

        let klines = fetch_binance_klines(&symbol, &interval, limit)
            .await
            .map_err(|err| AppError::BadGateway(format!("Upstream kline fetch failed: {err}")))?;

        Ok(klines.into_iter().map(KlinePayload::from).collect())
    }
}

// ===== Simulated Account =====

#[derive(Debug, Clone)]
struct SimPosition {
    side: String, // "long" | "short"
    qty: f64,
    entry_price: f64,
    leverage: i64,
    margin: f64, // locked margin = notional / leverage
}

#[derive(Debug)]
struct SimAccount {
    cash: f64,
    initial: f64,
    fee_rate: f64,
    slippage_rate: f64,
    positions: HashMap<String, Vec<SimPosition>>,
}

impl SimAccount {
    fn new(initial: f64, fee_bps: f64, slippage_bps: f64) -> Self {
        Self {
            cash: initial,
            initial,
            fee_rate: fee_bps / 10_000.0,
            slippage_rate: slippage_bps / 10_000.0,
            positions: HashMap::new(),
        }
    }

    fn total_equity(&self, prices: &HashMap<String, f64>) -> f64 {
        let unrealized: f64 = self
            .positions
            .iter()
            .flat_map(|(sym, poses)| {
                let price = prices.get(sym).copied().unwrap_or(0.0);
                poses.iter().map(move |p| {
                    if p.qty <= 0.0 || price <= 0.0 {
                        return 0.0;
                    }
                    let notional = p.qty * price;
                    if p.side == "long" {
                        notional - p.qty * p.entry_price
                    } else {
                        p.qty * p.entry_price - notional
                    }
                })
            })
            .sum();
        self.cash + unrealized
    }

    fn unrealized_pnl(&self, prices: &HashMap<String, f64>) -> f64 {
        self.positions
            .iter()
            .flat_map(|(sym, poses)| {
                let price = prices.get(sym).copied().unwrap_or(0.0);
                poses.iter().map(move |p| {
                    if p.qty <= 0.0 || price <= 0.0 {
                        return 0.0;
                    }
                    if p.side == "long" {
                        p.qty * (price - p.entry_price)
                    } else {
                        p.qty * (p.entry_price - price)
                    }
                })
            })
            .sum()
    }

    /// Apply slippage to execution price (adverse for taker).
    fn fill_price(&self, base: f64, side: &str) -> f64 {
        if side == "long" {
            base * (1.0 + self.slippage_rate)
        } else {
            base * (1.0 - self.slippage_rate)
        }
    }

    /// Open a position. Returns (fee, fill_price) or error string.
    fn open(
        &mut self,
        symbol: &str,
        side: &str,
        size_usd: f64,
        leverage: i64,
        base_price: f64,
    ) -> Result<(f64, f64), String> {
        if base_price <= 0.0 {
            return Err("invalid price".into());
        }
        let exec_price = self.fill_price(base_price, side);
        let notional = size_usd;
        let margin = notional / leverage as f64;
        let fee = notional * self.fee_rate;
        let cost = margin + fee;
        if cost > self.cash {
            return Err(format!(
                "insufficient cash: need {:.2} have {:.2}",
                cost, self.cash
            ));
        }
        let qty = notional / exec_price;
        if qty * exec_price < MIN_POSITION_SIZE_USD {
            return Err("position below minimum size".into());
        }
        self.cash -= cost;
        let pos = SimPosition {
            side: side.to_string(),
            qty,
            entry_price: exec_price,
            leverage,
            margin,
        };
        self.positions
            .entry(symbol.to_string())
            .or_default()
            .push(pos);
        Ok((fee, exec_price))
    }

    /// Close the first matching position side. Returns (realized_pnl, fee, fill_price).
    fn close(
        &mut self,
        symbol: &str,
        side: &str,
        base_price: f64,
    ) -> Result<(f64, f64, f64), String> {
        let poses = self.positions.get_mut(symbol).ok_or("no position")?;
        let idx = poses
            .iter()
            .position(|p| p.side == side)
            .ok_or("no matching position")?;
        let pos = poses.remove(idx);
        if poses.is_empty() {
            self.positions.remove(symbol);
        }
        let exec_price = self.fill_price(base_price, if side == "long" { "short" } else { "long" });
        let realized = if side == "long" {
            pos.qty * (exec_price - pos.entry_price)
        } else {
            pos.qty * (pos.entry_price - exec_price)
        };
        let fee = pos.qty * exec_price * self.fee_rate;
        let net = realized - fee;
        self.cash += pos.margin + net;
        Ok((realized, fee, exec_price))
    }

    /// Check whether equity has dropped below liquidation threshold (< 10% of initial).
    fn is_liquidated(&self, prices: &HashMap<String, f64>) -> bool {
        let eq = self.total_equity(prices);
        eq < self.initial * 0.1
    }
}

// ===== Run State =====

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum RunStatus {
    Running,
    Paused,
    Completed,
    Stopped,
    Failed,
}

impl std::fmt::Display for RunStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            RunStatus::Running => "running",
            RunStatus::Paused => "paused",
            RunStatus::Completed => "completed",
            RunStatus::Stopped => "stopped",
            RunStatus::Failed => "failed",
        };
        write!(f, "{s}")
    }
}

// ===== Runner =====

struct BacktestRunner {
    cfg: BacktestConfig,
    account: SimAccount,
    klines: Vec<KlineBar>,
    bar_index: usize,
    decision_cycle: usize,
    llm_service: Arc<LlmService>,
    llm_model: ResolvedModelRecord,
    status: RunStatus,
    last_error: String,
    metrics_cache: RunMetrics,
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
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
}

impl BacktestRunner {
    async fn run(
        mut self,
        backtest_repo: Arc<BacktestRepo>,
        stop_rx: oneshot::Receiver<()>,
        realtime_hub: RealtimeHub,
    ) {
        // Store stop receiver
        let mut stop_rx = stop_rx;
        let run_id = self.cfg.run_id.clone();
        let total_bars = self.klines.len();

        // Main step loop
        loop {
            // Check stop signal (non-blocking)
            if stop_rx.try_recv().is_ok() {
                self.status = RunStatus::Stopped;
                let _ =
                    write_run_status(&backtest_repo, &run_id, "stopped", "", &self.metrics_cache)
                        .await;
                return;
            }

            if self.bar_index >= self.klines.len() {
                self.status = RunStatus::Completed;
                break;
            }

            let bar = self.klines[self.bar_index].clone();
            self.bar_index += 1;

            // Build price map from current bar close
            let mut prices: HashMap<String, f64> = HashMap::new();
            for sym in &self.cfg.symbols {
                prices.insert(sym.clone(), bar.close);
            }

            // Liquidation check
            if self.account.is_liquidated(&prices) {
                self.status = RunStatus::Completed;
                let _ = append_equity_point(
                    &backtest_repo,
                    &run_id,
                    bar.open_time / 1000,
                    self.account.total_equity(&prices),
                    self.account.cash,
                    self.account.unrealized_pnl(&prices),
                    0.0,
                    self.decision_cycle,
                )
                .await;
                break;
            }

            // Decision step
            let should_decide = self.bar_index % self.cfg.decision_every.max(1) == 0;
            if should_decide {
                self.decision_cycle += 1;
                let cycle = self.decision_cycle;
                let ts_sec = bar.open_time / 1000;
                let equity = self.account.total_equity(&prices);

                // Build a simple market prompt
                let prompt = build_trading_prompt(
                    &self.cfg,
                    &bar,
                    equity,
                    self.account.cash,
                    &self.account.positions,
                    cycle,
                );

                match call_llm(&self.llm_service, &self.llm_model, &prompt).await {
                    Ok(decisions) => {
                        for dec in decisions {
                            let sym = dec.symbol.clone();
                            let price = prices.get(&sym).copied().unwrap_or(bar.close);

                            match dec.action.as_str() {
                                "open_long" => {
                                    let size = dec.size_usd.unwrap_or(equity * 0.05);
                                    let lev = resolve_leverage(&self.cfg, &sym);
                                    if let Ok((fee, exec_price)) =
                                        self.account.open(&sym, "long", size, lev, price)
                                    {
                                        let trade_id = Uuid::now_v7().to_string();
                                        let _ = append_trade(
                                            &backtest_repo,
                                            &trade_id,
                                            &run_id,
                                            ts_sec,
                                            &sym,
                                            "open_long",
                                            "long",
                                            size / exec_price,
                                            exec_price,
                                            fee,
                                            0.0,
                                            lev,
                                            cycle,
                                            false,
                                        )
                                        .await;
                                    }
                                }
                                "open_short" => {
                                    let size = dec.size_usd.unwrap_or(equity * 0.05);
                                    let lev = resolve_leverage(&self.cfg, &sym);
                                    if let Ok((fee, exec_price)) =
                                        self.account.open(&sym, "short", size, lev, price)
                                    {
                                        let trade_id = Uuid::now_v7().to_string();
                                        let _ = append_trade(
                                            &backtest_repo,
                                            &trade_id,
                                            &run_id,
                                            ts_sec,
                                            &sym,
                                            "open_short",
                                            "short",
                                            size / exec_price,
                                            exec_price,
                                            fee,
                                            0.0,
                                            lev,
                                            cycle,
                                            false,
                                        )
                                        .await;
                                    }
                                }
                                "close_long" => {
                                    if let Ok((realized, fee, exec_price)) =
                                        self.account.close(&sym, "long", price)
                                    {
                                        self.metrics_cache.total_trades += 1;
                                        if realized > 0.0 {
                                            self.metrics_cache.winning_trades += 1;
                                        }
                                        self.metrics_cache.total_realized_pnl += realized - fee;
                                        let trade_id = Uuid::now_v7().to_string();
                                        let _ = append_trade(
                                            &backtest_repo,
                                            &trade_id,
                                            &run_id,
                                            ts_sec,
                                            &sym,
                                            "close_long",
                                            "long",
                                            0.0,
                                            exec_price,
                                            fee,
                                            realized - fee,
                                            0,
                                            cycle,
                                            false,
                                        )
                                        .await;
                                    }
                                }
                                "close_short" => {
                                    if let Ok((realized, fee, exec_price)) =
                                        self.account.close(&sym, "short", price)
                                    {
                                        self.metrics_cache.total_trades += 1;
                                        if realized > 0.0 {
                                            self.metrics_cache.winning_trades += 1;
                                        }
                                        self.metrics_cache.total_realized_pnl += realized - fee;
                                        let trade_id = Uuid::now_v7().to_string();
                                        let _ = append_trade(
                                            &backtest_repo,
                                            &trade_id,
                                            &run_id,
                                            ts_sec,
                                            &sym,
                                            "close_short",
                                            "short",
                                            0.0,
                                            exec_price,
                                            fee,
                                            realized - fee,
                                            0,
                                            cycle,
                                            false,
                                        )
                                        .await;
                                    }
                                }
                                _ => {} // hold / wait — do nothing
                            }

                            // Persist AI decision record
                            let dec_id = Uuid::now_v7().to_string();
                            let _ = append_decision(
                                &backtest_repo,
                                &dec_id,
                                &run_id,
                                ts_sec,
                                &sym,
                                &dec.action,
                                dec.confidence,
                                &dec.reason,
                                cycle,
                            )
                            .await;
                        }
                    }
                    Err(e) => {
                        tracing::warn!("backtest {} AI error at cycle {}: {}", run_id, cycle, e);
                    }
                }

                // Equity snapshot
                let eq = self.account.total_equity(&prices);
                if eq > self.metrics_cache.max_equity {
                    self.metrics_cache.max_equity = eq;
                }
                let dd = if self.metrics_cache.max_equity > 0.0 {
                    (self.metrics_cache.max_equity - eq) / self.metrics_cache.max_equity * 100.0
                } else {
                    0.0
                };
                if dd > self.metrics_cache.max_drawdown_pct {
                    self.metrics_cache.max_drawdown_pct = dd;
                }
                self.metrics_cache.final_equity = eq;

                let _ = append_equity_point(
                    &backtest_repo,
                    &run_id,
                    ts_sec,
                    eq,
                    self.account.cash,
                    self.account.unrealized_pnl(&prices),
                    dd,
                    cycle,
                )
                .await;

                // Push backtest progress to realtime clients
                realtime_hub.publish(crate::realtime::RealtimeEvent::BacktestProgress {
                    run_id: run_id.clone(),
                    state: "running".to_string(),
                    bar_index: self.bar_index,
                    total_bars,
                    equity: eq,
                    ts: ts_sec,
                });

                // Write updated status periodically
                let _ =
                    write_run_status(&backtest_repo, &run_id, "running", "", &self.metrics_cache)
                        .await;
            }

            // Small yield so other tasks can run
            tokio::task::yield_now().await;
        }

        // Finalize
        let status_str = self.status.to_string();
        let _ = write_run_status(
            &backtest_repo,
            &run_id,
            &status_str,
            &self.last_error,
            &self.metrics_cache,
        )
        .await;

        // Push final status to realtime clients
        realtime_hub.publish(crate::realtime::RealtimeEvent::BacktestProgress {
            run_id,
            state: status_str,
            bar_index: self.bar_index,
            total_bars,
            equity: self.metrics_cache.final_equity,
            ts: now_ts(),
        });
    }
}

// ===== LLM interaction =====

#[derive(Debug)]
struct LlmDecision {
    symbol: String,
    action: String,
    confidence: f64,
    reason: String,
    size_usd: Option<f64>,
}

async fn call_llm(
    llm_service: &LlmService,
    model: &ResolvedModelRecord,
    prompt: &str,
) -> Result<Vec<LlmDecision>, String> {
    let messages = vec![
        LlmMessage { role: "system".to_string(), content: "You are a professional crypto futures trading AI. Respond with a JSON array of trading decisions.".to_string() },
        LlmMessage { role: "user".to_string(), content: prompt.to_string() },
    ];
    let raw = llm_service
        .chat_with_model(model, messages, None)
        .await
        .map_err(|e| e.to_string())?;
    parse_llm_decisions(&raw)
}

fn parse_llm_decisions(raw: &str) -> Result<Vec<LlmDecision>, String> {
    // Try to extract JSON array from the response
    let json_start = raw.find('[').unwrap_or(0);
    let json_end = raw.rfind(']').map(|i| i + 1).unwrap_or(raw.len());
    let slice = &raw[json_start..json_end.min(raw.len())];

    let arr: Vec<Value> = serde_json::from_str(slice).unwrap_or_default();
    let mut out = Vec::new();
    for v in arr {
        let sym = v
            .get("symbol")
            .and_then(Value::as_str)
            .unwrap_or("BTCUSDT")
            .to_uppercase();
        if !sym.ends_with("USDT") && !sym.is_empty() {
            // Accept bare symbols too
        }
        let action = v
            .get("action")
            .and_then(Value::as_str)
            .unwrap_or("hold")
            .to_lowercase();
        let confidence = v
            .get("confidence")
            .and_then(Value::as_f64)
            .unwrap_or(0.5)
            .clamp(0.0, 1.0);
        let reason = v
            .get("reason")
            .or_else(|| v.get("reasoning"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .chars()
            .take(300)
            .collect();
        let size_usd = v
            .get("size_usd")
            .and_then(Value::as_f64)
            .or_else(|| v.get("position_size_usd").and_then(Value::as_f64));
        out.push(LlmDecision {
            symbol: sym,
            action,
            confidence,
            reason,
            size_usd,
        });
    }

    // If no valid decisions, fall back to hold to avoid hanging
    if out.is_empty() {
        out.push(LlmDecision {
            symbol: "BTCUSDT".to_string(),
            action: "hold".to_string(),
            confidence: 0.5,
            reason: "no parseable decisions".to_string(),
            size_usd: None,
        });
    }
    Ok(out)
}

fn build_trading_prompt(
    cfg: &BacktestConfig,
    bar: &KlineBar,
    equity: f64,
    cash: f64,
    positions: &HashMap<String, Vec<SimPosition>>,
    cycle: usize,
) -> String {
    let pos_summary: Vec<Value> = positions
        .iter()
        .flat_map(|(sym, poses)| {
            let sym = sym.clone(); // clone so inner closure can move it
            poses
                .iter()
                .map(move |p| {
                    json!({
                        "symbol": sym,
                        "side": p.side,
                        "qty": p.qty,
                        "entry_price": p.entry_price,
                        "leverage": p.leverage
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect();

    let symbols_str = cfg.symbols.join(", ");
    format!(
        r#"## Backtest Cycle {cycle}

**Account**: equity={equity:.2} USDT, cash={cash:.2} USDT
**Open positions**: {pos_summary}
**Latest bar** (symbol={symbols_str}): O={open} H={high} L={low} C={close} V={vol:.0}
**Prompt style**: {variant}
**Leverage**: BTC/ETH={btc_lev}x, Altcoins={alt_lev}x

Analyze the market and respond with a JSON array of trading decisions. Each element:
{{"symbol":"BTCUSDT","action":"open_long|open_short|close_long|close_short|hold","confidence":0.7,"reason":"...","size_usd":500}}

Only include decisions you are confident about. Prefer HOLD when uncertain.
Respond with ONLY the JSON array, no markdown."#,
        cycle = cycle,
        equity = equity,
        cash = cash,
        pos_summary = serde_json::to_string(&pos_summary).unwrap_or_default(),
        symbols_str = symbols_str,
        open = bar.open,
        high = bar.high,
        low = bar.low,
        close = bar.close,
        vol = bar.volume,
        variant = cfg.prompt_variant,
        btc_lev = cfg.btc_eth_leverage,
        alt_lev = cfg.altcoin_leverage,
    )
}

fn resolve_leverage(cfg: &BacktestConfig, symbol: &str) -> i64 {
    let sym_upper = symbol.to_uppercase();
    if sym_upper.starts_with("BTC") || sym_upper.starts_with("ETH") {
        cfg.btc_eth_leverage.max(1)
    } else {
        cfg.altcoin_leverage.max(1)
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
        .update_run_status(run_id, status, last_error, summary, now_ts())
        .await
}

async fn append_equity_point(
    backtest_repo: &BacktestRepo,
    run_id: &str,
    ts: i64,
    equity: f64,
    available: f64,
    pnl: f64,
    dd_pct: f64,
    cycle: usize,
) -> Result<(), crate::database::DbErr> {
    backtest_repo
        .insert_equity_point(BacktestEquityPointRecord {
            run_id: run_id.to_string(),
            ts,
            equity,
            available,
            pnl,
            dd_pct,
            cycle,
        })
        .await
}

#[allow(clippy::too_many_arguments)]
async fn append_trade(
    backtest_repo: &BacktestRepo,
    id: &str,
    run_id: &str,
    ts: i64,
    symbol: &str,
    action: &str,
    side: &str,
    qty: f64,
    price: f64,
    fee: f64,
    realized_pnl: f64,
    leverage: i64,
    cycle: usize,
    liquidation: bool,
) -> Result<(), crate::database::DbErr> {
    backtest_repo
        .insert_trade(BacktestTradeRecord {
            id: id.to_string(),
            run_id: run_id.to_string(),
            ts,
            symbol: symbol.to_string(),
            action: action.to_string(),
            side: side.to_string(),
            qty,
            price,
            fee,
            realized_pnl,
            leverage,
            cycle,
            liquidation,
        })
        .await
}

async fn append_decision(
    backtest_repo: &BacktestRepo,
    id: &str,
    run_id: &str,
    ts: i64,
    symbol: &str,
    action: &str,
    confidence: f64,
    reason: &str,
    cycle: usize,
) -> Result<(), crate::database::DbErr> {
    backtest_repo
        .insert_decision(BacktestDecisionRecord {
            id: id.to_string(),
            run_id: run_id.to_string(),
            ts,
            symbol: symbol.to_string(),
            action: action.to_string(),
            confidence,
            reason: reason.to_string(),
            cycle,
        })
        .await
}

fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// ===== Kline fetching =====

async fn fetch_klines_from_binance(
    symbol: &str,
    interval: &str,
    start_ts_ms: i64,
    end_ts_ms: i64,
) -> Vec<KlineBar> {
    // Binance futures klines endpoint supports startTime/endTime
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
    let rows: Vec<Vec<Value>> = match serde_json::from_str(&resp.body) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("parse klines error: {e}");
            return vec![];
        }
    };
    rows.into_iter()
        .filter_map(|r| {
            if r.len() < 6 {
                return None;
            }
            Some(KlineBar {
                open_time: r[0].as_i64()?,
                open: r[1].as_str()?.parse().ok()?,
                high: r[2].as_str()?.parse().ok()?,
                low: r[3].as_str()?.parse().ok()?,
                close: r[4].as_str()?.parse().ok()?,
                volume: r[5].as_str()?.parse().ok()?,
            })
        })
        .collect()
}

async fn load_all_klines(cfg: &BacktestConfig) -> Vec<KlineBar> {
    let symbol = cfg
        .symbols
        .first()
        .cloned()
        .unwrap_or_else(|| "BTCUSDT".to_string());
    let start_ms = cfg.start_ts * 1000;
    let end_ms = cfg.end_ts * 1000;

    let mut all: Vec<KlineBar> = Vec::new();
    let mut cursor = start_ms;
    let batch_limit = 1500i64;
    // Approximate ms per bar
    let ms_per_bar = interval_to_ms(&cfg.interval).unwrap_or(300_000);
    let batch_end_ms = cursor + batch_limit * ms_per_bar;

    while cursor < end_ms {
        let batch_to = batch_end_ms.min(end_ms);
        let bars = fetch_klines_from_binance(&symbol, &cfg.interval, cursor, batch_to).await;
        if bars.is_empty() {
            break;
        }
        cursor = bars.last().map(|b| b.open_time + 1).unwrap_or(batch_to + 1);
        all.extend(bars);
        if cursor >= end_ms {
            break;
        }
        // Small sleep to avoid rate limits
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
    all.sort_by_key(|b| b.open_time);
    all
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

// ===== Manager =====

/// Entry tracking an in-flight backtest run.
struct RunEntry {
    stop_tx: oneshot::Sender<()>,
}

/// Global manager for active backtest runs.
struct BacktestManager {
    runs: HashMap<String, RunEntry>,
}

impl BacktestManager {
    fn new() -> Self {
        Self {
            runs: HashMap::new(),
        }
    }
}

type SharedBacktestManager = Arc<Mutex<BacktestManager>>;

/// Global singleton.
static BT_MANAGER: OnceLock<SharedBacktestManager> = OnceLock::new();

fn get_backtest_manager() -> SharedBacktestManager {
    BT_MANAGER
        .get_or_init(|| Arc::new(Mutex::new(BacktestManager::new())))
        .clone()
}

/// Start a new backtest run. Creates the DB row, spawns the background task.
/// Returns the run_id on success.
async fn start_backtest(
    cfg: BacktestConfig,
    backtest_repo: Arc<BacktestRepo>,
    llm_service: Arc<LlmService>,
    llm_model: ResolvedModelRecord,
    realtime_hub: RealtimeHub,
) -> Result<String, String> {
    let run_id = cfg.run_id.clone();

    // Persist initial run row
    let config_json = serde_json::to_string(&cfg).unwrap_or_default();
    let now = now_ts();
    backtest_repo
        .create_run(CreateBacktestRunRecord {
            run_id: run_id.clone(),
            config_json,
            created_at: now,
            updated_at: now,
        })
        .await
        .map_err(|e| e.to_string())?;

    // Fetch historical klines (may take time)
    let klines = load_all_klines(&cfg).await;
    if klines.is_empty() {
        let _ = write_run_status(
            &backtest_repo,
            &run_id,
            "failed",
            "No kline data available",
            &RunMetrics::default(),
        )
        .await;
        return Err("No kline data available for the requested period".into());
    }

    let initial = cfg.initial_balance;
    let fee_bps = cfg.fee_bps;
    let slippage_bps = cfg.slippage_bps;
    let mut metrics = RunMetrics::default();
    metrics.initial_balance = initial;
    metrics.max_equity = initial;
    metrics.final_equity = initial;

    let runner = BacktestRunner {
        cfg,
        account: SimAccount::new(initial, fee_bps, slippage_bps),
        klines,
        bar_index: 0,
        decision_cycle: 0,
        llm_service,
        llm_model,
        status: RunStatus::Running,
        last_error: String::new(),
        metrics_cache: metrics,
    };

    let (stop_tx, stop_rx) = oneshot::channel::<()>();

    // Register in manager
    {
        let mgr = get_backtest_manager();
        let mut guard = mgr.lock().unwrap();
        guard
            .runs
            .insert(run_id.clone(), RunEntry { stop_tx });
    }

    // Spawn the run loop
    tokio::spawn(runner.run(backtest_repo, stop_rx, realtime_hub));

    Ok(run_id)
}

/// Stop a running backtest by sending stop signal.
fn stop_backtest(run_id: &str) -> Result<(), String> {
    let mgr = get_backtest_manager();
    let mut guard = mgr.lock().unwrap();
    if let Some(entry) = guard.runs.remove(run_id) {
        // Sending on the channel signals the runner to stop
        let _ = entry.stop_tx.send(());
        Ok(())
    } else {
        Err("run not found or already finished".into())
    }
}

fn required_run_id(run_id: Option<String>) -> AppResult<String> {
    match run_id {
        Some(id) if !id.trim().is_empty() => Ok(id),
        _ => Err(AppError::BadRequest("run_id is required".into())),
    }
}

// ===== Internal query helpers =====

async fn query_run_status(
    backtest_repo: &BacktestRepo,
    run_id: &str,
) -> Option<Value> {
    let row = backtest_repo
        .get_run(run_id)
        .await
        .ok()
        .flatten()?;

    let mgr = get_backtest_manager();
    let is_active = mgr
        .lock()
        .ok()
        .map(|g| g.runs.contains_key(run_id))
        .unwrap_or(false);

    let config: Value = serde_json::from_str(row.config_json.as_str()).unwrap_or(json!({}));
    let summary: Value = serde_json::from_str(row.summary_json.as_str()).unwrap_or(json!({}));
    let effective_state = if is_active && row.state == "running" {
        "running".to_string()
    } else {
        row.state
    };

    Some(json!({
        "run_id": row.run_id,
        "label": row.label,
        "state": effective_state,
        "last_error": row.last_error,
        "config": config,
        "summary": summary,
        "created_at": row.created_at,
        "updated_at": row.updated_at,
    }))
}

async fn list_runs(backtest_repo: &BacktestRepo, limit: i64) -> Vec<Value> {
    backtest_repo
        .list_runs(limit)
        .await
        .unwrap_or_default()
}

async fn query_equity_points(
    backtest_repo: &BacktestRepo,
    run_id: &str,
    limit: i64,
) -> Vec<Value> {
    backtest_repo
        .list_equity_points(run_id, limit)
        .await
        .unwrap_or_default()
}

async fn query_trades(
    backtest_repo: &BacktestRepo,
    run_id: &str,
    limit: i64,
) -> Vec<Value> {
    backtest_repo
        .list_trades(run_id, limit)
        .await
        .unwrap_or_default()
}

async fn query_decisions(
    backtest_repo: &BacktestRepo,
    run_id: &str,
    limit: i64,
) -> Vec<Value> {
    backtest_repo
        .list_decisions(run_id, limit)
        .await
        .unwrap_or_default()
}

async fn compute_metrics(backtest_repo: &BacktestRepo, run_id: &str) -> Value {
    let Some(row) = backtest_repo.get_run(run_id).await.ok().flatten() else {
        return json!({"error": "run not found"});
    };

    let summary: Value = serde_json::from_str(row.summary_json.as_str()).unwrap_or(json!({}));
    let config: Value = serde_json::from_str(row.config_json.as_str()).unwrap_or(json!({}));
    let initial = config
        .get("initial_balance")
        .and_then(Value::as_f64)
        .unwrap_or(1000.0);
    let final_eq = summary
        .get("final_equity")
        .and_then(Value::as_f64)
        .unwrap_or(initial);
    let total_pnl = final_eq - initial;
    let total_pnl_pct = if initial > 0.0 {
        total_pnl / initial * 100.0
    } else {
        0.0
    };
    let total_trades = summary
        .get("total_trades")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let winning_trades = summary
        .get("winning_trades")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let win_rate = if total_trades > 0 {
        winning_trades as f64 / total_trades as f64 * 100.0
    } else {
        0.0
    };
    let max_dd = summary
        .get("max_drawdown_pct")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);

    json!({
        "run_id": run_id,
        "state": row.state,
        "initial_balance": initial,
        "final_equity": final_eq,
        "total_pnl": total_pnl,
        "total_pnl_pct": total_pnl_pct,
        "total_trades": total_trades,
        "winning_trades": winning_trades,
        "win_rate_pct": win_rate,
        "max_drawdown_pct": max_dd,
    })
}
