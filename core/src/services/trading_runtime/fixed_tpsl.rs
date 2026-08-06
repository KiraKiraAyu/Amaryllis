use crate::clients::exchanges::{
    ExchangeConditionalOrderType, ExchangeOpenOrder, ExchangePosition, ExchangeSide,
    ExchangeSymbolConstraints, LiveExchangeAdapter, PlaceConditionalOrderRequest, PositionSide,
};
use crate::error::AppError;
use crate::services::trading_runtime::{
    config_loaders::margin_mode_for_config, models::TraderRuntimeConfig,
};
use tracing::warn;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedTpSlRates {
    pub take_profit_rate: f64,
    pub stop_loss_rate: f64,
}

pub fn configured_fixed_tp_sl_rates(
    strategy_config: &serde_json::Value,
) -> Result<Option<FixedTpSlRates>, AppError> {
    let Some(tp_sl) = strategy_config.get("tp_sl") else {
        return Ok(None);
    };
    let Some(mode) = tp_sl.get("mode").and_then(|value| value.as_str()) else {
        return Err(AppError::InvalidExchangeConfig(
            "tp_sl.mode is required".to_string(),
        ));
    };
    if !mode.eq_ignore_ascii_case("fixed") {
        return Ok(None);
    }

    let Some(take_profit_rate) = tp_sl
        .get("fixed_tp_pnl_rate")
        .and_then(|value| value.as_f64())
    else {
        return Err(AppError::InvalidExchangeConfig(
            "fixed_tp_pnl_rate must be provided by the form".to_string(),
        ));
    };
    let Some(stop_loss_rate) = tp_sl
        .get("fixed_sl_pnl_rate")
        .and_then(|value| value.as_f64())
    else {
        return Err(AppError::InvalidExchangeConfig(
            "fixed_sl_pnl_rate must be provided by the form".to_string(),
        ));
    };
    let rates = FixedTpSlRates {
        take_profit_rate,
        stop_loss_rate,
    };
    if !rates.take_profit_rate.is_finite()
        || !rates.stop_loss_rate.is_finite()
        || rates.take_profit_rate <= 0.0
        || rates.stop_loss_rate >= 0.0
    {
        return Err(AppError::InvalidExchangeConfig(
            "fixed TP/SL requires a positive take-profit rate and negative stop-loss rate"
                .to_string(),
        ));
    }
    Ok(Some(rates))
}

pub fn build_fixed_tp_sl_orders(
    position: &ExchangePosition,
    constraints: &ExchangeSymbolConstraints,
    rates: FixedTpSlRates,
) -> Result<[PlaceConditionalOrderRequest; 2], AppError> {
    if position.entry_price <= 0.0 || !position.entry_price.is_finite() {
        return Err(AppError::InvalidExchangeConfig(
            "fixed TP/SL requires a positive entry price".to_string(),
        ));
    }
    if position.leverage < 1 {
        return Err(AppError::InvalidExchangeConfig(
            "fixed TP/SL requires leverage >= 1".to_string(),
        ));
    }
    if !rates.take_profit_rate.is_finite()
        || !rates.stop_loss_rate.is_finite()
        || rates.take_profit_rate <= 0.0
        || rates.stop_loss_rate >= 0.0
    {
        return Err(AppError::InvalidExchangeConfig(
            "fixed TP/SL requires a positive take-profit rate and negative stop-loss rate"
                .to_string(),
        ));
    }

    let quantity = normalize_quantity(position.quantity.abs(), constraints);
    if quantity <= f64::EPSILON {
        return Err(AppError::InvalidExchangeConfig(
            "fixed TP/SL position quantity is below the exchange minimum".to_string(),
        ));
    }

    let leverage = position.leverage as f64;
    let (side, position_side, take_profit_raw, stop_loss_raw) = match position_side(position) {
        PositionSide::Long => (
            ExchangeSide::Sell,
            PositionSide::Long,
            position.entry_price * (1.0 + rates.take_profit_rate / leverage),
            position.entry_price * (1.0 + rates.stop_loss_rate / leverage),
        ),
        PositionSide::Short => (
            ExchangeSide::Buy,
            PositionSide::Short,
            position.entry_price * (1.0 - rates.take_profit_rate / leverage),
            position.entry_price * (1.0 - rates.stop_loss_rate / leverage),
        ),
        PositionSide::Both => {
            return Err(AppError::InvalidExchangeConfig(
                "fixed TP/SL requires a long or short position side".to_string(),
            ));
        }
    };
    let take_profit_trigger = normalize_trigger_price(take_profit_raw, constraints.tick_size);
    let stop_loss_trigger = normalize_trigger_price(stop_loss_raw, constraints.tick_size);
    if take_profit_trigger <= 0.0 || stop_loss_trigger <= 0.0 {
        return Err(AppError::InvalidExchangeConfig(
            "fixed TP/SL trigger price is invalid for exchange tick size".to_string(),
        ));
    }

    Ok([
        PlaceConditionalOrderRequest {
            symbol: position.symbol.trim().to_uppercase(),
            side,
            conditional_type: ExchangeConditionalOrderType::TakeProfitMarket,
            quantity,
            trigger_price: take_profit_trigger,
            reduce_only: true,
            margin_mode: None,
            position_side: Some(position_side),
            client_order_id: Some(crate::clients::short_client_order_id("tpsl_tp_")),
        },
        PlaceConditionalOrderRequest {
            symbol: position.symbol.trim().to_uppercase(),
            side,
            conditional_type: ExchangeConditionalOrderType::StopMarket,
            quantity,
            trigger_price: stop_loss_trigger,
            reduce_only: true,
            margin_mode: None,
            position_side: Some(position_side),
            client_order_id: Some(crate::clients::short_client_order_id("tpsl_sl_")),
        },
    ])
}

