//! `SeaORM` Entity, hand-maintained for Debate Arena.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "debates")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub id: String,
    #[sea_orm(column_type = "Text")]
    pub name: String,
    #[sea_orm(column_type = "Text")]
    pub symbol: String,
    #[sea_orm(column_type = "Text")]
    pub status: String,
    pub max_rounds: i32,
    pub current_round: i32,
    #[sea_orm(column_type = "Text")]
    pub participants_json: String,
    #[sea_orm(column_type = "Text")]
    pub final_decision: String,
    #[sea_orm(column_type = "Text")]
    pub final_reasoning: String,
    #[sea_orm(column_type = "Text")]
    pub error_message: String,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
