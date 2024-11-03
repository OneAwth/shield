use chrono::{DateTime, Duration, FixedOffset, Utc};
use entity::{
    api_user,
    sea_orm_active_enums::{ApiUserAccess, ApiUserRole},
};
use jsonwebtoken::{EncodingKey, Header};
use once_cell::sync::Lazy;
use sea_orm::prelude::DateTimeWithTimeZone;
use serde::{Deserialize, Serialize};

use crate::packages::{api_token::ApiTokenClaims, settings::SETTINGS};

static HEADER: Lazy<Header> = Lazy::new(Header::default);

#[derive(Deserialize)]
pub enum TokenExpires {
    #[serde(rename = "never")]
    Never,
    #[serde(rename = "1h")]
    OneHour,
    #[serde(rename = "1d")]
    OneDay,
    #[serde(rename = "7d")]
    OneWeek,
    #[serde(rename = "1m")]
    OneMonth,
    #[serde(rename = "3m")]
    ThreeMonths,
    #[serde(rename = "1year")]
    OneYear,
}

impl TokenExpires {
    pub fn to_datetime(&self) -> DateTime<FixedOffset> {
        let utc_time = match self {
            TokenExpires::OneHour => Utc::now() + Duration::hours(1),
            TokenExpires::OneDay => Utc::now() + Duration::days(1),
            TokenExpires::OneWeek => Utc::now() + Duration::weeks(1),
            TokenExpires::OneMonth => Utc::now() + Duration::days(30),
            TokenExpires::ThreeMonths => Utc::now() + Duration::days(90),
            TokenExpires::OneYear => Utc::now() + Duration::weeks(52),
            _ => Utc::now() + Duration::weeks(52 * 99),
        };

        utc_time.into()
    }
}

#[derive(Deserialize)]
pub struct CreateApiUserRequest {
    pub name: String,
    pub description: Option<String>,
    pub role: ApiUserRole,
    pub access: ApiUserAccess,
    pub expires: Option<TokenExpires>,
}

#[derive(Serialize)]
pub struct CreateApiUserResponse {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub api_key: String,
    pub realm_id: String,
    pub client_id: String,
    pub role: ApiUserRole,
    pub access: ApiUserAccess,
    pub created_at: DateTimeWithTimeZone,
    pub expires_at: DateTimeWithTimeZone,
}

impl From<api_user::Model> for CreateApiUserResponse {
    fn from(api_user: api_user::Model) -> Self {
        Self {
            api_key: Self::create_token(api_user.clone(), &SETTINGS.read().secrets.signing_key).unwrap(),
            id: api_user.id.to_string(),
            name: api_user.name,
            description: api_user.description,
            realm_id: api_user.realm_id.to_string(),
            client_id: api_user.client_id.to_string(),
            role: api_user.role,
            access: api_user.access,
            created_at: api_user.created_at,
            expires_at: api_user.expires,
        }
    }
}

impl CreateApiUserResponse {
    pub fn create_token(api_user: api_user::Model, secret: &str) -> Result<String, jsonwebtoken::errors::Error> {
        create_api_key(api_user, secret)
    }
}

pub fn create_api_key(api_user: api_user::Model, secret: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let encoding_key = EncodingKey::from_secret(secret.as_ref());
    let claims = ApiTokenClaims::new(api_user);
    jsonwebtoken::encode(&HEADER, &claims, &encoding_key)
}

#[derive(Deserialize)]
pub struct UpdateApiUserRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub role: Option<ApiUserRole>,
    pub access: Option<ApiUserAccess>,
    pub expires: Option<TokenExpires>,
    pub lock: Option<bool>,
}

#[derive(Serialize)]
pub struct UpdateApiUserResponse {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub api_key: String,
    pub realm_id: String,
    pub client_id: String,
    pub role: ApiUserRole,
    pub access: ApiUserAccess,
    pub locked_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub expires_at: DateTimeWithTimeZone,
}

impl From<api_user::Model> for UpdateApiUserResponse {
    fn from(api_user: api_user::Model) -> Self {
        Self {
            api_key: create_api_key(api_user.clone(), &SETTINGS.read().secrets.api_key_signing_secret).unwrap(),
            id: api_user.id.to_string(),
            name: api_user.name,
            description: api_user.description,
            realm_id: api_user.realm_id.to_string(),
            client_id: api_user.client_id.to_string(),
            role: api_user.role,
            access: api_user.access,
            locked_at: api_user.locked_at,
            created_at: api_user.created_at,
            updated_at: api_user.updated_at,
            expires_at: api_user.expires,
        }
    }
}
