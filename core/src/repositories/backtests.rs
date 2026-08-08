use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, prelude::Expr,
};
use serde_json::{Value, json};

use crate::{
    database::DbErr,
    entity::backtest_runs,
    time::{dt_to_ts, ts_to_dt},
};

#[derive(Debug, Clone)]
pub struct BacktestRepo {
    db: DatabaseConnection,
}

#[derive(Debug, Clone)]
pub struct CreateBacktestRunRecord {
    pub run_id: String,
    pub config_json: String,
    pub created_at: i64,
    pub updated_at: i64,
}

impl BacktestRepo {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn create_run(&self, input: CreateBacktestRunRecord) -> Result<(), DbErr> {
        backtest_runs::ActiveModel {
            run_id: Set(input.run_id),
            last_error: Set(String::new()),
            state: Set("running".to_string()),
            config_json: Set(input.config_json),
            summary_json: Set("{}".to_string()),
            created_at: Set(ts_to_dt(input.created_at)),
            updated_at: Set(ts_to_dt(input.updated_at)),
        }
        .insert(&self.db)
        .await
        .map(|_| ())
    }

    pub async fn delete_run(&self, run_id: &str) -> Result<u64, DbErr> {
        let deleted = backtest_runs::Entity::delete_many()
            .filter(backtest_runs::Column::RunId.eq(run_id.trim()))
            .exec(&self.db)
            .await?;

        Ok(deleted.rows_affected)
    }

    pub async fn update_run_status(
        &self,
        run_id: &str,
        status: &str,
        last_error: &str,
        summary_json: String,
        updated_at: i64,
    ) -> Result<(), DbErr> {
        backtest_runs::Entity::update_many()
            .col_expr(
                backtest_runs::Column::State,
                Expr::value(status.to_string()),
            )
            .col_expr(
                backtest_runs::Column::LastError,
                Expr::value(last_error.to_string()),
            )
            .col_expr(
                backtest_runs::Column::SummaryJson,
                Expr::value(summary_json),
            )
            .col_expr(
                backtest_runs::Column::UpdatedAt,
                Expr::value(ts_to_dt(updated_at)),
            )
            .filter(backtest_runs::Column::RunId.eq(run_id.trim()))
            .exec(&self.db)
            .await
            .map(|_| ())
    }

    pub async fn list_runs(&self, limit: i64) -> Result<Vec<Value>, DbErr> {
        backtest_runs::Entity::find()
            .order_by_desc(backtest_runs::Column::CreatedAt)
            .limit(limit.max(0) as u64)
            .all(&self.db)
            .await
            .map(|rows| rows.into_iter().map(run_list_payload).collect())
    }
}

fn run_list_payload(row: backtest_runs::Model) -> Value {
    let summary: Value = serde_json::from_str(row.summary_json.as_str()).unwrap_or(json!({}));
    json!({
        "run_id": row.run_id,
        "state": row.state,
        "last_error": row.last_error,
        "summary": summary,
        "created_at": dt_to_ts(row.created_at),
        "updated_at": dt_to_ts(row.updated_at),
    })
}
