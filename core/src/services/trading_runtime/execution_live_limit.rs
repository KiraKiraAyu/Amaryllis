use super::service::*;

pub async fn place_live_open_order_limit_first(
    adapter: &dyn LiveExchangeAdapter,
    symbol: &str,
    desired_side: &str,
    qty: f64,
    reference_price: f64,
    constraints: &ExchangeSymbolConstraints,
    margin_mode: ExchangeMarginMode,
) -> Result<crate::clients::exchanges::PlaceOrderResponse, AppError> {
    let open_side = if desired_side == "LONG" {
        ExchangeSide::Buy
    } else {
        ExchangeSide::Sell
    };

    let position_side = if desired_side == "LONG" {
        PositionSide::Long
    } else {
        PositionSide::Short
    };

    // Slightly aggressive limit price to maximize fill probability while still attempting limit-first.
    let raw_limit_price = if desired_side == "LONG" {
        reference_price * 1.0005
    } else {
        reference_price * 0.9995
    };
    let limit_price = normalize_order_price_by_constraints(raw_limit_price, constraints);

    if limit_price > 0.0 {
        let limit_try = adapter
            .place_order(PlaceOrderRequest {
                symbol: symbol.to_string(),
                side: open_side,
                order_type: ExchangeOrderType::Limit,
                quantity: qty,
                price: Some(limit_price),
                reduce_only: false,
                margin_mode: Some(margin_mode),
                position_side: Some(position_side),
                time_in_force: Some(TimeInForce::Fok),
                client_order_id: Some(crate::clients::short_client_order_id("nfx_lmt_")),
            })
            .await;

        match limit_try {
            Ok(resp) => {
                let status = normalize_order_status(&resp.status);
                if status == "filled" || resp.executed_qty >= qty * 0.999 {
                    return Ok(resp);
                }
                warn!(
                    "live limit-first not filled, fallback to market symbol={} status={} executed_qty={} qty={}",
                    symbol, status, resp.executed_qty, qty
                );
            }
            Err(err) => {
                warn!(
                    "live limit-first place failed, fallback to market symbol={} err={}",
                    symbol, err
                );
            }
        }
    }

    adapter
        .place_order(PlaceOrderRequest {
            symbol: symbol.to_string(),
            side: open_side,
            order_type: ExchangeOrderType::Market,
            quantity: qty,
            price: None,
            reduce_only: false,
            margin_mode: Some(margin_mode),
            position_side: Some(position_side),
            time_in_force: None,
            client_order_id: Some(crate::clients::short_client_order_id("nfx_mkt_")),
        })
        .await
        .map_err(AppError::from)
}
