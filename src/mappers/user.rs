use entity::user;
use sea_orm::prelude::{DateTimeWithTimeZone, Uuid};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct ResourceGroup {
    pub name: String,
    pub client_id: Uuid,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum IdentifierValue {
    String(String),
    Number(i64),
    Boolean(bool),
    Array(Vec<String>),
    Object(HashMap<String, String>),
}

#[derive(Deserialize)]
pub struct ResourceSubset {
    pub group: ResourceGroup,
    pub identifiers: Option<HashMap<String, IdentifierValue>>,
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
pub struct UpdateUserRequest {
    pub email: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub phone: Option<String>,
    pub image: Option<String>,
    pub is_account_activated: Option<bool>,
    pub is_temp_password: Option<bool>,
}

#[derive(Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub first_name: String,
    pub last_name: Option<String>,
    pub email: String,
    pub email_verified_at: Option<DateTimeWithTimeZone>,
    pub phone: Option<String>,
    pub image: Option<String>,
    pub two_factor_enabled_at: Option<DateTimeWithTimeZone>,
    pub is_temp_password: bool,
    pub is_account_activated: bool,
    pub locked_at: Option<DateTimeWithTimeZone>,
    pub realm_id: Uuid,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

impl From<user::Model> for UserResponse {
    fn from(user: user::Model) -> UserResponse {
        UserResponse {
            id: user.id,
            first_name: user.first_name.clone(),
            last_name: user.last_name.clone(),
            email: user.email.clone(),
            email_verified_at: user.email_verified_at,
            phone: user.phone.clone(),
            image: user.image.clone(),
            two_factor_enabled_at: user.two_factor_enabled_at,
            is_temp_password: user.is_temp_password,
            is_account_activated: user.is_account_activated,
            locked_at: user.locked_at,
            realm_id: user.realm_id,
            created_at: user.created_at,
            updated_at: user.updated_at,
        }
    }
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

#[derive(Deserialize)]
pub struct SendEmailVerificationRequest {
    pub user_id: Uuid,
}

#[derive(Serialize)]
pub struct SendEmailVerificationResponse {
    pub ok: bool,
}

#[derive(Deserialize)]
pub struct VerifyEmailRequest {
    pub token: String,
}

#[derive(Serialize)]
pub struct VerifyEmailResponse {
    pub ok: bool,
}

#[derive(Serialize)]
pub struct InitiateForgotPasswordResponse {
    pub ok: bool,
    pub token: String,
    pub expires_at: usize,
}

#[derive(Deserialize)]
pub struct ForgotPasswordRequest {
    pub password: String,
    pub password_confirmation: String,
    pub token: String,
}

#[derive(Serialize)]
pub struct ForgotPasswordResponse {
    pub ok: bool,
}
