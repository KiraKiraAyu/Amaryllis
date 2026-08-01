use std::collections::HashMap;

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, prelude::Expr,
};
use serde_json::{Value, json};

use crate::{
    database::DbErr,
    entity::{debate_messages, debates},
    time::{dt_to_ts, ts_to_dt},
};

#[derive(Debug, Clone)]
pub struct DebateRepo {
    db: DatabaseConnection,
}

#[derive(Debug, Clone)]
pub struct CreateDebateRecord {
    pub id: String,
    pub name: String,
    pub symbol: String,
    pub status: String,
    pub max_rounds: i64,
    pub prompt_variant: String,
    pub participants_json: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct CreateDebateMessageRecord {
    pub id: String,
    pub debate_id: String,
    pub round: i64,
    pub personality: String,
    pub role: String,
    pub content: String,
    pub vote: String,
    pub created_at: i64,
}

impl DebateRepo {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn get(&self, debate_id: &str) -> Result<Option<Value>, DbErr> {
        debates::Entity::find_by_id(debate_id.trim().to_string())
            .one(&self.db)
            .await
            .map(|row| row.map(debate_to_json))
    }

    pub async fn list(&self, limit: i64) -> Result<Vec<Value>, DbErr> {
        debates::Entity::find()
            .order_by_desc(debates::Column::CreatedAt)
            .limit(limit.max(0) as u64)
            .all(&self.db)
            .await
            .map(|rows| rows.into_iter().map(debate_summary_to_json).collect())
    }

    pub async fn create(&self, input: CreateDebateRecord) -> Result<(), DbErr> {
        debates::ActiveModel {
            id: Set(input.id),
            name: Set(input.name),
            symbol: Set(input.symbol),
            status: Set(input.status),
            max_rounds: Set(input.max_rounds as i32),
            current_round: Set(0),
            prompt_variant: Set(input.prompt_variant),
            participants_json: Set(input.participants_json),
            final_decision: Set(String::new()),
            final_reasoning: Set(String::new()),
            error_message: Set(String::new()),
            created_at: Set(ts_to_dt(input.created_at)),
            updated_at: Set(ts_to_dt(input.updated_at)),
        }
        .insert(&self.db)
        .await
        .map(|_| ())
    }

    pub async fn delete(&self, debate_id: &str) -> Result<u64, DbErr> {
        let deleted = debates::Entity::delete_many()
            .filter(debates::Column::Id.eq(debate_id.trim()))
            .exec(&self.db)
            .await?;

        debate_messages::Entity::delete_many()
            .filter(debate_messages::Column::DebateId.eq(debate_id.trim()))
            .exec(&self.db)
            .await?;

        Ok(deleted.rows_affected)
    }

    pub async fn update_status(
        &self,
        debate_id: &str,
        status: &str,
        updated_at: i64,
    ) -> Result<(), DbErr> {
        debates::Entity::update_many()
            .col_expr(debates::Column::Status, Expr::value(status.to_string()))
            .col_expr(
                debates::Column::UpdatedAt,
                Expr::value(ts_to_dt(updated_at)),
            )
            .filter(debates::Column::Id.eq(debate_id.trim()))
            .exec(&self.db)
            .await
            .map(|_| ())
    }

    pub async fn update_current_round(
        &self,
        debate_id: &str,
        round: i64,
        updated_at: i64,
    ) -> Result<(), DbErr> {
        debates::Entity::update_many()
            .col_expr(debates::Column::CurrentRound, Expr::value(round as i32))
            .col_expr(
                debates::Column::UpdatedAt,
                Expr::value(ts_to_dt(updated_at)),
            )
            .filter(debates::Column::Id.eq(debate_id.trim()))
            .exec(&self.db)
            .await
            .map(|_| ())
    }

    pub async fn complete(
        &self,
        debate_id: &str,
        status: &str,
        final_decision: String,
        final_reasoning: String,
        updated_at: i64,
    ) -> Result<(), DbErr> {
        debates::Entity::update_many()
            .col_expr(debates::Column::Status, Expr::value(status.to_string()))
            .col_expr(debates::Column::FinalDecision, Expr::value(final_decision))
            .col_expr(
                debates::Column::FinalReasoning,
                Expr::value(final_reasoning),
            )
            .col_expr(
                debates::Column::UpdatedAt,
                Expr::value(ts_to_dt(updated_at)),
            )
            .filter(debates::Column::Id.eq(debate_id.trim()))
            .exec(&self.db)
            .await
            .map(|_| ())
    }

