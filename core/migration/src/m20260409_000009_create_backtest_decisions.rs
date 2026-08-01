use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(BacktestDecisions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(BacktestDecisions::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(BacktestDecisions::RunId).string().not_null())
                    .col(
                        ColumnDef::new(BacktestDecisions::Ts)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(BacktestDecisions::Symbol)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(BacktestDecisions::Timeframe)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(BacktestDecisions::Decision)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(BacktestDecisions::Confidence)
                            .double()
                            .not_null()
                            .default(0.0),
                    )
                    .col(
                        ColumnDef::new(BacktestDecisions::Reason)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(BacktestDecisions::PayloadJson)
                            .string()
                            .not_null()
                            .default("{}"),
                    )
                    .col(
                        ColumnDef::new(BacktestDecisions::Cycle)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_backtest_decisions_run_cycle")
                    .table(BacktestDecisions::Table)
                    .col(BacktestDecisions::RunId)
                    .col((BacktestDecisions::Cycle, IndexOrder::Asc))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .if_exists()
                    .table(BacktestDecisions::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum BacktestDecisions {
    Table,
    Id,
    RunId,
    Ts,
    Symbol,
    Timeframe,
    Decision,
    Confidence,
    Reason,
    PayloadJson,
    Cycle,
}
