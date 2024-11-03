use super::m20220101_000002_create_client_table::Client;
use crate::m20220101_000001_create_realm_table::Realm;
use sea_orm::sqlx::types::chrono;
use sea_orm::{ActiveEnum, DbBackend, DeriveActiveEnum, EnumIter, Schema};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let schema = Schema::new(DbBackend::Postgres);
        manager.create_type(schema.create_enum_from_active_enum::<ApiUserRole>()).await?;
        manager.create_type(schema.create_enum_from_active_enum::<ApiUserAccess>()).await?;
        manager
            .create_table(
                Table::create()
                    .table(ApiUser::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(ApiUser::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(ApiUser::Name).string().not_null())
                    .col(ColumnDef::new(ApiUser::Description).string())
                    .col(ColumnDef::new(ApiUser::RealmId).uuid().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_api_user_realm_id")
                            .from(ApiUser::Table, ApiUser::RealmId)
                            .to(Realm::Table, Realm::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(ColumnDef::new(ApiUser::ClientId).uuid().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_api_user_client_id")
                            .from(ApiUser::Table, ApiUser::ClientId)
                            .to(Client::Table, Client::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(ColumnDef::new(ApiUser::Role).custom(ApiUserRole::name()).not_null())
                    .col(ColumnDef::new(ApiUser::Access).custom(ApiUserAccess::name()).not_null())
                    .col(ColumnDef::new(ApiUser::Expires).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(ApiUser::LockedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(ApiUser::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(chrono::Utc::now()),
                    )
                    .col(
                        ColumnDef::new(ApiUser::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(chrono::Utc::now()),
                    )
                    .index(
                        Index::create()
                            .unique()
                            .name("realm_id_name_client_id_key")
                            .col(ApiUser::Name)
                            .col(ApiUser::RealmId)
                            .col(ApiUser::ClientId),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(ApiUser::Table).to_owned()).await
    }
}

#[derive(EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "api_user_role")]
pub enum ApiUserRole {
    #[sea_orm(string_value = "realm_admin")]
    RealmAdmin,
    #[sea_orm(string_value = "client_admin")]
    ClientAdmin,
}

#[derive(EnumIter, DeriveActiveEnum, PartialEq, Eq)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "api_user_access")]
pub enum ApiUserAccess {
    #[sea_orm(string_value = "read")]
    Read,
    #[sea_orm(string_value = "write")]
    Write,
    #[sea_orm(string_value = "update")]
    Update,
    #[sea_orm(string_value = "delete")]
    Delete,
    #[sea_orm(string_value = "admin")]
    Admin,
}

#[derive(DeriveIden)]
pub enum ApiUser {
    Table,
    Id,
    Name,
    Description,
    RealmId,
    ClientId,
    Role,
    Access,
    Expires,
    LockedAt,
    CreatedAt,
    UpdatedAt,
}
