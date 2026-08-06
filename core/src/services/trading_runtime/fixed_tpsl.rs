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
pub struct FixedTakeProfit {
    pub pnl_rate: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedStopLoss {
    pub pnl_rate: f64,
}

#[derive(Clone, Copy)]
enum FixedRuleKind {
    TakeProfit,
    StopLoss,
}

impl FixedRuleKind {
    fn config_key(self) -> &'static str {
        match self {
            Self::TakeProfit => "take_profit",
            Self::StopLoss => "stop_loss",
        }
    }

    fn accepts_rate(self, rate: f64) -> bool {
        match self {
            Self::TakeProfit => rate > 0.0,
            Self::StopLoss => rate < 0.0,
        }
    }

    fn expected_rate_description(self) -> &'static str {
        match self {
            Self::TakeProfit => "positive",
            Self::StopLoss => "negative",
        }
    }
}

pub fn configured_fixed_take_profit(
    strategy_config: &serde_json::Value,
) -> Result<Option<FixedTakeProfit>, AppError> {
    let Some(tp_sl) = strategy_config.get("tp_sl") else {
        return Ok(None);
    };
    configured_rule_rate(tp_sl, FixedRuleKind::TakeProfit)
        .map(|rate| rate.map(|pnl_rate| FixedTakeProfit { pnl_rate }))
}

pub fn configured_fixed_stop_loss(
    strategy_config: &serde_json::Value,
) -> Result<Option<FixedStopLoss>, AppError> {
    let Some(tp_sl) = strategy_config.get("tp_sl") else {
        return Ok(None);
    };
    configured_rule_rate(tp_sl, FixedRuleKind::StopLoss)
        .map(|rate| rate.map(|pnl_rate| FixedStopLoss { pnl_rate }))
}

fn configured_rule_rate(
    tp_sl: &serde_json::Value,
    rule_kind: FixedRuleKind,
) -> Result<Option<f64>, AppError> {
    let rule_name = rule_kind.config_key();
    let rule = tp_sl.get(rule_name).ok_or_else(|| {
        AppError::InvalidExchangeConfig(format!(
            "tp_sl.{rule_name} configuration must be provided by the form"
        ))
    })?;
    let mode = rule
        .get("mode")
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            AppError::InvalidExchangeConfig(format!("tp_sl.{rule_name}.mode is required"))
        })?;

    if mode.eq_ignore_ascii_case("custom") {
        return Ok(None);
    }
    if !mode.eq_ignore_ascii_case("fixed") {
        return Err(AppError::InvalidExchangeConfig(format!(
            "tp_sl.{rule_name}.mode must be fixed or custom"
        )));
    }

    let rate = rule.get("pnl_rate").and_then(|value| value.as_f64()).ok_or_else(|| {
        AppError::InvalidExchangeConfig(format!(
            "tp_sl.{rule_name}.pnl_rate must be provided by the form"
        ))
    })?;
    if !rate.is_finite() || !rule_kind.accepts_rate(rate) {
        return Err(AppError::InvalidExchangeConfig(format!(
            "tp_sl.{rule_name}.pnl_rate must be {} for fixed mode",
            rule_kind.expected_rate_description()
        )));
    }
    Ok(Some(rate))
}

