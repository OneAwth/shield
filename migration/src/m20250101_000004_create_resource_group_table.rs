use super::m20250101_000001_create_realm_table::Realm;
use super::m20250101_000002_create_client_table::Client;
use super::m20250101_000003_create_user_table::User;
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
                    .table(ResourceGroup::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(ResourceGroup::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(ResourceGroup::RealmId).uuid().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_resource_group_realm_id")
                            .from(ResourceGroup::Table, ResourceGroup::RealmId)
                            .to(Realm::Table, Realm::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(ColumnDef::new(ResourceGroup::ClientId).uuid().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_resource_group_client_id")
                            .from(ResourceGroup::Table, ResourceGroup::ClientId)
                            .to(Client::Table, Client::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(ColumnDef::new(ResourceGroup::UserId).uuid().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_resource_group_user_id")
                            .from(ResourceGroup::Table, ResourceGroup::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(ColumnDef::new(ResourceGroup::Name).string().not_null())
                    .col(ColumnDef::new(ResourceGroup::Description).string())
                    .col(ColumnDef::new(ResourceGroup::IsDefault).boolean().not_null().default(false))
                    .col(ColumnDef::new(ResourceGroup::LockedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(ResourceGroup::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(chrono::Utc::now()),
                    )
                    .col(
                        ColumnDef::new(ResourceGroup::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(chrono::Utc::now()),
                    )
                    .index(
                        Index::create()
                            .unique()
                            .name("client_user_resource_group_name_idx")
                            .col(ResourceGroup::Name)
                            .col(ResourceGroup::ClientId)
                            .col(ResourceGroup::UserId),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(ResourceGroup::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
pub enum ResourceGroup {
    Table,
    Id,
    RealmId,
    ClientId,
    UserId,
    Name,
    Description,
    IsDefault,
    LockedAt,
    CreatedAt,
    UpdatedAt,
}
