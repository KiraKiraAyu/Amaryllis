use sea_orm::{
    ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
    prelude::Expr,
};

use crate::{database::DbErr, entity, time::ts_to_dt};

use super::{
    TradingRepo,
    mappers::execution::map_execution_intent,
    records::execution::{ExecutionIntentRecord, InsertExecutionIntentRecord},
};

impl TradingRepo {
    pub async fn count_recent_execution_intents(
        &self,
        trader_id: &str,
        decision: &str,
        symbol: &str,
        side: &str,
        from_ts: i64,
    ) -> Result<i64, DbErr> {
        entity::execution_intents::Entity::find()
            .filter(entity::execution_intents::Column::TraderId.eq(trader_id.trim()))
            .filter(entity::execution_intents::Column::Decision.eq(decision.trim()))
            .filter(entity::execution_intents::Column::Symbol.eq(symbol.trim().to_uppercase()))
            .filter(entity::execution_intents::Column::Side.eq(side.trim().to_uppercase()))
            .filter(entity::execution_intents::Column::CreatedAt.gte(ts_to_dt(from_ts)))
            .count(&self.db)
            .await
            .map(|count| count as i64)
    }

    pub async fn execution_intent_by_key(
        &self,
        trader_id: &str,
        intent_key: &str,
    ) -> Result<Option<ExecutionIntentRecord>, DbErr> {
        entity::execution_intents::Entity::find()
            .filter(entity::execution_intents::Column::TraderId.eq(trader_id.trim()))
            .filter(entity::execution_intents::Column::IntentKey.eq(intent_key.trim()))
            .one(&self.db)
            .await
            .map(|row| row.map(map_execution_intent))
    }

    pub async fn insert_execution_intent(
        &self,
        input: InsertExecutionIntentRecord,
    ) -> Result<(), DbErr> {
        entity::execution_intents::Entity::insert(entity::execution_intents::ActiveModel {
            id: Set(input.id),
            trader_id: Set(input.trader_id),
            intent_key: Set(input.intent_key),
            symbol: Set(input.symbol),
            side: Set(input.side),
            decision: Set(input.decision),
            status: Set(input.status),
            exchange_order_id: Set(input.exchange_order_id),
            payload_json: Set(input.payload_json),
            created_at: Set(ts_to_dt(input.created_at)),
            updated_at: Set(ts_to_dt(input.updated_at)),
        })
        .exec(&self.db)
        .await
        .map(|_| ())
    }

    pub async fn mark_execution_intent_submitted(
        &self,
        trader_id: &str,
        intent_key: &str,
        exchange_order_id: &str,
        payload_json: String,
        updated_at: i64,
    ) -> Result<(), DbErr> {
        entity::execution_intents::Entity::update_many()
            .col_expr(
                entity::execution_intents::Column::Status,
                Expr::value("submitted"),
            )
            .col_expr(
                entity::execution_intents::Column::ExchangeOrderId,
                Expr::value(exchange_order_id.trim().to_string()),
            )
            .col_expr(
                entity::execution_intents::Column::PayloadJson,
                Expr::value(payload_json),
            )
            .col_expr(
                entity::execution_intents::Column::UpdatedAt,
                Expr::value(ts_to_dt(updated_at)),
            )
            .filter(entity::execution_intents::Column::TraderId.eq(trader_id.trim()))
            .filter(entity::execution_intents::Column::IntentKey.eq(intent_key.trim()))
            .filter(entity::execution_intents::Column::Status.eq("pending"))
            .exec(&self.db)
            .await
            .map(|_| ())
    }

    pub async fn submitted_execution_intents_by_exchange_order(
        &self,
        trader_id: &str,
        exchange_order_id: &str,
    ) -> Result<Vec<ExecutionIntentRecord>, DbErr> {
        entity::execution_intents::Entity::find()
            .filter(entity::execution_intents::Column::TraderId.eq(trader_id.trim()))
            .filter(entity::execution_intents::Column::ExchangeOrderId.eq(exchange_order_id.trim()))
            .filter(entity::execution_intents::Column::Status.eq("submitted"))
            .all(&self.db)
            .await
            .map(|rows| rows.into_iter().map(map_execution_intent).collect())
    }

    pub async fn update_execution_intent_status(
        &self,
        intent_id: &str,
        status: &str,
        payload_json: String,
        updated_at: i64,
    ) -> Result<(), DbErr> {
        entity::execution_intents::Entity::update_many()
            .col_expr(
                entity::execution_intents::Column::Status,
                Expr::value(status.to_string()),
            )
            .col_expr(
                entity::execution_intents::Column::PayloadJson,
                Expr::value(payload_json),
            )
            .col_expr(
                entity::execution_intents::Column::UpdatedAt,
                Expr::value(ts_to_dt(updated_at)),
            )
            .filter(entity::execution_intents::Column::Id.eq(intent_id.trim()))
            .exec(&self.db)
            .await
            .map(|_| ())
    }

    pub async fn stale_submitted_execution_intents(
        &self,
        trader_id: &str,
        threshold_ts: i64,
        limit: u64,
    ) -> Result<Vec<ExecutionIntentRecord>, DbErr> {
        entity::execution_intents::Entity::find()
            .filter(entity::execution_intents::Column::TraderId.eq(trader_id.trim()))
            .filter(entity::execution_intents::Column::Status.eq("submitted"))
            .filter(entity::execution_intents::Column::ExchangeOrderId.ne(""))
            .filter(entity::execution_intents::Column::UpdatedAt.lte(ts_to_dt(threshold_ts)))
            .order_by_asc(entity::execution_intents::Column::UpdatedAt)
            .limit(limit)
            .all(&self.db)
            .await
            .map(|rows| rows.into_iter().map(map_execution_intent).collect())
    }
}
