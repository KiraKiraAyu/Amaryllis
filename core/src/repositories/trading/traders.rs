use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set,
    TransactionTrait, prelude::Expr,
};

use crate::{database::DbErr, entity, time::{dt_to_ts, ts_to_dt}};

use super::{
    TradingRepo,
    mappers::traders::map_trader,
    records::traders::{
        CreateTraderRecord, EquityHistoryPointRecord, TraderRecord, UpdateTraderRecord,
    },
    values::{decimal_from_f64, decimal_to_f64},
};

impl TradingRepo {
    pub async fn list_traders(&self) -> Result<Vec<TraderRecord>, DbErr> {
        entity::traders::Entity::find()
            .order_by_desc(entity::traders::Column::CreatedAt)
            .all(&self.db)
            .await
            .map(|rows| rows.into_iter().map(map_trader).collect())
    }

    pub async fn get_trader(
        &self,
        trader_id: &str,
    ) -> Result<Option<TraderRecord>, DbErr> {
        entity::traders::Entity::find_by_id(trader_id.trim().to_string())
            .one(&self.db)
            .await
            .map(|row| row.map(map_trader))
    }

    pub async fn first_trader_id(&self) -> Result<Option<String>, DbErr> {
        entity::traders::Entity::find()
            .order_by_desc(entity::traders::Column::CreatedAt)
            .one(&self.db)
            .await
            .map(|row| row.map(|v| v.id))
    }

    pub async fn latest_trader_id(&self) -> Result<Option<String>, DbErr> {
        entity::traders::Entity::find()
            .order_by_desc(entity::traders::Column::UpdatedAt)
            .one(&self.db)
            .await
            .map(|row| row.map(|v| v.id))
    }

    pub async fn running_traders(&self) -> Result<Vec<String>, DbErr> {
        entity::traders::Entity::find()
            .filter(entity::traders::Column::IsRunning.eq(1))
            .order_by_desc(entity::traders::Column::UpdatedAt)
            .all(&self.db)
            .await
            .map(|rows| rows.into_iter().map(|row| row.id).collect())
    }

    pub async fn set_trader_running(
        &self,
        trader_id: &str,
        running: bool,
        updated_at: i64,
    ) -> Result<(), DbErr> {
        entity::traders::Entity::update_many()
            .col_expr(
                entity::traders::Column::IsRunning,
                Expr::value(if running { 1 } else { 0 }),
            )
            .col_expr(
                entity::traders::Column::UpdatedAt,
                Expr::value(ts_to_dt(updated_at)),
            )
            .filter(entity::traders::Column::Id.eq(trader_id.trim()))
            .exec(&self.db)
            .await
            .map(|_| ())
    }

    pub async fn create_trader_with_snapshot(
        &self,
        input: CreateTraderRecord,
    ) -> Result<(), DbErr> {
        let tx = self.db.begin().await?;
        entity::traders::ActiveModel {
            id: Set(input.id.clone()),
            name: Set(input.name),
            ai_model_id: Set(input.ai_model_id),
            exchange_id: Set(input.exchange_id.clone()),
            strategy_id: Set(input.strategy_id),
            initial_balance: Set(decimal_from_f64(input.initial_balance.max(0.0))),
            scan_interval_minutes: Set(input.scan_interval_minutes.max(1) as i32),
            is_running: Set(0),
            is_cross_margin: Set(if input.is_cross_margin { 1 } else { 0 }),
            btc_eth_leverage: Set(input.btc_eth_leverage as i32),
            altcoin_leverage: Set(input.altcoin_leverage as i32),
            trading_symbols: Set(input.trading_symbols),
            use_ai500: Set(if input.use_ai500 { 1 } else { 0 }),
            use_oi_top: Set(if input.use_oi_top { 1 } else { 0 }),
            custom_prompt: Set(input.custom_prompt),
            override_base_prompt: Set(if input.override_base_prompt { 1 } else { 0 }),
            system_prompt_template: Set(input.system_prompt_template),
            created_at: Set(ts_to_dt(input.created_at)),
            updated_at: Set(ts_to_dt(input.updated_at)),
        }
        .insert(&tx)
        .await?;

        entity::trader_accounts::ActiveModel {
            id: Set(input.snapshot_id),
            trader_id: Set(input.id),
            exchange_id: Set(input.exchange_id),
            total_balance: Set(decimal_from_f64(input.initial_balance.max(0.0))),
            available_balance: Set(decimal_from_f64(input.initial_balance.max(0.0))),
            used_margin: Set(Decimal::ZERO),
            unrealized_pnl: Set(Decimal::ZERO),
            realized_pnl: Set(Decimal::ZERO),
            currency: Set("USDT".to_string()),
            snapshot_at: Set(ts_to_dt(input.created_at)),
            created_at: Set(ts_to_dt(input.created_at)),
            updated_at: Set(ts_to_dt(input.updated_at)),
        }
        .insert(&tx)
        .await?;

        tx.commit().await
    }

