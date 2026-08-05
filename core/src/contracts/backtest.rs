use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use crate::contracts::public::KlinePayload;

#[derive(Debug, Deserialize)]
pub struct BacktestStartRequest {
    pub run_id: Option<String>,
    pub symbols: Option<Vec<String>>,
    pub start_ts: Option<i64>,
    pub end_ts: Option<i64>,
    pub initial_balance: Option<f64>,
    pub fee_bps: Option<f64>,
    pub slippage_bps: Option<f64>,
    pub ai_model_id: Option<String>,
    pub prompt_variant: Option<String>,
    pub leverage: Option<i64>,
    pub interval: Option<String>,
    pub decision_every: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct BacktestRunIdRequest {
    pub run_id: String,
}

#[derive(Debug, Deserialize)]
pub struct BacktestLabelRequest {
    pub run_id: String,
    pub label: String,
}

#[derive(Debug, Deserialize)]
pub struct BacktestQueryParams {
    pub run_id: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct KlinesQuery {
    pub symbol: String,
    pub interval: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BacktestRunActionPayload {
    pub run_id: String,
    pub message: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct BacktestMessagePayload {
    pub message: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct BacktestStatusPayload {
    pub status: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct BacktestRunsPayload {
    pub runs: Vec<Value>,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct BacktestEquityPayload {
    pub points: Vec<Value>,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct BacktestTradesPayload {
    pub trades: Vec<Value>,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct BacktestTracePayload {
    pub trace: Vec<Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BacktestDecisionsPayload {
    pub decisions: Vec<Value>,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct BacktestMetricsPayload {
    pub metrics: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct BacktestExportPayload {
    pub run_id: String,
    pub trades: Vec<Value>,
    pub equity: Vec<Value>,
    pub exported_at: i64,
}
