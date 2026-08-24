use std::{collections::HashMap, time::Duration};

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::{
    sync::{mpsc, watch},
    time::{self, MissedTickBehavior},
};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::warn;

use crate::clients::binance::{
    BinanceAccountUpdateEvent, BinanceOrderTradeUpdateEvent, BinanceUserStreamEvent,
    parse_binance_user_stream_event,
};

#[derive(Debug, Clone)]
pub struct ExchangeUserStreamSession {
    pub exchange_type: &'static str,
    pub ws_url: String,
    pub listen_key: Option<String>,
    pub initial_text_messages: Vec<String>,
    pub heartbeat_text: Option<String>,
    pub heartbeat_interval_secs: u64,
    pub quantity_multipliers: HashMap<String, f64>,
}

impl ExchangeUserStreamSession {
    pub fn listen_key(exchange_type: &'static str, ws_url: String, listen_key: String) -> Self {
        Self {
            exchange_type,
            ws_url,
            listen_key: Some(listen_key),
            initial_text_messages: Vec::new(),
            heartbeat_text: None,
            heartbeat_interval_secs: 0,
            quantity_multipliers: HashMap::new(),
        }
    }

    pub fn private_ws(exchange_type: &'static str, ws_url: String) -> Self {
        Self {
            exchange_type,
            ws_url,
            listen_key: None,
            initial_text_messages: Vec::new(),
            heartbeat_text: Some("ping".to_string()),
            heartbeat_interval_secs: 25,
            quantity_multipliers: HashMap::new(),
        }
    }

    pub fn with_initial_json(mut self, message: Value) -> Self {
        self.initial_text_messages.push(message.to_string());
        self
    }

    pub fn with_quantity_multipliers(mut self, multipliers: HashMap<String, f64>) -> Self {
        self.quantity_multipliers = multipliers;
        self
    }

