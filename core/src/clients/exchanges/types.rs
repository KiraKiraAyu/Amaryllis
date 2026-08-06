use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct ExchangeCredentials {
    pub api_key: String,
    pub secret_key: String,
    pub passphrase: Option<String>,
    pub wallet_addr: Option<String>,
    pub testnet: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ExchangeSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ExchangeOrderType {
    Market,
    Limit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExchangeConditionalOrderType {
    TakeProfitMarket,
    StopMarket,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum PositionSide {
    Both,
    Long,
    Short,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TimeInForce {
    Gtc,
    Ioc,
    Fok,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExchangeMarginMode {
    Cross,
    Isolated,
}

#[derive(Debug, Clone)]
pub struct PlaceOrderRequest {
    pub symbol: String,
    pub side: ExchangeSide,
    pub order_type: ExchangeOrderType,
    pub quantity: f64,
    pub price: Option<f64>,
    pub reduce_only: bool,
    pub margin_mode: Option<ExchangeMarginMode>,
    pub position_side: Option<PositionSide>,
    pub time_in_force: Option<TimeInForce>,
    pub client_order_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PlaceConditionalOrderRequest {
    pub symbol: String,
    pub side: ExchangeSide,
    pub conditional_type: ExchangeConditionalOrderType,
    pub quantity: f64,
    pub trigger_price: f64,
    pub reduce_only: bool,
    pub margin_mode: Option<ExchangeMarginMode>,
    pub position_side: Option<PositionSide>,
    pub client_order_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaceOrderResponse {
    pub order_id: String,
    pub client_order_id: String,
    pub symbol: String,
    pub side: String,
    pub position_side: String,
    pub reduce_only: bool,
    pub status: String,
    pub order_type: String,
    pub price: f64,
    pub orig_qty: f64,
    pub executed_qty: f64,
    pub update_time: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelOrderResponse {
    pub order_id: String,
    pub client_order_id: String,
    pub symbol: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeBalance {
    pub asset: String,
    pub wallet_balance: f64,
    pub available_balance: f64,
    pub unrealized_pnl: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangePosition {
    pub symbol: String,
    pub position_side: String,
    pub quantity: f64,
    pub entry_price: f64,
    pub mark_price: f64,
    pub unrealized_pnl: f64,
    pub leverage: i64,
    pub liquidation_price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeOpenOrder {
    pub order_id: String,
    pub client_order_id: String,
    pub symbol: String,
    pub side: String,
    pub position_side: String,
    pub reduce_only: bool,
    pub order_type: String,
    pub status: String,
    pub price: f64,
    pub orig_qty: f64,
    pub executed_qty: f64,
    pub update_time: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeOrderDetail {
    pub order_id: String,
    pub client_order_id: String,
    pub symbol: String,
    pub side: String,
    pub position_side: String,
    pub reduce_only: bool,
    pub order_type: String,
    pub status: String,
    pub price: f64,
    pub orig_qty: f64,
    pub executed_qty: f64,
    pub update_time: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeTradeFill {
    pub trade_id: String,
    pub order_id: String,
    pub symbol: String,
    pub side: String,
    pub price: f64,
    pub quantity: f64,
    pub fee: f64,
    pub fee_asset: String,
    pub realized_pnl: f64,
    pub executed_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeSymbolConstraints {
    pub symbol: String,
    pub base_asset: String,
    pub quote_asset: String,
    pub min_qty: f64,
    pub max_qty: f64,
    pub step_size: f64,
    pub min_notional: f64,
    pub tick_size: f64,
}
