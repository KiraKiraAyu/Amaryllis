use std::collections::HashSet;

use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait, prelude::Expr,
};

use crate::{
    database::DbErr,
    entity,
    time::{dt_to_ts, ts_to_dt},
};

use super::{
    TradingRepo,
    mappers::positions::map_position,
    records::{
        history::InsertTraderTradeRecord,
        positions::{
            InsertTraderPositionRecord, TraderPositionRecord, UpsertPositionFromExchangeRecord,
        },
    },
    values::{decimal_from_f64, decimal_to_f64},
};

impl TradingRepo {
    pub async fn close_open_positions(
        &self,
        trader_id: &str,
        symbol: &str,
        side: &str,
        trade_ids: Vec<String>,
        now: i64,
    ) -> Result<usize, DbErr> {
        let rows = entity::trader_positions::Entity::find()
            .filter(entity::trader_positions::Column::TraderId.eq(trader_id.trim()))
            .filter(entity::trader_positions::Column::Symbol.eq(symbol.trim()))
            .filter(entity::trader_positions::Column::Side.eq(side.trim()))
            .filter(entity::trader_positions::Column::Status.eq("open"))
            .all(&self.db)
            .await?;
        if rows.is_empty() {
            return Ok(0);
        }

        let tx = self.db.begin().await?;
        for (index, row) in rows.iter().enumerate() {
            let entry = decimal_to_f64(&row.entry_price);
            let mark = decimal_to_f64(&row.mark_price);
            let qty = decimal_to_f64(&row.quantity);
            let exit = if mark > 0.0 { mark } else { entry };
            let pnl = if side == "LONG" {
                (exit - entry) * qty
            } else {
                (entry - exit) * qty
            };

            entity::trader_positions::Entity::update_many()
                .col_expr(
                    entity::trader_positions::Column::Status,
                    Expr::value("closed"),
                )
                .col_expr(
                    entity::trader_positions::Column::ClosedAt,
                    Expr::value(ts_to_dt(now)),
                )
                .col_expr(
                    entity::trader_positions::Column::RealizedPnl,
                    Expr::value(decimal_from_f64(pnl)),
                )
                .col_expr(
                    entity::trader_positions::Column::UpdatedAt,
                    Expr::value(ts_to_dt(now)),
                )
                .filter(entity::trader_positions::Column::Id.eq(row.id.clone()))
                .exec(&tx)
                .await?;

            let roi_pct = if entry.abs() > f64::EPSILON {
                pnl / (entry * qty).abs().max(1e-9) * 100.0
            } else {
                0.0
            };
            let trade_id = trade_ids
                .get(index)
                .cloned()
                .unwrap_or_else(|| format!("{}-{index}", row.id));
            entity::trader_trades::ActiveModel {
                id: Set(trade_id),
                trader_id: Set(trader_id.to_string()),
                symbol: Set(symbol.to_string()),
                side: Set(side.to_string()),
                entry_price: Set(decimal_from_f64(entry)),
                exit_price: Set(decimal_from_f64(exit)),
                quantity: Set(decimal_from_f64(qty)),
                realized_pnl: Set(decimal_from_f64(pnl)),
                fees: Set(Decimal::ZERO),
                roi_pct: Set(decimal_from_f64(roi_pct)),
                opened_at: Set(ts_to_dt(row.opened_at.timestamp())),
                closed_at: Set(ts_to_dt(now)),
                created_at: Set(ts_to_dt(now)),
            }
            .insert(&tx)
            .await?;
        }
        tx.commit().await?;
        Ok(rows.len())
    }

    pub async fn open_position_records(
        &self,
        trader_id: &str,
        symbol: Option<&str>,
        side: Option<&str>,
    ) -> Result<Vec<TraderPositionRecord>, DbErr> {
        let mut query = entity::trader_positions::Entity::find()
            .filter(entity::trader_positions::Column::TraderId.eq(trader_id.trim()))
            .filter(entity::trader_positions::Column::Status.eq("open"));
        if let Some(symbol) = symbol {
            query = query.filter(entity::trader_positions::Column::Symbol.eq(symbol.trim()));
        }
        if let Some(side) = side {
            query = query.filter(entity::trader_positions::Column::Side.eq(side.trim()));
        }
        query
            .all(&self.db)
            .await
            .map(|rows| rows.into_iter().map(map_position).collect())
    }

