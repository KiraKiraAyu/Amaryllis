use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(LlmProviders::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(LlmProviders::Id).string().not_null())
                    .col(ColumnDef::new(LlmProviders::Name).string().not_null())
                    .col(
                        ColumnDef::new(LlmProviders::ProviderType)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LlmProviders::Enabled)
                            .integer()
                            .not_null()
                            .default(1),
                    )
                    .col(
                        ColumnDef::new(LlmProviders::ApiKey)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(LlmProviders::BaseUrl)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(LlmProviders::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LlmProviders::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .primary_key(Index::create().col(LlmProviders::Id))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .if_exists()
                    .table(LlmProviders::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum LlmProviders {
    Table,
    Id,
    Name,
    ProviderType,
    Enabled,
    ApiKey,
    BaseUrl,
    CreatedAt,
    UpdatedAt,
}
