use std::fs::read_to_string;

use sea_orm::prelude::Uuid;
use serde::{Deserialize, Serialize};

use crate::packages::errors::Error;

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DefaultCred {
    pub realm_id: Uuid,
    pub client_id: Uuid,
    pub master_admin_user_id: Uuid,
    pub master_api_key: String,
    pub resource_group_id: Uuid,
    pub resource_ids: Vec<Uuid>,
}

impl DefaultCred {
    pub fn from_file() -> Result<Self, Error> {
        let contents = read_to_string("./logs/default_cred.json").map_err(Error::from)?;
        let default_cred: DefaultCred = serde_json::from_str(&contents)?;
        Ok(default_cred)
    }
}