    pub async fn close_open_positions_for_symbol_side(
        &self,
        trader_id: &str,
        symbol: &str,
        side: &str,
        closed_at: i64,
        updated_at: i64,
    ) -> Result<(), DbErr> {
        entity::trader_positions::Entity::update_many()
            .col_expr(
                entity::trader_positions::Column::Quantity,
                Expr::value(decimal_from_f64(0.0)),
            )
            .col_expr(
                entity::trader_positions::Column::Status,
                Expr::value("closed"),
            )
            .col_expr(
                entity::trader_positions::Column::ClosedAt,
                Expr::value(ts_to_dt(closed_at)),
            )
            .col_expr(
                entity::trader_positions::Column::UpdatedAt,
                Expr::value(ts_to_dt(updated_at)),
            )
            .filter(entity::trader_positions::Column::TraderId.eq(trader_id.trim()))
            .filter(entity::trader_positions::Column::Symbol.eq(symbol.trim()))
            .filter(entity::trader_positions::Column::Side.eq(side.trim()))
            .filter(entity::trader_positions::Column::Status.eq("open"))
            .exec(&self.db)
            .await
            .map(|_| ())
    }

    pub async fn upsert_open_position_from_exchange(
        &self,
        input: UpsertPositionFromExchangeRecord,
    ) -> Result<(), DbErr> {
        let existing = entity::trader_positions::Entity::find()
            .filter(entity::trader_positions::Column::TraderId.eq(input.trader_id.trim()))
            .filter(entity::trader_positions::Column::Symbol.eq(input.symbol.trim()))
            .filter(entity::trader_positions::Column::Side.eq(input.side.trim()))
            .filter(entity::trader_positions::Column::Status.eq("open"))
            .one(&self.db)
            .await?;

        if let Some(existing) = existing {
            let mut active = existing.into_active_model();
            active.quantity = Set(decimal_from_f64(input.quantity));
            active.entry_price = Set(decimal_from_f64(input.entry_price));
            active.mark_price = Set(decimal_from_f64(input.mark_price));
            active.liquidation_price = Set(decimal_from_f64(input.liquidation_price));
            active.leverage = Set(input.leverage.max(1) as i32);
            active.unrealized_pnl = Set(decimal_from_f64(input.unrealized_pnl));
            active.updated_at = Set(ts_to_dt(input.updated_at));
            active.status = Set("open".to_string());
            active.update(&self.db).await?;
        } else {
            self.insert_position(InsertTraderPositionRecord {
                id: uuid::Uuid::now_v7().to_string(),
                trader_id: input.trader_id,
                symbol: input.symbol,
                side: input.side,
                quantity: input.quantity,
                entry_price: input.entry_price,
                mark_price: input.mark_price,
                liquidation_price: input.liquidation_price,
                leverage: input.leverage.max(1),
                margin_mode: "cross".to_string(),
                unrealized_pnl: input.unrealized_pnl,
                realized_pnl: 0.0,
                status: "open".to_string(),
                opened_at: input.event_at,
                closed_at: None,
                created_at: input.updated_at,
                updated_at: input.updated_at,
            })
            .await?;
        }
        Ok(())
    }

    pub async fn insert_position(&self, input: InsertTraderPositionRecord) -> Result<(), DbErr> {
        entity::trader_positions::ActiveModel {
            id: Set(input.id),
            trader_id: Set(input.trader_id),
            symbol: Set(input.symbol),
            side: Set(input.side),
            quantity: Set(decimal_from_f64(input.quantity)),
            entry_price: Set(decimal_from_f64(input.entry_price)),
            mark_price: Set(decimal_from_f64(input.mark_price)),
            liquidation_price: Set(decimal_from_f64(input.liquidation_price)),
            leverage: Set(input.leverage as i32),
            margin_mode: Set(input.margin_mode),
            unrealized_pnl: Set(decimal_from_f64(input.unrealized_pnl)),
            realized_pnl: Set(decimal_from_f64(input.realized_pnl)),
            status: Set(input.status),
            opened_at: Set(ts_to_dt(input.opened_at)),
            closed_at: Set(input.closed_at.map(ts_to_dt)),
            created_at: Set(ts_to_dt(input.created_at)),
            updated_at: Set(ts_to_dt(input.updated_at)),
        }
        .insert(&self.db)
        .await
        .map(|_| ())
    }