    pub async fn list_messages(&self, debate_id: &str) -> Result<Vec<Value>, DbErr> {
        if !self.debate_exists(debate_id).await? {
            return Ok(vec![]);
        }

        debate_messages::Entity::find()
            .filter(debate_messages::Column::DebateId.eq(debate_id.trim()))
            .order_by_asc(debate_messages::Column::Round)
            .order_by_asc(debate_messages::Column::CreatedAt)
            .all(&self.db)
            .await
            .map(|rows| rows.into_iter().map(debate_message_to_json).collect())
    }

    pub async fn votes(&self, debate_id: &str) -> Result<Value, DbErr> {
        if !self.debate_exists(debate_id).await? {
            return Ok(json!([]));
        }

        let rows = debate_messages::Entity::find()
            .filter(debate_messages::Column::DebateId.eq(debate_id.trim()))
            .filter(debate_messages::Column::Vote.ne(""))
            .order_by_asc(debate_messages::Column::Round)
            .all(&self.db)
            .await?;

        let mut by_round: HashMap<i64, Vec<(String, String)>> = HashMap::new();
        for row in rows {
            by_round
                .entry(i64::from(row.round))
                .or_default()
                .push((row.personality, row.vote));
        }

        let mut result: Vec<Value> = Vec::new();
        for (round, votes) in by_round {
            let mut tally: HashMap<&str, i64> = HashMap::new();
            for (_, vote) in &votes {
                *tally.entry(vote.as_str()).or_insert(0) += 1;
            }
            result.push(json!({
                "round": round,
                "votes": votes.iter().map(|(p, v)| json!({"personality": p, "vote": v})).collect::<Vec<_>>(),
                "tally": tally,
            }));
        }
        result.sort_by_key(|v| v["round"].as_i64().unwrap_or(0));
        Ok(json!(result))
    }

    pub async fn insert_message(&self, input: CreateDebateMessageRecord) -> Result<(), DbErr> {
        debate_messages::ActiveModel {
            id: Set(input.id),
            debate_id: Set(input.debate_id),
            round: Set(input.round as i32),
            personality: Set(input.personality),
            role: Set(input.role),
            content: Set(input.content),
            vote: Set(input.vote),
            created_at: Set(ts_to_dt(input.created_at)),
        }
        .insert(&self.db)
        .await
        .map(|_| ())
    }

    pub async fn vote_tally(&self, debate_id: &str) -> Result<HashMap<String, i64>, DbErr> {
        let rows = debate_messages::Entity::find()
            .filter(debate_messages::Column::DebateId.eq(debate_id.trim()))
            .filter(debate_messages::Column::Vote.ne(""))
            .all(&self.db)
            .await?;

        let mut tally = HashMap::new();
        for row in rows {
            *tally.entry(row.vote).or_insert(0) += 1;
        }
        Ok(tally)
    }

    async fn debate_exists(&self, debate_id: &str) -> Result<bool, DbErr> {
        debates::Entity::find_by_id(debate_id.trim().to_string())
            .one(&self.db)
            .await
            .map(|row| row.is_some())
    }
}

fn parse_participants(raw: &str) -> Value {
    serde_json::from_str(raw).unwrap_or_else(|_| json!([]))
}

fn debate_to_json(row: debates::Model) -> Value {
    json!({
        "id": row.id,
        "name": row.name,
        "symbol": row.symbol,
        "status": row.status,
        "max_rounds": row.max_rounds,
        "current_round": row.current_round,
        "prompt_variant": row.prompt_variant,
        "participants": parse_participants(&row.participants_json),
        "final_decision": row.final_decision,
        "final_reasoning": row.final_reasoning,
        "error_message": row.error_message,
        "created_at": dt_to_ts(row.created_at),
        "updated_at": dt_to_ts(row.updated_at),
    })
}

fn debate_summary_to_json(row: debates::Model) -> Value {
    json!({
        "id": row.id,
        "name": row.name,
        "symbol": row.symbol,
        "status": row.status,
        "max_rounds": row.max_rounds,
        "current_round": row.current_round,
        "final_decision": row.final_decision,
        "created_at": dt_to_ts(row.created_at),
    })
}

fn debate_message_to_json(row: debate_messages::Model) -> Value {
    json!({
        "id": row.id,
        "round": row.round,
        "personality": row.personality,
        "role": row.role,
        "content": row.content,
        "vote": row.vote,
        "created_at": dt_to_ts(row.created_at),
    })
}
