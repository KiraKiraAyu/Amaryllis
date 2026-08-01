use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Strategies::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Strategies::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Strategies::Name).string().not_null())
                    .col(
                        ColumnDef::new(Strategies::Description)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(Strategies::IsActive)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Strategies::Config)
                            .string()
                            .not_null()
                            .default("{}"),
                    )
                    .col(
                        ColumnDef::new(Strategies::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Strategies::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
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
                    .table(Strategies::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Strategies {
    Table,
    Id,
    Name,
    Description,
    IsActive,
    Config,
    CreatedAt,
    UpdatedAt,
}