    pub async fn close_position(
        &self,
        trader_id: &str,
        position_id: &str,
        exit_price: f64,
        realized_pnl: f64,
        closed_at: i64,
    ) -> Result<(), DbErr> {
        entity::trader_positions::Entity::update_many()
            .col_expr(
                entity::trader_positions::Column::MarkPrice,
                Expr::value(decimal_from_f64(exit_price)),
            )
            .col_expr(
                entity::trader_positions::Column::UnrealizedPnl,
                Expr::value(Decimal::ZERO),
            )
            .col_expr(
                entity::trader_positions::Column::RealizedPnl,
                Expr::value(decimal_from_f64(realized_pnl)),
            )
            .col_expr(
                entity::trader_positions::Column::Status,
                Expr::value("closed"),
            )
            .col_expr(
                entity::trader_positions::Column::ClosedAt,
                Expr::value(ts_to_dt(closed_at)),
            )
            .col_expr(
                entity::trader_positions::Column::UpdatedAt,
                Expr::value(ts_to_dt(closed_at)),
            )
            .filter(entity::trader_positions::Column::Id.eq(position_id.trim()))
            .filter(entity::trader_positions::Column::TraderId.eq(trader_id.trim()))
            .filter(entity::trader_positions::Column::Status.eq("open"))
            .exec(&self.db)
            .await
            .map(|_| ())
    }

    pub async fn close_open_positions_missing_from_exchange(
        &self,
        trader_id: &str,
        live_keys: &HashSet<String>,
        closed_at: i64,
    ) -> Result<(), DbErr> {
        let rows = entity::trader_positions::Entity::find()
            .filter(entity::trader_positions::Column::TraderId.eq(trader_id.trim()))
            .filter(entity::trader_positions::Column::Status.eq("open"))
            .all(&self.db)
            .await?;

        for row in rows {
            let key = format!(
                "{}:{}",
                row.symbol.trim().to_uppercase(),
                row.side.trim().to_uppercase()
            );
            if !live_keys.contains(&key) {
                entity::trader_positions::Entity::update_many()
                    .col_expr(
                        entity::trader_positions::Column::Status,
                        Expr::value("closed"),
                    )
                    .col_expr(
                        entity::trader_positions::Column::ClosedAt,
                        Expr::value(ts_to_dt(closed_at)),
                    )
                    .col_expr(
                        entity::trader_positions::Column::UpdatedAt,
                        Expr::value(ts_to_dt(closed_at)),
                    )
                    .filter(entity::trader_positions::Column::Id.eq(row.id))
                    .filter(entity::trader_positions::Column::TraderId.eq(trader_id.trim()))
                    .filter(entity::trader_positions::Column::Status.eq("open"))
                    .exec(&self.db)
                    .await?;
            }
        }

        Ok(())
    }

    pub async fn update_position_mark_to_market(
        &self,
        trader_id: &str,
        position_id: &str,
        mark_price: f64,
        unrealized_pnl: f64,
        updated_at: i64,
    ) -> Result<(), DbErr> {
        entity::trader_positions::Entity::update_many()
            .col_expr(
                entity::trader_positions::Column::MarkPrice,
                Expr::value(decimal_from_f64(mark_price)),
            )
            .col_expr(
                entity::trader_positions::Column::UnrealizedPnl,
                Expr::value(decimal_from_f64(unrealized_pnl)),
            )
            .col_expr(
                entity::trader_positions::Column::UpdatedAt,
                Expr::value(ts_to_dt(updated_at)),
            )
            .filter(entity::trader_positions::Column::Id.eq(position_id.trim()))
            .filter(entity::trader_positions::Column::TraderId.eq(trader_id.trim()))
            .exec(&self.db)
            .await
            .map(|_| ())
    }