pub fn build_fixed_tp_sl_orders(
    position: &ExchangePosition,
    constraints: &ExchangeSymbolConstraints,
    take_profit: Option<FixedTakeProfit>,
    stop_loss: Option<FixedStopLoss>,
) -> Result<Vec<PlaceConditionalOrderRequest>, AppError> {
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
    if take_profit.is_none() && stop_loss.is_none() {
        return Err(AppError::InvalidExchangeConfig(
            "fixed TP/SL requires at least one fixed rule".to_string(),
        ));
    }

    let quantity = normalize_quantity(position.quantity.abs(), constraints);
    if quantity <= f64::EPSILON {
        return Err(AppError::InvalidExchangeConfig(
            "fixed TP/SL position quantity is below the exchange minimum".to_string(),
        ));
    }

    let leverage = position.leverage as f64;
    let (side, position_side) = match position_side(position) {
        PositionSide::Long => (ExchangeSide::Sell, PositionSide::Long),
        PositionSide::Short => (ExchangeSide::Buy, PositionSide::Short),
        PositionSide::Both => {
            return Err(AppError::InvalidExchangeConfig(
                "fixed TP/SL requires a long or short position side".to_string(),
            ));
        }
    };
    let mut orders = Vec::with_capacity(2);
    if let Some(take_profit_rate) = take_profit.map(|rule| rule.pnl_rate) {
        let take_profit_raw = match position_side {
            PositionSide::Long => position.entry_price * (1.0 + take_profit_rate / leverage),
            PositionSide::Short => position.entry_price * (1.0 - take_profit_rate / leverage),
            PositionSide::Both => unreachable!("position side validated above"),
        };
        let take_profit_trigger = normalize_trigger_price(take_profit_raw, constraints.tick_size);
        if take_profit_trigger <= 0.0 {
            return Err(AppError::InvalidExchangeConfig(
                "fixed TP/SL take-profit trigger price is invalid".to_string(),
            ));
        }
        orders.push(PlaceConditionalOrderRequest {
            symbol: position.symbol.trim().to_uppercase(),
            side,
            conditional_type: ExchangeConditionalOrderType::TakeProfitMarket,
            quantity,
            trigger_price: take_profit_trigger,
            reduce_only: true,
            margin_mode: None,
            position_side: Some(position_side),
            client_order_id: Some(crate::clients::short_client_order_id("tpsl_tp_")),
        });
    }
    if let Some(stop_loss_rate) = stop_loss.map(|rule| rule.pnl_rate) {
        let stop_loss_raw = match position_side {
            PositionSide::Long => position.entry_price * (1.0 + stop_loss_rate / leverage),
            PositionSide::Short => position.entry_price * (1.0 - stop_loss_rate / leverage),
            PositionSide::Both => unreachable!("position side validated above"),
        };
        let stop_loss_trigger = normalize_trigger_price(stop_loss_raw, constraints.tick_size);
        if stop_loss_trigger <= 0.0 {
            return Err(AppError::InvalidExchangeConfig(
                "fixed TP/SL stop-loss trigger price is invalid".to_string(),
            ));
        }
        orders.push(PlaceConditionalOrderRequest {
            symbol: position.symbol.trim().to_uppercase(),
            side,
            conditional_type: ExchangeConditionalOrderType::StopMarket,
            quantity,
            trigger_price: stop_loss_trigger,
            reduce_only: true,
            margin_mode: None,
            position_side: Some(position_side),
            client_order_id: Some(crate::clients::short_client_order_id("tpsl_sl_")),
        });
    }

    Ok(orders)
}

