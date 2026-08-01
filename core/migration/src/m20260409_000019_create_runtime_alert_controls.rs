use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(RuntimeAlertControls::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(RuntimeAlertControls::TraderId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RuntimeAlertControls::IsMuted)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(RuntimeAlertControls::MutedUntil)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(RuntimeAlertControls::MuteReason)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(RuntimeAlertControls::AckedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(RuntimeAlertControls::AckedBy)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(RuntimeAlertControls::AckNote)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(RuntimeAlertControls::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RuntimeAlertControls::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(RuntimeAlertControls::TraderId)
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_runtime_alert_controls_muted_until")
                    .table(RuntimeAlertControls::Table)
                    .col(RuntimeAlertControls::IsMuted)
                    .col(RuntimeAlertControls::MutedUntil)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .if_exists()
                    .table(RuntimeAlertControls::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum RuntimeAlertControls {
    Table,
    TraderId,
    IsMuted,
    MutedUntil,
    MuteReason,
    AckedAt,
    AckedBy,
    AckNote,
    UpdatedAt,
    CreatedAt,
}
