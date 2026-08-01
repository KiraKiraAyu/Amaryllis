use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(TraderDecisions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TraderDecisions::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(TraderDecisions::TraderId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TraderDecisions::Symbol)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(TraderDecisions::Timeframe)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(TraderDecisions::Decision)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TraderDecisions::Confidence)
                            .double()
                            .not_null()
                            .default(0.0),
                    )
                    .col(
                        ColumnDef::new(TraderDecisions::Reason)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(TraderDecisions::PayloadJson)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(TraderDecisions::CreatedAt)
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
                    .name("idx_trader_decisions_trader_created")
                    .table(TraderDecisions::Table)
                    .col(TraderDecisions::TraderId)
                    .col((TraderDecisions::CreatedAt, IndexOrder::Desc))
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .if_exists()
                    .table(TraderDecisions::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum TraderDecisions {
    Table,
    Id,
    TraderId,
    Symbol,
    Timeframe,
    Decision,
    Confidence,
    Reason,
    PayloadJson,
    CreatedAt,
}
