pub use sea_orm_migration::prelude::*;

mod m20250101_000001_create_realm_table;
mod m20250101_000002_create_client_table;
mod m20250101_000003_create_user_table;
mod m20250101_000004_create_resource_group_table;
mod m20250101_000005_create_resource_table;
mod m20250101_000006_create_refresh_token_table;
mod m20250101_000007_create_api_user_table;
mod m20250101_000008_create_session_table;
mod m20250101_000009_create_verification_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250101_000001_create_realm_table::Migration),
            Box::new(m20250101_000002_create_client_table::Migration),
            Box::new(m20250101_000003_create_user_table::Migration),
            Box::new(m20250101_000004_create_resource_group_table::Migration),
            Box::new(m20250101_000005_create_resource_table::Migration),
            Box::new(m20250101_000006_create_refresh_token_table::Migration),
            Box::new(m20250101_000007_create_api_user_table::Migration),
            Box::new(m20250101_000008_create_session_table::Migration),
            Box::new(m20250101_000009_create_verification_table::Migration),
        ]
    }
}
