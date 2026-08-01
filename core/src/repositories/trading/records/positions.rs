#[derive(Debug, Clone)]
pub struct TraderPositionRecord {
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
}

#[derive(Debug, Clone)]
pub struct InsertTraderPositionRecord {
    pub id: String,
    pub trader_id: String,
    pub symbol: String,
    pub side: String,
    pub quantity: f64,
    pub entry_price: f64,
    pub mark_price: f64,
    pub liquidation_price: f64,
    pub leverage: i64,
    pub margin_mode: String,
    pub unrealized_pnl: f64,
    pub realized_pnl: f64,
    pub status: String,
    pub opened_at: i64,
    pub closed_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct UpsertPositionFromExchangeRecord {
    pub trader_id: String,
    pub symbol: String,
    pub side: String,
    pub quantity: f64,
    pub entry_price: f64,
    pub mark_price: f64,
    pub liquidation_price: f64,
    pub leverage: i64,
    pub unrealized_pnl: f64,
    pub event_at: i64,
    pub updated_at: i64,
}