    pub fn without_heartbeat(mut self) -> Self {
        self.heartbeat_text = None;
        self.heartbeat_interval_secs = 0;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExchangeOrderStreamUpdate {
    pub exchange_type: String,
    pub symbol: String,
    pub order_id: String,
    pub client_order_id: String,
    pub side: String,
    pub position_side: String,
    pub order_type: String,
    pub status: String,
    pub execution_type: String,
    pub trade_id: Option<String>,
    pub orig_qty: f64,
    pub filled_qty: f64,
    pub last_fill_price: f64,
    pub last_fill_qty: f64,
    pub fee: f64,
    pub fee_asset: String,
    pub realized_pnl: f64,
    pub reduce_only: bool,
    pub event_time: i64,
    pub trade_time: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExchangeAccountBalanceUpdate {
    pub asset: String,
    pub wallet_balance: f64,
    pub available_balance: f64,
    pub unrealized_pnl: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExchangeAccountPositionUpdate {
    pub symbol: String,
    pub position_side: String,
    pub quantity: f64,
    pub entry_price: f64,
    pub mark_price: f64,
    pub unrealized_pnl: f64,
    pub leverage: i64,
    pub liquidation_price: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExchangeAccountStreamUpdate {
    pub exchange_type: String,
    pub balances: Vec<ExchangeAccountBalanceUpdate>,
    pub positions: Vec<ExchangeAccountPositionUpdate>,
    pub event_time: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExchangeUserStreamEvent {
    OrderUpdate(Box<ExchangeOrderStreamUpdate>),
    AccountUpdate(ExchangeAccountStreamUpdate),
    ListenKeyExpired {
        exchange_type: String,
        event_time: i64,
    },
    Unknown,
}

pub fn spawn_exchange_user_stream_reader(
    session: ExchangeUserStreamSession,
    mut stop_rx: watch::Receiver<bool>,
) -> mpsc::Receiver<ExchangeUserStreamEvent> {
    let (tx, rx) = mpsc::channel(1024);

    tokio::spawn(async move {
        let connect = connect_async(&session.ws_url).await;
        let (mut ws_stream, _) = match connect {
            Ok(v) => v,
            Err(err) => {
                warn!("{} user stream connect failed: {}", session.exchange_type, err);
                let _ = tx.send(ExchangeUserStreamEvent::Unknown).await;
                return;
            }
        };

        for text in &session.initial_text_messages {
            if let Err(err) = ws_stream.send(Message::Text(text.clone())).await {
                warn!("{} user stream initial message failed: {}", session.exchange_type, err);
                let _ = tx.send(ExchangeUserStreamEvent::Unknown).await;
                return;
            }
        }

        let (mut write, mut read) = ws_stream.split();
        let heartbeat_enabled = session.heartbeat_text.is_some() && session.heartbeat_interval_secs > 0;
        let mut heartbeat = time::interval(Duration::from_secs(session.heartbeat_interval_secs.max(1)));
        heartbeat.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                changed = stop_rx.changed() => {
                    match changed {
                        Ok(_) => {
                            if *stop_rx.borrow() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
                _ = heartbeat.tick(), if heartbeat_enabled => {
                    if let Some(text) = session.heartbeat_text.as_deref()
                        && let Err(err) = write.send(Message::Text(text.to_string())).await
                    {
                        warn!("{} user stream heartbeat failed: {}", session.exchange_type, err);
                        break;
                    }
                }
                message = read.next() => {
                    match message {
                        Some(Ok(Message::Text(text))) => {
                            if is_heartbeat_reply(&text) {
                                continue;
                            }
                            for event in parse_exchange_user_stream_events(&session, &text) {
                                if tx.send(event).await.is_err() {
                                    return;
                                }
                            }
                        }
                        Some(Ok(Message::Ping(payload))) => {
                            if let Err(err) = write.send(Message::Pong(payload)).await {
                                warn!("{} user stream pong failed: {}", session.exchange_type, err);
                                break;
                            }
                        }
                        Some(Ok(Message::Pong(_))) => {}
                        Some(Ok(Message::Binary(_))) => {}
                        Some(Ok(Message::Frame(_))) => {}
                        Some(Ok(Message::Close(_))) => break,
                        Some(Err(err)) => {
                            warn!("{} user stream read error: {}", session.exchange_type, err);
                            break;
                        }
                        None => break,
                    }
                }
            }
        }
    });

    rx
}

pub fn parse_exchange_user_stream_events(
    session: &ExchangeUserStreamSession,
    text: &str,
) -> Vec<ExchangeUserStreamEvent> {
    match session.exchange_type {
        "binance" | "aster" => parse_binance_user_stream_events(text),
        "okx" => parse_okx_user_stream_events_with_multipliers(text, &session.quantity_multipliers),
        "bitget" => parse_bitget_user_stream_events(text),
        "hyperliquid" => parse_hyperliquid_user_stream_events(text),
        _ => Vec::new(),
    }
}

pub fn parse_binance_user_stream_events(text: &str) -> Vec<ExchangeUserStreamEvent> {
    binance_user_stream_event_to_exchange_events(parse_binance_user_stream_event(text))
}

pub fn binance_user_stream_event_to_exchange_events(
    event: BinanceUserStreamEvent,
) -> Vec<ExchangeUserStreamEvent> {
    match event {
        BinanceUserStreamEvent::OrderTradeUpdate(ev) => {
            vec![binance_order_update(*ev)]
        }
        BinanceUserStreamEvent::AccountUpdate(ev) => {
            vec![binance_account_update(ev)]
        }
        BinanceUserStreamEvent::ListenKeyExpired { event_time } => {
            vec![ExchangeUserStreamEvent::ListenKeyExpired {
                exchange_type: "binance".to_string(),
                event_time,
            }]
        }
        BinanceUserStreamEvent::Unknown => Vec::new(),
    }
}

pub fn parse_okx_user_stream_events(text: &str) -> Vec<ExchangeUserStreamEvent> {
    parse_okx_user_stream_events_with_multipliers(text, &HashMap::new())
}

pub fn parse_okx_user_stream_events_with_multipliers(
    text: &str,
    quantity_multipliers: &HashMap<String, f64>,
) -> Vec<ExchangeUserStreamEvent> {
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return Vec::new();
    };
    if value.get("event").and_then(Value::as_str).is_some() {
        return Vec::new();
    }

    let channel = value
        .pointer("/arg/channel")
        .and_then(Value::as_str)
        .unwrap_or("");
    let rows = value
        .get("data")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    match channel {
        "orders" => rows
            .iter()
            .map(|row| okx_order_update(row, quantity_multipliers))
            .collect(),
        "account" => okx_account_event(rows.iter().flat_map(okx_account_balances), Vec::new()),
        "positions" => okx_account_event(
            Vec::<ExchangeAccountBalanceUpdate>::new(),
            rows.iter()
                .filter_map(|row| okx_position_update(row, quantity_multipliers))
                .collect::<Vec<_>>(),
        ),
        "balance_and_position" => {
            let balances = rows
                .iter()
                .flat_map(|row| {
                    row.get("balData")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                        .flat_map(okx_account_balances)
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            let positions = rows
                .iter()
                .flat_map(|row| {
                    row.get("posData")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                        .filter_map(|pos| okx_position_update(pos, quantity_multipliers))
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            okx_account_event(balances, positions)
        }
        _ => Vec::new(),
    }
}

pub fn parse_bitget_user_stream_events(text: &str) -> Vec<ExchangeUserStreamEvent> {
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return Vec::new();
    };
    if value.get("event").and_then(Value::as_str).is_some() {
        return Vec::new();
    }

    let channel = value
        .pointer("/arg/channel")
        .and_then(Value::as_str)
        .unwrap_or("");
    let rows = value
        .get("data")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    match channel {
        "orders" => rows.iter().map(bitget_order_update).collect(),
        "account" => {
            bitget_account_event(rows.iter().filter_map(bitget_balance_update), Vec::new())
        }
        "positions" => bitget_account_event(
            Vec::<ExchangeAccountBalanceUpdate>::new(),
            rows.iter()
                .filter_map(bitget_position_update)
                .collect::<Vec<_>>(),
        ),
        _ => Vec::new(),
    }
}

pub fn parse_hyperliquid_user_stream_events(text: &str) -> Vec<ExchangeUserStreamEvent> {
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return Vec::new();
    };
    let channel = value.get("channel").and_then(Value::as_str).unwrap_or("");
    let data = value.get("data").unwrap_or(&Value::Null);

    match channel {
        "userEvents" => data
            .get("fills")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(hyperliquid_fill_update)
            .collect(),
        "orderUpdates" => data
            .as_array()
            .into_iter()
            .flatten()
            .map(hyperliquid_order_update)
            .collect(),
        "webData2" => hyperliquid_account_events(data),
        _ => Vec::new(),
    }
}

pub fn binance_order_update(ev: BinanceOrderTradeUpdateEvent) -> ExchangeUserStreamEvent {
    let event_time = positive_i64(ev.event_time, 0);
    let trade_time = positive_i64(ev.order.trade_time, event_time);
    let reduce_only = ev.order.reduce_only;
    ExchangeUserStreamEvent::OrderUpdate(Box::new(ExchangeOrderStreamUpdate {
        exchange_type: "binance".to_string(),
        symbol: ev.order.symbol.trim().to_uppercase(),
        order_id: ev.order.order_id.to_string(),
        client_order_id: ev.order.client_order_id,
        side: ev.order.side.trim().to_uppercase(),
        position_side: infer_position_side(&ev.order.side, reduce_only),
        order_type: ev.order.order_type.trim().to_uppercase(),
        status: ev.order.order_status.trim().to_uppercase(),
        execution_type: ev.order.execution_type.trim().to_uppercase(),
        trade_id: (ev.order.trade_id > 0).then(|| ev.order.trade_id.to_string()),
        orig_qty: parse_f64(&ev.order.orig_qty),
        filled_qty: parse_f64(&ev.order.cum_qty),
        last_fill_price: parse_f64(&ev.order.last_fill_price),
        last_fill_qty: parse_f64(&ev.order.last_fill_qty),
        fee: parse_f64(&ev.order.fee),
        fee_asset: default_asset(&ev.order.fee_asset, "USDT"),
        realized_pnl: parse_f64(&ev.order.realized_pnl),
        reduce_only,
        event_time,
        trade_time,
    }))
}

pub fn binance_account_update(ev: BinanceAccountUpdateEvent) -> ExchangeUserStreamEvent {
    let event_time = positive_i64(ev.event_time, 0);
    let balances = ev
        .account
        .balances
        .into_iter()
        .map(|b| ExchangeAccountBalanceUpdate {
            asset: b.asset.trim().to_uppercase(),
            wallet_balance: parse_f64(&b.wallet_balance),
            available_balance: parse_f64(&b.cross_wallet_balance),
            unrealized_pnl: 0.0,
        })
        .collect();
    let positions = ev
        .account
        .positions
        .into_iter()
        .map(|p| {
            let signed_qty = parse_f64(&p.position_amt);
            let quantity = signed_qty.abs();
            ExchangeAccountPositionUpdate {
                symbol: p.symbol.trim().to_uppercase(),
                position_side: normalize_position_side(&p.position_side, signed_qty),
                quantity,
                entry_price: parse_f64(&p.entry_price),
                mark_price: parse_f64(&p.entry_price),
                unrealized_pnl: parse_f64(&p.unrealized_pnl),
                leverage: 1,
                liquidation_price: 0.0,
            }
        })
        .collect();

    ExchangeUserStreamEvent::AccountUpdate(ExchangeAccountStreamUpdate {
        exchange_type: "binance".to_string(),
        balances,
        positions,
        event_time,
    })
}

fn okx_order_update(
    row: &Value,
    quantity_multipliers: &HashMap<String, f64>,
) -> ExchangeUserStreamEvent {
    let inst_id = str_value(row, "instId");
    let multiplier = quantity_multiplier(quantity_multipliers, &inst_id);
    let side = str_value(row, "side");
    let reduce_only = boolish_value(row, "reduceOnly");
    let fill_qty = f64_value(row, "fillSz") * multiplier;
    let event_time = i64_value(row, "uTime")
        .or_else(|| i64_value(row, "ts"))
        .unwrap_or(0);
    let trade_id = optional_string(row, "tradeId");

    ExchangeUserStreamEvent::OrderUpdate(Box::new(ExchangeOrderStreamUpdate {
        exchange_type: "okx".to_string(),
        symbol: okx_internal_symbol(&inst_id),
        order_id: str_value(row, "ordId"),
        client_order_id: str_value(row, "clOrdId"),
        side: side.to_ascii_uppercase(),
        position_side: explicit_or_inferred_position_side(
            &str_value(row, "posSide"),
            &side,
            reduce_only,
        ),
        order_type: str_value(row, "ordType").to_ascii_uppercase(),
        status: okx_status(&str_value(row, "state")),
        execution_type: if fill_qty > 0.0 || trade_id.is_some() {
            "TRADE".to_string()
        } else {
            str_value(row, "state").to_ascii_uppercase()
        },
        trade_id,
        orig_qty: f64_value(row, "sz") * multiplier,
        filled_qty: f64_value(row, "accFillSz") * multiplier,
        last_fill_price: f64_value(row, "fillPx"),
        last_fill_qty: fill_qty,
        fee: f64_value(row, "fee"),
        fee_asset: default_asset(&str_value(row, "feeCcy"), "USDT"),
        realized_pnl: f64_value(row, "pnl"),
        reduce_only,
        event_time,
        trade_time: event_time,
    }))
}

fn okx_account_balances(row: &Value) -> Vec<ExchangeAccountBalanceUpdate> {
    if let Some(details) = row.get("details").and_then(Value::as_array) {
        return details.iter().flat_map(okx_account_balances).collect();
    }

    let asset = str_value(row, "ccy");
    if asset.trim().is_empty() {
        return Vec::new();
    }

    let wallet_balance = first_present([
        number_value(row, "cashBal"),
        number_value(row, "eq"),
        number_value(row, "bal"),
    ])
    .unwrap_or(0.0);
    let available_balance = first_present([
        number_value(row, "availEq"),
        number_value(row, "availBal"),
        number_value(row, "cashBal"),
    ])
    .unwrap_or(0.0);

    vec![ExchangeAccountBalanceUpdate {
        asset: asset.to_ascii_uppercase(),
        wallet_balance,
        available_balance,
        unrealized_pnl: f64_value(row, "upl"),
    }]
}

fn okx_position_update(
    row: &Value,
    quantity_multipliers: &HashMap<String, f64>,
) -> Option<ExchangeAccountPositionUpdate> {
    let inst_id = str_value(row, "instId");
    if inst_id.trim().is_empty() {
        return None;
    }
    let multiplier = quantity_multiplier(quantity_multipliers, &inst_id);
    let signed_qty = f64_value(row, "pos") * multiplier;
    let quantity = signed_qty.abs();

    Some(ExchangeAccountPositionUpdate {
        symbol: okx_internal_symbol(&inst_id),
        position_side: explicit_or_quantity_position_side(&str_value(row, "posSide"), signed_qty),
        quantity,
        entry_price: f64_value(row, "avgPx"),
        mark_price: f64_value(row, "markPx"),
        unrealized_pnl: f64_value(row, "upl"),
        leverage: f64_value(row, "lever").round().max(1.0) as i64,
        liquidation_price: f64_value(row, "liqPx"),
    })
}

fn okx_account_event(
    balances: impl IntoIterator<Item = ExchangeAccountBalanceUpdate>,
    positions: impl IntoIterator<Item = ExchangeAccountPositionUpdate>,
) -> Vec<ExchangeUserStreamEvent> {
    let balances = balances.into_iter().collect::<Vec<_>>();
    let positions = positions.into_iter().collect::<Vec<_>>();
    if balances.is_empty() && positions.is_empty() {
        return Vec::new();
    }
    vec![ExchangeUserStreamEvent::AccountUpdate(
        ExchangeAccountStreamUpdate {
            exchange_type: "okx".to_string(),
            event_time: 0,
            balances,
            positions,
        },
    )]
}

fn bitget_order_update(row: &Value) -> ExchangeUserStreamEvent {
    let side = str_value(row, "side");
    let reduce_only = str_value(row, "tradeSide").eq_ignore_ascii_case("close")
        || boolish_value(row, "reduceOnly");
    let last_fill_qty = first_positive_or_any([
        number_value(row, "fillSize"),
        number_value(row, "baseVolume"),
        number_value(row, "accBaseVolume"),
    ])
    .unwrap_or(0.0);
    let event_time = i64_value(row, "uTime")
        .or_else(|| i64_value(row, "cTime"))
        .unwrap_or(0);
    let trade_id = optional_string(row, "tradeId");

    ExchangeUserStreamEvent::OrderUpdate(Box::new(ExchangeOrderStreamUpdate {
        exchange_type: "bitget".to_string(),
        symbol: bitget_internal_symbol(
            &str_value(row, "instId").or_else_nonempty(str_value(row, "symbol")),
        ),
        order_id: str_value(row, "orderId"),
        client_order_id: str_value(row, "clientOid"),
        side: side.to_ascii_uppercase(),
        position_side: infer_position_side(&side, reduce_only),
        order_type: str_value(row, "orderType").to_ascii_uppercase(),
        status: bitget_status(&str_value(row, "status")),
        execution_type: if last_fill_qty > 0.0 || trade_id.is_some() {
            "TRADE".to_string()
        } else {
            bitget_status(&str_value(row, "status"))
        },
        trade_id,
        orig_qty: f64_value(row, "size"),
        filled_qty: first_positive_or_any([
            number_value(row, "accBaseVolume"),
            number_value(row, "baseVolume"),
        ])
        .unwrap_or(0.0),
        last_fill_price: first_positive_or_any([
            number_value(row, "fillPrice"),
            number_value(row, "priceAvg"),
            number_value(row, "price"),
        ])
        .unwrap_or(0.0),
        last_fill_qty,
        fee: first_present([number_value(row, "fillFee"), number_value(row, "fee")]).unwrap_or(0.0),
        fee_asset: default_asset(
            &str_value(row, "feeCoin").or_else_nonempty(str_value(row, "feeCcy")),
            "USDT",
        ),
        realized_pnl: f64_value(row, "profit"),
        reduce_only,
        event_time,
        trade_time: event_time,
    }))
}

fn bitget_balance_update(row: &Value) -> Option<ExchangeAccountBalanceUpdate> {
    let asset = str_value(row, "marginCoin").or_else_nonempty(str_value(row, "coin"));
    if asset.trim().is_empty() {
        return None;
    }
    Some(ExchangeAccountBalanceUpdate {
        asset: asset.to_ascii_uppercase(),
        wallet_balance: f64_value(row, "accountEquity"),
        available_balance: f64_value(row, "available"),
        unrealized_pnl: f64_value(row, "unrealizedPL"),
    })
}

fn bitget_position_update(row: &Value) -> Option<ExchangeAccountPositionUpdate> {
    let symbol = str_value(row, "instId").or_else_nonempty(str_value(row, "symbol"));
    if symbol.trim().is_empty() {
        return None;
    }
    let quantity = f64_value(row, "total").abs();
    Some(ExchangeAccountPositionUpdate {
        symbol: bitget_internal_symbol(&symbol),
        position_side: normalize_position_side(&str_value(row, "holdSide"), 1.0),
        quantity,
        entry_price: f64_value(row, "openPriceAvg"),
        mark_price: f64_value(row, "markPrice"),
        unrealized_pnl: f64_value(row, "unrealizedPL"),
        leverage: f64_value(row, "leverage").round().max(1.0) as i64,
        liquidation_price: f64_value(row, "liquidationPrice"),
    })
}

fn bitget_account_event(
    balances: impl IntoIterator<Item = ExchangeAccountBalanceUpdate>,
    positions: impl IntoIterator<Item = ExchangeAccountPositionUpdate>,
) -> Vec<ExchangeUserStreamEvent> {
    let balances = balances.into_iter().collect::<Vec<_>>();
    let positions = positions.into_iter().collect::<Vec<_>>();
    if balances.is_empty() && positions.is_empty() {
        return Vec::new();
    }
    vec![ExchangeUserStreamEvent::AccountUpdate(
        ExchangeAccountStreamUpdate {
            exchange_type: "bitget".to_string(),
            balances,
            positions,
            event_time: 0,
        },
    )]
}

fn hyperliquid_fill_update(row: &Value) -> ExchangeUserStreamEvent {
    let side = hyperliquid_side(&str_value(row, "side"));
    let dir = str_value(row, "dir");
    let reduce_only = dir.to_ascii_lowercase().contains("close");
    let qty = f64_value(row, "sz");
    let event_time = i64_value(row, "time").unwrap_or(0);

    ExchangeUserStreamEvent::OrderUpdate(Box::new(ExchangeOrderStreamUpdate {
        exchange_type: "hyperliquid".to_string(),
        symbol: hyperliquid_internal_symbol(&str_value(row, "coin")),
        order_id: i64_or_string(row, "oid"),
        client_order_id: str_value(row, "cloid"),
        side: side.clone(),
        position_side: hyperliquid_position_side(&dir, &side, reduce_only),
        order_type: "MARKET".to_string(),
        status: "FILLED".to_string(),
        execution_type: "TRADE".to_string(),
        trade_id: optional_i64_or_string(row, "tid"),
        orig_qty: qty,
        filled_qty: qty,
        last_fill_price: f64_value(row, "px"),
        last_fill_qty: qty,
        fee: f64_value(row, "fee"),
        fee_asset: default_asset(&str_value(row, "feeToken"), "USDC"),
        realized_pnl: f64_value(row, "closedPnl"),
        reduce_only,
        event_time,
        trade_time: event_time,
    }))
}

fn hyperliquid_order_update(row: &Value) -> ExchangeUserStreamEvent {
    let status = str_value(row, "status");
    let order = row.get("order").unwrap_or(row);
    let side = hyperliquid_side(&str_value(order, "side"));
    let reduce_only = boolish_value(order, "reduceOnly");
    let qty = first_positive_or_any([number_value(order, "origSz"), number_value(order, "sz")])
        .unwrap_or(0.0);
    let remaining = f64_value(order, "sz");
    let event_time = i64_value(row, "statusTimestamp")
        .or_else(|| i64_value(order, "timestamp"))
        .unwrap_or(0);

    ExchangeUserStreamEvent::OrderUpdate(Box::new(ExchangeOrderStreamUpdate {
        exchange_type: "hyperliquid".to_string(),
        symbol: hyperliquid_internal_symbol(&str_value(order, "coin")),
        order_id: i64_or_string(order, "oid"),
        client_order_id: str_value(order, "cloid"),
        side: side.clone(),
        position_side: infer_position_side(&side, reduce_only),
        order_type: "LIMIT".to_string(),
        status: status.to_ascii_uppercase(),
        execution_type: status.to_ascii_uppercase(),
        trade_id: None,
        orig_qty: qty,
        filled_qty: (qty - remaining).max(0.0),
        last_fill_price: 0.0,
        last_fill_qty: 0.0,
        fee: 0.0,
        fee_asset: "USDC".to_string(),
        realized_pnl: 0.0,
        reduce_only,
        event_time,
        trade_time: event_time,
    }))
}

fn hyperliquid_account_events(data: &Value) -> Vec<ExchangeUserStreamEvent> {
    let state = data
        .get("clearinghouseState")
        .or_else(|| data.get("clearinghouse_state"))
        .unwrap_or(data);
    let margin = state.get("marginSummary").unwrap_or(&Value::Null);
    let positions = state
        .get("assetPositions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| {
            let position = row.get("position")?;
            let signed_qty = f64_value(position, "szi");
            let quantity = signed_qty.abs();
            if quantity <= f64::EPSILON {
                return None;
            }
            Some(ExchangeAccountPositionUpdate {
                symbol: hyperliquid_internal_symbol(&str_value(position, "coin")),
                position_side: if signed_qty < 0.0 { "SHORT" } else { "LONG" }.to_string(),
                quantity,
                entry_price: f64_value(position, "entryPx"),
                mark_price: f64_value(position, "markPx"),
                unrealized_pnl: f64_value(position, "unrealizedPnl"),
                leverage: position
                    .get("leverage")
                    .and_then(|v| v.get("value"))
                    .and_then(Value::as_i64)
                    .unwrap_or(1),
                liquidation_price: f64_value(position, "liquidationPx"),
            })
        })
        .collect::<Vec<_>>();
    let unrealized_pnl = positions.iter().map(|p| p.unrealized_pnl).sum::<f64>();
    let balances = vec![ExchangeAccountBalanceUpdate {
        asset: "USDC".to_string(),
        wallet_balance: f64_value(margin, "accountValue"),
        available_balance: f64_value(state, "withdrawable"),
        unrealized_pnl,
    }];

    vec![ExchangeUserStreamEvent::AccountUpdate(
        ExchangeAccountStreamUpdate {
            exchange_type: "hyperliquid".to_string(),
            balances,
            positions,
            event_time: 0,
        },
    )]
}

fn is_heartbeat_reply(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed.eq_ignore_ascii_case("pong") || trimmed.eq_ignore_ascii_case("ping")
}

fn str_value(row: &Value, key: &str) -> String {
    row.get(key)
        .and_then(|value| {
            value
                .as_str()
                .map(ToString::to_string)
                .or_else(|| value.as_i64().map(|v| v.to_string()))
                .or_else(|| value.as_u64().map(|v| v.to_string()))
                .or_else(|| value.as_f64().map(trim_float))
        })
        .unwrap_or_default()
}

trait NonEmptyString {
    fn or_else_nonempty(self, fallback: String) -> String;
}

impl NonEmptyString for String {
    fn or_else_nonempty(self, fallback: String) -> String {
        if self.trim().is_empty() {
            fallback
        } else {
            self
        }
    }
}

fn optional_string(row: &Value, key: &str) -> Option<String> {
    let value = str_value(row, key);
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

fn i64_or_string(row: &Value, key: &str) -> String {
    optional_i64_or_string(row, key).unwrap_or_default()
}

fn optional_i64_or_string(row: &Value, key: &str) -> Option<String> {
    row.get(key).and_then(|value| {
        value
            .as_i64()
            .map(|v| v.to_string())
            .or_else(|| value.as_u64().map(|v| v.to_string()))
            .or_else(|| value.as_str().map(ToString::to_string))
    })
}

fn number_value(row: &Value, key: &str) -> Option<f64> {
    row.get(key).and_then(|value| {
        value
            .as_f64()
            .or_else(|| value.as_i64().map(|v| v as f64))
            .or_else(|| value.as_u64().map(|v| v as f64))
            .or_else(|| value.as_str().and_then(|s| s.parse::<f64>().ok()))
    })
}

fn f64_value(row: &Value, key: &str) -> f64 {
    number_value(row, key).unwrap_or(0.0)
}

fn i64_value(row: &Value, key: &str) -> Option<i64> {
    row.get(key).and_then(|value| {
        value
            .as_i64()
            .or_else(|| value.as_u64().and_then(|v| i64::try_from(v).ok()))
            .or_else(|| value.as_str().and_then(|s| s.parse::<i64>().ok()))
    })
}

fn boolish_value(row: &Value, key: &str) -> bool {
    row.get(key)
        .and_then(|value| {
            value.as_bool().or_else(|| {
                value
                    .as_str()
                    .map(|s| matches!(s.trim().to_ascii_lowercase().as_str(), "true" | "yes" | "1"))
            })
        })
        .unwrap_or(false)
}

fn first_positive_or_any(values: impl IntoIterator<Item = Option<f64>>) -> Option<f64> {
    let mut first = None;
    for value in values.into_iter().flatten() {
        if first.is_none() {
            first = Some(value);
        }
        if value > 0.0 {
            return Some(value);
        }
    }
    first
}

fn first_present(values: impl IntoIterator<Item = Option<f64>>) -> Option<f64> {
    values.into_iter().flatten().next()
}

fn parse_f64(value: &str) -> f64 {
    value.parse::<f64>().unwrap_or(0.0)
}

fn positive_i64(value: i64, fallback: i64) -> i64 {
    if value > 0 { value } else { fallback }
}

fn default_asset(value: &str, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_ascii_uppercase()
    }
}

fn trim_float(value: f64) -> String {
    let mut out = format!("{value:.10}");
    while out.contains('.') && out.ends_with('0') {
        out.pop();
    }
    if out.ends_with('.') {
        out.pop();
    }
    out
}

fn normalize_position_side(position_side: &str, signed_qty: f64) -> String {
    match position_side.trim().to_ascii_uppercase().as_str() {
        "LONG" | "LONG_SIDE" | "BUY" => "LONG".to_string(),
        "SHORT" | "SHORT_SIDE" | "SELL" => "SHORT".to_string(),
        "LONG_SHORT_MODE" => "LONG".to_string(),
        "SHORT_MODE" => "SHORT".to_string(),
        _ if signed_qty < 0.0 => "SHORT".to_string(),
        _ => "LONG".to_string(),
    }
}

fn infer_position_side(side: &str, reduce_only: bool) -> String {
    match (side.trim().to_ascii_uppercase().as_str(), reduce_only) {
        ("SELL" | "A" | "ASK", false) | ("BUY" | "B" | "BID", true) => "SHORT".to_string(),
        _ => "LONG".to_string(),
    }
}

fn explicit_or_inferred_position_side(pos_side: &str, side: &str, reduce_only: bool) -> String {
    match pos_side.trim().to_ascii_uppercase().as_str() {
        "LONG" => "LONG".to_string(),
        "SHORT" => "SHORT".to_string(),
        "NET" | "" => infer_position_side(side, reduce_only),
        _ => normalize_position_side(pos_side, 1.0),
    }
}

fn explicit_or_quantity_position_side(pos_side: &str, signed_qty: f64) -> String {
    match pos_side.trim().to_ascii_uppercase().as_str() {
        "LONG" => "LONG".to_string(),
        "SHORT" => "SHORT".to_string(),
        _ => normalize_position_side(pos_side, signed_qty),
    }
}

fn okx_internal_symbol(inst_id: &str) -> String {
    let upper = inst_id.trim().to_ascii_uppercase();
    upper
        .strip_suffix("-SWAP")
        .unwrap_or(&upper)
        .replace('-', "")
}

fn bitget_internal_symbol(symbol: &str) -> String {
    symbol.trim().to_ascii_uppercase().replace('-', "")
}

fn hyperliquid_internal_symbol(coin: &str) -> String {
    let upper = coin.trim().to_ascii_uppercase();
    if upper.ends_with("USDT") || upper.ends_with("USDC") {
        upper
    } else {
        format!("{upper}USDT")
    }
}

fn hyperliquid_side(side: &str) -> String {
    match side.trim().to_ascii_uppercase().as_str() {
        "B" | "BUY" => "BUY".to_string(),
        "A" | "SELL" => "SELL".to_string(),
        other => other.to_string(),
    }
}

fn hyperliquid_position_side(dir: &str, side: &str, reduce_only: bool) -> String {
    let dir_lower = dir.trim().to_ascii_lowercase();
    if dir_lower.contains("long") {
        "LONG".to_string()
    } else if dir_lower.contains("short") {
        "SHORT".to_string()
    } else {
        infer_position_side(side, reduce_only)
    }
}

fn okx_status(state: &str) -> String {
    match state.trim().to_ascii_lowercase().as_str() {
        "live" => "NEW",
        "partially_filled" => "PARTIALLY_FILLED",
        "filled" => "FILLED",
        "canceled" | "cancelled" => "CANCELED",
        other => other,
    }
    .to_ascii_uppercase()
}

fn bitget_status(status: &str) -> String {
    match status.trim().to_ascii_lowercase().as_str() {
        "new" | "live" | "init" => "NEW",
        "partial-fill" | "partially_filled" | "partial_filled" => "PARTIALLY_FILLED",
        "filled" | "full-fill" => "FILLED",
        "canceled" | "cancelled" => "CANCELED",
        "rejected" => "REJECTED",
        "expired" => "EXPIRED",
        other => other,
    }
    .to_ascii_uppercase()
}

fn quantity_multiplier(multipliers: &HashMap<String, f64>, symbol: &str) -> f64 {
    let symbol_upper = symbol.trim().to_ascii_uppercase();
    multipliers
        .get(&symbol_upper)
        .copied()
        .filter(|v| *v > 0.0)
        .unwrap_or(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn okx_order_message_normalizes_to_exchange_order_update() {
        let events = parse_okx_user_stream_events(
            r#"{
              "arg": {"channel": "orders", "instType": "SWAP"},
              "data": [{
                "instId": "BTC-USDT-SWAP",
                "ordId": "12345",
                "clOrdId": "amx1",
                "side": "buy",
                "posSide": "long",
                "ordType": "market",
                "state": "filled",
                "sz": "2",
                "accFillSz": "2",
                "fillSz": "2",
                "fillPx": "65000",
                "tradeId": "789",
                "fee": "-0.5",
                "feeCcy": "USDT",
                "pnl": "12.3",
                "reduceOnly": "false",
                "uTime": "1700000000000"
              }]
            }"#,
        );

        assert_eq!(events.len(), 1);
        let ExchangeUserStreamEvent::OrderUpdate(update) = &events[0] else {
            panic!("expected order update");
        };
        assert_eq!(update.exchange_type, "okx");
        assert_eq!(update.symbol, "BTCUSDT");
        assert_eq!(update.order_id, "12345");
        assert_eq!(update.status, "FILLED");
        assert_eq!(update.execution_type, "TRADE");
        assert_eq!(update.position_side, "LONG");
        assert_eq!(update.last_fill_qty, 2.0);
        assert_eq!(update.last_fill_price, 65000.0);
        assert_eq!(update.trade_id.as_deref(), Some("789"));
    }

    #[test]
    fn bitget_order_message_normalizes_reduce_only_close() {
        let events = parse_bitget_user_stream_events(
            r#"{
              "action": "snapshot",
              "arg": {"instType": "USDT-FUTURES", "channel": "orders"},
              "data": [{
                "instId": "ETHUSDT",
                "orderId": "987",
                "clientOid": "amx2",
                "side": "sell",
                "tradeSide": "close",
                "orderType": "market",
                "status": "filled",
                "size": "1.5",
                "accBaseVolume": "1.5",
                "priceAvg": "2500",
                "fillFee": "-0.02",
                "feeCoin": "USDT",
                "profit": "5.5",
                "tradeId": "trade-1",
                "uTime": "1700000001000"
              }]
            }"#,
        );

        assert_eq!(events.len(), 1);
        let ExchangeUserStreamEvent::OrderUpdate(update) = &events[0] else {
            panic!("expected order update");
        };
        assert_eq!(update.exchange_type, "bitget");
        assert_eq!(update.symbol, "ETHUSDT");
        assert_eq!(update.order_id, "987");
        assert!(update.reduce_only);
        assert_eq!(update.position_side, "LONG");
        assert_eq!(update.execution_type, "TRADE");
        assert_eq!(update.last_fill_qty, 1.5);
    }

    #[test]
    fn hyperliquid_user_event_message_normalizes_fill() {
        let events = parse_hyperliquid_user_stream_events(
            r#"{
              "channel": "userEvents",
              "data": {
                "fills": [{
                  "coin": "SOL",
                  "oid": 555,
                  "cloid": "0x11111111111111111111111111111111",
                  "side": "B",
                  "px": "150.5",
                  "sz": "3",
                  "fee": "0.03",
                  "feeToken": "USDC",
                  "closedPnl": "1.2",
                  "tid": 777,
                  "time": 1700000002000
                }]
              }
            }"#,
        );

        assert_eq!(events.len(), 1);
        let ExchangeUserStreamEvent::OrderUpdate(update) = &events[0] else {
            panic!("expected order update");
        };
        assert_eq!(update.exchange_type, "hyperliquid");
        assert_eq!(update.symbol, "SOLUSDT");
        assert_eq!(update.order_id, "555");
        assert_eq!(update.status, "FILLED");
        assert_eq!(update.execution_type, "TRADE");
        assert_eq!(update.last_fill_qty, 3.0);
        assert_eq!(update.last_fill_price, 150.5);
        assert_eq!(update.trade_id.as_deref(), Some("777"));
    }
}
