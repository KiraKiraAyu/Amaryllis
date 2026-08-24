use super::service::*;
use crate::clients::exchanges::user_stream::{
    binance_account_update, binance_order_update, binance_user_stream_event_to_exchange_events,
};
use crate::repositories::trading::records::{
    accounts::TraderAccountRecord,
    orders::{InsertOrderFillRecord, InsertTraderOrderRecord, UpdateTraderOrderRecord},
    positions::UpsertPositionFromExchangeRecord,
};
use crate::services::trading::account::{TpSlRates, position_payload};

pub async fn recv_user_stream_event(
    rx: &mut Option<mpsc::Receiver<ExchangeUserStreamEvent>>,
) -> Option<ExchangeUserStreamEvent> {
    match rx {
        Some(ch) => ch.recv().await,
        None => std::future::pending::<Option<ExchangeUserStreamEvent>>().await,
    }
}

pub async fn handle_exchange_user_stream_event(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    event: ExchangeUserStreamEvent,
    ts: i64,
) -> Result<(), AppError> {
    match event {
        ExchangeUserStreamEvent::OrderUpdate(ev) => {
            apply_order_stream_update_event(state, cfg, &ev, ts).await?;
        }
        ExchangeUserStreamEvent::AccountUpdate(ev) => {
            apply_account_stream_update_event(state, cfg, &ev, ts).await?;
        }
        ExchangeUserStreamEvent::ListenKeyExpired {
            exchange_type,
            event_time,
        } => {
            let _ = state.set_runtime_engine_running(
                &cfg.trader_id,
                true,
                Some(format!(
                    "{exchange_type} user stream expired at {event_time}"
                )),
            );
            warn!(
                "exchange user stream expired trader={} exchange={} event_time={}",
                cfg.trader_id, exchange_type, event_time
            );
        }
        ExchangeUserStreamEvent::Unknown => {}
    }
    Ok(())
}

pub async fn handle_binance_user_stream_event(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    event: BinanceUserStreamEvent,
    ts: i64,
) -> Result<(), AppError> {
    for event in binance_user_stream_event_to_exchange_events(event) {
        handle_exchange_user_stream_event(state, cfg, event, ts).await?;
    }
    Ok(())
}

pub async fn apply_order_trade_update_event(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    ev: &BinanceOrderTradeUpdateEvent,
    ts: i64,
) -> Result<(), AppError> {
    if ev.event_type != "ORDER_TRADE_UPDATE" {
        return Ok(());
    }
    let ExchangeUserStreamEvent::OrderUpdate(update) = binance_order_update(ev.clone()) else {
        return Ok(());
    };
    apply_order_stream_update_event(state, cfg, &update, ts).await
}