fn position_order_is_configured(
    order: &ExchangeOpenOrder,
    take_profit: Option<FixedTakeProfit>,
    stop_loss: Option<FixedStopLoss>,
) -> bool {
    match order.order_type.trim().to_ascii_uppercase().as_str() {
        "TAKE_PROFIT_MARKET" => take_profit.is_some(),
        "STOP_MARKET" => stop_loss.is_some(),
        _ => true,
    }
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
    let take_profit = configured_fixed_take_profit(&cfg.strategy_config)?;
    let stop_loss = configured_fixed_stop_loss(&cfg.strategy_config)?;
    if adapter.exchange_type() != "aster" {
        return Ok(());
    }

    let positions = adapter.get_positions().await?;
    let open_orders = adapter.get_open_orders(None).await?;

    // Remove protections left behind after a position was closed or after the
    // corresponding rule was switched to custom mode.
    for order in open_orders
        .iter()
        .filter(|order| is_managed_protection(order))
    {
        let matching_position = positions
            .iter()
            .find(|position| order_matches_position(order, position));
        if matching_position.is_none()
            || !position_order_is_configured(order, take_profit, stop_loss)
        {
            adapter.cancel_order(&order.symbol, &order.order_id).await?;
        }
    }

    if take_profit.is_none() && stop_loss.is_none() {
        return Ok(());
    }

    for position in &positions {
        let constraints = adapter.get_symbol_constraints(&position.symbol).await?;
        let mut requests =
            build_fixed_tp_sl_orders(position, &constraints, take_profit, stop_loss)?;
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
            configured_fixed_take_profit(&serde_json::json!({})).expect("take-profit"),
            None
        );
        assert_eq!(
            configured_fixed_stop_loss(&serde_json::json!({})).expect("stop-loss"),
            None
        );
        assert!(
            configured_fixed_take_profit(&serde_json::json!({
                "tp_sl": {
                    "take_profit": { "mode": "fixed" },
                    "stop_loss": { "mode": "custom", "custom_prompt": "use structure" }
                }
            }))
            .is_err()
        );
        assert!(
            configured_fixed_take_profit(&serde_json::json!({
                "tp_sl": {
                    "mode": "fixed",
                    "fixed_tp_pnl_rate": 0.2,
                    "fixed_sl_pnl_rate": -0.1
                }
            }))
            .is_err()
        );
    }

    #[test]
    fn fixed_tp_sl_does_not_default_null_or_empty_form_values() {
        for value in [serde_json::Value::Null, serde_json::json!("")] {
            let config = serde_json::json!({
                "tp_sl": {
                    "take_profit": { "mode": "fixed", "pnl_rate": value },
                    "stop_loss": { "mode": "fixed", "pnl_rate": -0.1 }
                }
            });
            assert!(configured_fixed_take_profit(&config).is_err());
        }

        let config = serde_json::json!({
            "tp_sl": {
                "take_profit": { "mode": "fixed", "pnl_rate": 0.2 },
                "stop_loss": { "mode": "fixed", "pnl_rate": null }
            }
        });
        assert!(configured_fixed_stop_loss(&config).is_err());
    }

    #[test]
    fn fixed_and_custom_rules_are_configured_independently() {
        let config = serde_json::json!({
            "tp_sl": {
                "take_profit": { "mode": "fixed", "pnl_rate": 0.2 },
                "stop_loss": { "mode": "custom", "custom_prompt": "close on structure break" }
            }
        });
        assert_eq!(
            configured_fixed_take_profit(&config).expect("take-profit"),
            Some(FixedTakeProfit { pnl_rate: 0.2 })
        );
        assert_eq!(
            configured_fixed_stop_loss(&config).expect("stop-loss"),
            None
        );

        let orders = build_fixed_tp_sl_orders(
            &position("LONG"),
            &constraints(),
            Some(FixedTakeProfit { pnl_rate: 0.2 }),
            None,
        )
        .expect("orders");
        assert_eq!(orders.len(), 1);
        assert_eq!(
            orders[0].conditional_type,
            ExchangeConditionalOrderType::TakeProfitMarket
        );
    }

    #[test]
    fn long_fixed_tp_sl_uses_entry_price_and_leverage_for_two_trigger_orders() {
        let orders = build_fixed_tp_sl_orders(
            &position("LONG"),
            &constraints(),
            Some(FixedTakeProfit { pnl_rate: 0.2 }),
            Some(FixedStopLoss { pnl_rate: -0.1 }),
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
            Some(FixedTakeProfit { pnl_rate: 0.2 }),
            Some(FixedStopLoss { pnl_rate: -0.1 }),
        )
        .expect("orders");

        assert_eq!(orders[0].side, ExchangeSide::Buy);
        assert_eq!(orders[0].position_side, Some(PositionSide::Short));
        assert_eq!(orders[0].trigger_price, 98.0);
        assert_eq!(orders[1].trigger_price, 101.0);
    }
}
