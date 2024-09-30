use sea_orm::{prelude::Uuid, sqlx::types::chrono};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Realm::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Realm::Id).uuid().not_null().primary_key().default(Uuid::now_v7()))
                    .col(ColumnDef::new(Realm::Name).string().unique_key().not_null())
                    .col(ColumnDef::new(Realm::Slug).string().unique_key().not_null())
                    .col(ColumnDef::new(Realm::MaxConcurrentSessions).integer())
                    .col(ColumnDef::new(Realm::SessionLifetime).integer().not_null().default(300))
                    .col(ColumnDef::new(Realm::RefreshTokenLifetime).integer().not_null().default(3600))
                    .col(ColumnDef::new(Realm::RefreshTokenReuseLimit).integer().not_null().default(0))
                    .col(ColumnDef::new(Realm::LockedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(Realm::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(chrono::Utc::now()),
                    )
                    .col(
                        ColumnDef::new(Realm::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(chrono::Utc::now()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Realm::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
pub enum Realm {
    Table,
    Id,
    Name,
    Slug,
    MaxConcurrentSessions,
    SessionLifetime,
    RefreshTokenLifetime,
    RefreshTokenReuseLimit,
    LockedAt,
    CreatedAt,
    UpdatedAt,
}