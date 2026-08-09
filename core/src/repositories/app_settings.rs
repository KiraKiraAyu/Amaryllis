use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, prelude::Expr,
};

use crate::entity::app_settings;
use crate::error::{AppError, Result as AppResult};
use crate::time::ts_to_dt;

/// Simple key-value persistence for instance-level settings
/// (TOTP secret, session secret, ...).
#[derive(Debug, Clone)]
pub struct AppSettingsRepo {
    db: DatabaseConnection,
}

impl AppSettingsRepo {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn get(&self, key: &str) -> AppResult<Option<String>> {
        app_settings::Entity::find_by_id(key.trim().to_string())
            .one(&self.db)
            .await
            .map(|row| row.map(|model| model.value))
            .map_err(AppError::from)
    }

    pub async fn set(&self, key: &str, value: &str) -> AppResult<()> {
        let key = key.trim();
        if key.is_empty() {
            return Err(AppError::BadRequest("Setting key cannot be empty".into()));
        }

        let now = ts_to_dt(now_unix_ts() as i64);
        let model = app_settings::ActiveModel {
            key: Set(key.to_string()),
            value: Set(value.to_string()),
            updated_at: Set(now),
        };

        // Upsert: try update first, insert when the row does not exist.
        let affected = app_settings::Entity::update_many()
            .col_expr(app_settings::Column::Value, Expr::value(value.to_string()))
            .col_expr(app_settings::Column::UpdatedAt, Expr::value(now))
            .filter(app_settings::Column::Key.eq(key))
            .exec(&self.db)
            .await?
            .rows_affected;

        if affected == 0 {
            model.insert(&self.db).await?;
        }

        Ok(())
    }

    pub async fn delete(&self, key: &str) -> AppResult<()> {
        app_settings::Entity::delete_by_id(key.trim().to_string())
            .exec(&self.db)
            .await?;
        Ok(())
    }
}

fn now_unix_ts() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
