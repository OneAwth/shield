use super::m20250101_000001_create_realm_table::Realm;
use sea_orm::sqlx::types::chrono;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Client::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Client::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Client::Name).string().not_null())
                    .col(ColumnDef::new(Client::TwoFactorEnabledAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Client::MaxConcurrentSessions).integer().not_null().default(1))
                    .col(ColumnDef::new(Client::SessionLifetime).integer().not_null().default(300))
                    .col(ColumnDef::new(Client::UseRefreshToken).boolean().not_null().default(true))
                    .col(ColumnDef::new(Client::RefreshTokenLifetime).integer().not_null().default(3600))
                    .col(ColumnDef::new(Client::RefreshTokenReuseLimit).integer().not_null().default(0))
                    .col(ColumnDef::new(Client::IsAccountActivationRequired).boolean().not_null().default(false))
                    .col(ColumnDef::new(Client::LockedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Client::RealmId).uuid().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_client_realm_id")
                            .from(Client::Table, Client::RealmId)
                            .to(Realm::Table, Realm::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(
                        ColumnDef::new(Client::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(chrono::Utc::now()),
                    )
                    .col(
                        ColumnDef::new(Client::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(chrono::Utc::now()),
                    )
                    .index(Index::create().unique().name("realm_id_name_key").col(Client::Name).col(Client::RealmId))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Client::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
pub enum Client {
    Table,
    Id,
    Name,
    TwoFactorEnabledAt,
    MaxConcurrentSessions,
    SessionLifetime,
    UseRefreshToken,
    RefreshTokenLifetime,
    RefreshTokenReuseLimit,
    IsAccountActivationRequired,
    LockedAt,
    RealmId,
    CreatedAt,
    UpdatedAt,
}