fn position_side(position: &ExchangePosition) -> PositionSide {
    match position.position_side.trim().to_ascii_uppercase().as_str() {
        "LONG" => PositionSide::Long,
        "SHORT" => PositionSide::Short,
        _ if position.quantity < 0.0 => PositionSide::Short,
        _ => PositionSide::Long,
    }
}

fn normalize_quantity(quantity: f64, constraints: &ExchangeSymbolConstraints) -> f64 {
    let step_size = constraints.step_size.max(0.0);
    let normalized = if step_size > f64::EPSILON {
        (quantity / step_size).floor() * step_size
    } else {
        quantity
    };
    if normalized + f64::EPSILON < constraints.min_qty.max(0.0) {
        0.0
    } else {
        normalized
    }
}

fn normalize_trigger_price(price: f64, tick_size: f64) -> f64 {
    if !price.is_finite() || price <= 0.0 {
        return 0.0;
    }
    if tick_size > f64::EPSILON {
        (price / tick_size).floor() * tick_size
    } else {
        price
    }
}

pub async fn ensure_fixed_tp_sl_orders(
    adapter: &dyn LiveExchangeAdapter,
    cfg: &TraderRuntimeConfig,
) -> Result<(), AppError> {
    let Some(rates) = configured_fixed_tp_sl_rates(&cfg.strategy_config)? else {
        return Ok(());
    };
    if adapter.exchange_type() != "aster" {
        return Ok(());
    }

    let positions = adapter.get_positions().await?;
    let open_orders = adapter.get_open_orders(None).await?;

    // Remove protections left behind after a position was closed. This also clears
    // the sibling order after either the take-profit or stop-loss has fired.
    for order in open_orders
        .iter()
        .filter(|order| is_managed_protection(order))
    {
        if !positions
            .iter()
            .any(|position| order_matches_position(order, position))
        {
            adapter.cancel_order(&order.symbol, &order.order_id).await?;
        }
    }

    for position in &positions {
        let constraints = adapter.get_symbol_constraints(&position.symbol).await?;
        let mut requests = build_fixed_tp_sl_orders(position, &constraints, rates)?;
        for request in &mut requests {
            request.margin_mode = Some(margin_mode_for_config(cfg));
        }

        let mut created_order_ids = Vec::new();
        for request in requests {
            if open_orders.iter().any(|order| {
                is_managed_protection(order)
                    && order_matches_position(order, position)
                    && order_type_matches(order, request.conditional_type)
            }) {
                continue;
            }

            match adapter.place_conditional_order(request.clone()).await {
                Ok(response) => created_order_ids.push((request.symbol.clone(), response.order_id)),
                Err(err) => {
                    for (symbol, order_id) in created_order_ids {
                        if let Err(cancel_err) = adapter.cancel_order(&symbol, &order_id).await {
                            warn!(
                                "failed to roll back fixed TP/SL order symbol={} order_id={} err={}",
                                symbol, order_id, cancel_err
                            );
                        }
                    }
                    return Err(AppError::BadGateway(format!(
                        "failed to install fixed TP/SL protection for {}: {err}",
                        position.symbol
                    )));
                }
            }
        }
    }

    Ok(())
}

fn is_managed_protection(order: &ExchangeOpenOrder) -> bool {
    let client_id = order.client_order_id.trim().to_ascii_lowercase();
    client_id.starts_with("tpsl_tp_") || client_id.starts_with("tpsl_sl_")
}

