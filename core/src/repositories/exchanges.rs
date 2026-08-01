use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
    prelude::Expr,
};

use crate::entity::{exchanges, traders};
use crate::time::ts_to_dt;

#[derive(Debug, Clone)]
pub struct ExchangeRepo {
    db: DatabaseConnection,
}

pub struct ExchangeConfigRecord {
    pub id: String,
    pub exchange_type: String,
    pub account_name: String,
    pub name: String,
    pub exchange_kind: String,
    pub enabled: i64,
    pub testnet: i64,
    pub hyperliquid_wallet_addr: String,
}

pub struct ExchangeSecretRecord {
    pub api_key: String,
    pub secret_key: String,
    pub passphrase: String,
}

#[derive(Debug, Clone)]
pub struct ExchangeRuntimeRecord {
    pub exchange_type: String,
    pub enabled: bool,
    pub api_key: String,
    pub secret_key: String,
    pub passphrase: String,
    pub testnet: bool,
    pub hyperliquid_wallet_addr: String,
}

pub struct TraderUsageRecord {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct CreateExchangeAccount {
    pub id: String,
    pub exchange_type: String,
    pub account_name: String,
    pub name: String,
    pub exchange_kind: String,
    pub enabled: bool,
    pub api_key: String,
    pub secret_key: String,
    pub passphrase: String,
    pub testnet: bool,
    pub hyperliquid_wallet_addr: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct UpdateExchangeAccount {
    pub enabled: bool,
    pub api_key: String,
    pub secret_key: String,
    pub passphrase: String,
    pub testnet: bool,
    pub hyperliquid_wallet_addr: String,
    pub updated_at: i64,
}

impl ExchangeRepo {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn list_for_user(
        &self,
    ) -> Result<Vec<ExchangeConfigRecord>, crate::database::DbErr> {
        exchanges::Entity::find()
            .order_by_asc(exchanges::Column::ExchangeType)
            .order_by_asc(exchanges::Column::AccountName)
            .all(&self.db)
            .await
            .map(|rows| {
                rows.into_iter()
                    .map(|row| ExchangeConfigRecord {
                        id: row.id,
                        exchange_type: row.exchange_type,
                        account_name: row.account_name,
                        name: row.name,
                        exchange_kind: row.r#type,
                        enabled: i64::from(row.enabled),
                        testnet: i64::from(row.testnet),
                        hyperliquid_wallet_addr: row.hyperliquid_wallet_addr,
                    })
                    .collect()
            })
    }

    pub async fn create(
        &self,
        account: CreateExchangeAccount,
    ) -> Result<(), crate::database::DbErr> {
        exchanges::ActiveModel {
            id: Set(account.id),
            exchange_type: Set(account.exchange_type),
            account_name: Set(account.account_name),
            name: Set(account.name),
            r#type: Set(account.exchange_kind),
            enabled: Set(if account.enabled { 1 } else { 0 }),
            api_key: Set(account.api_key),
            secret_key: Set(account.secret_key),
            passphrase: Set(account.passphrase),
            testnet: Set(if account.testnet { 1 } else { 0 }),
            hyperliquid_wallet_addr: Set(account.hyperliquid_wallet_addr),
            created_at: Set(ts_to_dt(account.created_at)),
            updated_at: Set(ts_to_dt(account.updated_at)),
        }
        .insert(&self.db)
        .await?;

        Ok(())
    }

    pub async fn find_secrets(
        &self,
        exchange_id: &str,
    ) -> Result<Option<ExchangeSecretRecord>, crate::database::DbErr> {
        exchanges::Entity::find_by_id(exchange_id.trim().to_string())
            .one(&self.db)
            .await
            .map(|row| {
                row.map(|row| ExchangeSecretRecord {
                    api_key: row.api_key,
                    secret_key: row.secret_key,
                    passphrase: row.passphrase,
                })
            })
    }

    pub async fn find_runtime_config(
        &self,
        exchange_id: &str,
    ) -> Result<Option<ExchangeRuntimeRecord>, crate::database::DbErr> {
        exchanges::Entity::find_by_id(exchange_id.trim().to_string())
            .one(&self.db)
            .await
            .map(|row| {
                row.map(|row| ExchangeRuntimeRecord {
                    exchange_type: row.exchange_type,
                    enabled: row.enabled != 0,
                    api_key: row.api_key,
                    secret_key: row.secret_key,
                    passphrase: row.passphrase,
                    testnet: row.testnet != 0,
                    hyperliquid_wallet_addr: row.hyperliquid_wallet_addr,
                })
            })
    }

    pub async fn update(
        &self,
        exchange_id: &str,
        update: UpdateExchangeAccount,
    ) -> Result<(), crate::database::DbErr> {
        exchanges::Entity::update_many()
            .col_expr(
                exchanges::Column::Enabled,
                Expr::value(if update.enabled { 1 } else { 0 }),
            )
            .col_expr(exchanges::Column::ApiKey, Expr::value(update.api_key))
            .col_expr(exchanges::Column::SecretKey, Expr::value(update.secret_key))
            .col_expr(
                exchanges::Column::Passphrase,
                Expr::value(update.passphrase),
            )
            .col_expr(
                exchanges::Column::Testnet,
                Expr::value(if update.testnet { 1 } else { 0 }),
            )
            .col_expr(
                exchanges::Column::HyperliquidWalletAddr,
                Expr::value(update.hyperliquid_wallet_addr),
            )
            .col_expr(
                exchanges::Column::UpdatedAt,
                Expr::value(ts_to_dt(update.updated_at)),
            )
            .filter(exchanges::Column::Id.eq(exchange_id.trim()))
            .exec(&self.db)
            .await?;

        Ok(())
    }

    pub async fn find_trader_usage(
        &self,
        exchange_id: &str,
    ) -> Result<Option<TraderUsageRecord>, crate::database::DbErr> {
        traders::Entity::find()
            .filter(traders::Column::ExchangeId.eq(exchange_id.trim()))
            .one(&self.db)
            .await
            .map(|row| {
                row.map(|row| TraderUsageRecord {
                    id: row.id,
                    name: row.name,
                })
            })
    }

    pub async fn delete(
        &self,
        exchange_id: &str,
    ) -> Result<u64, crate::database::DbErr> {
        let result = exchanges::Entity::delete_many()
            .filter(exchanges::Column::Id.eq(exchange_id.trim()))
            .exec(&self.db)
            .await?;

        Ok(result.rows_affected)
    }
}
