use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, prelude::Expr,
};

use crate::{
    database::DbErr,
    entity,
    time::{dt_to_ts, ts_to_dt},
};

use super::{
    TradingRepo,
    mappers::orders::map_order,
    records::orders::{InsertTraderOrderRecord, TraderOrderRecord, UpdateTraderOrderRecord},
    values::decimal_from_f64,
};

impl TradingRepo {
    pub async fn insert_order(&self, input: InsertTraderOrderRecord) -> Result<(), DbErr> {
        entity::trader_orders::ActiveModel {
            id: Set(input.id),
            trader_id: Set(input.trader_id),
            exchange_order_id: Set(input.exchange_order_id),
            client_order_id: Set(input.client_order_id),
            symbol: Set(input.symbol),
            side: Set(input.side),
            position_side: Set(input.position_side),
            order_type: Set(input.order_type),
            status: Set(input.status),
            price: Set(decimal_from_f64(input.price)),
            quantity: Set(decimal_from_f64(input.quantity)),
            filled_quantity: Set(decimal_from_f64(input.filled_quantity)),
            avg_fill_price: Set(decimal_from_f64(input.avg_fill_price)),
            reduce_only: Set(if input.reduce_only { 1 } else { 0 }),
            time_in_force: Set(input.time_in_force),
            placed_at: Set(ts_to_dt(input.placed_at)),
            updated_at: Set(ts_to_dt(input.updated_at)),
            closed_at: Set(input.closed_at.map(ts_to_dt)),
        }
        .insert(&self.db)
        .await
        .map(|_| ())
    }

    pub async fn stale_live_open_orders(
        &self,
        trader_id: &str,
        threshold_ts: i64,
        limit: u64,
    ) -> Result<Vec<TraderOrderRecord>, DbErr> {
        entity::trader_orders::Entity::find()
            .filter(entity::trader_orders::Column::TraderId.eq(trader_id.trim()))
            .filter(entity::trader_orders::Column::ReduceOnly.eq(0))
            .filter(entity::trader_orders::Column::OrderType.ne("limit"))
            .filter(entity::trader_orders::Column::ExchangeOrderId.ne(""))
            .filter(entity::trader_orders::Column::Status.is_in([
                "new",
                "open",
                "partially_filled",
            ]))
            .filter(entity::trader_orders::Column::PlacedAt.lte(ts_to_dt(threshold_ts)))
            .order_by_asc(entity::trader_orders::Column::PlacedAt)
            .limit(limit)
            .all(&self.db)
            .await
            .map(|rows| rows.into_iter().map(map_order).collect())
    }

    pub async fn stale_limit_live_open_orders(
        &self,
        trader_id: &str,
        threshold_ts: i64,
        limit: u64,
    ) -> Result<Vec<TraderOrderRecord>, DbErr> {
        entity::trader_orders::Entity::find()
            .filter(entity::trader_orders::Column::TraderId.eq(trader_id.trim()))
            .filter(entity::trader_orders::Column::ReduceOnly.eq(0))
            .filter(entity::trader_orders::Column::OrderType.eq("limit"))
            .filter(entity::trader_orders::Column::ExchangeOrderId.ne(""))
            .filter(entity::trader_orders::Column::Status.is_in([
                "new",
                "open",
                "partially_filled",
            ]))
            .filter(entity::trader_orders::Column::PlacedAt.lte(ts_to_dt(threshold_ts)))
            .order_by_asc(entity::trader_orders::Column::PlacedAt)
            .limit(limit)
            .all(&self.db)
            .await
            .map(|rows| rows.into_iter().map(map_order).collect())
    }

