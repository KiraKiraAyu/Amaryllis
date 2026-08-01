use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};

use crate::{database::DbErr, entity, time::ts_to_dt};

use super::{
    TradingRepo,
    mappers::alerts::map_runtime_alert_delivery,
    records::alerts::{InsertRuntimeAlertDeliveryRecord, RuntimeAlertDeliveryRecord},
};

impl TradingRepo {
    pub async fn runtime_alert_deliveries(
        &self,
        trader_id: &str,
        from_ts: i64,
        success: Option<bool>,
        destination: &str,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<RuntimeAlertDeliveryRecord>), DbErr> {
        let mut query = entity::runtime_alert_delivery_log::Entity::find()
            .filter(entity::runtime_alert_delivery_log::Column::TraderId.eq(trader_id.trim()))
            .filter(entity::runtime_alert_delivery_log::Column::CreatedAt.gte(ts_to_dt(from_ts)));
        if let Some(success) = success {
            query = query.filter(
                entity::runtime_alert_delivery_log::Column::Success.eq(if success { 1 } else { 0 }),
            );
        }
        if !destination.trim().is_empty() {
            query = query.filter(
                entity::runtime_alert_delivery_log::Column::Destination.eq(destination.trim()),
            );
        }
        let total = query.clone().count(&self.db).await? as i64;
        let rows = query
            .order_by_desc(entity::runtime_alert_delivery_log::Column::CreatedAt)
            .limit(limit.max(0) as u64)
            .offset(offset.max(0) as u64)
            .all(&self.db)
            .await?
            .into_iter()
            .map(map_runtime_alert_delivery)
            .collect();
        Ok((total, rows))
    }

    pub async fn insert_runtime_alert_delivery(
        &self,
        input: InsertRuntimeAlertDeliveryRecord,
    ) -> Result<(), DbErr> {
        entity::runtime_alert_delivery_log::ActiveModel {
            id: Set(input.id),
            trader_id: Set(input.trader_id),
            alert_history_id: Set(input.alert_history_id),
            destination: Set(input.destination),
            endpoint: Set(input.endpoint),
            request_headers_json: Set(input.request_headers_json),
            request_body_json: Set(input.request_body_json),
            response_status: Set(input.response_status as i32),
            response_body: Set(input.response_body),
            attempt: Set(input.attempt as i32),
            max_attempts: Set(input.max_attempts as i32),
            success: Set(if input.success { 1 } else { 0 }),
            error_message: Set(input.error_message),
            latency_ms: Set(input.latency_ms as i32),
            created_at: Set(ts_to_dt(input.created_at)),
        }
        .insert(&self.db)
        .await
        .map(|_| ())
    }
}
