use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(LlmModels::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(LlmModels::Id).string().not_null())
                    .col(ColumnDef::new(LlmModels::ProviderId).string().not_null())
                    .col(ColumnDef::new(LlmModels::Name).string().not_null())
                    .col(ColumnDef::new(LlmModels::ModelId).string().not_null())
                    .col(
                        ColumnDef::new(LlmModels::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LlmModels::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .primary_key(Index::create().col(LlmModels::Id))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("uq_llm_models_provider_model")
                    .table(LlmModels::Table)
                    .col(LlmModels::ProviderId)
                    .col(LlmModels::ModelId)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().if_exists().table(LlmModels::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum LlmModels {
    Table,
    Id,
    ProviderId,
    Name,
    ModelId,
    CreatedAt,
    UpdatedAt,
}