    pub async fn active_orders_for_reconciliation(
        &self,
        trader_id: &str,
        limit: u64,
    ) -> Result<Vec<TraderOrderRecord>, DbErr> {
        entity::trader_orders::Entity::find()
            .filter(entity::trader_orders::Column::TraderId.eq(trader_id.trim()))
            .filter(entity::trader_orders::Column::ExchangeOrderId.ne(""))
            .filter(entity::trader_orders::Column::Status.is_in([
                "open",
                "new",
                "partially_filled",
            ]))
            .order_by_asc(entity::trader_orders::Column::UpdatedAt)
            .limit(limit)
            .all(&self.db)
            .await
            .map(|rows| rows.into_iter().map(map_order).collect())
    }

    pub async fn reduce_only_filled_orders(
        &self,
        trader_id: &str,
        limit: u64,
    ) -> Result<Vec<TraderOrderRecord>, DbErr> {
        entity::trader_orders::Entity::find()
            .filter(entity::trader_orders::Column::TraderId.eq(trader_id.trim()))
            .filter(entity::trader_orders::Column::ReduceOnly.eq(1))
            .filter(entity::trader_orders::Column::Status.eq("filled"))
            .filter(entity::trader_orders::Column::FilledQuantity.gt(decimal_from_f64(0.0)))
            .filter(entity::trader_orders::Column::ExchangeOrderId.ne(""))
            .order_by_desc(entity::trader_orders::Column::UpdatedAt)
            .limit(limit)
            .all(&self.db)
            .await
            .map(|rows| rows.into_iter().map(map_order).collect())
    }

    pub async fn update_order_fill_summary(
        &self,
        order_id: &str,
        filled_quantity: f64,
        avg_fill_price: f64,
        updated_at: i64,
    ) -> Result<(), DbErr> {
        entity::trader_orders::Entity::update_many()
            .col_expr(
                entity::trader_orders::Column::FilledQuantity,
                Expr::value(decimal_from_f64(filled_quantity)),
            )
            .col_expr(
                entity::trader_orders::Column::AvgFillPrice,
                Expr::value(decimal_from_f64(avg_fill_price)),
            )
            .col_expr(
                entity::trader_orders::Column::UpdatedAt,
                Expr::value(ts_to_dt(updated_at)),
            )
            .filter(entity::trader_orders::Column::Id.eq(order_id.trim()))
            .exec(&self.db)
            .await
            .map(|_| ())
    }

    pub async fn update_order_status(
        &self,
        trader_id: &str,
        order_id: &str,
        status: &str,
        updated_at: i64,
        closed_at: Option<i64>,
    ) -> Result<(), DbErr> {
        entity::trader_orders::Entity::update_many()
            .col_expr(
                entity::trader_orders::Column::Status,
                Expr::value(status.to_string()),
            )
            .col_expr(
                entity::trader_orders::Column::UpdatedAt,
                Expr::value(ts_to_dt(updated_at)),
            )
            .col_expr(
                entity::trader_orders::Column::ClosedAt,
                Expr::value(closed_at.map(ts_to_dt)),
            )
            .filter(entity::trader_orders::Column::Id.eq(order_id.trim()))
            .filter(entity::trader_orders::Column::TraderId.eq(trader_id.trim()))
            .exec(&self.db)
            .await
            .map(|_| ())
    }

    pub async fn order_by_exchange_order_id(
        &self,
        trader_id: &str,
        exchange_order_id: &str,
    ) -> Result<Option<TraderOrderRecord>, DbErr> {
        entity::trader_orders::Entity::find()
            .filter(entity::trader_orders::Column::TraderId.eq(trader_id.trim()))
            .filter(entity::trader_orders::Column::ExchangeOrderId.eq(exchange_order_id.trim()))
            .one(&self.db)
            .await
            .map(|row| row.map(map_order))
    }

