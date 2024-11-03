use super::m20220101_000002_create_client_table::Client;
use crate::{m20220101_000001_create_realm_table::Realm, m20220101_000005_create_user_table::User};
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
                    .table(RefreshToken::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(RefreshToken::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(RefreshToken::UserId).uuid().not_null())
                    .col(ColumnDef::new(RefreshToken::ClientId).uuid())
                    .col(ColumnDef::new(RefreshToken::RealmId).uuid().not_null())
                    .col(ColumnDef::new(RefreshToken::ReUsedCount).integer().not_null().default(0))
                    .col(ColumnDef::new(RefreshToken::LockedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(RefreshToken::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(chrono::Utc::now()),
                    )
                    .col(
                        ColumnDef::new(RefreshToken::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(chrono::Utc::now()),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_refresh_token_user_id")
                    .from(RefreshToken::Table, RefreshToken::UserId)
                    .to(User::Table, User::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_refresh_token_client_id")
                    .from(RefreshToken::Table, RefreshToken::ClientId)
                    .to(Client::Table, Client::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_refresh_token_realm_id")
                    .from(RefreshToken::Table, RefreshToken::RealmId)
                    .to(Realm::Table, Realm::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        // Add indexes
        manager
            .create_index(
                Index::create()
                    .name("refresh_token_user_id_idx")
                    .table(RefreshToken::Table)
                    .col(RefreshToken::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("refresh_token_client_id_idx")
                    .table(RefreshToken::Table)
                    .col(RefreshToken::ClientId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("refresh_token_realm_id_idx")
                    .table(RefreshToken::Table)
                    .col(RefreshToken::RealmId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(RefreshToken::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
pub enum RefreshToken {
    Table,
    Id,
    UserId,
    ClientId,
    RealmId,
    ReUsedCount,
    LockedAt,
    CreatedAt,
    UpdatedAt,
}
