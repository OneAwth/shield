use super::m20250101_000004_create_resource_group_table::ResourceGroup;
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
                    .table(Resource::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Resource::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Resource::GroupId).uuid().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_resource_group_id")
                            .from(Resource::Table, Resource::GroupId)
                            .to(ResourceGroup::Table, ResourceGroup::Id)
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
                            .col(Resource::GroupId),
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
    GroupId,
    Name,
    Value,
    Description,
    LockedAt,
    CreatedAt,
    UpdatedAt,
}
