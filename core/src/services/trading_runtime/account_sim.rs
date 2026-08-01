use super::service::*;
use crate::repositories::trading::records::accounts::TraderAccountRecord;

#[derive(Debug, Clone)]
pub struct AccountMetrics {
    pub total_balance: f64,
    pub available_balance: f64,
    pub used_margin: f64,
    pub unrealized_pnl: f64,
    pub realized_pnl: f64,
    pub margin_used_ratio: f64,
}

pub async fn compute_account_metrics(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
) -> Result<AccountMetrics, AppError> {
    let (used_margin, unrealized_pnl, realized_pnl) = state
        .trading_repo
        .compute_account_totals(&cfg.trader_id)
        .await?;

    let total_balance = (cfg.initial_balance + realized_pnl + unrealized_pnl).max(0.0);
    let available_balance = (total_balance - used_margin).max(0.0);
    let margin_used_ratio = if total_balance > 0.0 {
        used_margin / total_balance
    } else {
        0.0
    };

    Ok(AccountMetrics {
        total_balance,
        available_balance,
        used_margin,
        unrealized_pnl,
        realized_pnl,
        margin_used_ratio,
    })
}

pub async fn insert_account_snapshot(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    m: &AccountMetrics,
    ts: i64,
) -> Result<(), AppError> {
    state
        .trading_repo
        .insert_account_snapshot(
            Uuid::now_v7().to_string(),
            &cfg.trader_id,
            &cfg.exchange_id,
            &TraderAccountRecord {
                trader_id: cfg.trader_id.clone(),
                total_balance: m.total_balance,
                available_balance: m.available_balance,
                used_margin: m.used_margin,
                unrealized_pnl: m.unrealized_pnl,
                realized_pnl: m.realized_pnl,
                currency: "USDT".to_string(),
                snapshot_at: ts,
            },
            ts,
        )
        .await?;

    Ok(())
}

pub async fn load_open_positions(
    state: &SharedState,
    trader_id: &str,
) -> Result<Vec<PositionView>, AppError> {
    let rows = state
        .trading_repo
        .open_position_records(trader_id, None, None)
        .await?;

    Ok(rows.into_iter().map(position_view_from_record).collect())
}

pub async fn mark_to_market_positions(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    positions: &mut [PositionView],
    market: &HashMap<String, MarketState>,
    ts: i64,
) -> Result<(), AppError> {
    for p in positions {
        let price = market
            .get(&p.symbol)
            .map(|m| m.price)
            .unwrap_or_else(|| p.mark_price.max(1e-9));

        let upnl = if p.side == "LONG" {
            (price - p.entry_price) * p.quantity
        } else {
            (p.entry_price - price) * p.quantity
        };

        state
            .trading_repo
            .update_position_mark_to_market(&cfg.trader_id, &p.id, price, upnl, ts)
            .await?;
    }
    Ok(())
}
