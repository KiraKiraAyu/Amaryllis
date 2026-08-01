use super::service::*;

pub async fn statistics(
    app: &SharedState,
    q: StatisticsQuery,
) -> AppResult<TraderStatisticsPayload> {
    let trader_id = match resolve_trader_id(app, q.trader_id).await {
        Ok(v) => v,
        Err(e) => return Err(e),
    };

    let days = q.days.unwrap_or(30).clamp(1, 3650);
    let from_ts = now_ts() - days * 86_400;

    let stats = app
        .trading_repo
        .statistics(&trader_id, from_ts)
        .await
        .unwrap_or_default();

    let win_rate = if stats.total_trades > 0 {
        (stats.winning_trades as f64 * 100.0) / (stats.total_trades as f64)
    } else {
        0.0
    };

    Ok(TraderStatisticsPayload {
        trader_id,
        period_days: days,
        total_trades: stats.total_trades,
        winning_trades: stats.winning_trades,
        win_rate_pct: win_rate,
        total_realized_pnl: stats.total_realized_pnl,
        total_fees: stats.total_fees,
        net_pnl: stats.total_realized_pnl - stats.total_fees,
        avg_roi_pct: stats.avg_roi_pct,
        open_positions: stats.open_positions,
    })
}
