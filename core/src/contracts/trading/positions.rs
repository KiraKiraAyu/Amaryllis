use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct PositionQuery {
    pub trader_id: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionPayload {
    pub id: String,
    pub trader_id: String,
    pub symbol: String,
    pub side: String,
    pub quantity: f64,
    pub entry_price: f64,
    pub mark_price: f64,
    pub liquidation_price: f64,
    pub leverage: i32,
    pub margin_mode: String,
    pub unrealized_pnl: f64,
    pub realized_pnl: f64,
    pub status: String,
    pub opened_at: i64,
    pub closed_at: Option<i64>,
    pub updated_at: i64,
    pub tp_price: Option<f64>,
    pub sl_price: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PositionListPayload {
    pub trader_id: String,
    pub items: Vec<PositionPayload>,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct StrategyPositionListPayload {
    pub strategy_id: String,
    pub items: Vec<PositionPayload>,
    pub count: usize,
}
