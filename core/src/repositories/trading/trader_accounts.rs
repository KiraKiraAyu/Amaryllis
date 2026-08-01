use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};

use crate::{database::DbErr, entity, time::ts_to_dt};

use super::{
    TradingRepo,
    mappers::accounts::map_account,
    records::accounts::TraderAccountRecord,
    values::{decimal_from_f64, decimal_to_f64},
};

impl TradingRepo {
    pub async fn latest_account(
        &self,
        trader_id: &str,
    ) -> Result<Option<TraderAccountRecord>, DbErr> {
        entity::trader_accounts::Entity::find()
            .filter(entity::trader_accounts::Column::TraderId.eq(trader_id.trim()))
            .order_by_desc(entity::trader_accounts::Column::SnapshotAt)
            .one(&self.db)
            .await
            .map(|row| row.map(map_account))
    }

    pub async fn insert_account_snapshot(
        &self,
        snapshot_id: String,
        trader_id: &str,
        exchange_id: &str,
        account: &TraderAccountRecord,
        now: i64,
    ) -> Result<(), DbErr> {
        entity::trader_accounts::ActiveModel {
            id: Set(snapshot_id),
            trader_id: Set(trader_id.to_string()),
            exchange_id: Set(exchange_id.to_string()),
            total_balance: Set(decimal_from_f64(account.total_balance)),
            available_balance: Set(decimal_from_f64(account.available_balance)),
            used_margin: Set(decimal_from_f64(account.used_margin)),
            unrealized_pnl: Set(decimal_from_f64(account.unrealized_pnl)),
            realized_pnl: Set(decimal_from_f64(account.realized_pnl)),
            currency: Set(account.currency.clone()),
            snapshot_at: Set(ts_to_dt(now)),
            created_at: Set(ts_to_dt(now)),
            updated_at: Set(ts_to_dt(now)),
        }
        .insert(&self.db)
        .await
        .map(|_| ())
    }

    pub async fn compute_account_totals(
        &self,
        trader_id: &str,
    ) -> Result<(f64, f64, f64), DbErr> {
        let positions = entity::trader_positions::Entity::find()
            .filter(entity::trader_positions::Column::TraderId.eq(trader_id.trim()))
            .filter(entity::trader_positions::Column::Status.eq("open"))
            .all(&self.db)
            .await?;

        let unrealized_pnl = positions
            .iter()
            .map(|row| decimal_to_f64(&row.unrealized_pnl))
            .sum();
        let used_margin = positions
            .iter()
            .map(|row| {
                let leverage = i64::from(row.leverage).max(1) as f64;
                (decimal_to_f64(&row.quantity) * decimal_to_f64(&row.mark_price)).abs() / leverage
            })
            .sum();

        let trades = entity::trader_trades::Entity::find()
            .filter(entity::trader_trades::Column::TraderId.eq(trader_id.trim()))
            .all(&self.db)
            .await?;
        let realized_pnl = trades
            .iter()
            .map(|row| decimal_to_f64(&row.realized_pnl) - decimal_to_f64(&row.fees))
            .sum();

        Ok((used_margin, unrealized_pnl, realized_pnl))
    }
}
