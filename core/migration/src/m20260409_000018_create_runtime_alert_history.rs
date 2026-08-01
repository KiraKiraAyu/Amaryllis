use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(RuntimeAlertHistory::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(RuntimeAlertHistory::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(RuntimeAlertHistory::TraderId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RuntimeAlertHistory::WindowHours)
                            .integer()
                            .not_null()
                            .default(24),
                    )
                    .col(
                        ColumnDef::new(RuntimeAlertHistory::ThresholdsJson)
                            .string()
                            .not_null()
                            .default("{}"),
                    )
                    .col(
                        ColumnDef::new(RuntimeAlertHistory::RatesJson)
                            .string()
                            .not_null()
                            .default("{}"),
                    )
                    .col(
                        ColumnDef::new(RuntimeAlertHistory::AlertsJson)
                            .string()
                            .not_null()
                            .default("[]"),
                    )
                    .col(
                        ColumnDef::new(RuntimeAlertHistory::Breached)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(RuntimeAlertHistory::Severity)
                            .string()
                            .not_null()
                            .default("ok"),
                    )
                    .col(
                        ColumnDef::new(RuntimeAlertHistory::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_runtime_alert_history_trader_created")
                    .table(RuntimeAlertHistory::Table)
                    .col(RuntimeAlertHistory::TraderId)
                    .col((RuntimeAlertHistory::CreatedAt, IndexOrder::Desc))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_runtime_alert_history_breached_created")
                    .table(RuntimeAlertHistory::Table)
                    .col(RuntimeAlertHistory::Breached)
                    .col((RuntimeAlertHistory::CreatedAt, IndexOrder::Desc))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .if_exists()
                    .table(RuntimeAlertHistory::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum RuntimeAlertHistory {
    Table,
    Id,
    TraderId,
    WindowHours,
    ThresholdsJson,
    RatesJson,
    AlertsJson,
    Breached,
    Severity,
    CreatedAt,
}
