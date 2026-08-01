use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};

use crate::{database::DbErr, entity, time::ts_to_dt};

use super::{
    TradingRepo,
    mappers::orders::map_fill,
    records::orders::{InsertOrderFillRecord, OrderFillRecord},
    values::{decimal_from_f64, decimal_to_f64},
};

impl TradingRepo {
    pub async fn insert_order_fill(&self, input: InsertOrderFillRecord) -> Result<(), DbErr> {
        entity::order_fills::ActiveModel {
            id: Set(input.id),
            order_id: Set(input.order_id),
            trader_id: Set(input.trader_id),
            exchange_trade_id: Set(input.exchange_trade_id),
            symbol: Set(input.symbol),
            side: Set(input.side),
            price: Set(decimal_from_f64(input.price)),
            quantity: Set(decimal_from_f64(input.quantity)),
            fee: Set(decimal_from_f64(input.fee)),
            fee_asset: Set(input.fee_asset),
            realized_pnl: Set(decimal_from_f64(input.realized_pnl)),
            executed_at: Set(ts_to_dt(input.executed_at)),
            created_at: Set(ts_to_dt(input.created_at)),
        }
        .insert(&self.db)
        .await
        .map(|_| ())
    }

    pub async fn order_fill_exists(
        &self,
        trader_id: &str,
        exchange_trade_id: &str,
    ) -> Result<bool, DbErr> {
        entity::order_fills::Entity::find()
            .filter(entity::order_fills::Column::TraderId.eq(trader_id.trim()))
            .filter(entity::order_fills::Column::ExchangeTradeId.eq(exchange_trade_id.trim()))
            .one(&self.db)
            .await
            .map(|row| row.is_some())
    }

    pub async fn order_fill_summary(
        &self,
        trader_id: &str,
        order_id: &str,
    ) -> Result<(f64, f64), DbErr> {
        let fills = entity::order_fills::Entity::find()
            .filter(entity::order_fills::Column::OrderId.eq(order_id.trim()))
            .filter(entity::order_fills::Column::TraderId.eq(trader_id.trim()))
            .all(&self.db)
            .await?;
        let filled_qty: f64 = fills
            .iter()
            .map(|fill| decimal_to_f64(&fill.quantity))
            .sum();
        let weighted_notional: f64 = fills
            .iter()
            .map(|fill| decimal_to_f64(&fill.price) * decimal_to_f64(&fill.quantity))
            .sum();
        let avg_price = if filled_qty > 0.0 {
            weighted_notional / filled_qty
        } else {
            0.0
        };
        Ok((filled_qty, avg_price))
    }

    pub async fn order_fills(
        &self,
        trader_id: &str,
        order_id: &str,
    ) -> Result<Vec<OrderFillRecord>, DbErr> {
        entity::order_fills::Entity::find()
            .filter(entity::order_fills::Column::OrderId.eq(order_id.trim()))
            .filter(entity::order_fills::Column::TraderId.eq(trader_id.trim()))
            .order_by_desc(entity::order_fills::Column::ExecutedAt)
            .all(&self.db)
            .await
            .map(|rows| rows.into_iter().map(map_fill).collect())
    }
}