    pub async fn update_order(&self, input: UpdateTraderOrderRecord) -> Result<(), DbErr> {
        entity::trader_orders::Entity::update_many()
            .col_expr(
                entity::trader_orders::Column::ClientOrderId,
                Expr::value(input.client_order_id),
            )
            .col_expr(
                entity::trader_orders::Column::Symbol,
                Expr::value(input.symbol),
            )
            .col_expr(entity::trader_orders::Column::Side, Expr::value(input.side))
            .col_expr(
                entity::trader_orders::Column::PositionSide,
                Expr::value(input.position_side),
            )
            .col_expr(
                entity::trader_orders::Column::OrderType,
                Expr::value(input.order_type),
            )
            .col_expr(
                entity::trader_orders::Column::Status,
                Expr::value(input.status),
            )
            .col_expr(
                entity::trader_orders::Column::Quantity,
                Expr::value(decimal_from_f64(input.quantity)),
            )
            .col_expr(
                entity::trader_orders::Column::FilledQuantity,
                Expr::value(decimal_from_f64(input.filled_quantity)),
            )
            .col_expr(
                entity::trader_orders::Column::AvgFillPrice,
                Expr::value(decimal_from_f64(input.avg_fill_price)),
            )
            .col_expr(
                entity::trader_orders::Column::ReduceOnly,
                Expr::value(if input.reduce_only { 1 } else { 0 }),
            )
            .col_expr(
                entity::trader_orders::Column::UpdatedAt,
                Expr::value(ts_to_dt(input.updated_at)),
            )
            .col_expr(
                entity::trader_orders::Column::ClosedAt,
                Expr::value(input.closed_at.map(ts_to_dt)),
            )
            .filter(entity::trader_orders::Column::Id.eq(input.id))
            .exec(&self.db)
            .await
            .map(|_| ())
    }

    pub async fn has_recent_open_order(
        &self,
        trader_id: &str,
        symbol: &str,
        side: &str,
        threshold_ts: i64,
    ) -> Result<bool, DbErr> {
        let orders = entity::trader_orders::Entity::find()
            .filter(entity::trader_orders::Column::TraderId.eq(trader_id.trim()))
            .filter(entity::trader_orders::Column::Symbol.eq(symbol.trim().to_uppercase()))
            .filter(entity::trader_orders::Column::Side.eq(side.trim().to_uppercase()))
            .filter(entity::trader_orders::Column::ReduceOnly.eq(0))
            .filter(entity::trader_orders::Column::Status.is_in([
                "new",
                "open",
                "partially_filled",
                "filled",
            ]))
            .all(&self.db)
            .await?;

        Ok(orders
            .into_iter()
            .any(|order| dt_to_ts(order.placed_at.max(order.updated_at)) >= threshold_ts))
    }

    pub async fn orders(
        &self,
        trader_id: &str,
        open_only: bool,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<TraderOrderRecord>, DbErr> {
        let mut query = entity::trader_orders::Entity::find()
            .filter(entity::trader_orders::Column::TraderId.eq(trader_id.trim()))
            .order_by_desc(entity::trader_orders::Column::PlacedAt)
            .limit(limit.max(0) as u64)
            .offset(offset.max(0) as u64);
        if open_only {
            query = query.filter(entity::trader_orders::Column::Status.is_in([
                "open",
                "new",
                "partially_filled",
            ]));
        }
        query
            .all(&self.db)
            .await
            .map(|rows| rows.into_iter().map(map_order).collect())
    }

    pub async fn open_counts(&self, trader_id: &str) -> Result<(i64, i64), DbErr> {
        let positions = entity::trader_positions::Entity::find()
            .filter(entity::trader_positions::Column::TraderId.eq(trader_id.trim()))
            .filter(entity::trader_positions::Column::Status.eq("open"))
            .count(&self.db)
            .await? as i64;
        let orders = entity::trader_orders::Entity::find()
            .filter(entity::trader_orders::Column::TraderId.eq(trader_id.trim()))
            .filter(entity::trader_orders::Column::Status.is_in([
                "open",
                "new",
                "partially_filled",
            ]))
            .count(&self.db)
            .await? as i64;
        Ok((positions, orders))
    }
}
