use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct BinanceUserStreamEnvelope {
    #[serde(rename = "e", default)]
    pub event_type: String,
    #[serde(default)]
    #[serde(rename = "E")]
    pub event_time: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BinanceOrderTradeUpdateEvent {
    #[serde(rename = "e", default)]
    pub event_type: String,
    #[serde(rename = "E", default)]
    pub event_time: i64,
    #[serde(rename = "o")]
    pub order: BinanceOrderPayload,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BinanceOrderPayload {
    #[serde(rename = "s", default)]
    pub symbol: String,
    #[serde(rename = "S", default)]
    pub side: String,
    #[serde(rename = "o", default)]
    pub order_type: String,
    #[serde(rename = "X", default)]
    pub order_status: String,
    #[serde(rename = "x", default)]
    pub execution_type: String,
    #[serde(rename = "i", default)]
    pub order_id: i64,
    #[serde(rename = "t", default)]
    pub trade_id: i64,
    #[serde(rename = "c", default)]
    pub client_order_id: String,
    #[serde(rename = "q", default)]
    pub orig_qty: String,
    #[serde(rename = "z", default)]
    pub cum_qty: String,
    #[serde(rename = "L", default)]
    pub last_fill_price: String,
    #[serde(rename = "l", default)]
    pub last_fill_qty: String,
    #[serde(rename = "n", default)]
    pub fee: String,
    #[serde(rename = "N", default)]
    pub fee_asset: String,
    #[serde(rename = "rp", default)]
    pub realized_pnl: String,
    #[serde(rename = "R", default)]
    pub reduce_only: bool,
    #[serde(rename = "T", default)]
    pub trade_time: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BinanceAccountUpdateEvent {
    #[serde(rename = "e", default)]
    pub event_type: String,
    #[serde(rename = "E", default)]
    pub event_time: i64,
    #[serde(rename = "a")]
    pub account: BinanceAccountPayload,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BinanceAccountPayload {
    #[serde(rename = "B", default)]
    pub balances: Vec<BinanceBalancePayload>,
    #[serde(rename = "P", default)]
    pub positions: Vec<BinancePositionPayload>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BinanceBalancePayload {
    #[serde(rename = "a", default)]
    pub asset: String,
    #[serde(rename = "wb", default)]
    pub wallet_balance: String,
    #[serde(rename = "cw", default)]
    pub cross_wallet_balance: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BinancePositionPayload {
    #[serde(rename = "s", default)]
    pub symbol: String,
    #[serde(rename = "pa", default)]
    pub position_amt: String,
    #[serde(rename = "ep", default)]
    pub entry_price: String,
    #[serde(rename = "up", default)]
    pub unrealized_pnl: String,
    #[serde(rename = "ps", default)]
    pub position_side: String,
}

#[derive(Debug, Clone)]
pub enum BinanceUserStreamEvent {
    OrderTradeUpdate(Box<BinanceOrderTradeUpdateEvent>),
    AccountUpdate(BinanceAccountUpdateEvent),
    ListenKeyExpired { event_time: i64 },
    Unknown,
}

pub fn parse_binance_user_stream_event(text: &str) -> BinanceUserStreamEvent {
    let envelope = match serde_json::from_str::<BinanceUserStreamEnvelope>(text) {
        Ok(v) => v,
        Err(_) => return BinanceUserStreamEvent::Unknown,
    };

    match envelope.event_type.as_str() {
        "ORDER_TRADE_UPDATE" => match serde_json::from_str::<BinanceOrderTradeUpdateEvent>(text) {
            Ok(v) => BinanceUserStreamEvent::OrderTradeUpdate(Box::new(v)),
            Err(_) => BinanceUserStreamEvent::Unknown,
        },
        "ACCOUNT_UPDATE" => match serde_json::from_str::<BinanceAccountUpdateEvent>(text) {
            Ok(v) => BinanceUserStreamEvent::AccountUpdate(v),
            Err(_) => BinanceUserStreamEvent::Unknown,
        },
        "listenKeyExpired" => BinanceUserStreamEvent::ListenKeyExpired {
            event_time: envelope.event_time,
        },
        _ => BinanceUserStreamEvent::Unknown,
    }
}
