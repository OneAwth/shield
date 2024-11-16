use sea_orm::prelude::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub realm_id: Uuid,
    pub client_id: Uuid,
    pub access_token: String,
    pub expires_at: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
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
