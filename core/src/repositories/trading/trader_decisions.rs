use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};

use crate::{database::DbErr, entity, time::ts_to_dt};

use super::{
    TradingRepo,
    mappers::history::map_decision,
    records::history::{InsertTraderDecisionRecord, TraderDecisionRecord},
    values::decimal_from_f64,
};

impl TradingRepo {
    pub async fn insert_decision(&self, input: InsertTraderDecisionRecord) -> Result<(), DbErr> {
        entity::trader_decisions::ActiveModel {
            id: Set(input.id),
            trader_id: Set(input.trader_id),
            symbol: Set(input.symbol),
            timeframe: Set(input.timeframe),
            decision: Set(input.decision),
            confidence: Set(decimal_from_f64(input.confidence)),
            reason: Set(input.reason),
            payload_json: Set(input.payload_json),
            created_at: Set(ts_to_dt(input.created_at)),
        }
        .insert(&self.db)
        .await
        .map(|_| ())
    }

    pub async fn system_decision_exists(
        &self,
        trader_id: &str,
        reason: &str,
    ) -> Result<bool, DbErr> {
        entity::trader_decisions::Entity::find()
            .filter(entity::trader_decisions::Column::TraderId.eq(trader_id.trim()))
            .filter(entity::trader_decisions::Column::Decision.eq("SYSTEM"))
            .filter(entity::trader_decisions::Column::Reason.eq(reason.trim()))
            .one(&self.db)
            .await
            .map(|row| row.is_some())
    }

    pub async fn decisions(
        &self,
        trader_id: &str,
        symbol: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<TraderDecisionRecord>, DbErr> {
        let mut query = entity::trader_decisions::Entity::find()
            .filter(entity::trader_decisions::Column::TraderId.eq(trader_id.trim()))
            .order_by_desc(entity::trader_decisions::Column::CreatedAt)
            .order_by_desc(entity::trader_decisions::Column::Id)
            .limit(limit.max(0) as u64)
            .offset(offset.max(0) as u64);
        if let Some(symbol) = symbol {
            query = query.filter(entity::trader_decisions::Column::Symbol.eq(symbol.trim()));
        }
        query
            .all(&self.db)
            .await
            .map(|rows| rows.into_iter().map(map_decision).collect())
    }
}
