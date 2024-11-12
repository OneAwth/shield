use std::collections::HashMap;

use sea_orm::prelude::Uuid;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ResourceGroup {
    pub name: String,
    pub client_id: Uuid,
}

#[derive(Deserialize)]
pub struct ResourceSubset {
    pub group: ResourceGroup,
    pub identifiers: HashMap<String, String>,
}

#[derive(Deserialize)]
pub struct CreateUserRequest {
    pub email: String,
    pub password: String,
    pub first_name: String,
    pub last_name: Option<String>,
    pub phone: Option<String>,
    pub image: Option<String>,
    pub resource: ResourceSubset,
}

#[derive(Deserialize)]
pub struct AddResourceRequest {
    pub group_name: Option<String>,
    pub group_id: Option<Uuid>,
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
    pub name: String,
    pub description: Option<String>,
    pub is_default: Option<bool>,
    pub lock: Option<bool>,
}
