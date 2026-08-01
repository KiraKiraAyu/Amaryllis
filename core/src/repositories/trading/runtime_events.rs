use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};

use crate::{database::DbErr, entity, time::ts_to_dt};

use super::{
    TradingRepo,
    mappers::runtime_observability::map_runtime_event,
    records::runtime_observability::{InsertRuntimeEventRecord, RuntimeEventRecord},
};

impl TradingRepo {
    pub async fn insert_runtime_event(&self, input: InsertRuntimeEventRecord) -> Result<(), DbErr> {
        entity::runtime_events::ActiveModel {
            id: Set(input.id),
            trader_id: Set(input.trader_id),
            event_type: Set(input.event_type),
            symbol: Set(input.symbol),
            side: Set(input.side),
            risk_level: Set(input.risk_level),
            trigger_source: Set(input.trigger_source),
            action_taken: Set(input.action_taken),
            correlation_id: Set(input.correlation_id),
            payload_json: Set(input.payload_json),
            created_at: Set(ts_to_dt(input.created_at)),
        }
        .insert(&self.db)
        .await
        .map(|_| ())
    }

    pub async fn count_runtime_events(
        &self,
        trader_id: &str,
        event_type: Option<&str>,
        action_taken: Option<&str>,
        from_ts: i64,
    ) -> Result<i64, DbErr> {
        let mut query = entity::runtime_events::Entity::find()
            .filter(entity::runtime_events::Column::TraderId.eq(trader_id.trim()))
            .filter(entity::runtime_events::Column::CreatedAt.gte(ts_to_dt(from_ts)));
        if let Some(event_type) = event_type {
            query = query.filter(entity::runtime_events::Column::EventType.eq(event_type));
        }
        if let Some(action_taken) = action_taken {
            query = query.filter(entity::runtime_events::Column::ActionTaken.eq(action_taken));
        }
        query.count(&self.db).await.map(|count| count as i64)
    }

    pub async fn runtime_events(
        &self,
        trader_id: &str,
        from_ts: i64,
        event_type: &str,
        risk_level: &str,
        correlation_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<RuntimeEventRecord>), DbErr> {
        let mut query = entity::runtime_events::Entity::find()
            .filter(entity::runtime_events::Column::TraderId.eq(trader_id.trim()))
            .filter(entity::runtime_events::Column::CreatedAt.gte(ts_to_dt(from_ts)));
        if !event_type.trim().is_empty() {
            query = query.filter(entity::runtime_events::Column::EventType.eq(event_type.trim()));
        }
        if !risk_level.trim().is_empty() {
            query = query.filter(entity::runtime_events::Column::RiskLevel.eq(risk_level.trim()));
        }
        if !correlation_id.trim().is_empty() {
            query = query.filter(
                entity::runtime_events::Column::CorrelationId.contains(correlation_id.trim()),
            );
        }
        let total = query.clone().count(&self.db).await? as i64;
        let rows = query
            .order_by_desc(entity::runtime_events::Column::CreatedAt)
            .limit(limit.max(0) as u64)
            .offset(offset.max(0) as u64)
            .all(&self.db)
            .await?
            .into_iter()
            .map(map_runtime_event)
            .collect();
        Ok((total, rows))
    }

    pub async fn runtime_events_since(
        &self,
        trader_id: &str,
        from_ts: i64,
    ) -> Result<Vec<RuntimeEventRecord>, DbErr> {
        entity::runtime_events::Entity::find()
            .filter(entity::runtime_events::Column::TraderId.eq(trader_id.trim()))
            .filter(entity::runtime_events::Column::CreatedAt.gte(ts_to_dt(from_ts)))
            .all(&self.db)
            .await
            .map(|rows| rows.into_iter().map(map_runtime_event).collect())
    }
}
