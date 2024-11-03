use crate::m20220101_000008_create_refresh_token_table::RefreshToken;

use super::m20220101_000002_create_client_table::Client;
use super::m20220101_000005_create_user_table::User;
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
                    .table(Session::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Session::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Session::ClientId).uuid().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_session_client_id")
                            .from(Session::Table, Session::ClientId)
                            .to(Client::Table, Client::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(ColumnDef::new(Session::RefreshTokenId).uuid())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_session_refresh_token_id")
                            .from(Session::Table, Session::RefreshTokenId)
                            .to(RefreshToken::Table, RefreshToken::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(ColumnDef::new(Session::UserId).uuid().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_session_user_id")
                            .from(Session::Table, Session::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(ColumnDef::new(Session::IpAddress).string().not_null())
                    .col(ColumnDef::new(Session::UserAgent).string())
                    .col(ColumnDef::new(Session::Browser).string())
                    .col(ColumnDef::new(Session::BrowserVersion).string())
                    .col(ColumnDef::new(Session::OperatingSystem).string())
                    .col(ColumnDef::new(Session::DeviceType).string())
                    .col(ColumnDef::new(Session::CountryCode).string().not_null())
                    .col(ColumnDef::new(Session::Expires).timestamp_with_time_zone().not_null())
                    .col(
                        ColumnDef::new(Session::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(chrono::Utc::now()),
                    )
                    .col(
                        ColumnDef::new(Session::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(chrono::Utc::now()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Session::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
pub enum Session {
    Table,
    Id,
    UserId,
    ClientId,
    RefreshTokenId,
    IpAddress,
    UserAgent,
    Browser,
    BrowserVersion,
    OperatingSystem,
    DeviceType,
    CountryCode,
    Expires,
    CreatedAt,
    UpdatedAt,
}
