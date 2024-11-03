use sea_orm::sqlx::types::chrono::Utc;
use sea_orm_migration::prelude::*;

use crate::{m20220101_000004_create_group_table::Group, m20220101_000005_create_user_table::User};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(UserGroup::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(UserGroup::UserId).uuid().not_null())
                    .col(ColumnDef::new(UserGroup::GroupId).uuid().not_null())
                    .col(ColumnDef::new(UserGroup::LockedAt).timestamp_with_time_zone())
                    .primary_key(Index::create().name("pk_user_group").col(UserGroup::UserId).col(UserGroup::GroupId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_group_user_id")
                            .from(UserGroup::Table, UserGroup::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_group_group_id")
                            .from(UserGroup::Table, UserGroup::GroupId)
                            .to(Group::Table, Group::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(
                        ColumnDef::new(UserGroup::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Utc::now()),
                    )
                    .col(
                        ColumnDef::new(UserGroup::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Utc::now()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(UserGroup::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
pub enum UserGroup {
    Table,
    UserId,
    GroupId,
    LockedAt,
    CreatedAt,
    UpdatedAt,
}
