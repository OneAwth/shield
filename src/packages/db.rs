use std::time::Duration;

use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbBackend, DbErr, Statement};

use super::settings::SETTINGS;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
}

pub async fn get_db_connection_pool() -> Result<AppState, DbErr> {
    let uri = &SETTINGS.read().database.uri;
    let db_name = &SETTINGS.read().database.name;
    let connection_string = format!("{}/{}", uri, db_name);

    let mut opts = ConnectOptions::new(&connection_string);
    opts.max_connections(20).connect_timeout(Duration::from_secs(5));

    let db = Database::connect(uri).await?;
    let db = match db.get_database_backend() {
        DbBackend::MySql => {
            db.execute(Statement::from_string(
                db.get_database_backend(),
                format!("CREATE DATABASE IF NOT EXISTS `{}`;", db_name),
            ))
            .await?;

            Database::connect(opts).await?
        }
        DbBackend::Postgres => {
            let exists = db
                .query_one(Statement::from_string(
                    db.get_database_backend(),
                    format!("SELECT 1 FROM pg_database WHERE datname = '{}'", db_name),
                ))
                .await?;

            match exists {
                None => {
                    println!("🪹 Database does not exist, creating it");
                    db.execute(Statement::from_string(
                        db.get_database_backend(),
                        format!("CREATE DATABASE \"{}\";", db_name),
                    ))
                    .await?;
                }
                _ => {
                    println!("🛢️ Database already exists");
                }
            }

            Database::connect(opts).await?
        }
        DbBackend::Sqlite => db,
    };

    Migrator::up(&db, None).await?;

    Ok(AppState { db })
}
