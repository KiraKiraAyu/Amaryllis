use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(TraderAccounts::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TraderAccounts::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TraderAccounts::TraderId).string().not_null())
                    .col(
                        ColumnDef::new(TraderAccounts::ExchangeId)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(TraderAccounts::TotalBalance)
                            .double()
                            .not_null()
                            .default(0.0),
                    )
                    .col(
                        ColumnDef::new(TraderAccounts::AvailableBalance)
                            .double()
                            .not_null()
                            .default(0.0),
                    )
                    .col(
                        ColumnDef::new(TraderAccounts::UsedMargin)
                            .double()
                            .not_null()
                            .default(0.0),
                    )
                    .col(
                        ColumnDef::new(TraderAccounts::UnrealizedPnl)
                            .double()
                            .not_null()
                            .default(0.0),
                    )
                    .col(
                        ColumnDef::new(TraderAccounts::RealizedPnl)
                            .double()
                            .not_null()
                            .default(0.0),
                    )
                    .col(
                        ColumnDef::new(TraderAccounts::Currency)
                            .string()
                            .not_null()
                            .default("USDT"),
                    )
                    .col(
                        ColumnDef::new(TraderAccounts::SnapshotAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TraderAccounts::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TraderAccounts::UpdatedAt)
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
                    .name("idx_trader_accounts_trader_snapshot")
                    .table(TraderAccounts::Table)
                    .col(TraderAccounts::TraderId)
                    .col((TraderAccounts::SnapshotAt, IndexOrder::Desc))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .if_exists()
                    .table(TraderAccounts::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum TraderAccounts {
    Table,
    Id,
    TraderId,
    ExchangeId,
    TotalBalance,
    AvailableBalance,
    UsedMargin,
    UnrealizedPnl,
    RealizedPnl,
    Currency,
    SnapshotAt,
    CreatedAt,
    UpdatedAt,
}
