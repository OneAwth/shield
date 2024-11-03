use sea_orm::sqlx::types::chrono;
use sea_orm_migration::prelude::*;

use crate::{m20220101_000002_create_client_table::Client, m20220101_000005_create_user_table::User};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Resource::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Resource::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Resource::UserId).uuid().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_resource_user_id")
                            .from(Resource::Table, Resource::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(ColumnDef::new(Resource::ClientId).uuid().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_resource_client_id")
                            .from(Resource::Table, Resource::ClientId)
                            .to(Client::Table, Client::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(ColumnDef::new(Resource::Name).string().not_null())
                    .col(ColumnDef::new(Resource::Value).string().not_null())
                    .col(ColumnDef::new(Resource::Description).string())
                    .col(ColumnDef::new(Resource::LockedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(Resource::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(chrono::Utc::now()),
                    )
                    .col(
                        ColumnDef::new(Resource::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(chrono::Utc::now()),
                    )
                    .index(
                        Index::create()
                            .unique()
                            .name("resource_group_id_and_resource_name_idx")
                            .col(Resource::Name)
                            .col(Resource::UserId)
                            .col(Resource::ClientId),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Resource::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
pub enum Resource {
    Table,
    Id,
    UserId,
    ClientId,
    Name,
    Value,
    Description,
    LockedAt,
    CreatedAt,
    UpdatedAt,
}
