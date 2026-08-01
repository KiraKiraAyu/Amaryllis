#[derive(Debug, Clone)]
pub struct InsertTraderDecisionRecord {
    pub id: String,
    pub trader_id: String,
    pub symbol: String,
    pub timeframe: String,
    pub decision: String,
    pub confidence: f64,
    pub reason: String,
    pub payload_json: String,
    pub created_at: i64,
}

#[derive(Debug, Clone)]
pub struct TraderDecisionRecord {
    pub id: String,
    pub symbol: String,
    pub timeframe: String,
    pub decision: String,
    pub confidence: f64,
    pub reason: String,
    pub payload_json: String,
    pub created_at: i64,
}

#[derive(Debug, Clone)]
pub struct InsertTraderTradeRecord {
    pub id: String,
    pub trader_id: String,
    pub symbol: String,
    pub side: String,
    pub entry_price: f64,
    pub exit_price: f64,
    pub quantity: f64,
    pub realized_pnl: f64,
    pub fees: f64,
    pub roi_pct: f64,
    pub opened_at: i64,
    pub closed_at: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone)]
pub struct TraderTradeRecord {
    pub id: String,
    pub symbol: String,
    pub side: String,
    pub entry_price: f64,
    pub exit_price: f64,
    pub quantity: f64,
    pub realized_pnl: f64,
    pub fees: f64,
    pub roi_pct: f64,
    pub opened_at: i64,
    pub closed_at: i64,
}

#[derive(Debug, Clone, Default)]
pub struct TraderStatisticsRecord {
    pub total_trades: i64,
    pub winning_trades: i64,
    pub total_realized_pnl: f64,
    pub total_fees: f64,
    pub avg_roi_pct: f64,
    pub open_positions: i64,
}
