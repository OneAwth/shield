use crate::m20250101_000003_create_user_table::User;
use sea_orm::{ActiveEnum, DbBackend, DeriveActiveEnum, EnumIter, Schema};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let schema = Schema::new(DbBackend::Postgres);
        manager.create_type(schema.create_enum_from_active_enum::<VerificationType>()).await?;
        manager
            .create_table(
                Table::create()
                    .table(Verification::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Verification::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Verification::UserId).uuid().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_verification_user_id")
                            .from(Verification::Table, Verification::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(ColumnDef::new(Verification::Type).custom(VerificationType::name()).not_null())
                    .col(ColumnDef::new(Verification::Expires).timestamp_with_time_zone().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Verification::Table).to_owned()).await
    }
}

#[derive(EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "verification_type")]
pub enum VerificationType {
    #[sea_orm(string_value = "email")]
    Email,
    #[sea_orm(string_value = "forgot_password")]
    ForgotPassword,
    #[sea_orm(string_value = "phone")]
    Phone,
    #[sea_orm(string_value = "mfa")]
    Mfa,
    #[sea_orm(string_value = "passwordless")]
    Passwordless,
    #[sea_orm(string_value = "otp")]
    Otp,
}

#[derive(DeriveIden)]
pub enum Verification {
    Table,
    Id,
    UserId,
    Type,
    Expires,
}
