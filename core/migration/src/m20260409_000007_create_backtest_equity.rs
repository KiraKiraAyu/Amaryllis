use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(BacktestEquity::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(BacktestEquity::RunId).string().not_null())
                    .col(
                        ColumnDef::new(BacktestEquity::Ts)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(BacktestEquity::Equity).double().not_null())
                    .col(
                        ColumnDef::new(BacktestEquity::Available)
                            .double()
                            .not_null(),
                    )
                    .col(ColumnDef::new(BacktestEquity::Pnl).double().not_null())
                    .col(ColumnDef::new(BacktestEquity::PnlPct).double().not_null())
                    .col(ColumnDef::new(BacktestEquity::DdPct).double().not_null())
                    .col(ColumnDef::new(BacktestEquity::Cycle).integer().not_null())
                    .primary_key(
                        Index::create()
                            .col(BacktestEquity::RunId)
                            .col(BacktestEquity::Ts),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_backtest_equity_run_ts")
                    .table(BacktestEquity::Table)
                    .col(BacktestEquity::RunId)
                    .col((BacktestEquity::Ts, IndexOrder::Asc))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .if_exists()
                    .table(BacktestEquity::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum BacktestEquity {
    Table,
    RunId,
    Ts,
    Equity,
    Available,
    Pnl,
    PnlPct,
    DdPct,
    Cycle,
}
