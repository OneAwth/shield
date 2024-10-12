use sea_orm::prelude::Uuid;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct ResourceSubset {
    pub group_name: String,
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
pub struct LogoutRequest {
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
}

#[derive(Serialize)]
pub struct LogoutResponse {
    pub ok: bool,
    pub user_id: Uuid,
    pub session_id: Uuid,
}

#[derive(Deserialize)]
pub struct IntrospectRequest {
    pub access_token: String,
}

#[derive(Serialize)]
pub struct IntrospectResponse {
    pub active: bool,
    pub client_id: Uuid,
    pub sub: Uuid,
    pub first_name: String,
    pub last_name: Option<String>,
    pub token_type: String,
    pub exp: usize,
    pub iat: usize,
    pub iss: String,
    pub client_name: String,
    pub resource_group: String,
    pub resources: Vec<String>,
}

#[derive(Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

#[derive(Serialize)]
pub struct RefreshTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: usize,
}