    pub async fn apply_close_fill_to_open_positions(
        &self,
        trader_id: &str,
        symbol: &str,
        position_side: &str,
        fill_qty: f64,
        fill_price: f64,
        fill_fee: f64,
        trade_time: i64,
        updated_at: i64,
    ) -> Result<f64, DbErr> {
        if fill_qty <= f64::EPSILON || fill_price <= f64::EPSILON {
            return Ok(0.0);
        }

        let symbol = symbol.trim().to_uppercase();
        let position_side = position_side.trim().to_uppercase();
        let mut remaining_qty = fill_qty;
        let mut remaining_fee = fill_fee.max(0.0);
        let mut applied_qty = 0.0_f64;

        while remaining_qty > 1e-9 {
            let Some(pos) = entity::trader_positions::Entity::find()
                .filter(entity::trader_positions::Column::TraderId.eq(trader_id.trim()))
                .filter(entity::trader_positions::Column::Symbol.eq(&symbol))
                .filter(entity::trader_positions::Column::Side.eq(&position_side))
                .filter(entity::trader_positions::Column::Status.eq("open"))
                .order_by_asc(entity::trader_positions::Column::OpenedAt)
                .one(&self.db)
                .await?
            else {
                break;
            };

            let entry_price = decimal_to_f64(&pos.entry_price);
            let pos_qty = decimal_to_f64(&pos.quantity);
            let opened_at = dt_to_ts(pos.opened_at);
            let current_realized_pnl = decimal_to_f64(&pos.realized_pnl);

            if pos_qty <= f64::EPSILON {
                break;
            }

            let close_qty = remaining_qty.min(pos_qty);
            let fee_for_this_close = if remaining_qty > 0.0 {
                (remaining_fee * (close_qty / remaining_qty)).max(0.0)
            } else {
                0.0
            };

            let pnl = if position_side == "LONG" {
                (fill_price - entry_price) * close_qty
            } else {
                (entry_price - fill_price) * close_qty
            };
            let net_pnl = pnl - fee_for_this_close;
            let remaining_pos_qty = (pos_qty - close_qty).max(0.0);

            let mut active = pos.into_active_model();
            if remaining_pos_qty <= 1e-9 {
                active.quantity = Set(decimal_from_f64(0.0));
                active.mark_price = Set(decimal_from_f64(fill_price));
                active.unrealized_pnl = Set(Decimal::ZERO);
                active.realized_pnl = Set(decimal_from_f64(current_realized_pnl + net_pnl));
                active.status = Set("closed".to_string());
                active.closed_at = Set(Some(ts_to_dt(trade_time)));
                active.updated_at = Set(ts_to_dt(updated_at));
            } else {
                active.quantity = Set(decimal_from_f64(remaining_pos_qty));
                active.mark_price = Set(decimal_from_f64(fill_price));
                active.realized_pnl = Set(decimal_from_f64(current_realized_pnl + net_pnl));
                active.updated_at = Set(ts_to_dt(updated_at));
            }
            active.update(&self.db).await?;

            let roi_pct = if entry_price.abs() > f64::EPSILON {
                ((fill_price - entry_price) / entry_price)
                    * 100.0
                    * if position_side == "LONG" { 1.0 } else { -1.0 }
            } else {
                0.0
            };

            self.insert_trade(InsertTraderTradeRecord {
                id: uuid::Uuid::now_v7().to_string(),
                trader_id: trader_id.to_string(),
                symbol: symbol.clone(),
                side: position_side.clone(),
                entry_price,
                exit_price: fill_price,
                quantity: close_qty,
                realized_pnl: net_pnl,
                fees: fee_for_this_close,
                roi_pct,
                opened_at,
                closed_at: trade_time,
                created_at: updated_at,
            })
            .await?;

            remaining_qty = (remaining_qty - close_qty).max(0.0);
            remaining_fee = (remaining_fee - fee_for_this_close).max(0.0);
            applied_qty += close_qty;
        }

        Ok(applied_qty)
    }

    pub async fn positions_by_status(
        &self,
        trader_id: &str,
        status: &str,
    ) -> Result<Vec<TraderPositionRecord>, DbErr> {
        entity::trader_positions::Entity::find()
            .filter(entity::trader_positions::Column::TraderId.eq(trader_id.trim()))
            .filter(entity::trader_positions::Column::Status.eq(status.trim()))
            .order_by_desc(entity::trader_positions::Column::OpenedAt)
            .all(&self.db)
            .await
            .map(|rows| rows.into_iter().map(map_position).collect())
    }

    /// Batch-load positions for multiple traders in a single query.
    /// Returns empty vec when `trader_ids` is empty.
    pub async fn positions_by_trader_ids(
        &self,
        trader_ids: &[String],
        status: &str,
    ) -> Result<Vec<TraderPositionRecord>, DbErr> {
        if trader_ids.is_empty() {
            return Ok(Vec::new());
        }
        entity::trader_positions::Entity::find()
            .filter(entity::trader_positions::Column::TraderId.is_in(trader_ids.to_vec()))
            .filter(entity::trader_positions::Column::Status.eq(status.trim()))
            .order_by_desc(entity::trader_positions::Column::OpenedAt)
            .all(&self.db)
            .await
            .map(|rows| rows.into_iter().map(map_position).collect())
    }

    pub async fn closed_positions(
        &self,
        trader_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<TraderPositionRecord>, DbErr> {
        entity::trader_positions::Entity::find()
            .filter(entity::trader_positions::Column::TraderId.eq(trader_id.trim()))
            .filter(entity::trader_positions::Column::Status.eq("closed"))
            .order_by_desc(entity::trader_positions::Column::ClosedAt)
            .order_by_desc(entity::trader_positions::Column::UpdatedAt)
            .limit(limit.max(0) as u64)
            .offset(offset.max(0) as u64)
            .all(&self.db)
            .await
            .map(|rows| rows.into_iter().map(map_position).collect())
    }
}
