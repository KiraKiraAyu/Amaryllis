use super::service::*;
use crate::repositories::trading::records::{
    history::InsertTraderDecisionRecord,
    orders::{InsertOrderFillRecord, InsertTraderOrderRecord},
    positions::InsertTraderPositionRecord,
};

pub async fn execute_decisions(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    decisions: &[DecisionSignal],
    open_positions: &[PositionView],
    metrics: &AccountMetrics,
    market: &HashMap<String, MarketState>,
    ts: i64,
) -> Result<(), AppError> {
    let mut by_symbol: HashMap<String, Vec<&PositionView>> = HashMap::new();
    for p in open_positions {
        by_symbol.entry(p.symbol.clone()).or_default().push(p);
    }

    let mut open_count = open_positions.len() as i64;
    let max_positions = 8_i64;

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
            // close opposite positions first
            for p in existing.iter().filter(|x| x.side == opposite_side) {
                close_position(state, cfg, p, price, ts, "decision reversal").await?;
                open_count = (open_count - 1).max(0);
            }

            // if same-side already exists, skip add
            if existing.iter().any(|x| x.side == desired_side) {
                continue;
            }
        }

        if open_count >= max_positions {
            continue;
        }

        if metrics.available_balance <= 5.0 || metrics.margin_used_ratio > 0.75 {
            continue;
        }

        let leverage = leverage_for_symbol(cfg, &d.symbol).max(1);
        let mut risk_budget = (metrics.total_balance * 0.06).max(5.0); // 6% capital-at-risk per position
        if let Some(sym_cfg) = cfg
            .symbols_config
            .iter()
            .find(|s| s.symbol.to_uppercase() == d.symbol.to_uppercase())
        {
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
        let qty = (risk_budget * leverage as f64 / price).max(0.0001);

        open_position(
            state,
            cfg,
            &d.symbol,
            desired_side,
            price,
            qty,
            leverage,
            ts,
        )
        .await?;
        open_count += 1;
    }

    Ok(())
}

pub async fn close_worst_positions(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    open_positions: &[PositionView],
    market: &HashMap<String, MarketState>,
    ts: i64,
) -> Result<(), AppError> {
    // close up to 2 worst positions by unrealized pnl to reduce risk quickly
    let mut scored = Vec::new();
    for p in open_positions {
        let px = market
            .get(&p.symbol)
            .map(|m| m.price)
            .unwrap_or(p.mark_price.max(1e-9));
        let upnl = if p.side == "LONG" {
            (px - p.entry_price) * p.quantity
        } else {
            (p.entry_price - px) * p.quantity
        };
        scored.push((upnl, p, px));
    }
    scored.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    for (_, p, px) in scored.into_iter().take(2) {
        close_position(state, cfg, p, px, ts, "risk reduction").await?;
    }

    Ok(())
}

pub async fn open_position(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    symbol: &str,
    side: &str,
    price: f64,
    quantity: f64,
    leverage: i64,
    ts: i64,
) -> Result<(), AppError> {
    let order_id = Uuid::now_v7().to_string();
    let fill_id = Uuid::now_v7().to_string();
    let pos_id = Uuid::now_v7().to_string();
    let order_side = if side == "LONG" { "BUY" } else { "SELL" };

    state
        .trading_repo
        .insert_order(InsertTraderOrderRecord {
            id: order_id.clone(),
            trader_id: cfg.trader_id.clone(),
            exchange_order_id: format!("sim-ex-{}", &order_id[..8]),
            client_order_id: format!("sim-cl-{}", &order_id[..8]),
            symbol: symbol.to_string(),
            side: order_side.to_string(),
            position_side: side.to_string(),
            order_type: "market".to_string(),
            status: "filled".to_string(),
            price,
            quantity,
            filled_quantity: quantity,
            avg_fill_price: price,
            reduce_only: false,
            time_in_force: "IOC".to_string(),
            placed_at: ts,
            updated_at: ts,
            closed_at: Some(ts),
        })
        .await?;

    let fee = (price * quantity * 0.0004).max(0.0);
    state
        .trading_repo
        .insert_order_fill(InsertOrderFillRecord {
            id: fill_id,
            order_id: order_id.clone(),
            trader_id: cfg.trader_id.clone(),
            exchange_trade_id: format!("sim-tr-{}", &order_id[..8]),
            symbol: symbol.to_string(),
            side: order_side.to_string(),
            price,
            quantity,
            fee,
            fee_asset: "USDT".to_string(),
            realized_pnl: 0.0,
            executed_at: ts,
            created_at: ts,
        })
        .await?;

    state
        .trading_repo
        .insert_position(InsertTraderPositionRecord {
            id: pos_id,
            trader_id: cfg.trader_id.clone(),
            symbol: symbol.to_string(),
            side: side.to_string(),
            quantity,
            entry_price: price,
            mark_price: price,
            liquidation_price: liquidation_price(side, price, leverage),
            leverage,
            margin_mode: "cross".to_string(),
            unrealized_pnl: 0.0,
            realized_pnl: 0.0,
            status: "open".to_string(),
            opened_at: ts,
            closed_at: None,
            created_at: ts,
            updated_at: ts,
        })
        .await?;

    Ok(())
}

pub async fn close_position(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    p: &PositionView,
    exit_price: f64,
    ts: i64,
    reason: &str,
) -> Result<(), AppError> {
    let pnl = if p.side == "LONG" {
        (exit_price - p.entry_price) * p.quantity
    } else {
        (p.entry_price - exit_price) * p.quantity
    };

    let fee = (exit_price * p.quantity * 0.0004).max(0.0);
    let net_pnl = pnl - fee;
    let roi_pct = if p.entry_price.abs() > f64::EPSILON {
        ((exit_price - p.entry_price) / p.entry_price)
            * 100.0
            * if p.side == "LONG" { 1.0 } else { -1.0 }
    } else {
        0.0
    };

    state
        .trading_repo
        .close_position(&cfg.trader_id, &p.id, exit_price, net_pnl, ts)
        .await?;

    insert_trade_record(
        state,
        cfg,
        &p.symbol,
        &p.side,
        p.entry_price,
        exit_price,
        p.quantity,
        net_pnl,
        fee,
        roi_pct,
        p.opened_at,
        ts,
        ts,
    )
    .await?;

    state
        .trading_repo
        .insert_decision(InsertTraderDecisionRecord {
            id: Uuid::now_v7().to_string(),
            trader_id: cfg.trader_id.clone(),
            symbol: p.symbol.clone(),
            timeframe: "3m".to_string(),
            decision: "CLOSE".to_string(),
            confidence: 0.9,
            reason: format!("{} ({})", reason, p.side),
            payload_json: json!({
                "entry_price": p.entry_price,
                "exit_price": exit_price,
                "quantity": p.quantity,
                "realized_pnl": net_pnl,
                "risk_level": "normal",
                "trigger_source": "decision-engine",
                "action_taken": "close-position",
                "correlation_id": format!("close:{}:{}:{}", cfg.trader_id, p.symbol, ts)
            })
            .to_string(),
            created_at: ts,
        })
        .await?;

    Ok(())
}