    pub async fn update_trader(
        &self,
        trader_id: &str,
        patch: UpdateTraderRecord,
    ) -> Result<u64, DbErr> {
        entity::traders::Entity::update_many()
            .col_expr(entity::traders::Column::Name, Expr::value(patch.name))
            .col_expr(
                entity::traders::Column::AiModelId,
                Expr::value(patch.ai_model_id),
            )
            .col_expr(
                entity::traders::Column::ExchangeId,
                Expr::value(patch.exchange_id),
            )
            .col_expr(
                entity::traders::Column::StrategyId,
                Expr::value(patch.strategy_id),
            )
            .col_expr(
                entity::traders::Column::InitialBalance,
                Expr::value(decimal_from_f64(patch.initial_balance.max(0.0))),
            )
            .col_expr(
                entity::traders::Column::ScanIntervalMinutes,
                Expr::value(patch.scan_interval_minutes.max(1) as i32),
            )
            .col_expr(
                entity::traders::Column::IsCrossMargin,
                Expr::value(if patch.is_cross_margin { 1 } else { 0 }),
            )
            .col_expr(
                entity::traders::Column::BtcEthLeverage,
                Expr::value(patch.btc_eth_leverage as i32),
            )
            .col_expr(
                entity::traders::Column::AltcoinLeverage,
                Expr::value(patch.altcoin_leverage as i32),
            )
            .col_expr(
                entity::traders::Column::TradingSymbols,
                Expr::value(patch.trading_symbols),
            )
            .col_expr(
                entity::traders::Column::UseAi500,
                Expr::value(if patch.use_ai500 { 1 } else { 0 }),
            )
            .col_expr(
                entity::traders::Column::UseOiTop,
                Expr::value(if patch.use_oi_top { 1 } else { 0 }),
            )
            .col_expr(
                entity::traders::Column::CustomPrompt,
                Expr::value(patch.custom_prompt),
            )
            .col_expr(
                entity::traders::Column::OverrideBasePrompt,
                Expr::value(if patch.override_base_prompt { 1 } else { 0 }),
            )
            .col_expr(
                entity::traders::Column::SystemPromptTemplate,
                Expr::value(patch.system_prompt_template),
            )
            .col_expr(
                entity::traders::Column::UpdatedAt,
                Expr::value(ts_to_dt(patch.updated_at)),
            )
            .filter(entity::traders::Column::Id.eq(trader_id.trim()))
            .exec(&self.db)
            .await
            .map(|res| res.rows_affected)
    }

