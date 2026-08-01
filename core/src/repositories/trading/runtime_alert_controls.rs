use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set, prelude::Expr};

use crate::{database::DbErr, entity, time::ts_to_dt};

use super::{
    TradingRepo, mappers::alerts::map_runtime_alert_controls,
    records::alerts::RuntimeAlertControlsRecord,
};

impl TradingRepo {
    pub async fn runtime_alert_controls(
        &self,
        trader_id: &str,
    ) -> Result<Option<RuntimeAlertControlsRecord>, DbErr> {
        entity::runtime_alert_controls::Entity::find_by_id(trader_id.trim().to_string())
            .one(&self.db)
        .await
        .map(|row| row.map(map_runtime_alert_controls))
    }

    pub async fn unmute_expired_runtime_alerts(
        &self,
        trader_id: &str,
        updated_at: i64,
    ) -> Result<(), DbErr> {
        entity::runtime_alert_controls::Entity::update_many()
            .col_expr(
                entity::runtime_alert_controls::Column::IsMuted,
                Expr::value(0),
            )
            .col_expr(
                entity::runtime_alert_controls::Column::MutedUntil,
                Expr::value(None::<sea_orm::entity::prelude::DateTimeWithTimeZone>),
            )
            .col_expr(
                entity::runtime_alert_controls::Column::MuteReason,
                Expr::value(""),
            )
            .col_expr(
                entity::runtime_alert_controls::Column::UpdatedAt,
                Expr::value(ts_to_dt(updated_at)),
            )
            .filter(entity::runtime_alert_controls::Column::TraderId.eq(trader_id.trim()))
            .exec(&self.db)
            .await
            .map(|_| ())
    }

    pub async fn set_runtime_alert_mute(
        &self,
        trader_id: &str,
        muted_until: Option<i64>,
        reason: String,
        now: i64,
    ) -> Result<(), DbErr> {
        let existing = self.runtime_alert_controls(trader_id).await?;
        let is_muted = muted_until.is_some();
        if existing.is_some() {
            entity::runtime_alert_controls::Entity::update_many()
                .col_expr(
                    entity::runtime_alert_controls::Column::IsMuted,
                    Expr::value(if is_muted { 1 } else { 0 }),
                )
                .col_expr(
                    entity::runtime_alert_controls::Column::MutedUntil,
                    Expr::value(muted_until.map(ts_to_dt)),
                )
                .col_expr(
                    entity::runtime_alert_controls::Column::MuteReason,
                    Expr::value(reason),
                )
                .col_expr(
                    entity::runtime_alert_controls::Column::UpdatedAt,
                    Expr::value(ts_to_dt(now)),
                )
                .filter(entity::runtime_alert_controls::Column::TraderId.eq(trader_id.trim()))
                .exec(&self.db)
                .await
                .map(|_| ())
        } else {
            entity::runtime_alert_controls::ActiveModel {
                trader_id: Set(trader_id.to_string()),
                is_muted: Set(if is_muted { 1 } else { 0 }),
                muted_until: Set(muted_until.map(ts_to_dt)),
                mute_reason: Set(reason),
                acked_at: Set(None),
                acked_by: Set(String::new()),
                ack_note: Set(String::new()),
                updated_at: Set(ts_to_dt(now)),
                created_at: Set(ts_to_dt(now)),
            }
            .insert(&self.db)
            .await
            .map(|_| ())
        }
    }

    pub async fn ack_runtime_alerts(
        &self,
        trader_id: &str,
        note: String,
        now: i64,
    ) -> Result<(), DbErr> {
        let existing = self.runtime_alert_controls(trader_id).await?;
        if existing.is_some() {
            entity::runtime_alert_controls::Entity::update_many()
                .col_expr(
                    entity::runtime_alert_controls::Column::AckedAt,
                    Expr::value(ts_to_dt(now)),
                )
                .col_expr(
                    entity::runtime_alert_controls::Column::AckedBy,
                    Expr::value("admin".to_string()),
                )
                .col_expr(
                    entity::runtime_alert_controls::Column::AckNote,
                    Expr::value(note),
                )
                .col_expr(
                    entity::runtime_alert_controls::Column::UpdatedAt,
                    Expr::value(ts_to_dt(now)),
                )
                .filter(entity::runtime_alert_controls::Column::TraderId.eq(trader_id.trim()))
                .exec(&self.db)
                .await
                .map(|_| ())
        } else {
            entity::runtime_alert_controls::ActiveModel {
                trader_id: Set(trader_id.to_string()),
                is_muted: Set(0),
                muted_until: Set(None),
                mute_reason: Set(String::new()),
                acked_at: Set(Some(ts_to_dt(now))),
                acked_by: Set("admin".to_string()),
                ack_note: Set(note),
                updated_at: Set(ts_to_dt(now)),
                created_at: Set(ts_to_dt(now)),
            }
            .insert(&self.db)
            .await
            .map(|_| ())
        }
    }
}
