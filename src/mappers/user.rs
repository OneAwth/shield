use std::collections::HashMap;

use sea_orm::prelude::Uuid;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct AddResourceRequest {
    pub client_id: Uuid,
    pub identifiers: HashMap<String, String>,
}

#[derive(Deserialize)]
pub struct UpdateResourceRequest {
    pub name: String,
    pub value: String,
    pub description: Option<String>,
    pub lock: Option<bool>,
}

#[derive(Deserialize)]
pub struct UpdateResourceGroupRequest {
    pub identifiers: HashMap<String, String>,
    pub is_default: Option<bool>,
    pub lock: Option<bool>,
}
