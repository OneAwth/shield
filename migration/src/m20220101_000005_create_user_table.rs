use super::m20220101_000001_create_realm_table::Realm;
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
                    .table(User::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(User::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(User::FirstName).string().not_null())
                    .col(ColumnDef::new(User::LastName).string())
                    .col(ColumnDef::new(User::Email).string().not_null())
                    .col(ColumnDef::new(User::EmailVerifiedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(User::Phone).string())
                    .col(ColumnDef::new(User::Image).string())
                    .col(ColumnDef::new(User::TwoFactorEnabledAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(User::PasswordHash).string())
                    .col(ColumnDef::new(User::IsTempPassword).boolean().not_null().default(true))
                    .col(ColumnDef::new(User::LockedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(User::RealmId).uuid().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_realm_id")
                            .from(User::Table, User::RealmId)
                            .to(Realm::Table, Realm::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(
                        ColumnDef::new(User::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(chrono::Utc::now()),
                    )
                    .col(
                        ColumnDef::new(User::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(chrono::Utc::now()),
                    )
                    .index(
                        Index::create()
                            .unique()
                            .name("user_email_and_realm_id_idx")
                            .col(User::Email)
                            .col(User::RealmId),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(User::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
pub enum User {
    Table,
    Id,
    FirstName,
    LastName,
    Email,
    EmailVerifiedAt,
    Phone,
    Image,
    TwoFactorEnabledAt,
    PasswordHash,
    IsTempPassword,
    LockedAt,
    RealmId,
    CreatedAt,
    UpdatedAt,
}
