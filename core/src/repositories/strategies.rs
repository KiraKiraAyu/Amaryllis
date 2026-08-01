use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
    prelude::Expr,
};

use crate::{
    database::DbErr,
    entity::strategies,
    time::{dt_to_ts, ts_to_dt},
};

#[derive(Debug, Clone)]
pub struct StrategyRepo {
    db: DatabaseConnection,
}

#[derive(Debug, Clone)]
pub struct StrategyRecord {
    pub id: String,
    pub name: String,
    pub description: String,
    pub is_active: bool,
    pub config: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct CreateStrategyRecord {
    pub id: String,
    pub name: String,
    pub description: String,
    pub config: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct UpdateStrategyRecord {
    pub name: String,
    pub description: String,
    pub config: String,
    pub updated_at: i64,
}

impl StrategyRepo {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn list_for_user_with_defaults(
        &self,
    ) -> Result<Vec<StrategyRecord>, DbErr> {
        // Strategies are global (no per-user ownership column), so the list
        // includes every record, newest first.
        strategies::Entity::find()
            .order_by_desc(strategies::Column::CreatedAt)
            .all(&self.db)
            .await
            .map(map_strategy_rows)
    }

    pub async fn get_accessible(
        &self,
        id: &str,
    ) -> Result<Option<StrategyRecord>, DbErr> {
        strategies::Entity::find_by_id(id.trim().to_string())
            .one(&self.db)
            .await
            .map(|row| row.map(map_strategy_row))
    }

    pub async fn get_owned(
        &self,
        id: &str,
    ) -> Result<Option<StrategyRecord>, DbErr> {
        strategies::Entity::find_by_id(id.trim().to_string())
            .one(&self.db)
            .await
            .map(|row| row.map(map_strategy_row))
    }

    pub async fn get_duplicable(
        &self,
        id: &str,
    ) -> Result<Option<StrategyRecord>, DbErr> {
        strategies::Entity::find_by_id(id.trim().to_string())
            .one(&self.db)
            .await
            .map(|row| row.map(map_strategy_row))
    }

    pub async fn create(&self, input: CreateStrategyRecord) -> Result<(), DbErr> {
        strategies::ActiveModel {
            id: Set(input.id),
            name: Set(input.name),
            description: Set(input.description),
            is_active: Set(0),
            config: Set(input.config),
            created_at: Set(ts_to_dt(input.created_at)),
            updated_at: Set(ts_to_dt(input.updated_at)),
        }
        .insert(&self.db)
        .await
        .map(|_| ())
    }

    pub async fn update_owned(
        &self,
        id: &str,
        patch: UpdateStrategyRecord,
    ) -> Result<u64, DbErr> {
        strategies::Entity::update_many()
            .col_expr(strategies::Column::Name, Expr::value(patch.name))
            .col_expr(
                strategies::Column::Description,
                Expr::value(patch.description),
            )
            .col_expr(strategies::Column::Config, Expr::value(patch.config))
            .col_expr(
                strategies::Column::UpdatedAt,
                Expr::value(ts_to_dt(patch.updated_at)),
            )
            .filter(strategies::Column::Id.eq(id.trim()))
            .exec(&self.db)
            .await
            .map(|res| res.rows_affected)
    }

    pub async fn delete_owned(&self, id: &str) -> Result<u64, DbErr> {
        strategies::Entity::delete_many()
            .filter(strategies::Column::Id.eq(id.trim()))
            .exec(&self.db)
            .await
            .map(|res| res.rows_affected)
    }

    pub async fn deactivate_all_for_user(
        &self,
        updated_at: i64,
    ) -> Result<(), DbErr> {
        strategies::Entity::update_many()
            .col_expr(strategies::Column::IsActive, Expr::value(0))
            .col_expr(
                strategies::Column::UpdatedAt,
                Expr::value(ts_to_dt(updated_at)),
            )
            .exec(&self.db)
            .await
            .map(|_| ())
    }

    pub async fn activate_owned(
        &self,
        id: &str,
        updated_at: i64,
    ) -> Result<u64, DbErr> {
        strategies::Entity::update_many()
            .col_expr(strategies::Column::IsActive, Expr::value(1))
            .col_expr(
                strategies::Column::UpdatedAt,
                Expr::value(ts_to_dt(updated_at)),
            )
            .filter(strategies::Column::Id.eq(id.trim()))
            .exec(&self.db)
            .await
            .map(|res| res.rows_affected)
    }

    pub async fn active_for_user(&self) -> Result<Option<StrategyRecord>, DbErr> {
        strategies::Entity::find()
            .filter(strategies::Column::IsActive.eq(1))
            .order_by_desc(strategies::Column::UpdatedAt)
            .one(&self.db)
            .await
            .map(|row| row.map(map_strategy_row))
    }
}

fn map_strategy_rows(rows: Vec<strategies::Model>) -> Vec<StrategyRecord> {
    rows.into_iter().map(map_strategy_row).collect()
}

fn map_strategy_row(row: strategies::Model) -> StrategyRecord {
    StrategyRecord {
        id: row.id,
        name: row.name,
        description: row.description,
        is_active: row.is_active != 0,
        config: row.config,
        created_at: dt_to_ts(row.created_at),
        updated_at: dt_to_ts(row.updated_at),
    }
}
