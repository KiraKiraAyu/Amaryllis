#[derive(Debug, Clone)]
pub struct InsertTraderOrderRecord {
    pub id: String,
    pub trader_id: String,
    pub exchange_order_id: String,
    pub client_order_id: String,
    pub symbol: String,
    pub side: String,
    pub position_side: String,
    pub order_type: String,
    pub status: String,
    pub price: f64,
    pub quantity: f64,
    pub filled_quantity: f64,
    pub avg_fill_price: f64,
    pub reduce_only: bool,
    pub time_in_force: String,
    pub placed_at: i64,
    pub updated_at: i64,
    pub closed_at: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct UpdateTraderOrderRecord {
    pub id: String,
    pub client_order_id: String,
    pub symbol: String,
    pub side: String,
    pub position_side: String,
    pub order_type: String,
    pub status: String,
    pub price: f64,
    pub quantity: f64,
    pub filled_quantity: f64,
    pub avg_fill_price: f64,
    pub reduce_only: bool,
    pub updated_at: i64,
    pub closed_at: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct TraderOrderRecord {
    pub id: String,
    pub exchange_order_id: String,
    pub client_order_id: String,
    pub symbol: String,
    pub side: String,
    pub position_side: String,
    pub order_type: String,
    pub status: String,
    pub price: f64,
    pub quantity: f64,
    pub filled_quantity: f64,
    pub avg_fill_price: f64,
    pub reduce_only: bool,
    pub time_in_force: String,
    pub placed_at: i64,
    pub updated_at: i64,
    pub closed_at: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct InsertOrderFillRecord {
    pub id: String,
    pub order_id: String,
    pub trader_id: String,
    pub exchange_trade_id: String,
    pub symbol: String,
    pub side: String,
    pub price: f64,
    pub quantity: f64,
    pub fee: f64,
    pub fee_asset: String,
    pub realized_pnl: f64,
    pub executed_at: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone)]
pub struct OrderFillRecord {
    pub id: String,
    pub exchange_trade_id: String,
    pub symbol: String,
    pub side: String,
    pub price: f64,
    pub quantity: f64,
    pub fee: f64,
    pub fee_asset: String,
    pub realized_pnl: f64,
    pub executed_at: i64,
}
