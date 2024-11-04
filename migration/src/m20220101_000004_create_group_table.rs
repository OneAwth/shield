use super::m20220101_000001_create_realm_table::Realm;
use crate::m20220101_000005_create_user_table::User;
use sea_orm::{sqlx::types::chrono::Utc, ActiveEnum, DbBackend, DeriveActiveEnum, EnumIter, Schema};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let schema = Schema::new(DbBackend::Postgres);
        manager.create_type(schema.create_enum_from_active_enum::<GroupRole>()).await?;
        manager.create_type(schema.create_enum_from_active_enum::<GroupAccess>()).await?;
        manager
            .create_table(
                Table::create()
                    .table(Group::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Group::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Group::Name).string().not_null())
                    .col(ColumnDef::new(Group::Description).string())
                    .col(ColumnDef::new(Group::Role).custom(GroupRole::name()).not_null())
                    .col(ColumnDef::new(Group::Access).custom(GroupAccess::name()).not_null())
                    .col(ColumnDef::new(Group::LockedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Group::ClientId).uuid().not_null())
                    .col(ColumnDef::new(Group::RealmId).uuid().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_realm_id")
                            .from(Group::Table, User::RealmId)
                            .to(Realm::Table, Realm::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(ColumnDef::new(Group::CreatedAt).timestamp_with_time_zone().not_null().default(Utc::now()))
                    .col(ColumnDef::new(Group::UpdatedAt).timestamp_with_time_zone().not_null().default(Utc::now()))
                    .index(
                        Index::create()
                            .unique()
                            .name("group_name_client_id_realm_id_idx")
                            .col(Group::Name)
                            .col(Group::ClientId)
                            .col(Group::RealmId),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Group::Table).to_owned()).await
    }
}

#[derive(EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "group_role")]
pub enum GroupRole {
    #[sea_orm(string_value = "user")]
    User,
}

#[derive(EnumIter, DeriveActiveEnum, PartialEq, Eq)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "group_access")]
pub enum GroupAccess {
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
pub enum Group {
    Table,
    Id,
    Name,
    Description,
    Role,
    Access,
    LockedAt,
    RealmId,
    ClientId,
    CreatedAt,
    UpdatedAt,
}
