use super::service::*;

pub async fn decisions(
    app: &SharedState,
    trader_id: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
    symbol: Option<String>,
) -> AppResult<DecisionListPayload> {
    let trader_id = match resolve_trader_id(app, trader_id).await {
        Ok(v) => v,
        Err(e) => return Err(e),
    };

    let limit = limit.unwrap_or(100).clamp(1, 500);
    let offset = offset.unwrap_or(0).max(0);
    let symbol_filter = symbol
        .clone()
        .map(|v| v.trim().to_uppercase())
        .filter(|v| !v.is_empty());

    match app
        .trading_repo
        .decisions(&trader_id, symbol_filter.as_deref(), limit, offset)
        .await
    {
        Ok(items) => {
            let items: Vec<DecisionPayload> = items.into_iter().map(decision_payload).collect();
            Ok(DecisionListPayload {
                trader_id,
                count: items.len(),
                items,
                limit,
                offset,
                symbol: symbol_filter,
            })
        }
        Err(_) => Err(app_error(
            AppErrorKind::Internal,
            "Failed to load decisions",
        )),
    }
}

pub async fn latest_decisions(
    app: &SharedState,
    trader_id: Option<String>,
) -> AppResult<LatestDecisionsPayload> {
    let trader_id = match resolve_trader_id(app, trader_id).await {
        Ok(v) => v,
        Err(e) => return Err(e),
    };

    match app.trading_repo.decisions(&trader_id, None, 20, 0).await {
        Ok(items) => {
            let items: Vec<DecisionPayload> = items.into_iter().map(decision_payload).collect();
            Ok(LatestDecisionsPayload {
                trader_id,
                count: items.len(),
                items,
            })
        }
        Err(_) => Err(app_error(
            AppErrorKind::Internal,
            "Failed to load latest decisions",
        )),
    }
}

pub async fn trades(
    app: &SharedState,
    trader_id: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> AppResult<TradeListPayload> {
    let trader_id = match resolve_trader_id(app, trader_id).await {
        Ok(v) => v,
        Err(e) => return Err(e),
    };
    let limit = limit.unwrap_or(100).clamp(1, 500);
    let offset = offset.unwrap_or(0).max(0);

    match app.trading_repo.trades(&trader_id, limit, offset).await {
        Ok(items) => {
            let items: Vec<TradePayload> = items.into_iter().map(trade_payload).collect();
            Ok(TradeListPayload {
                trader_id,
                count: items.len(),
                items,
                limit,
                offset,
            })
        }
        Err(_) => Err(app_error(AppErrorKind::Internal, "Failed to load trades")),
    }
}

pub async fn orders(
    app: &SharedState,
    trader_id: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> AppResult<OrderListPayload> {
    order_list(
        app,
        trader_id,
        limit,
        offset,
        false,
        true,
        true,
        "Failed to load orders",
    )
    .await
}

pub async fn order_fills(
    app: &SharedState,
    order_id: &str,
    trader_id: Option<String>,
) -> AppResult<FillListPayload> {
    let trader_id = match resolve_trader_id(app, trader_id).await {
        Ok(v) => v,
        Err(e) => return Err(e),
    };

    match app.trading_repo.order_fills(&trader_id, order_id).await {
        Ok(items) => {
            let items: Vec<FillPayload> = items.into_iter().map(fill_payload).collect();
            Ok(FillListPayload {
                trader_id,
                order_id: order_id.to_string(),
                count: items.len(),
                items,
            })
        }
        Err(_) => Err(app_error(
            AppErrorKind::Internal,
            "Failed to load order fills",
        )),
    }
}

pub async fn open_orders(
    app: &SharedState,
    trader_id: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> AppResult<OrderListPayload> {
    order_list(
        app,
        trader_id,
        limit,
        offset,
        true,
        false,
        false,
        "Failed to load open orders",
    )
    .await
}

async fn order_list(
    app: &SharedState,
    trader_id: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
    open_only: bool,
    include_avg_fill: bool,
    include_closed_at: bool,
    error_message: &str,
) -> AppResult<OrderListPayload> {
    let trader_id = match resolve_trader_id(app, trader_id).await {
        Ok(v) => v,
        Err(e) => return Err(e),
    };
    let limit = limit.unwrap_or(100).clamp(1, 500);
    let offset = offset.unwrap_or(0).max(0);

    match app
        .trading_repo
        .orders(&trader_id, open_only, limit, offset)
        .await
    {
        Ok(items) => {
            let items: Vec<OrderPayload> = items
                .into_iter()
                .map(|row| order_payload(row, include_avg_fill, include_closed_at))
                .collect();
            Ok(OrderListPayload {
                trader_id,
                count: items.len(),
                items,
                limit,
                offset,
            })
        }
        Err(_) => Err(app_error(AppErrorKind::Internal, error_message)),
    }
}

fn decision_payload(row: TraderDecisionRecord) -> DecisionPayload {
    DecisionPayload {
        id: row.id,
        symbol: row.symbol,
        timeframe: row.timeframe,
        decision: row.decision,
        confidence: row.confidence,
        reason: row.reason,
        payload_json: row.payload_json,
        created_at: row.created_at,
    }
}

fn trade_payload(row: TraderTradeRecord) -> TradePayload {
    TradePayload {
        id: row.id,
        symbol: row.symbol,
        side: row.side,
        entry_price: row.entry_price,
        exit_price: row.exit_price,
        quantity: row.quantity,
        realized_pnl: row.realized_pnl,
        fees: row.fees,
        roi_pct: row.roi_pct,
        opened_at: row.opened_at,
        closed_at: row.closed_at,
    }
}

fn order_payload(
    row: TraderOrderRecord,
    include_avg_fill: bool,
    include_closed_at: bool,
) -> OrderPayload {
    OrderPayload {
        id: row.id,
        exchange_order_id: row.exchange_order_id,
        client_order_id: row.client_order_id,
        symbol: row.symbol,
        side: row.side,
        position_side: row.position_side,
        order_type: row.order_type,
        status: row.status,
        price: row.price,
        quantity: row.quantity,
        filled_quantity: row.filled_quantity,
        avg_fill_price: if include_avg_fill {
            Some(row.avg_fill_price)
        } else {
            None
        },
        reduce_only: row.reduce_only,
        time_in_force: row.time_in_force,
        placed_at: row.placed_at,
        updated_at: row.updated_at,
        closed_at: if include_closed_at {
            row.closed_at
        } else {
            None
        },
    }
}

fn fill_payload(row: OrderFillRecord) -> FillPayload {
    FillPayload {
        id: row.id,
        exchange_trade_id: row.exchange_trade_id,
        symbol: row.symbol,
        side: row.side,
        price: row.price,
        quantity: row.quantity,
        fee: row.fee,
        fee_asset: row.fee_asset,
        realized_pnl: row.realized_pnl,
        executed_at: row.executed_at,
    }
}
