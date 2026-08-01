use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};

use crate::{database::DbErr, entity, time::ts_to_dt};

use super::{
    TradingRepo,
    mappers::history::map_trade,
    records::history::{InsertTraderTradeRecord, TraderStatisticsRecord, TraderTradeRecord},
    values::{decimal_from_f64, decimal_to_f64},
};

impl TradingRepo {
    pub async fn insert_trade(&self, input: InsertTraderTradeRecord) -> Result<(), DbErr> {
        entity::trader_trades::ActiveModel {
            id: Set(input.id),
            trader_id: Set(input.trader_id),
            symbol: Set(input.symbol),
            side: Set(input.side),
            entry_price: Set(decimal_from_f64(input.entry_price)),
            exit_price: Set(decimal_from_f64(input.exit_price)),
            quantity: Set(decimal_from_f64(input.quantity)),
            realized_pnl: Set(decimal_from_f64(input.realized_pnl)),
            fees: Set(decimal_from_f64(input.fees)),
            roi_pct: Set(decimal_from_f64(input.roi_pct)),
            opened_at: Set(ts_to_dt(input.opened_at)),
            closed_at: Set(ts_to_dt(input.closed_at)),
            created_at: Set(ts_to_dt(input.created_at)),
        }
        .insert(&self.db)
        .await
        .map(|_| ())
    }

    pub async fn trades(
        &self,
        trader_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<TraderTradeRecord>, DbErr> {
        entity::trader_trades::Entity::find()
            .filter(entity::trader_trades::Column::TraderId.eq(trader_id.trim()))
            .order_by_desc(entity::trader_trades::Column::ClosedAt)
            .limit(limit.max(0) as u64)
            .offset(offset.max(0) as u64)
            .all(&self.db)
            .await
            .map(|rows| rows.into_iter().map(map_trade).collect())
    }

    pub async fn statistics(
        &self,
        trader_id: &str,
        from_ts: i64,
    ) -> Result<TraderStatisticsRecord, DbErr> {
        let trades = entity::trader_trades::Entity::find()
            .filter(entity::trader_trades::Column::TraderId.eq(trader_id.trim()))
            .filter(entity::trader_trades::Column::ClosedAt.gte(ts_to_dt(from_ts)))
            .all(&self.db)
            .await?;

        let total_trades = trades.len() as i64;
        let winning_trades = trades
            .iter()
            .filter(|row| decimal_to_f64(&row.realized_pnl) > 0.0)
            .count() as i64;
        let total_realized_pnl: f64 = trades
            .iter()
            .map(|row| decimal_to_f64(&row.realized_pnl))
            .sum();
        let total_fees: f64 = trades.iter().map(|row| decimal_to_f64(&row.fees)).sum();
        let avg_roi_pct = if trades.is_empty() {
            0.0
        } else {
            trades
                .iter()
                .map(|row| decimal_to_f64(&row.roi_pct))
                .sum::<f64>()
                / trades.len() as f64
        };
        let open_positions = entity::trader_positions::Entity::find()
            .filter(entity::trader_positions::Column::TraderId.eq(trader_id.trim()))
            .filter(entity::trader_positions::Column::Status.eq("open"))
            .count(&self.db)
            .await? as i64;

        Ok(TraderStatisticsRecord {
            total_trades,
            winning_trades,
            total_realized_pnl,
            total_fees,
            avg_roi_pct,
            open_positions,
        })
    }
}