pub async fn apply_order_stream_update_event(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    ev: &ExchangeOrderStreamUpdate,
    ts: i64,
) -> Result<(), AppError> {
    let symbol = ev.symbol.trim().to_uppercase();
    let exchange_order_id = ev.order_id.trim().to_string();
    if symbol.is_empty() || exchange_order_id.trim().is_empty() {
        return Ok(());
    }

    let normalized_status = normalize_order_status(&ev.status);
    let is_terminal = is_terminal_order_status(&normalized_status);
    let event_ts = if ev.event_time > 0 { ev.event_time } else { ts };

    if is_terminal {
        finalize_execution_intent_for_exchange_order(
            state,
            cfg,
            &exchange_order_id,
            &normalized_status,
            event_ts,
        )
        .await?;
    }

    let existing_order = state
        .trading_repo
        .order_by_exchange_order_id(&cfg.trader_id, &exchange_order_id)
        .await?;

    let local_order_id = if let Some(existing) = existing_order {
        let effective_status =
            resolve_order_status_transition(&existing.status, &normalized_status);
        let preserved_closed_at = if existing.closed_at.is_some() {
            existing.closed_at
        } else if is_terminal_order_status(&effective_status) {
            Some(event_ts)
        } else {
            None
        };

        let existing_id = existing.id.clone();
        state
            .trading_repo
            .update_order(UpdateTraderOrderRecord {
                id: existing.id,
                client_order_id: ev.client_order_id.clone(),
                symbol: symbol.clone(),
                side: ev.side.trim().to_uppercase(),
                position_side: if ev.position_side.trim().is_empty() {
                    existing.position_side.clone()
                } else {
                    ev.position_side.trim().to_uppercase()
                },
                order_type: ev.order_type.trim().to_ascii_lowercase(),
                status: effective_status,
                price: existing.price,
                quantity: ev.orig_qty,
                filled_quantity: ev.filled_qty,
                avg_fill_price: ev.last_fill_price,
                reduce_only: ev.reduce_only,
                updated_at: event_ts,
                closed_at: preserved_closed_at,
            })
            .await?;
        existing_id
    } else {
        let id = Uuid::now_v7().to_string();
        state
            .trading_repo
            .insert_order(InsertTraderOrderRecord {
                id: id.clone(),
                trader_id: cfg.trader_id.clone(),
                exchange_order_id: exchange_order_id.clone(),
                client_order_id: ev.client_order_id.clone(),
                symbol: symbol.clone(),
                side: ev.side.trim().to_uppercase(),
                position_side: ev.position_side.trim().to_uppercase(),
                order_type: ev.order_type.trim().to_ascii_lowercase(),
                status: normalized_status.clone(),
                price: ev.last_fill_price,
                quantity: ev.orig_qty,
                filled_quantity: ev.filled_qty,
                avg_fill_price: ev.last_fill_price,
                reduce_only: ev.reduce_only,
                time_in_force: String::new(),
                placed_at: event_ts,
                updated_at: event_ts,
                closed_at: if is_terminal { Some(event_ts) } else { None },
            })
            .await?;
        id
    };

    let execution_type = ev.execution_type.trim().to_ascii_uppercase();
    let last_fill_qty = ev.last_fill_qty;
    let last_fill_price = ev.last_fill_price;
    if execution_type == "TRADE" && last_fill_qty > 0.0 && last_fill_price > 0.0 {
        let fee = ev.fee;
        let fee_asset = if ev.fee_asset.trim().is_empty() {
            "USDT".to_string()
        } else {
            ev.fee_asset.trim().to_uppercase()
        };
        let trade_time = if ev.trade_time > 0 {
            ev.trade_time
        } else if ev.event_time > 0 {
            ev.event_time
        } else {
            ts
        };

        let exchange_trade_id = if let Some(trade_id) = ev.trade_id.as_deref() {
            format!("ws-{}-{}", exchange_order_id, trade_id)
        } else {
            format!("ws-{}-{}", exchange_order_id, trade_time)
        };
        let existed = state
            .trading_repo
            .order_fill_exists(&cfg.trader_id, &exchange_trade_id)
            .await?;

        let mut fill_inserted = false;
        if !existed {
            state
                .trading_repo
                .insert_order_fill(InsertOrderFillRecord {
                    id: Uuid::now_v7().to_string(),
                    order_id: local_order_id.clone(),
                    trader_id: cfg.trader_id.clone(),
                    exchange_trade_id,
                    symbol: symbol.clone(),
                    side: ev.side.trim().to_uppercase(),
                    price: last_fill_price,
                    quantity: last_fill_qty,
                    fee,
                    fee_asset,
                    realized_pnl: ev.realized_pnl,
                    executed_at: trade_time,
                    created_at: ts,
                })
                .await?;
            fill_inserted = true;
        }

        if fill_inserted && ev.reduce_only {
            apply_reduce_only_fill_to_local_positions(
                state,
                cfg,
                &symbol,
                &ev.side,
                last_fill_qty,
                last_fill_price,
                fee,
                if trade_time > 0 { trade_time } else { ts },
                ts,
            )
            .await?;
        }

        // Push trade execution event to realtime clients in real-time
        state
            .realtime_hub
            .publish(crate::realtime::RealtimeEvent::TradeExecution {
                trader_id: cfg.trader_id.clone(),
                trade: json!({
                    "symbol": symbol,
                    "side": ev.side.trim().to_uppercase(),
                    "price": last_fill_price,
                    "qty": last_fill_qty,
                    "fee": fee,
                    "realized_pnl": ev.realized_pnl,
                    "reduce_only": ev.reduce_only,
                    "order_id": local_order_id,
                    "ts": trade_time,
                }),
            });
    }

    Ok(())
}

