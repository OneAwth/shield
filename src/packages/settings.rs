use config::{Config, ConfigError, Environment, File, Value};
use dotenvy::dotenv;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde::Deserialize;
use std::{env, fmt, path::Path, sync::Arc};

use crate::utils::helpers::default_cred::DefaultCred;

// pub static SETTINGS: Lazy<Settings> = Lazy::new(|| Settings::new().expect("Failed to setup settings"));
pub static SETTINGS: Lazy<Arc<RwLock<Settings>>> = Lazy::new(|| Arc::new(RwLock::new(Settings::new().expect("Failed to setup settings"))));

#[derive(Debug, Clone, Deserialize)]
pub struct Server {
    pub port: u16,
    // pub domain: String,
    pub host: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Logger {
    pub level: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Database {
    pub uri: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Admin {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Secrets {
    pub signing_key: String,
    pub api_key_signing_secret: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    // pub environment: String,
    pub server: Server,
    pub logger: Logger,
    pub database: Database,
    pub admin: Admin,
    pub secrets: Secrets,
    pub default_cred: DefaultCred,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        dotenv().ok();
        let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "development".into());

        let mut builder = Config::builder()
            .add_source(File::with_name("config/env/default"))
            .add_source(File::with_name(&format!("config/env/{run_mode}")).required(false))
            .add_source(File::with_name("config/env/local").required(false))
            .add_source(Environment::default().separator("__"));

        // Some cloud services like Heroku exposes a randomly assigned port in
        // the PORT env var and there is no way to change the env var name.
        if let Ok(signing_key) = env::var("SIGNING_KEY") {
            builder = builder.set_override("secrets.signing_key", signing_key)?;
        }
        if let Ok(api_key_signing_secret) = env::var("API_KEY_SIGNING_SECRET") {
            builder = builder.set_override("secrets.api_key_signing_secret", api_key_signing_secret)?;
        }
        if let Ok(port) = env::var("PORT") {
            builder = builder.set_override("server.port", port)?;
        }
        if let Ok(domain) = env::var("DOMAIN") {
            builder = builder.set_override("server.domain", domain)?;
        }
        if let Ok(host) = env::var("HOST") {
            builder = builder.set_override("server.host", host)?;
        }
        if let Ok(database_uri) = env::var("DATABASE_URL") {
            builder = builder.set_override("database.uri", database_uri)?;
        }
        if let Ok(database_name) = env::var("DATABASE_NAME") {
            builder = builder.set_override("database.name", database_name)?;
        }
        if let Ok(admin_email) = env::var("ADMIN_USERNAME") {
            builder = builder.set_override("admin.email", admin_email)?;
        }
        if let Ok(admin_password) = env::var("ADMIN_PASSWORD") {
            builder = builder.set_override("admin.password", admin_password)?;
        }

        // "./logs/default_cred.json" exists then read it else skip
        if Path::new("./logs/default_cred.json").exists() {
            let default_cred = DefaultCred::from_file().expect("Failed to read credentials");
            builder = builder.set_override("default_cred.realm_id", default_cred.realm_id.to_string())?;
            builder = builder.set_override("default_cred.client_id", default_cred.client_id.to_string())?;
            builder = builder.set_override("default_cred.master_admin_user_id", default_cred.master_admin_user_id.to_string())?;
            builder = builder.set_override("default_cred.master_api_key", default_cred.master_api_key.to_string())?;
            builder = builder.set_override("default_cred.resource_group_id", default_cred.resource_group_id.to_string())?;

            let resource_ids_value: Vec<Value> = default_cred.resource_ids.iter().map(|uuid| Value::new(None, uuid.to_string())).collect();
            builder = builder.set_override("default_cred.resource_ids", resource_ids_value)?;
        } else {
            builder = builder.set_override("default_cred.realm_id", "00000000-0000-0000-0000-000000000000")?;
            builder = builder.set_override("default_cred.client_id", "00000000-0000-0000-0000-000000000000")?;
            builder = builder.set_override("default_cred.master_admin_user_id", "00000000-0000-0000-0000-000000000000")?;
            builder = builder.set_override("default_cred.resource_group_id", "00000000-0000-0000-0000-000000000000")?;
            builder = builder.set_override("default_cred.master_api_key", "00000000-0000-0000-0000-000000000000")?;
            builder = builder.set_override("default_cred.resource_ids", vec!["00000000-0000-0000-0000-000000000000"])?;
        }

        builder
            .build()?
            // Deserialize (and thus freeze) the entire configuration.
            .try_deserialize()
    }

    pub fn reload() -> Result<(), ConfigError> {
        let new_settings = Settings::new()?;
        let mut settings = SETTINGS.write();
        *settings = new_settings;
        Ok(())
    }

    // pub fn get<F, T>(f: F) -> T
    // where
    //     F: FnOnce(&Settings) -> T,
    // {
    //     let settings = SETTINGS.read();
    //     f(&settings)
    // }
}

impl fmt::Display for Server {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "http://localhost:{}", &self.port)
    }
}
