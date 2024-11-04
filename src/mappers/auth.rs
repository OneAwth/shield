use entity::{
    sea_orm_active_enums::{GroupAccess, GroupRole},
    user,
};
use sea_orm::prelude::Uuid;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    pub group: Option<Group>,
    pub resource_group_key: Option<Uuid>,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub user: user::Model,
    pub session_id: Uuid,
    pub realm_id: Uuid,
    pub client_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
}

#[derive(Deserialize)]
pub struct Group {
    pub name: String,
    pub description: Option<String>,
    pub role: GroupRole,
    pub access: GroupAccess,
}

#[derive(Deserialize)]
pub struct ResourceSubset {
    pub group: Option<Group>,
    pub identifiers: HashMap<String, String>,
}

#[derive(Deserialize)]
pub struct LogoutRequest {
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
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

#[derive(Serialize)]
pub struct CreateUserResponse {
    pub id: Uuid,
    pub first_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_verified_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub two_factor_enabled_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub is_temp_password: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locked_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub realm_id: Uuid,
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
    pub updated_at: chrono::DateTime<chrono::FixedOffset>,
}

impl From<user::Model> for CreateUserResponse {
    fn from(user: user::Model) -> Self {
        Self {
            id: user.id,
            first_name: user.first_name,
            last_name: user.last_name,
            email: user.email,
            email_verified_at: user.email_verified_at,
            phone: user.phone,
            image: user.image,
            two_factor_enabled_at: user.two_factor_enabled_at,
            is_temp_password: user.is_temp_password,
            locked_at: user.locked_at,
            realm_id: user.realm_id,
            created_at: user.created_at,
            updated_at: user.updated_at,
        }
    }
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,
    pub token_type: String,
    pub exp: usize,
    pub iat: usize,
    pub iss: String,
    pub client_name: String,
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