    pub async fn delete_trader(&self, trader_id: &str) -> Result<u64, DbErr> {
        let tx = self.db.begin().await?;
        let owner = entity::traders::Entity::find_by_id(trader_id.trim().to_string())
            .one(&tx)
            .await?;
        if owner.is_none() {
            return Ok(0);
        }

        entity::trader_accounts::Entity::delete_many()
            .filter(entity::trader_accounts::Column::TraderId.eq(trader_id.trim()))
            .exec(&tx)
            .await?;
        entity::trader_positions::Entity::delete_many()
            .filter(entity::trader_positions::Column::TraderId.eq(trader_id.trim()))
            .exec(&tx)
            .await?;
        entity::order_fills::Entity::delete_many()
            .filter(entity::order_fills::Column::TraderId.eq(trader_id.trim()))
            .exec(&tx)
            .await?;
        entity::trader_orders::Entity::delete_many()
            .filter(entity::trader_orders::Column::TraderId.eq(trader_id.trim()))
            .exec(&tx)
            .await?;
        entity::trader_trades::Entity::delete_many()
            .filter(entity::trader_trades::Column::TraderId.eq(trader_id.trim()))
            .exec(&tx)
            .await?;

        let deleted = entity::traders::Entity::delete_many()
            .filter(entity::traders::Column::Id.eq(trader_id.trim()))
            .exec(&tx)
            .await?;

        tx.commit().await?;
        Ok(deleted.rows_affected)
    }

    pub async fn update_prompt(
        &self,
        trader_id: &str,
        custom_prompt: String,
        override_base_prompt: bool,
        updated_at: i64,
    ) -> Result<u64, DbErr> {
        entity::traders::Entity::update_many()
            .col_expr(
                entity::traders::Column::CustomPrompt,
                Expr::value(custom_prompt),
            )
            .col_expr(
                entity::traders::Column::OverrideBasePrompt,
                Expr::value(if override_base_prompt { 1 } else { 0 }),
            )
            .col_expr(
                entity::traders::Column::UpdatedAt,
                Expr::value(ts_to_dt(updated_at)),
            )
            .filter(entity::traders::Column::Id.eq(trader_id.trim()))
            .exec(&self.db)
            .await
            .map(|res| res.rows_affected)
    }

    pub async fn equity_history_points(
        &self,
        trader_id: &str,
        since_ts: Option<i64>,
        limit: i64,
    ) -> Result<Vec<EquityHistoryPointRecord>, DbErr> {
        let mut query = entity::trader_accounts::Entity::find()
            .filter(entity::trader_accounts::Column::TraderId.eq(trader_id.trim()))
            .order_by_asc(entity::trader_accounts::Column::SnapshotAt)
            .limit(limit.max(0) as u64);
        if let Some(since) = since_ts {
            query = query.filter(entity::trader_accounts::Column::SnapshotAt.gte(ts_to_dt(since)));
        }

        let mut rows = query.all(&self.db).await?;
        if rows.is_empty() {
            return Ok(vec![]);
        }

        rows.sort_by_key(|row| row.snapshot_at);
        let first_total = rows
            .first()
            .map(|row| decimal_to_f64(&row.total_balance))
            .unwrap_or(1.0)
            .max(1e-9);

        Ok(rows
            .into_iter()
            .map(|row| {
                let total_balance = decimal_to_f64(&row.total_balance);
                let available_balance = decimal_to_f64(&row.available_balance);
                let total_pnl = decimal_to_f64(&row.unrealized_pnl);
                let total_pnl_pct = ((total_balance - first_total) / first_total) * 100.0;
                let used_margin = decimal_to_f64(&row.used_margin);
                let margin_used_pct = if total_balance.abs() > f64::EPSILON {
                    (used_margin / total_balance) * 100.0
                } else {
                    0.0
                };
                EquityHistoryPointRecord {
                    timestamp: dt_to_ts(row.snapshot_at),
                    total_equity: total_balance + total_pnl,
                    available_balance,
                    total_pnl,
                    total_pnl_pct,
                    position_count: 0,
                    margin_used_pct,
                    balance: total_balance,
                }
            })
            .collect())
    }
}
