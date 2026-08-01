use super::service::*;
use crate::repositories::trading::records::history::InsertTraderTradeRecord;

pub async fn insert_trade_record(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    symbol: &str,
    side: &str,
    entry_price: f64,
    exit_price: f64,
    quantity: f64,
    realized_pnl: f64,
    fees: f64,
    roi_pct: f64,
    opened_at: i64,
    closed_at: i64,
    created_at: i64,
) -> Result<(), AppError> {
    state
        .trading_repo
        .insert_trade(InsertTraderTradeRecord {
            id: Uuid::now_v7().to_string(),
            trader_id: cfg.trader_id.clone(),
            symbol: symbol.trim().to_uppercase(),
            side: side.trim().to_uppercase(),
            entry_price,
            exit_price,
            quantity,
            realized_pnl,
            fees,
            roi_pct,
            opened_at,
            closed_at,
            created_at,
        })
        .await?;

    Ok(())
}

pub async fn apply_close_fill_to_open_positions(
    state: &SharedState,
    cfg: &TraderRuntimeConfig,
    symbol: &str,
    position_side: &str,
    fill_qty: f64,
    fill_price: f64,
    fill_fee: f64,
    trade_time: i64,
    ts: i64,
) -> Result<f64, AppError> {
    Ok(state
        .trading_repo
        .apply_close_fill_to_open_positions(
            &cfg.trader_id,
            symbol,
            position_side,
            fill_qty,
            fill_price,
            fill_fee,
            trade_time,
            ts,
        )
        .await?)
}
