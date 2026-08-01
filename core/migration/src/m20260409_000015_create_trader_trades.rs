use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(TraderTrades::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TraderTrades::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TraderTrades::TraderId).string().not_null())
                    .col(ColumnDef::new(TraderTrades::Symbol).string().not_null())
                    .col(ColumnDef::new(TraderTrades::Side).string().not_null())
                    .col(
                        ColumnDef::new(TraderTrades::EntryPrice)
                            .double()
                            .not_null()
                            .default(0.0),
                    )
                    .col(
                        ColumnDef::new(TraderTrades::ExitPrice)
                            .double()
                            .not_null()
                            .default(0.0),
                    )
                    .col(
                        ColumnDef::new(TraderTrades::Quantity)
                            .double()
                            .not_null()
                            .default(0.0),
                    )
                    .col(
                        ColumnDef::new(TraderTrades::RealizedPnl)
                            .double()
                            .not_null()
                            .default(0.0),
                    )
                    .col(
                        ColumnDef::new(TraderTrades::Fees)
                            .double()
                            .not_null()
                            .default(0.0),
                    )
                    .col(
                        ColumnDef::new(TraderTrades::RoiPct)
                            .double()
                            .not_null()
                            .default(0.0),
                    )
                    .col(
                        ColumnDef::new(TraderTrades::OpenedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TraderTrades::ClosedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TraderTrades::CreatedAt)
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
                    .name("idx_trader_trades_trader_closed")
                    .table(TraderTrades::Table)
                    .col(TraderTrades::TraderId)
                    .col((TraderTrades::ClosedAt, IndexOrder::Desc))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .if_exists()
                    .table(TraderTrades::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum TraderTrades {
    Table,
    Id,
    TraderId,
    Symbol,
    Side,
    EntryPrice,
    ExitPrice,
    Quantity,
    RealizedPnl,
    Fees,
    RoiPct,
    OpenedAt,
    ClosedAt,
    CreatedAt,
}
