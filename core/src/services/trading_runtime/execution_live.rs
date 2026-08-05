use super::service::*;
use crate::repositories::trading::records::{
    accounts::TraderAccountRecord,
    execution::InsertExecutionIntentRecord,
    history::InsertTraderDecisionRecord,
    orders::{InsertOrderFillRecord, InsertTraderOrderRecord, UpdateTraderOrderRecord},
    positions::UpsertPositionFromExchangeRecord,
};

pub async fn sync_live_positions_and_balances(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    adapter: &dyn LiveExchangeAdapter,
    ts: i64,
) -> Result<(), AppError> {
    let live_positions = get_positions_with_retry(adapter).await?;
    let mut live_keys: HashSet<String> = HashSet::new();

    for p in live_positions {
        let quantity = p.quantity.abs();
        if quantity <= f64::EPSILON {
            continue;
        }

        let side = normalize_position_side(&p.position_side, p.quantity);
        let symbol = p.symbol.trim().to_uppercase();
        live_keys.insert(format!("{}:{}", symbol, side));

        state
            .trading_repo
            .upsert_open_position_from_exchange(UpsertPositionFromExchangeRecord {
                trader_id: cfg.trader_id.clone(),
                symbol,
                side: side.to_string(),
                quantity,
                entry_price: p.entry_price,
                mark_price: p.mark_price,
                liquidation_price: p.liquidation_price,
                leverage: p.leverage.max(1),
                unrealized_pnl: p.unrealized_pnl,
                event_at: ts,
                updated_at: ts,
            })
            .await?;
    }

    state
        .trading_repo
        .close_open_positions_missing_from_exchange(&cfg.trader_id, &live_keys, ts)
        .await?;

    let balances = get_balances_with_retry(adapter).await?;
    if let Some(bal) = balances
        .iter()
        .find(|b| b.asset.eq_ignore_ascii_case("USDT"))
        .or_else(|| balances.first())
    {
        let total_balance = bal.wallet_balance.max(0.0);
        let available_balance = bal.available_balance.max(0.0);
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
                    unrealized_pnl: bal.unrealized_pnl,
                    realized_pnl: 0.0,
                    currency: bal.asset.trim().to_uppercase(),
                    snapshot_at: ts,
                },
                ts,
            )
            .await?;
    }

    Ok(())
}

pub fn normalize_position_side(position_side: &str, quantity: f64) -> &'static str {
    match position_side.trim().to_ascii_uppercase().as_str() {
        "LONG" => "LONG",
        "SHORT" => "SHORT",
        _ => {
            if quantity >= 0.0 {
                "LONG"
            } else {
                "SHORT"
            }
        }
    }
}