pub async fn apply_reduce_only_fill_to_local_positions(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    symbol: &str,
    order_side: &str,
    fill_qty: f64,
    fill_price: f64,
    fill_fee: f64,
    trade_time: i64,
    ts: i64,
) -> Result<(), AppError> {
    let position_side = match order_side.trim().to_ascii_uppercase().as_str() {
        "SELL" => "LONG",
        "BUY" => "SHORT",
        _ => return Ok(()),
    };

    let _ = apply_close_fill_to_open_positions(
        state,
        cfg,
        symbol,
        position_side,
        fill_qty,
        fill_price,
        fill_fee,
        trade_time,
        ts,
    )
    .await?;

    Ok(())
}

pub async fn apply_account_update_event(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    ev: &BinanceAccountUpdateEvent,
    ts: i64,
) -> Result<(), AppError> {
    if ev.event_type != "ACCOUNT_UPDATE" {
        return Ok(());
    }
    let ExchangeUserStreamEvent::AccountUpdate(update) = binance_account_update(ev.clone()) else {
        return Ok(());
    };
    apply_account_stream_update_event(state, cfg, &update, ts).await
}

pub async fn apply_account_stream_update_event(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    ev: &ExchangeAccountStreamUpdate,
    ts: i64,
) -> Result<(), AppError> {
    let event_ts = if ev.event_time > 0 { ev.event_time } else { ts };

    for b in &ev.balances {
        let total_balance = b.wallet_balance.max(0.0);
        let available_balance = b.available_balance.max(0.0);
        let used_margin = (total_balance - available_balance).max(0.0);

        state
            .trading_repo
            .insert_account_snapshot(
                Uuid::now_v7().to_string(),
                &cfg.trader_id,
                &cfg.exchange_id,
                &TraderAccountRecord {
                    trader_id: cfg.trader_id.clone(),
                    total_balance,
                    available_balance,
                    used_margin,
                    unrealized_pnl: b.unrealized_pnl,
                    realized_pnl: 0.0,
                    currency: b.asset.trim().to_uppercase(),
                    snapshot_at: event_ts,
                },
                ts,
            )
            .await?;
    }

    for p in &ev.positions {
        let symbol = p.symbol.trim().to_uppercase();
        if symbol.is_empty() {
            continue;
        }

        let qty = p.quantity.abs();
        let side = normalize_position_side(
            &p.position_side,
            if p.position_side == "SHORT" {
                -qty
            } else {
                qty
            },
        );

        if qty <= f64::EPSILON {
            state
                .trading_repo
                .close_open_positions_for_symbol_side(&cfg.trader_id, &symbol, side, event_ts, ts)
                .await?;
            continue;
        }

        state
            .trading_repo
            .upsert_open_position_from_exchange(UpsertPositionFromExchangeRecord {
                trader_id: cfg.trader_id.clone(),
                symbol: symbol.clone(),
                side: side.to_string(),
                quantity: qty,
                entry_price: p.entry_price,
                mark_price: p.mark_price,
                liquidation_price: p.liquidation_price,
                leverage: p.leverage.max(1),
                unrealized_pnl: p.unrealized_pnl,
                event_at: event_ts,
                updated_at: ts,
            })
            .await?;
    }

    // Push all current open positions to realtime clients after Binance account update
    let open_positions_ws = state
        .trading_repo
        .open_position_records(&cfg.trader_id, None, None)
        .await
        .unwrap_or_default();

    let tp_sl_rates = TpSlRates::from_config(&cfg.strategy_config);

    let positions_snapshot = open_positions_ws
        .into_iter()
        .map(|p| position_payload(p, tp_sl_rates))
        .collect();

    state
        .realtime_hub
        .publish(crate::realtime::RealtimeEvent::PositionUpdate {
            trader_id: cfg.trader_id.clone(),
            positions: positions_snapshot,
        });

    Ok(())
}
