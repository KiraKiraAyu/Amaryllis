use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(BacktestRuns::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(BacktestRuns::RunId)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(BacktestRuns::Label)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(BacktestRuns::LastError)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(BacktestRuns::Version)
                            .integer()
                            .not_null()
                            .default(1),
                    )
                    .col(
                        ColumnDef::new(BacktestRuns::State)
                            .string()
                            .not_null()
                            .default("running"),
                    )
                    .col(
                        ColumnDef::new(BacktestRuns::ConfigJson)
                            .string()
                            .not_null()
                            .default("{}"),
                    )
                    .col(
                        ColumnDef::new(BacktestRuns::SummaryJson)
                            .string()
                            .not_null()
                            .default("{}"),
                    )
                    .col(
                        ColumnDef::new(BacktestRuns::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(BacktestRuns::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .if_exists()
                    .table(BacktestRuns::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum BacktestRuns {
    Table,
    RunId,
    Label,
    LastError,
    Version,
    State,
    ConfigJson,
    SummaryJson,
    CreatedAt,
    UpdatedAt,
}
