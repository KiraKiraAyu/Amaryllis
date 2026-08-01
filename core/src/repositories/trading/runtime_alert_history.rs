use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};

use crate::{database::DbErr, entity, time::ts_to_dt};

use super::{
    TradingRepo,
    mappers::alerts::map_runtime_alert_history,
    records::alerts::{InsertRuntimeAlertHistoryRecord, RuntimeAlertHistoryRecord},
};

impl TradingRepo {
    pub async fn recent_runtime_alert_history_count(
        &self,
        trader_id: &str,
        breached: bool,
        severity: &str,
        since_ts: i64,
    ) -> Result<i64, DbErr> {
        entity::runtime_alert_history::Entity::find()
            .filter(entity::runtime_alert_history::Column::TraderId.eq(trader_id.trim()))
            .filter(
                entity::runtime_alert_history::Column::Breached.eq(if breached { 1 } else { 0 }),
            )
            .filter(entity::runtime_alert_history::Column::Severity.eq(severity.trim()))
            .filter(entity::runtime_alert_history::Column::CreatedAt.gte(ts_to_dt(since_ts)))
            .count(&self.db)
            .await
            .map(|count| count as i64)
    }

    pub async fn insert_runtime_alert_history(
        &self,
        input: InsertRuntimeAlertHistoryRecord,
    ) -> Result<(), DbErr> {
        entity::runtime_alert_history::ActiveModel {
            id: Set(input.id),
            trader_id: Set(input.trader_id),
            window_hours: Set(input.window_hours as i32),
            thresholds_json: Set(input.thresholds_json),
            rates_json: Set(input.rates_json),
            alerts_json: Set(input.alerts_json),
            breached: Set(if input.breached { 1 } else { 0 }),
            severity: Set(input.severity),
            created_at: Set(ts_to_dt(input.created_at)),
        }
        .insert(&self.db)
        .await
        .map(|_| ())
    }

    pub async fn runtime_alert_history(
        &self,
        trader_id: &str,
        from_ts: i64,
        breached: Option<bool>,
        severity: &str,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<RuntimeAlertHistoryRecord>), DbErr> {
        let mut query = entity::runtime_alert_history::Entity::find()
            .filter(entity::runtime_alert_history::Column::TraderId.eq(trader_id.trim()))
            .filter(entity::runtime_alert_history::Column::CreatedAt.gte(ts_to_dt(from_ts)));
        if let Some(breached) = breached {
            query = query.filter(
                entity::runtime_alert_history::Column::Breached.eq(if breached { 1 } else { 0 }),
            );
        }
        if !severity.trim().is_empty() {
            query =
                query.filter(entity::runtime_alert_history::Column::Severity.eq(severity.trim()));
        }
        let total = query.clone().count(&self.db).await? as i64;
        let rows = query
            .order_by_desc(entity::runtime_alert_history::Column::CreatedAt)
            .limit(limit.max(0) as u64)
            .offset(offset.max(0) as u64)
            .all(&self.db)
            .await?
            .into_iter()
            .map(map_runtime_alert_history)
            .collect();
        Ok((total, rows))
    }
}
