use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(OrderFills::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(OrderFills::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(OrderFills::OrderId).string().not_null())
                    .col(ColumnDef::new(OrderFills::TraderId).string().not_null())
                    .col(
                        ColumnDef::new(OrderFills::ExchangeTradeId)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .col(ColumnDef::new(OrderFills::Symbol).string().not_null())
                    .col(ColumnDef::new(OrderFills::Side).string().not_null())
                    .col(
                        ColumnDef::new(OrderFills::Price)
                            .double()
                            .not_null()
                            .default(0.0),
                    )
                    .col(
                        ColumnDef::new(OrderFills::Quantity)
                            .double()
                            .not_null()
                            .default(0.0),
                    )
                    .col(
                        ColumnDef::new(OrderFills::Fee)
                            .double()
                            .not_null()
                            .default(0.0),
                    )
                    .col(
                        ColumnDef::new(OrderFills::FeeAsset)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(OrderFills::RealizedPnl)
                            .double()
                            .not_null()
                            .default(0.0),
                    )
                    .col(
                        ColumnDef::new(OrderFills::ExecutedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OrderFills::CreatedAt)
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
                    .name("idx_order_fills_order")
                    .table(OrderFills::Table)
                    .col(OrderFills::OrderId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_order_fills_trader_executed")
                    .table(OrderFills::Table)
                    .col(OrderFills::TraderId)
                    .col((OrderFills::ExecutedAt, IndexOrder::Desc))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .if_exists()
                    .table(OrderFills::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum OrderFills {
    Table,
    Id,
    OrderId,
    TraderId,
    ExchangeTradeId,
    Symbol,
    Side,
    Price,
    Quantity,
    Fee,
    FeeAsset,
    RealizedPnl,
    ExecutedAt,
    CreatedAt,
}