pub fn normalize_order_position_side(position_side: &str, side: &str) -> String {
    let ps = position_side.trim().to_ascii_uppercase();
    if ps == "LONG" || ps == "SHORT" {
        return ps;
    }

    match side.trim().to_ascii_uppercase().as_str() {
        "BUY" => "LONG".to_string(),
        "SELL" => "SHORT".to_string(),
        _ => String::new(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveRiskLevel {
    Normal,
    Soft,
    Medium,
    Hard,
}

impl LiveRiskLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            LiveRiskLevel::Normal => "normal",
            LiveRiskLevel::Soft => "soft",
            LiveRiskLevel::Medium => "medium",
            LiveRiskLevel::Hard => "hard",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LiveRiskDecision {
    pub level: LiveRiskLevel,
    pub drawdown_pct: f64,
    pub open_order_cooldown_secs: i64,
    pub medium_reduce_count: usize,
    pub hard_close_count: usize,
}

pub fn evaluate_live_risk(
    config: &RuntimeConfig,
    cfg: &TraderRuntimeConfig,
    metrics: &AccountMetrics,
    hard_risk_trigger: bool,
) -> LiveRiskDecision {
    let live = &config.live;
    let open_order_cooldown_secs_base = live.open_order_cooldown_secs as i64;
    let drawdown_pct = if cfg.initial_balance > 0.0 {
        ((cfg.initial_balance - metrics.total_balance) / cfg.initial_balance) * 100.0
    } else {
        0.0
    };

    let risk_soft = drawdown_pct >= live.risk_soft_drawdown_pct
        || metrics.margin_used_ratio >= live.risk_soft_margin_ratio;
    let risk_medium = drawdown_pct >= live.risk_medium_drawdown_pct
        || metrics.margin_used_ratio >= live.risk_medium_margin_ratio;
    let risk_hard = drawdown_pct >= live.risk_hard_drawdown_pct
        || metrics.margin_used_ratio >= live.risk_hard_margin_ratio;

    let level = if hard_risk_trigger || risk_hard {
        LiveRiskLevel::Hard
    } else if risk_medium {
        LiveRiskLevel::Medium
    } else if risk_soft {
        LiveRiskLevel::Soft
    } else {
        LiveRiskLevel::Normal
    };

    let open_order_cooldown_secs = if matches!(level, LiveRiskLevel::Soft | LiveRiskLevel::Medium) {
        open_order_cooldown_secs_base
            .saturating_mul(live.risk_soft_open_cooldown_multiplier as i64)
            .max(1)
    } else {
        open_order_cooldown_secs_base.max(1)
    };

    LiveRiskDecision {
        level,
        drawdown_pct,
        open_order_cooldown_secs,
        medium_reduce_count: (live.risk_medium_reduce_positions_count as usize).max(1),
        hard_close_count: (live.risk_hard_close_worst_positions_count as usize).max(1),
    }
}

async fn emit_live_open_skipped_constraints(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    decision: &DecisionSignal,
    desired_side: &str,
    risk_level: &str,
    cycle_correlation_id: &str,
    reason: &str,
    configured_cost: f64,
    leverage: i64,
    reference_price: f64,
    raw_quantity: f64,
    normalized_quantity: f64,
    constraints: &ExchangeSymbolConstraints,
    ts: i64,
) {
    let minimum_quantity = constraints.min_qty.max(0.0);
    let minimum_notional = constraints.min_notional.max(0.0);
    let required_minimum_notional = (minimum_quantity * reference_price).max(minimum_notional);
    let minimum_required_cost = required_minimum_notional / leverage.max(1) as f64;

    emit_runtime_event_best_effort(
        state,
        cfg,
        EVENT_LIVE_OPEN_SKIPPED_CONSTRAINTS,
        &decision.symbol,
        desired_side,
        risk_level,
        "exchange-constraints",
        "skip-open-constraints",
        cycle_correlation_id,
        json!({
            "reason": reason,
            "decision_action": decision.action,
            "configured_cost": configured_cost,
            "leverage": leverage,
            "reference_price": reference_price,
            "raw_quantity": raw_quantity,
            "normalized_quantity": normalized_quantity,
            "minimum_quantity": minimum_quantity,
            "step_size": constraints.step_size,
            "normalized_notional": normalized_quantity * reference_price,
            "minimum_notional": minimum_notional,
            "required_minimum_notional": required_minimum_notional,
            "minimum_required_cost": minimum_required_cost,
        }),
        ts,
    )
    .await;
}

pub async fn execute_decisions_live(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    decisions: &[DecisionSignal],
    open_positions: &[PositionView],
    metrics: &AccountMetrics,
    market: &HashMap<String, MarketState>,
    adapter: &dyn LiveExchangeAdapter,
    ts: i64,
    _hard_risk_trigger: bool,
    risk_decision: &LiveRiskDecision,
    cycle_correlation_id: &str,
) -> Result<(), AppError> {
    let live = &state.config.live;
    let stale_open_order_cancel_secs = live.stale_open_order_cancel_secs as i64;
    let stale_limit_order_replace_secs = live.stale_limit_order_replace_secs as i64;
    let replace_max_attempts_per_window = live.replace_max_attempts_per_window as i64;
    let replace_attempt_window_secs = live.replace_attempt_window_secs as i64;
    let submitted_intent_reconcile_secs = live.submitted_intent_reconcile_secs as i64;

    let risk_decision = *risk_decision;
    let hard_risk_mode = matches!(risk_decision.level, LiveRiskLevel::Hard);
    let medium_risk_mode = matches!(risk_decision.level, LiveRiskLevel::Medium);
    let risk_adjusted_open_order_cooldown_secs = risk_decision.open_order_cooldown_secs;
    let medium_reduce_count = risk_decision.medium_reduce_count;
    let hard_close_count = risk_decision.hard_close_count;
    let drawdown_pct = risk_decision.drawdown_pct;

    emit_runtime_event_best_effort(
        state,
        cfg,
        EVENT_LIVE_RISK_SNAPSHOT,
        "",
        "",
        risk_decision.level.as_str(),
        "risk-engine",
        "evaluate-live-risk",
        cycle_correlation_id,
        json!({
            "drawdown_pct": drawdown_pct,
            "margin_used_ratio": metrics.margin_used_ratio,
            "open_order_cooldown_secs": risk_adjusted_open_order_cooldown_secs,
            "medium_reduce_count": medium_reduce_count,
            "hard_close_count": hard_close_count
        }),
        ts,
    )
    .await;

    cancel_stale_live_open_orders(state, cfg, adapter, ts, stale_open_order_cancel_secs).await?;
    cancel_replace_stale_live_limit_open_orders(
        state,
        cfg,
        adapter,
        ts,
        stale_limit_order_replace_secs,
        replace_max_attempts_per_window,
        replace_attempt_window_secs,
        risk_decision.level.as_str(),
        cycle_correlation_id,
    )
    .await?;

    if hard_risk_mode {
        close_worst_positions_live(
            state,
            cfg,
            open_positions,
            adapter,
            ts,
            hard_close_count,
            risk_decision.level.as_str(),
            cycle_correlation_id,
        )
        .await?;
        sync_open_orders_from_exchange(state, cfg, adapter, ts).await?;
        sync_terminal_orders_from_exchange(state, cfg, adapter, ts).await?;
        reconcile_stale_submitted_execution_intents(
            state,
            cfg,
            adapter,
            ts,
            submitted_intent_reconcile_secs,
        )
        .await?;
        reconcile_local_positions_from_terminal_reduce_only_orders(state, cfg, ts).await?;
        sync_live_positions_and_balances(state, cfg, adapter, ts).await?;
        return Ok(());
    }

    if medium_risk_mode {
        close_worst_positions_live(
            state,
            cfg,
            open_positions,
            adapter,
            ts,
            medium_reduce_count,
            risk_decision.level.as_str(),
            cycle_correlation_id,
        )
        .await?;
    }

    let mut by_symbol: HashMap<String, Vec<&PositionView>> = HashMap::new();
    for p in open_positions {
        by_symbol.entry(p.symbol.clone()).or_default().push(p);
    }

    let mut open_count = open_positions.len() as i64;
    let mut max_positions = 8_i64;
    if medium_risk_mode {
        max_positions = max_positions.min(4);
    }

    for d in decisions {
        if d.action == "NO ACTION" {
            continue;
        }

        let desired_side = if d.action == "LONG" { "LONG" } else { "SHORT" };
        let opposite_side = if desired_side == "LONG" {
            "SHORT"
        } else {
            "LONG"
        };

        let price = market
            .get(&d.symbol)
            .map(|x| x.price)
            .unwrap_or(d.price.max(1e-9));

        if let Some(existing) = by_symbol.get(&d.symbol) {
            // close opposite side first (reduce-only)
            for p in existing.iter().filter(|x| x.side == opposite_side) {
                let close_side = if p.side == "LONG" {
                    ExchangeSide::Sell
                } else {
                    ExchangeSide::Buy
                };

                let constraints = get_symbol_constraints_with_retry(adapter, &p.symbol).await?;
                let close_qty =
                    normalize_order_quantity_by_constraints(p.quantity.max(0.0001), &constraints);

                if close_qty <= f64::EPSILON {
                    warn!(
                        "skip live reduce-only close due to normalized qty<=0 symbol={} raw_qty={} step_size={}",
                        p.symbol, p.quantity, constraints.step_size
                    );
                    continue;
                }

                let intent_key = format!(
                    "close:{}:{}:{}:{}",
                    cfg.trader_id,
                    p.symbol.trim().to_uppercase(),
                    p.side.trim().to_uppercase(),
                    ts / 60
                );
                if !try_register_execution_intent(
                    state,
                    cfg,
                    &intent_key,
                    &p.symbol,
                    &p.side,
                    "close-opposite",
                    "decision-engine",
                    "close-opposite",
                    risk_decision.level.as_str(),
                    cycle_correlation_id,
                    ts,
                )
                .await?
                {
                    warn!(
                        "skip duplicate close intent trader={} symbol={} side={} key={}",
                        cfg.trader_id, p.symbol, p.side, intent_key
                    );
                    continue;
                }

                let close_resp = adapter
                    .place_order(PlaceOrderRequest {
                        symbol: p.symbol.clone(),
                        side: close_side,
                        order_type: ExchangeOrderType::Market,
                        quantity: close_qty,
                        price: None,
                        reduce_only: true,
                        margin_mode: Some(margin_mode_for_config(cfg)),
                        position_side: Some(if p.side == "LONG" {
                            PositionSide::Long
                        } else {
                            PositionSide::Short
                        }),
                        time_in_force: None,
                        client_order_id: Some(format!("nfx_{}", Uuid::now_v7().simple())),
                    })
                    .await?;

                mark_execution_intent_submitted(state, cfg, &intent_key, &close_resp.order_id, ts)
                    .await?;
                persist_live_order_record(state, cfg, adapter, &close_resp, &p.side, true, ts)
                    .await?;
                emit_runtime_event_best_effort(
                    state,
                    cfg,
                    EVENT_LIVE_ORDER_SUBMITTED,
                    &p.symbol,
                    &p.side,
                    risk_decision.level.as_str(),
                    "decision-engine",
                    "submit-close-opposite",
                    cycle_correlation_id,
                    json!({
                        "intent_key": intent_key,
                        "exchange_order_id": close_resp.order_id,
                        "order_type": close_resp.order_type,
                        "reduce_only": true
                    }),
                    ts,
                )
                .await;
                open_count = (open_count - 1).max(0);
            }

            // same-side position exists, skip opening more
            if existing.iter().any(|x| x.side == desired_side) {
                continue;
            }
        }

        let open_order_side = if desired_side == "LONG" {
            "BUY"
        } else {
            "SELL"
        };
        if has_recent_live_open_order(
            state,
            cfg,
            &d.symbol,
            open_order_side,
            ts,
            risk_adjusted_open_order_cooldown_secs,
        )
        .await?
        {
            warn!(
                "skip live open by cooldown trader={} symbol={} side={} cooldown_secs={}",
                cfg.trader_id, d.symbol, open_order_side, risk_adjusted_open_order_cooldown_secs
            );
            continue;
        }

        if medium_risk_mode {
            warn!(
                "skip live open by medium risk guard trader={} symbol={} drawdown_pct={:.2} margin_used_ratio={:.4}",
                cfg.trader_id, d.symbol, drawdown_pct, metrics.margin_used_ratio
            );
            emit_runtime_event_best_effort(
                state,
                cfg,
                EVENT_LIVE_OPEN_SKIPPED_MEDIUM_RISK,
                &d.symbol,
                open_order_side,
                risk_decision.level.as_str(),
                "risk-engine",
                "skip-open-medium-risk",
                cycle_correlation_id,
                json!({
                    "drawdown_pct": drawdown_pct,
                    "margin_used_ratio": metrics.margin_used_ratio,
                    "decision_action": d.action,
                    "decision_confidence": d.confidence
                }),
                ts,
            )
            .await;
            continue;
        }

        if open_count >= max_positions {
            continue;
        }

        if metrics.available_balance <= 5.0 || metrics.margin_used_ratio > 0.75 {
            continue;
        }

        let leverage = leverage_for_symbol(cfg, &d.symbol).max(1);
        let mut risk_budget = (metrics.total_balance * 0.06).max(5.0);
        if let Some(sym_cfg) = cfg.symbols_config.iter().find(|s| s.symbol.to_uppercase() == d.symbol.to_uppercase()) {
            if let Some(fixed) = sym_cfg.fixed_cost {
                risk_budget = fixed;
            } else {
                if let Some(min_c) = sym_cfg.min_cost {
                    risk_budget = risk_budget.max(min_c);
                }
                if let Some(max_c) = sym_cfg.max_cost {
                    risk_budget = risk_budget.min(max_c);
                }
            }
        }
        let raw_qty = (risk_budget * leverage as f64 / price).max(0.0001);

        let constraints = get_symbol_constraints_with_retry(adapter, &d.symbol).await?;
        let qty = normalize_order_quantity_by_constraints(raw_qty, &constraints);
        let est_notional = qty * price;

        if qty <= f64::EPSILON {
            warn!(
                "skip live open due to normalized qty<=0 symbol={} raw_qty={} step_size={}",
                d.symbol, raw_qty, constraints.step_size
            );
            emit_live_open_skipped_constraints(
                state,
                cfg,
                d,
                desired_side,
                risk_decision.level.as_str(),
                cycle_correlation_id,
                "quantity_below_minimum",
                risk_budget,
                leverage,
                price,
                raw_qty,
                qty,
                &constraints,
                ts,
            )
            .await;
            continue;
        }

        if constraints.min_notional > 0.0 && est_notional < constraints.min_notional {
            warn!(
                "skip live open due to min_notional symbol={} est_notional={} min_notional={}",
                d.symbol, est_notional, constraints.min_notional
            );
            emit_live_open_skipped_constraints(
                state,
                cfg,
                d,
                desired_side,
                risk_decision.level.as_str(),
                cycle_correlation_id,
                "notional_below_minimum",
                risk_budget,
                leverage,
                price,
                raw_qty,
                qty,
                &constraints,
                ts,
            )
            .await;
            continue;
        }

        let _open_side = if desired_side == "LONG" {
            ExchangeSide::Buy
        } else {
            ExchangeSide::Sell
        };

        let intent_key = format!(
            "open:{}:{}:{}:{}",
            cfg.trader_id,
            d.symbol.trim().to_uppercase(),
            desired_side,
            ts / 60
        );
        if !try_register_execution_intent(
            state,
            cfg,
            &intent_key,
            &d.symbol,
            desired_side,
            "open",
            "decision-engine",
            "open",
            risk_decision.level.as_str(),
            cycle_correlation_id,
            ts,
        )
        .await?
        {
            warn!(
                "skip duplicate open intent trader={} symbol={} side={} key={}",
                cfg.trader_id, d.symbol, desired_side, intent_key
            );
            continue;
        }

        let open_resp = place_live_open_order_limit_first(
            adapter,
            &d.symbol,
            desired_side,
            qty,
            price,
            &constraints,
            margin_mode_for_config(cfg),
        )
        .await?;

        mark_execution_intent_submitted(state, cfg, &intent_key, &open_resp.order_id, ts).await?;
        persist_live_order_record(state, cfg, adapter, &open_resp, desired_side, false, ts).await?;
        emit_runtime_event_best_effort(
            state,
            cfg,
            EVENT_LIVE_ORDER_SUBMITTED,
            &d.symbol,
            desired_side,
            risk_decision.level.as_str(),
            "decision-engine",
            "submit-open",
            cycle_correlation_id,
            json!({
                "intent_key": intent_key,
                "exchange_order_id": open_resp.order_id,
                "order_type": open_resp.order_type,
                "reduce_only": false
            }),
            ts,
        )
        .await;

        if open_resp.order_type.trim().eq_ignore_ascii_case("MARKET") {
            emit_runtime_event_best_effort(
                state,
                cfg,
                EVENT_LIVE_OPEN_USED_MARKET_FALLBACK,
                &d.symbol,
                desired_side,
                risk_decision.level.as_str(),
                "execution-engine",
                "limit-first-fallback-market",
                cycle_correlation_id,
                json!({
                    "intent_key": intent_key,
                    "exchange_order_id": open_resp.order_id,
                    "reason": "limit_not_filled_or_failed"
                }),
                ts,
            )
            .await;
        }

        open_count += 1;
    }

    sync_open_orders_from_exchange(state, cfg, adapter, ts).await?;
    sync_terminal_orders_from_exchange(state, cfg, adapter, ts).await?;
    reconcile_stale_submitted_execution_intents(
        state,
        cfg,
        adapter,
        ts,
        submitted_intent_reconcile_secs,
    )
    .await?;
    reconcile_local_positions_from_terminal_reduce_only_orders(state, cfg, ts).await?;
    sync_live_positions_and_balances(state, cfg, adapter, ts).await?;
    Ok(())
}

pub async fn close_worst_positions_live(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    open_positions: &[PositionView],
    adapter: &dyn LiveExchangeAdapter,
    ts: i64,
    close_count: usize,
    risk_level: &str,
    cycle_correlation_id: &str,
) -> Result<(), AppError> {
    let mut scored = Vec::new();
    for p in open_positions {
        let upnl = if p.side == "LONG" {
            (p.mark_price - p.entry_price) * p.quantity
        } else {
            (p.entry_price - p.mark_price) * p.quantity
        };
        scored.push((upnl, p));
    }

    scored.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    for (_, p) in scored.into_iter().take(close_count.max(1)) {
        let close_side = if p.side == "LONG" {
            ExchangeSide::Sell
        } else {
            ExchangeSide::Buy
        };

        let constraints = get_symbol_constraints_with_retry(adapter, &p.symbol).await?;
        let close_qty =
            normalize_order_quantity_by_constraints(p.quantity.max(0.0001), &constraints);

        if close_qty <= f64::EPSILON {
            warn!(
                "skip risk-close due to normalized qty<=0 symbol={} raw_qty={} step_size={}",
                p.symbol, p.quantity, constraints.step_size
            );
            continue;
        }

        let intent_key = format!(
            "risk-close:{}:{}:{}:{}",
            cfg.trader_id,
            p.symbol.trim().to_uppercase(),
            p.side.trim().to_uppercase(),
            ts / 60
        );
        if !try_register_execution_intent(
            state,
            cfg,
            &intent_key,
            &p.symbol,
            &p.side,
            "risk-close",
            "risk-engine",
            "risk-close",
            risk_level,
            cycle_correlation_id,
            ts,
        )
        .await?
        {
            warn!(
                "skip duplicate risk-close intent trader={} symbol={} side={} key={}",
                cfg.trader_id, p.symbol, p.side, intent_key
            );
            continue;
        }

        let close_resp = adapter
            .place_order(PlaceOrderRequest {
                symbol: p.symbol.clone(),
                side: close_side,
                order_type: ExchangeOrderType::Market,
                quantity: close_qty,
                price: None,
                reduce_only: true,
                margin_mode: Some(margin_mode_for_config(cfg)),
                position_side: Some(if p.side == "LONG" {
                    PositionSide::Long
                } else {
                    PositionSide::Short
                }),
                time_in_force: None,
                client_order_id: Some(format!("nfx_{}", Uuid::now_v7().simple())),
            })
            .await?;

        mark_execution_intent_submitted(state, cfg, &intent_key, &close_resp.order_id, ts).await?;
        persist_live_order_record(state, cfg, adapter, &close_resp, &p.side, true, ts).await?;
        emit_runtime_event_best_effort(
            state,
            cfg,
            EVENT_LIVE_ORDER_SUBMITTED,
            &p.symbol,
            &p.side,
            risk_level,
            "risk-engine",
            "submit-risk-close",
            cycle_correlation_id,
            json!({
                "intent_key": intent_key,
                "exchange_order_id": close_resp.order_id,
                "order_type": close_resp.order_type,
                "reduce_only": true
            }),
            ts,
        )
        .await;
    }

    Ok(())
}

pub async fn has_recent_live_open_order(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    symbol: &str,
    side: &str,
    now_ts: i64,
    cooldown_secs: i64,
) -> Result<bool, AppError> {
    let threshold = now_ts.saturating_sub(cooldown_secs.max(1));
    Ok(state
        .trading_repo
        .has_recent_open_order(&cfg.trader_id, symbol, side, threshold)
        .await?)
}

pub async fn try_register_execution_intent(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    intent_key: &str,
    symbol: &str,
    side: &str,
    decision: &str,
    trigger_source: &str,
    action_taken: &str,
    risk_level: &str,
    correlation_id: &str,
    ts: i64,
) -> Result<bool, AppError> {
    if state
        .trading_repo
        .execution_intent_by_key(&cfg.trader_id, intent_key)
        .await?
        .is_some()
    {
        return Ok(false);
    }

    let payload_json = json!({
        "trigger_source": trigger_source,
        "action_taken": action_taken,
        "risk_level": risk_level,
        "correlation_id": correlation_id
    })
    .to_string();

    state
        .trading_repo
        .insert_execution_intent(InsertExecutionIntentRecord {
            id: Uuid::now_v7().to_string(),
            trader_id: cfg.trader_id.clone(),
            intent_key: intent_key.to_string(),
            symbol: symbol.trim().to_uppercase(),
            side: side.trim().to_uppercase(),
            decision: decision.to_string(),
            status: "pending".to_string(),
            exchange_order_id: String::new(),
            payload_json,
            created_at: ts,
            updated_at: ts,
        })
        .await?;

    Ok(true)
}

pub async fn mark_execution_intent_submitted(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    intent_key: &str,
    exchange_order_id: &str,
    ts: i64,
) -> Result<(), AppError> {
    if let Some(intent) = state
        .trading_repo
        .execution_intent_by_key(&cfg.trader_id, intent_key)
        .await?
        .filter(|intent| intent.status == "pending")
    {
        let payload_json = patch_json_payload(
            &intent.payload_json,
            &[
                (
                    "exchange_order_id",
                    Value::String(exchange_order_id.trim().to_string()),
                ),
                ("submitted_at", json!(ts)),
                ("lifecycle_status", Value::String("submitted".to_string())),
            ],
        );
        state
            .trading_repo
            .mark_execution_intent_submitted(
                &cfg.trader_id,
                intent_key,
                exchange_order_id,
                payload_json,
                ts,
            )
            .await?;
    }

    Ok(())
}

pub async fn finalize_execution_intent_for_exchange_order(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    exchange_order_id: &str,
    order_status: &str,
    ts: i64,
) -> Result<(), AppError> {
    let status = order_status.trim().to_ascii_lowercase();
    let mapped_intent_status = match status.as_str() {
        "filled" => "filled",
        "canceled" => "canceled",
        "rejected" => "rejected",
        "expired" => "expired",
        _ => return Ok(()),
    };

    let intents = state
        .trading_repo
        .submitted_execution_intents_by_exchange_order(
            &cfg.trader_id,
            exchange_order_id,
        )
        .await?;

    for intent in intents {
        let payload_json = patch_json_payload(
            &intent.payload_json,
            &[
                (
                    "final_status",
                    Value::String(mapped_intent_status.to_string()),
                ),
                ("finalized_at", json!(ts)),
                (
                    "lifecycle_status",
                    Value::String(mapped_intent_status.to_string()),
                ),
            ],
        );
        state
            .trading_repo
            .update_execution_intent_status(&intent.id, mapped_intent_status, payload_json, ts)
            .await?;
    }

    Ok(())
}

pub async fn reconcile_stale_submitted_execution_intents(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    adapter: &dyn LiveExchangeAdapter,
    ts: i64,
    stale_after_secs: i64,
) -> Result<(), AppError> {
    let threshold = ts.saturating_sub(stale_after_secs.max(60));

    let rows = state
        .trading_repo
        .stale_submitted_execution_intents(&cfg.trader_id, threshold, 200)
        .await?;

    for row in rows {
        let intent_key = row.intent_key.clone();
        let symbol = row.symbol.clone();
        let exchange_order_id = row.exchange_order_id.clone();

        if symbol.trim().is_empty() || exchange_order_id.trim().is_empty() {
            continue;
        }

        match get_order_with_retry(adapter, &symbol, &exchange_order_id).await {
            Ok(detail) => {
                let normalized_status = normalize_order_status(&detail.status);

                if is_terminal_order_status(&normalized_status) {
                    finalize_execution_intent_for_exchange_order(
                        state,
                        cfg,
                        &exchange_order_id,
                        &normalized_status,
                        ts,
                    )
                    .await?;
                    emit_runtime_event_best_effort(
                        state,
                        cfg,
                        EVENT_STALE_INTENT_RECONCILE_TERMINAL,
                        &symbol,
                        "",
                        "normal",
                        "order-reconciler",
                        "finalize-stale-submitted-intent",
                        &intent_key,
                        json!({
                            "exchange_order_id": exchange_order_id,
                            "final_status": normalized_status
                        }),
                        ts,
                    )
                    .await;
                } else {
                    if let Some(intent) = state
                        .trading_repo
                        .execution_intent_by_key(&cfg.trader_id, &intent_key)
                        .await?
                        .filter(|intent| intent.status == "submitted")
                    {
                        let payload_json = patch_json_payload(
                            &intent.payload_json,
                            &[
                                ("last_reconciled_at", json!(ts)),
                                ("lifecycle_status", Value::String("submitted".to_string())),
                            ],
                        );
                        state
                            .trading_repo
                            .update_execution_intent_status(
                                &intent.id,
                                "submitted",
                                payload_json,
                                ts,
                            )
                            .await?;
                    }
                    emit_runtime_event_best_effort(
                        state,
                        cfg,
                        EVENT_STALE_INTENT_RECONCILE_PENDING,
                        &symbol,
                        "",
                        "normal",
                        "order-reconciler",
                        "touch-stale-submitted-intent",
                        &intent_key,
                        json!({
                            "exchange_order_id": exchange_order_id,
                            "observed_status": normalized_status
                        }),
                        ts,
                    )
                    .await;
                }
            }
            Err(err) if err.is_exchange_order_missing() => {
                finalize_execution_intent_for_exchange_order(
                    state,
                    cfg,
                    &exchange_order_id,
                    "expired",
                    ts,
                )
                .await?;
                emit_runtime_event_best_effort(
                    state,
                    cfg,
                    EVENT_STALE_INTENT_RECONCILE_TERMINAL,
                    &symbol,
                    "",
                    "normal",
                    "order-reconciler",
                    "finalize-stale-submitted-intent",
                    &intent_key,
                    json!({
                        "exchange_order_id": exchange_order_id,
                        "final_status": "expired",
                        "reason": "exchange_order_not_found"
                    }),
                    ts,
                )
                .await;
            }
            Err(err) => {
                warn!(
                    "reconcile stale submitted intent failed trader={} intent_key={} symbol={} exchange_order_id={} err={}",
                    cfg.trader_id, intent_key, symbol, exchange_order_id, err
                );
            }
        }
    }

    Ok(())
}

pub async fn cancel_stale_live_open_orders(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    adapter: &dyn LiveExchangeAdapter,
    now_ts: i64,
    stale_after_secs: i64,
) -> Result<(), AppError> {
    let threshold = now_ts.saturating_sub(stale_after_secs.max(30));

    let stale_rows = state
        .trading_repo
        .stale_live_open_orders(&cfg.trader_id, threshold, 100)
        .await?;

    for row in stale_rows {
        let order_id = row.id.clone();
        let symbol = row.symbol.clone();
        let exchange_order_id = row.exchange_order_id.clone();

        match adapter.cancel_order(&symbol, &exchange_order_id).await {
            Ok(_) => {
                state
                    .trading_repo
                    .update_order_status(
                        &cfg.trader_id,
                        &order_id,
                        "canceled",
                        now_ts,
                        Some(now_ts),
                    )
                    .await?;

                finalize_execution_intent_for_exchange_order(
                    state,
                    cfg,
                    &exchange_order_id,
                    "canceled",
                    now_ts,
                )
                .await?;
            }
            Err(err) if err.is_exchange_order_missing() => {
                state
                    .trading_repo
                    .update_order_status(
                        &cfg.trader_id,
                        &order_id,
                        "expired",
                        now_ts,
                        Some(now_ts),
                    )
                    .await?;

                finalize_execution_intent_for_exchange_order(
                    state,
                    cfg,
                    &exchange_order_id,
                    "expired",
                    now_ts,
                )
                .await?;
            }
            Err(err) => {
                warn!(
                    "cancel stale live order failed trader={} symbol={} exchange_order_id={} err={}",
                    cfg.trader_id, symbol, exchange_order_id, err
                );
            }
        }
    }

    Ok(())
}

pub async fn cancel_replace_stale_live_limit_open_orders(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    adapter: &dyn LiveExchangeAdapter,
    ts: i64,
    stale_after_secs: i64,
    max_attempts_per_window: i64,
    attempt_window_secs: i64,
    risk_level: &str,
    cycle_correlation_id: &str,
) -> Result<(), AppError> {
    let threshold = ts.saturating_sub(stale_after_secs.max(30));

    let rows = state
        .trading_repo
        .stale_limit_live_open_orders(&cfg.trader_id, threshold, 100)
        .await?;

    for row in rows {
        let symbol = row.symbol.clone();
        let side_raw = row.side.clone();
        let position_side_raw = row.position_side.clone();
        let exchange_order_id = row.exchange_order_id.clone();
        let remaining_qty = (row.quantity - row.filled_quantity).max(0.0);
        let old_price = row.price;
        if remaining_qty <= f64::EPSILON {
            continue;
        }

        let side_for_guard = normalize_order_position_side(&position_side_raw, &side_raw);
        if side_for_guard.is_empty() {
            continue;
        }

        if has_recent_replace_attempt(
            state,
            cfg,
            &symbol,
            &side_for_guard,
            ts,
            attempt_window_secs,
            max_attempts_per_window,
        )
        .await?
        {
            warn!(
                "cancel-replace throttled trader={} symbol={} side={} max_attempts={} window_secs={}",
                cfg.trader_id, symbol, side_for_guard, max_attempts_per_window, attempt_window_secs
            );
            emit_runtime_event_best_effort(
                state,
                cfg,
                EVENT_CANCEL_REPLACE_THROTTLED,
                &symbol,
                &side_for_guard,
                risk_level,
                "order-reconciler",
                "skip-replace-throttled",
                cycle_correlation_id,
                json!({
                    "max_attempts_per_window": max_attempts_per_window,
                    "attempt_window_secs": attempt_window_secs,
                    "reason": "recent_attempts_limit_reached"
                }),
                ts,
            )
            .await;
            continue;
        }

        let cancel_result = adapter.cancel_order(&symbol, &exchange_order_id).await;
        let old_terminal_status = match cancel_result {
            Ok(_) => "canceled",
            Err(err) if err.is_exchange_order_missing() => "expired",
            Err(err) => {
                warn!(
                    "cancel-replace cancel failed trader={} symbol={} exchange_order_id={} err={}",
                    cfg.trader_id, symbol, exchange_order_id, err
                );
                continue;
            }
        };

        state
            .trading_repo
            .update_order_status(
                &cfg.trader_id,
                &row.id,
                old_terminal_status,
                ts,
                row.closed_at.or(Some(ts)),
            )
            .await?;

        finalize_execution_intent_for_exchange_order(
            state,
            cfg,
            &exchange_order_id,
            old_terminal_status,
            ts,
        )
        .await?;

        let constraints = match get_symbol_constraints_with_retry(adapter, &symbol).await {
            Ok(v) => v,
            Err(err) => {
                warn!(
                    "cancel-replace constraints fetch failed trader={} symbol={} err={}",
                    cfg.trader_id, symbol, err
                );
                continue;
            }
        };

        let mark_price = get_price_with_retry(adapter, &symbol)
            .await
            .unwrap_or(old_price.max(1e-9));
        let normalized_qty = normalize_order_quantity_by_constraints(remaining_qty, &constraints);
        if normalized_qty <= f64::EPSILON {
            continue;
        }

        let desired_side = normalize_order_position_side(&position_side_raw, &side_raw);
        if desired_side.is_empty() {
            continue;
        }

        let desired_side_text = desired_side.clone();
        let est_notional = normalized_qty * mark_price;
        if constraints.min_notional > 0.0 && est_notional < constraints.min_notional {
            warn!(
                "cancel-replace skip by min_notional trader={} symbol={} est_notional={} min_notional={}",
                cfg.trader_id, symbol, est_notional, constraints.min_notional
            );
            continue;
        }

        let intent_key = format!(
            "replace:{}:{}:{}:{}:{}",
            cfg.trader_id,
            symbol.trim().to_uppercase(),
            desired_side_text,
            exchange_order_id.trim(),
            ts / 60
        );

        if !try_register_execution_intent(
            state,
            cfg,
            &intent_key,
            &symbol,
            &desired_side_text,
            "replace-open-limit",
            "order-reconciler",
            "replace-open-limit",
            risk_level,
            cycle_correlation_id,
            ts,
        )
        .await?
        {
            continue;
        }

        let new_resp = place_live_open_order_limit_first(
            adapter,
            &symbol,
            &desired_side_text,
            normalized_qty,
            mark_price,
            &constraints,
            margin_mode_for_config(cfg),
        )
        .await?;

        mark_execution_intent_submitted(state, cfg, &intent_key, &new_resp.order_id, ts).await?;
        persist_live_order_record(
            state,
            cfg,
            adapter,
            &new_resp,
            &desired_side_text,
            false,
            ts,
        )
        .await?;
        emit_runtime_event_best_effort(
            state,
            cfg,
            EVENT_LIVE_ORDER_SUBMITTED,
            &symbol,
            &desired_side_text,
            risk_level,
            "order-reconciler",
            "submit-replace-open-limit",
            cycle_correlation_id,
            json!({
                "intent_key": intent_key,
                "exchange_order_id": new_resp.order_id,
                "order_type": new_resp.order_type,
                "reduce_only": false
            }),
            ts,
        )
        .await;

        if new_resp.order_type.trim().eq_ignore_ascii_case("MARKET") {
            emit_runtime_event_best_effort(
                state,
                cfg,
                EVENT_CANCEL_REPLACE_USED_MARKET_FALLBACK,
                &symbol,
                &desired_side_text,
                risk_level,
                "order-reconciler",
                "replace-fallback-market",
                cycle_correlation_id,
                json!({
                    "intent_key": intent_key,
                    "old_exchange_order_id": exchange_order_id,
                    "new_exchange_order_id": new_resp.order_id
                }),
                ts,
            )
            .await;
        }

        emit_runtime_event_best_effort(
            state,
            cfg,
            EVENT_CANCEL_REPLACE_SUCCEEDED,
            &symbol,
            &desired_side_text,
            risk_level,
            "order-reconciler",
            "replace-open-limit",
            cycle_correlation_id,
            json!({
                "old_exchange_order_id": exchange_order_id,
                "new_exchange_order_id": new_resp.order_id,
                "remaining_qty": normalized_qty,
                "mark_price": mark_price
            }),
            ts,
        )
        .await;
    }

    Ok(())
}

pub async fn has_recent_replace_attempt(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    symbol: &str,
    side: &str,
    now_ts: i64,
    window_secs: i64,
    max_attempts: i64,
) -> Result<bool, AppError> {
    let threshold = now_ts.saturating_sub(window_secs.max(30));
    let attempts = state
        .trading_repo
        .count_recent_execution_intents(
            &cfg.trader_id,
            "replace-open-limit",
            symbol,
            side,
            threshold,
        )
        .await
        .unwrap_or(0);

    Ok(attempts >= max_attempts.max(1))
}

pub async fn sync_open_orders_from_exchange(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    adapter: &dyn LiveExchangeAdapter,
    ts: i64,
) -> Result<(), AppError> {
    let open_orders = get_open_orders_with_retry(adapter).await?;

    for o in &open_orders {
        let normalized_status = normalize_order_status(&o.status);
        let normalized_side = o.side.trim().to_uppercase();
        let normalized_position_side = normalize_order_position_side(&o.position_side, &o.side);
        let existing = state
            .trading_repo
            .order_by_exchange_order_id(&cfg.trader_id, &o.order_id)
            .await?;

        let local_order_id = if let Some(existing) = existing {
            let effective_status =
                resolve_order_status_transition(&existing.status, &normalized_status);
            let close_ts = if is_terminal_order_status(&effective_status) {
                Some(ts)
            } else {
                None
            };

            let preserved_closed_at = existing.closed_at.or(close_ts);
            let order_id = existing.id.clone();
            state
                .trading_repo
                .update_order(UpdateTraderOrderRecord {
                    id: existing.id,
                    client_order_id: o.client_order_id.clone(),
                    symbol: o.symbol.trim().to_uppercase(),
                    side: normalized_side.clone(),
                    position_side: normalized_position_side.clone(),
                    order_type: o.order_type.to_ascii_lowercase(),
                    status: effective_status,
                    price: o.price,
                    quantity: o.orig_qty,
                    filled_quantity: o.executed_qty,
                    avg_fill_price: if o.executed_qty > 0.0 { o.price } else { 0.0 },
                    reduce_only: o.reduce_only,
                    updated_at: ts,
                    closed_at: preserved_closed_at,
                })
                .await?;
            order_id
        } else {
            let id = Uuid::now_v7().to_string();
            state
                .trading_repo
                .insert_order(InsertTraderOrderRecord {
                    id: id.clone(),
                    trader_id: cfg.trader_id.clone(),
                    exchange_order_id: o.order_id.clone(),
                    client_order_id: o.client_order_id.clone(),
                    symbol: o.symbol.trim().to_uppercase(),
                    side: normalized_side.clone(),
                    position_side: normalized_position_side.clone(),
                    order_type: o.order_type.to_ascii_lowercase(),
                    status: normalized_status.clone(),
                    price: o.price,
                    quantity: o.orig_qty,
                    filled_quantity: o.executed_qty,
                    avg_fill_price: if o.executed_qty > 0.0 { o.price } else { 0.0 },
                    reduce_only: o.reduce_only,
                    time_in_force: String::new(),
                    placed_at: ts,
                    updated_at: ts,
                    closed_at: if is_terminal_order_status(&normalized_status) {
                        Some(ts)
                    } else {
                        None
                    },
                })
                .await?;
            id
        };

        let (filled_qty, avg_price) = ingest_live_fills_for_order(
            state,
            cfg,
            adapter,
            &local_order_id,
            &o.symbol,
            &o.order_id,
            &o.side,
            ts,
        )
        .await?;

        if filled_qty > 0.0 {
            state
                .trading_repo
                .update_order_fill_summary(&local_order_id, filled_qty, avg_price, ts)
                .await?;
        }

        finalize_execution_intent_for_exchange_order(
            state,
            cfg,
            &o.order_id,
            &normalized_status,
            ts,
        )
        .await?;
    }

    Ok(())
}

pub async fn sync_terminal_orders_from_exchange(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    adapter: &dyn LiveExchangeAdapter,
    ts: i64,
) -> Result<(), AppError> {
    let rows = state
        .trading_repo
        .active_orders_for_reconciliation(&cfg.trader_id, 200)
        .await?;

    for row in rows {
        let local_order_id = row.id.clone();
        let exchange_order_id = row.exchange_order_id.clone();
        let symbol = row.symbol.clone();
        let side = row.side.clone();

        let detail = get_order_with_retry(adapter, &symbol, &exchange_order_id).await?;
        let normalized_status = normalize_order_status(&detail.status);
        let normalized_side = detail.side.trim().to_uppercase();
        let normalized_position_side =
            normalize_order_position_side(&detail.position_side, &detail.side);
        let effective_status = resolve_order_status_transition(&row.status, &normalized_status);
        let close_ts = if is_terminal_order_status(&effective_status) {
            Some(ts)
        } else {
            None
        };

        state
            .trading_repo
            .update_order(UpdateTraderOrderRecord {
                id: row.id.clone(),
                client_order_id: row.client_order_id.clone(),
                symbol: detail.symbol.trim().to_uppercase(),
                side: normalized_side.clone(),
                position_side: normalized_position_side.clone(),
                order_type: detail.order_type.trim().to_ascii_lowercase(),
                status: effective_status,
                price: detail.price,
                quantity: detail.orig_qty,
                filled_quantity: detail.executed_qty,
                avg_fill_price: if detail.executed_qty > 0.0 {
                    detail.price
                } else {
                    0.0
                },
                reduce_only: detail.reduce_only,
                updated_at: ts,
                closed_at: row.closed_at.or(close_ts),
            })
            .await?;

        let (filled_qty, avg_price) = ingest_live_fills_for_order(
            state,
            cfg,
            adapter,
            &local_order_id,
            &symbol,
            &exchange_order_id,
            &side,
            ts,
        )
        .await?;

        if filled_qty > 0.0 {
            state
                .trading_repo
                .update_order_fill_summary(&local_order_id, filled_qty, avg_price, ts)
                .await?;
        }

        finalize_execution_intent_for_exchange_order(
            state,
            cfg,
            &exchange_order_id,
            &normalized_status,
            ts,
        )
        .await?;
    }

    Ok(())
}

pub async fn reconcile_local_positions_from_terminal_reduce_only_orders(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    ts: i64,
) -> Result<(), AppError> {
    let rows = state
        .trading_repo
        .reduce_only_filled_orders(&cfg.trader_id, 200)
        .await?;

    for row in rows {
        let exchange_order_id = row.exchange_order_id.clone();
        let symbol = row.symbol.clone();
        let position_side_raw = row.position_side.clone();
        let side_raw = row.side.clone();
        let filled_qty = row.filled_quantity;
        let avg_fill_price = row.avg_fill_price;
        let order_updated_at = row.updated_at;

        if filled_qty <= f64::EPSILON {
            continue;
        }

        let decision_reason = format!("compensation-close:{}", exchange_order_id);
        if state
            .trading_repo
            .system_decision_exists(&cfg.trader_id, &decision_reason)
            .await?
        {
            continue;
        }

        let position_side = match position_side_raw.trim().to_ascii_uppercase().as_str() {
            "LONG" => "LONG",
            "SHORT" => "SHORT",
            _ => match side_raw.trim().to_ascii_uppercase().as_str() {
                "SELL" => "LONG",
                "BUY" => "SHORT",
                _ => continue,
            },
        };

        let exit_price = if avg_fill_price > 0.0 {
            avg_fill_price
        } else {
            0.0
        };

        let px = if exit_price > 0.0 { exit_price } else { 0.0 };
        let applied_qty = apply_close_fill_to_open_positions(
            state,
            cfg,
            &symbol,
            position_side,
            filled_qty,
            px.max(1e-9),
            (px.max(1e-9) * filled_qty * 0.0004).max(0.0),
            ts,
            ts,
        )
        .await?;

        if applied_qty > 0.0 {
            state
                .trading_repo
                .insert_decision(InsertTraderDecisionRecord {
                    id: Uuid::now_v7().to_string(),
                    trader_id: cfg.trader_id.clone(),
                    symbol: symbol.trim().to_uppercase(),
                    timeframe: "3m".to_string(),
                    decision: "SYSTEM".to_string(),
                    confidence: 1.0,
                    reason: decision_reason,
                    payload_json: json!({
                        "source": "terminal_reduce_only_compensation",
                        "exchange_order_id": exchange_order_id,
                        "position_side": position_side,
                        "filled_quantity": filled_qty,
                        "applied_quantity": applied_qty,
                        "remaining_quantity": (filled_qty - applied_qty).max(0.0),
                        "avg_fill_price": avg_fill_price,
                        "order_updated_at": order_updated_at,
                        "risk_level": "normal",
                        "trigger_source": "order-reconciler",
                        "action_taken": "terminal-reduce-only-compensation",
                        "correlation_id": format!(
                            "reconcile:{}:{}:{}",
                            cfg.trader_id,
                            exchange_order_id,
                            ts
                        )
                    })
                    .to_string(),
                    created_at: ts,
                })
                .await?;
        }
    }

    Ok(())
}

pub async fn persist_live_order_record(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    adapter: &dyn LiveExchangeAdapter,
    order: &crate::clients::exchanges::PlaceOrderResponse,
    position_side: &str,
    reduce_only: bool,
    ts: i64,
) -> Result<(), AppError> {
    let status = normalize_order_status(&order.status);
    let closed_at = if is_terminal_order_status(&status) {
        Some(ts)
    } else {
        None
    };

    let symbol = order.symbol.trim().to_uppercase();
    let order_side = order.side.trim().to_uppercase();
    let position_side_norm = position_side.trim().to_uppercase();

    let existing_order = state
        .trading_repo
        .order_by_exchange_order_id(&cfg.trader_id, &order.order_id)
        .await?;

    let local_order_id = if let Some(existing) = existing_order {
        let id = existing.id.clone();
        state
            .trading_repo
            .update_order(UpdateTraderOrderRecord {
                id: existing.id,
                client_order_id: order.client_order_id.clone(),
                symbol: symbol.clone(),
                side: order_side.clone(),
                position_side: position_side_norm.clone(),
                order_type: order.order_type.trim().to_ascii_lowercase(),
                status: status.clone(),
                price: order.price,
                quantity: order.orig_qty,
                filled_quantity: order.executed_qty,
                avg_fill_price: if order.executed_qty > 0.0 {
                    order.price
                } else {
                    0.0
                },
                reduce_only,
                updated_at: ts,
                closed_at,
            })
            .await?;
        id
    } else {
        let id = Uuid::now_v7().to_string();
        state
            .trading_repo
            .insert_order(InsertTraderOrderRecord {
                id: id.clone(),
                trader_id: cfg.trader_id.clone(),
                exchange_order_id: order.order_id.clone(),
                client_order_id: order.client_order_id.clone(),
                symbol: symbol.clone(),
                side: order_side.clone(),
                position_side: position_side_norm.clone(),
                order_type: order.order_type.trim().to_ascii_lowercase(),
                status: status.clone(),
                price: order.price,
                quantity: order.orig_qty,
                filled_quantity: order.executed_qty,
                avg_fill_price: if order.executed_qty > 0.0 {
                    order.price
                } else {
                    0.0
                },
                reduce_only,
                time_in_force: String::new(),
                placed_at: ts,
                updated_at: ts,
                closed_at,
            })
            .await?;
        id
    };

    let (filled_qty, avg_fill_price) = ingest_live_fills_for_order(
        state,
        cfg,
        adapter,
        &local_order_id,
        &symbol,
        &order.order_id,
        &order_side,
        ts,
    )
    .await?;

    if filled_qty > 0.0 {
        state
            .trading_repo
            .update_order_fill_summary(&local_order_id, filled_qty, avg_fill_price, ts)
            .await?;
    }

    if reduce_only && status == "filled" && order.executed_qty > 0.0 {
        let _ = apply_close_fill_to_open_positions(
            state,
            cfg,
            &symbol,
            &position_side_norm,
            order.executed_qty,
            avg_fill_price.max(1e-9),
            (avg_fill_price.max(1e-9) * order.executed_qty * 0.0004).max(0.0),
            ts,
            ts,
        )
        .await?;
    }

    Ok(())
}

pub async fn ingest_live_fills_for_order(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    adapter: &dyn LiveExchangeAdapter,
    local_order_id: &str,
    symbol: &str,
    exchange_order_id: &str,
    default_side: &str,
    ts: i64,
) -> Result<(f64, f64), AppError> {
    let fills = get_order_fills_with_retry(adapter, symbol, exchange_order_id)
        .await
        .unwrap_or_default();

    for fill in fills {
        let existed = state
            .trading_repo
            .order_fill_exists(&cfg.trader_id, &fill.trade_id)
            .await?;

        if existed {
            continue;
        }

        state
            .trading_repo
            .insert_order_fill(InsertOrderFillRecord {
                id: Uuid::now_v7().to_string(),
                order_id: local_order_id.to_string(),
                trader_id: cfg.trader_id.clone(),
                exchange_trade_id: fill.trade_id.clone(),
                symbol: fill.symbol.trim().to_uppercase(),
                side: if fill.side.trim().is_empty() {
                    default_side.trim().to_uppercase()
                } else {
                    fill.side.trim().to_uppercase()
                },
                price: fill.price,
                quantity: fill.quantity,
                fee: fill.fee,
                fee_asset: if fill.fee_asset.trim().is_empty() {
                    "USDT".to_string()
                } else {
                    fill.fee_asset.trim().to_uppercase()
                },
                realized_pnl: fill.realized_pnl,
                executed_at: if fill.executed_at > 0 {
                    fill.executed_at
                } else {
                    ts
                },
                created_at: ts,
            })
            .await?;
    }

    Ok(state
        .trading_repo
        .order_fill_summary(&cfg.trader_id, local_order_id)
        .await?)
}

pub fn normalize_order_status(status: &str) -> String {
    match status.to_ascii_uppercase().as_str() {
        "NEW" => "new".to_string(),
        "PARTIALLY_FILLED" => "partially_filled".to_string(),
        "FILLED" => "filled".to_string(),
        "CANCELED" => "canceled".to_string(),
        "REJECTED" => "rejected".to_string(),
        "EXPIRED" => "expired".to_string(),
        _ => "open".to_string(),
    }
}

pub fn is_terminal_order_status(status: &str) -> bool {
    matches!(status, "filled" | "canceled" | "rejected" | "expired")
}

pub fn resolve_order_status_transition(current_status: &str, incoming_status: &str) -> String {
    let normalize = |s: &str| {
        let n = s.trim().to_ascii_lowercase();
        match n.as_str() {
            "new" => "new".to_string(),
            "open" => "new".to_string(),
            "partially_filled" => "partially_filled".to_string(),
            "filled" => "filled".to_string(),
            "canceled" => "canceled".to_string(),
            "rejected" => "rejected".to_string(),
            "expired" => "expired".to_string(),
            _ => n,
        }
    };

    let current = normalize(current_status);
    let incoming = normalize(incoming_status);

    if current.is_empty() {
        return if incoming.is_empty() {
            "open".to_string()
        } else {
            incoming
        };
    }

    if incoming.is_empty() {
        return current;
    }

    if is_terminal_order_status(&current) {
        return current;
    }

    if is_terminal_order_status(&incoming) {
        return incoming;
    }

    if current == "partially_filled" && (incoming == "new" || incoming == "open") {
        return current;
    }

    if current == "new" && incoming == "open" {
        return current;
    }

    incoming
}
