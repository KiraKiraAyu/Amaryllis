use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, prelude::Expr,
};
use serde_json::{Value, json};

use crate::{
    database::DbErr,
    entity::{backtest_decisions, backtest_equity, backtest_runs, backtest_trades},
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

#[derive(Debug, Clone)]
pub struct BacktestRunRecord {
    pub run_id: String,
    pub label: String,
    pub state: String,
    pub last_error: String,
    pub config_json: String,
    pub summary_json: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct BacktestEquityPointRecord {
    pub run_id: String,
    pub ts: i64,
    pub equity: f64,
    pub available: f64,
    pub pnl: f64,
    pub dd_pct: f64,
    pub cycle: usize,
}

#[derive(Debug, Clone)]
pub struct BacktestTradeRecord {
    pub id: String,
    pub run_id: String,
    pub ts: i64,
    pub symbol: String,
    pub action: String,
    pub side: String,
    pub qty: f64,
    pub price: f64,
    pub fee: f64,
    pub realized_pnl: f64,
    pub leverage: i64,
    pub cycle: usize,
    pub liquidation: bool,
}

#[derive(Debug, Clone)]
pub struct BacktestDecisionRecord {
    pub id: String,
    pub run_id: String,
    pub ts: i64,
    pub symbol: String,
    pub action: String,
    pub confidence: f64,
    pub reason: String,
    pub cycle: usize,
}

impl BacktestRepo {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn create_run(&self, input: CreateBacktestRunRecord) -> Result<(), DbErr> {
        backtest_runs::ActiveModel {
            run_id: Set(input.run_id),
            label: Set(String::new()),
            last_error: Set(String::new()),
            version: Set(1),
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

    pub async fn update_label(
        &self,
        run_id: &str,
        label: String,
        updated_at: i64,
    ) -> Result<u64, DbErr> {
        backtest_runs::Entity::update_many()
            .col_expr(backtest_runs::Column::Label, Expr::value(label))
            .col_expr(
                backtest_runs::Column::UpdatedAt,
                Expr::value(ts_to_dt(updated_at)),
            )
            .filter(backtest_runs::Column::RunId.eq(run_id.trim()))
            .exec(&self.db)
            .await
            .map(|res| res.rows_affected)
    }

    pub async fn delete_run(&self, run_id: &str) -> Result<u64, DbErr> {
        let deleted = backtest_runs::Entity::delete_many()
            .filter(backtest_runs::Column::RunId.eq(run_id.trim()))
            .exec(&self.db)
            .await?;

        backtest_equity::Entity::delete_many()
            .filter(backtest_equity::Column::RunId.eq(run_id.trim()))
            .exec(&self.db)
            .await?;
        backtest_trades::Entity::delete_many()
            .filter(backtest_trades::Column::RunId.eq(run_id.trim()))
            .exec(&self.db)
            .await?;
        backtest_decisions::Entity::delete_many()
            .filter(backtest_decisions::Column::RunId.eq(run_id.trim()))
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

    pub async fn get_run(
        &self,
        run_id: &str,
    ) -> Result<Option<BacktestRunRecord>, DbErr> {
        backtest_runs::Entity::find_by_id(run_id.trim().to_string())
            .one(&self.db)
            .await
            .map(|row| row.map(map_run_record))
    }

    pub async fn list_runs(&self, limit: i64) -> Result<Vec<Value>, DbErr> {
        backtest_runs::Entity::find()
            .order_by_desc(backtest_runs::Column::CreatedAt)
            .limit(limit.max(0) as u64)
            .all(&self.db)
            .await
            .map(|rows| rows.into_iter().map(run_list_payload).collect())
    }

    pub async fn list_equity_points(
        &self,
        run_id: &str,
        limit: i64,
    ) -> Result<Vec<Value>, DbErr> {
        backtest_equity::Entity::find()
            .filter(backtest_equity::Column::RunId.eq(run_id.trim()))
            .order_by_asc(backtest_equity::Column::Ts)
            .limit(limit.max(0) as u64)
            .all(&self.db)
            .await
            .map(|rows| rows.into_iter().map(equity_payload).collect())
    }

    pub async fn list_trades(
        &self,
        run_id: &str,
        limit: i64,
    ) -> Result<Vec<Value>, DbErr> {
        backtest_trades::Entity::find()
            .filter(backtest_trades::Column::RunId.eq(run_id.trim()))
            .order_by_asc(backtest_trades::Column::Ts)
            .limit(limit.max(0) as u64)
            .all(&self.db)
            .await
            .map(|rows| rows.into_iter().map(trade_payload).collect())
    }

    pub async fn list_decisions(
        &self,
        run_id: &str,
        limit: i64,
    ) -> Result<Vec<Value>, DbErr> {
        backtest_decisions::Entity::find()
            .filter(backtest_decisions::Column::RunId.eq(run_id.trim()))
            .order_by_asc(backtest_decisions::Column::Ts)
            .limit(limit.max(0) as u64)
            .all(&self.db)
            .await
            .map(|rows| rows.into_iter().map(decision_payload).collect())
    }

    pub async fn insert_equity_point(&self, input: BacktestEquityPointRecord) -> Result<(), DbErr> {
        let pnl_pct = if input.equity > 0.0 {
            (input.pnl / input.equity) * 100.0
        } else {
            0.0
        };

        backtest_equity::Entity::insert(backtest_equity::ActiveModel {
            run_id: Set(input.run_id),
            ts: Set(ts_to_dt(input.ts)),
            equity: Set(decimal_from_f64(input.equity)),
            available: Set(decimal_from_f64(input.available)),
            pnl: Set(decimal_from_f64(input.pnl)),
            pnl_pct: Set(decimal_from_f64(pnl_pct)),
            dd_pct: Set(decimal_from_f64(input.dd_pct)),
            cycle: Set(input.cycle as i32),
        })
        .exec(&self.db)
        .await
        .map(|_| ())
    }

    pub async fn insert_trade(&self, input: BacktestTradeRecord) -> Result<(), DbErr> {
        backtest_trades::ActiveModel {
            id: Set(input.id),
            run_id: Set(input.run_id),
            ts: Set(ts_to_dt(input.ts)),
            symbol: Set(input.symbol),
            action: Set(input.action),
            side: Set(input.side),
            qty: Set(decimal_from_f64(input.qty)),
            price: Set(decimal_from_f64(input.price)),
            fee: Set(decimal_from_f64(input.fee)),
            slippage: Set(Decimal::ZERO),
            order_value: Set(Decimal::ZERO),
            realized_pnl: Set(decimal_from_f64(input.realized_pnl)),
            leverage: Set(input.leverage as i32),
            cycle: Set(input.cycle as i32),
            position_after: Set(Decimal::ZERO),
            liquidation: Set(if input.liquidation { 1 } else { 0 }),
            note: Set(String::new()),
        }
        .insert(&self.db)
        .await
        .map(|_| ())
    }

    pub async fn insert_decision(&self, input: BacktestDecisionRecord) -> Result<(), DbErr> {
        backtest_decisions::ActiveModel {
            id: Set(input.id),
            run_id: Set(input.run_id),
            ts: Set(ts_to_dt(input.ts)),
            symbol: Set(input.symbol),
            timeframe: Set(String::new()),
            decision: Set(input.action),
            confidence: Set(decimal_from_f64(input.confidence)),
            reason: Set(input.reason),
            payload_json: Set("{}".to_string()),
            cycle: Set(input.cycle as i32),
        }
        .insert(&self.db)
        .await
        .map(|_| ())
    }
}

fn map_run_record(row: backtest_runs::Model) -> BacktestRunRecord {
    BacktestRunRecord {
        run_id: row.run_id,
        label: row.label,
        state: row.state,
        last_error: row.last_error,
        config_json: row.config_json,
        summary_json: row.summary_json,
        created_at: dt_to_ts(row.created_at),
        updated_at: dt_to_ts(row.updated_at),
    }
}

fn run_list_payload(row: backtest_runs::Model) -> Value {
    let summary: Value = serde_json::from_str(row.summary_json.as_str()).unwrap_or(json!({}));
    json!({
        "run_id": row.run_id,
        "label": row.label,
        "state": row.state,
        "last_error": row.last_error,
        "summary": summary,
        "created_at": dt_to_ts(row.created_at),
        "updated_at": dt_to_ts(row.updated_at),
    })
}

fn equity_payload(row: backtest_equity::Model) -> Value {
    json!({
        "ts": dt_to_ts(row.ts),
        "equity": decimal_to_f64(&row.equity),
        "available": decimal_to_f64(&row.available),
        "pnl": decimal_to_f64(&row.pnl),
        "pnl_pct": decimal_to_f64(&row.pnl_pct),
        "dd_pct": decimal_to_f64(&row.dd_pct),
        "cycle": row.cycle,
    })
}

fn trade_payload(row: backtest_trades::Model) -> Value {
    json!({
        "id": row.id,
        "ts": dt_to_ts(row.ts),
        "symbol": row.symbol,
        "action": row.action,
        "side": row.side,
        "qty": decimal_to_f64(&row.qty),
        "price": decimal_to_f64(&row.price),
        "fee": decimal_to_f64(&row.fee),
        "realized_pnl": decimal_to_f64(&row.realized_pnl),
        "leverage": row.leverage,
        "cycle": row.cycle,
        "liquidation": row.liquidation != 0,
    })
}

fn decision_payload(row: backtest_decisions::Model) -> Value {
    json!({
        "id": row.id,
        "ts": dt_to_ts(row.ts),
        "symbol": row.symbol,
        "decision": row.decision,
        "confidence": decimal_to_f64(&row.confidence),
        "reason": row.reason,
        "cycle": row.cycle,
    })
}

fn decimal_from_f64(value: f64) -> Decimal {
    Decimal::from_f64_retain(value).unwrap_or(Decimal::ZERO)
}

fn decimal_to_f64(value: &Decimal) -> f64 {
    value.to_f64().unwrap_or(0.0)
}
