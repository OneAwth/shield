pub use sea_orm_migration::prelude::*;

mod m20220101_000001_create_realm_table;
mod m20220101_000002_create_client_table;
mod m20220101_000003_create_user_table;
mod m20220101_000004_create_resource_group_table;
mod m20220101_000005_create_resource_table;
mod m20220101_000006_create_session_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_realm_table::Migration),
            Box::new(m20220101_000002_create_client_table::Migration),
            Box::new(m20220101_000003_create_user_table::Migration),
            Box::new(m20220101_000004_create_resource_group_table::Migration),
            Box::new(m20220101_000005_create_resource_table::Migration),
            Box::new(m20220101_000006_create_session_table::Migration),
        ]
    }
}