fn order_type_matches(
    order: &ExchangeOpenOrder,
    conditional_type: ExchangeConditionalOrderType,
) -> bool {
    let order_type = order.order_type.trim().to_ascii_uppercase();
    match conditional_type {
        ExchangeConditionalOrderType::TakeProfitMarket => order_type == "TAKE_PROFIT_MARKET",
        ExchangeConditionalOrderType::StopMarket => order_type == "STOP_MARKET",
    }
}

fn order_matches_position(order: &ExchangeOpenOrder, position: &ExchangePosition) -> bool {
    if !order.symbol.trim().eq_ignore_ascii_case(&position.symbol) {
        return false;
    }
    let expected_side = position_side(position);
    match order.position_side.trim().to_ascii_uppercase().as_str() {
        "LONG" => expected_side == PositionSide::Long,
        "SHORT" => expected_side == PositionSide::Short,
        _ => match order.side.trim().to_ascii_uppercase().as_str() {
            "SELL" => expected_side == PositionSide::Long,
            "BUY" => expected_side == PositionSide::Short,
            _ => false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn position(side: &str) -> ExchangePosition {
        ExchangePosition {
            symbol: "BTCUSDT".to_string(),
            position_side: side.to_string(),
            quantity: if side == "LONG" { 0.01 } else { -0.01 },
            entry_price: 100.0,
            mark_price: 100.0,
            unrealized_pnl: 0.0,
            leverage: 10,
            liquidation_price: 0.0,
        }
    }

    fn constraints() -> ExchangeSymbolConstraints {
        ExchangeSymbolConstraints {
            symbol: "BTCUSDT".to_string(),
            base_asset: "BTC".to_string(),
            quote_asset: "USDT".to_string(),
            min_qty: 0.001,
            max_qty: 100.0,
            step_size: 0.001,
            min_notional: 5.0,
            tick_size: 0.01,
        }
    }

    #[test]
    fn missing_tp_sl_config_does_not_create_fixed_protection() {
        assert_eq!(
            configured_fixed_tp_sl_rates(&serde_json::json!({})).expect("rates"),
            None
        );
        assert!(
            configured_fixed_tp_sl_rates(&serde_json::json!({
                "tp_sl": { "mode": "fixed" }
            }))
            .is_err()
        );
    }

    #[test]
    fn fixed_tp_sl_does_not_default_null_or_empty_form_values() {
        for value in [serde_json::Value::Null, serde_json::json!("")] {
            let config = serde_json::json!({
                "tp_sl": {
                    "mode": "fixed",
                    "fixed_tp_pnl_rate": value,
                    "fixed_sl_pnl_rate": -0.1
                }
            });
            assert!(configured_fixed_tp_sl_rates(&config).is_err());
        }

        let config = serde_json::json!({
            "tp_sl": {
                "mode": "fixed",
                "fixed_tp_pnl_rate": 0.2,
                "fixed_sl_pnl_rate": null
            }
        });
        assert!(configured_fixed_tp_sl_rates(&config).is_err());
    }

    #[test]
    fn long_fixed_tp_sl_uses_entry_price_and_leverage_for_two_trigger_orders() {
        let orders = build_fixed_tp_sl_orders(
            &position("LONG"),
            &constraints(),
            FixedTpSlRates {
                take_profit_rate: 0.2,
                stop_loss_rate: -0.1,
            },
        )
        .expect("orders");

        assert_eq!(
            orders[0].conditional_type,
            ExchangeConditionalOrderType::TakeProfitMarket
        );
        assert_eq!(orders[0].side, ExchangeSide::Sell);
        assert_eq!(orders[0].position_side, Some(PositionSide::Long));
        assert_eq!(orders[0].trigger_price, 102.0);
        assert_eq!(
            orders[1].conditional_type,
            ExchangeConditionalOrderType::StopMarket
        );
        assert_eq!(orders[1].trigger_price, 99.0);
        assert!(
            orders
                .iter()
                .all(|order| order.reduce_only && order.quantity == 0.01)
        );
    }

    #[test]
    fn short_fixed_tp_sl_inverts_trigger_direction() {
        let orders = build_fixed_tp_sl_orders(
            &position("SHORT"),
            &constraints(),
            FixedTpSlRates {
                take_profit_rate: 0.2,
                stop_loss_rate: -0.1,
            },
        )
        .expect("orders");

        assert_eq!(orders[0].side, ExchangeSide::Buy);
        assert_eq!(orders[0].position_side, Some(PositionSide::Short));
        assert_eq!(orders[0].trigger_price, 98.0);
        assert_eq!(orders[1].trigger_price, 101.0);
    }
}
