use std::collections::{HashMap, HashSet};

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect};

use crate::{
    database::DbErr,
    entity::{trader_accounts, trader_positions, traders},
    time::{dt_to_ts, ts_to_dt},
};

#[derive(Debug, Clone)]
pub struct CompetitionRepo {
    db: DatabaseConnection,
}

#[derive(Debug, Clone)]
pub struct CompetitionTraderRecord {
    pub trader_id: String,
    pub trader_name: String,
    pub ai_model: String,
    pub exchange: String,
    pub total_equity: f64,
    pub total_pnl: f64,
    pub total_pnl_pct: f64,
    pub unrealized_pnl: f64,
    pub realized_pnl: f64,
    pub position_count: i64,
    pub margin_used_pct: f64,
    pub is_running: bool,
}

#[derive(Debug, Clone)]
pub struct EquityHistoryPointRecord {
    pub timestamp: i64,
    pub total_equity: f64,
    pub available_balance: f64,
    pub total_pnl: f64,
    pub total_pnl_pct: f64,
    pub position_count: i64,
    pub margin_used_pct: f64,
    pub balance: f64,
}

#[derive(Debug, Clone)]
pub struct PublicTraderConfigRecord {
    pub trader_id: String,
    pub trader_name: String,
    pub ai_model: String,
    pub exchange_id: String,
    pub strategy_id: String,
    pub is_cross_margin: bool,
    pub show_in_competition: bool,
    pub scan_interval_minutes: i32,
    pub initial_balance: f64,
    pub is_running: bool,
    pub btc_eth_leverage: i32,
    pub altcoin_leverage: i32,
    pub trading_symbols: String,
    pub custom_prompt: String,
    pub override_base_prompt: bool,
    pub system_prompt_template: String,
    pub use_ai500: bool,
    pub use_oi_top: bool,
}

impl CompetitionRepo {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn public_traders(&self) -> Result<Vec<CompetitionTraderRecord>, DbErr> {
        let traders_list = traders::Entity::find()
            .filter(traders::Column::ShowInCompetition.eq(1))
            .order_by_desc(traders::Column::UpdatedAt)
            .all(&self.db)
            .await?;

        if traders_list.is_empty() {
            return Ok(vec![]);
        }

        let trader_ids: Vec<String> = traders_list.iter().map(|row| row.id.clone()).collect();
        let trader_id_set: HashSet<String> = trader_ids.iter().cloned().collect();

        let account_rows = trader_accounts::Entity::find()
            .filter(trader_accounts::Column::TraderId.is_in(trader_ids.clone()))
            .order_by_desc(trader_accounts::Column::SnapshotAt)
            .all(&self.db)
            .await?;
        let mut latest_accounts: HashMap<String, trader_accounts::Model> = HashMap::new();
        for row in account_rows {
            latest_accounts.entry(row.trader_id.clone()).or_insert(row);
        }

        let open_positions = trader_positions::Entity::find()
            .filter(trader_positions::Column::TraderId.is_in(trader_ids))
            .filter(trader_positions::Column::Status.eq("open"))
            .all(&self.db)
            .await?;
        let mut position_counts: HashMap<String, i64> = HashMap::new();
        for row in open_positions {
            if trader_id_set.contains(&row.trader_id) {
                *position_counts.entry(row.trader_id).or_insert(0) += 1;
            }
        }

        let mut out = Vec::with_capacity(traders_list.len());
        for trader in traders_list {
            let latest = latest_accounts.get(&trader.id);
            let total_balance = latest
                .map(|row| decimal_to_f64(&row.total_balance))
                .unwrap_or(0.0);
            let unrealized_pnl = latest
                .map(|row| decimal_to_f64(&row.unrealized_pnl))
                .unwrap_or(0.0);
            let realized_pnl = latest
                .map(|row| decimal_to_f64(&row.realized_pnl))
                .unwrap_or(0.0);
            let used_margin = latest
                .map(|row| decimal_to_f64(&row.used_margin))
                .unwrap_or(0.0);
            let total_pnl = unrealized_pnl;
            let total_pnl_pct = if total_balance.abs() > f64::EPSILON {
                (total_pnl / total_balance) * 100.0
            } else {
                0.0
            };
            let margin_used_pct = if total_balance.abs() > f64::EPSILON {
                (used_margin / total_balance) * 100.0
            } else {
                0.0
            };

            out.push(CompetitionTraderRecord {
                trader_id: trader.id.clone(),
                trader_name: trader.name,
                ai_model: trader.ai_model_id,
                exchange: trader.exchange_id,
                total_equity: total_balance + total_pnl,
                total_pnl,
                total_pnl_pct,
                unrealized_pnl,
                realized_pnl,
                position_count: position_counts.get(&trader.id).copied().unwrap_or(0),
                margin_used_pct,
                is_running: trader.is_running != 0,
            });
        }

        Ok(out)
    }

    pub async fn latest_public_trader_id(&self) -> Result<Option<String>, DbErr> {
        traders::Entity::find()
            .filter(traders::Column::ShowInCompetition.eq(1))
            .order_by_desc(traders::Column::UpdatedAt)
            .one(&self.db)
            .await
            .map(|row| row.map(|v| v.id))
    }

    pub async fn equity_history_points(
        &self,
        trader_id: &str,
        since_ts: Option<i64>,
        limit: i64,
    ) -> Result<Vec<EquityHistoryPointRecord>, DbErr> {
        let mut query = trader_accounts::Entity::find()
            .filter(trader_accounts::Column::TraderId.eq(trader_id.trim()))
            .order_by_asc(trader_accounts::Column::SnapshotAt)
            .limit(limit.max(0) as u64);
        if let Some(since) = since_ts {
            query = query.filter(trader_accounts::Column::SnapshotAt.gte(ts_to_dt(since)));
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

    pub async fn public_trader_config(
        &self,
        id: &str,
    ) -> Result<Option<PublicTraderConfigRecord>, DbErr> {
        traders::Entity::find_by_id(id.trim().to_string())
            .filter(traders::Column::ShowInCompetition.eq(1))
            .one(&self.db)
            .await
            .map(|row| {
                row.map(|row| PublicTraderConfigRecord {
                    trader_id: row.id,
                    trader_name: row.name,
                    ai_model: row.ai_model_id,
                    exchange_id: row.exchange_id,
                    strategy_id: row.strategy_id,
                    is_cross_margin: row.is_cross_margin != 0,
                    show_in_competition: row.show_in_competition != 0,
                    scan_interval_minutes: row.scan_interval_minutes,
                    initial_balance: decimal_to_f64(&row.initial_balance),
                    is_running: row.is_running != 0,
                    btc_eth_leverage: row.btc_eth_leverage,
                    altcoin_leverage: row.altcoin_leverage,
                    trading_symbols: row.trading_symbols,
                    custom_prompt: row.custom_prompt,
                    override_base_prompt: row.override_base_prompt != 0,
                    system_prompt_template: row.system_prompt_template,
                    use_ai500: row.use_ai500 != 0,
                    use_oi_top: row.use_oi_top != 0,
                })
            })
    }
}

fn decimal_to_f64(value: &Decimal) -> f64 {
    value.to_f64().unwrap_or(0.0)
}
