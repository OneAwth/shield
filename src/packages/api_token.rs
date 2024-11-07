use jsonwebtoken::{errors::Error as JwtError, DecodingKey, EncodingKey, Header, TokenData, Validation};
use once_cell::sync::Lazy;
use sea_orm::{
    prelude::{DateTimeWithTimeZone, Uuid},
    DatabaseConnection, EntityTrait,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use entity::{
    api_user, client, refresh_token,
    sea_orm_active_enums::{ApiUserAccess, ApiUserScope},
};

use crate::mappers::client::api_user::create_api_key;

use super::{
    errors::{AuthenticateError, Error},
    settings::SETTINGS,
};

static VALIDATION: Lazy<Validation> = Lazy::new(Validation::default);
static HEADER: Lazy<Header> = Lazy::new(Header::default);

#[derive(Deserialize, Serialize)]
pub struct ApiTokenClaims {
    pub exp: usize,  // Expiration time (as UTC timestamp). validate_exp defaults to true in validation
    pub iat: usize,  // Issued at (as UTC timestamp)
    pub sub: Uuid,   // Subject
    pub iss: String, // Issuer
    pub role: ApiUserScope,
    pub access: ApiUserAccess,
}

impl ApiTokenClaims {
    pub fn new(api_user: api_user::Model) -> Self {
        Self {
            exp: chrono::Local::now().timestamp() as usize + api_user.expires.timestamp() as usize,
            iat: chrono::Local::now().timestamp() as usize,
            sub: api_user.id,
            iss: SETTINGS.read().server.host.clone(),
            role: api_user.role,
            access: api_user.access,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiUser {
    pub id: Uuid,
    pub realm_id: Uuid,
    pub client_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub role: ApiUserScope,
    pub access: ApiUserAccess,
    pub expires: DateTimeWithTimeZone,
}

impl ApiUser {
    fn from(api_user: api_user::Model) -> Self {
        Self {
            id: api_user.id,
            realm_id: api_user.realm_id,
            client_id: api_user.client_id,
            name: api_user.name,
            description: api_user.description,
            role: api_user.role,
            access: api_user.access,
            expires: api_user.expires,
        }
    }

    pub fn create_token(api_user: api_user::Model, secret: &str) -> Result<String, JwtError> {
        create_api_key(api_user, secret)
    }

    pub async fn validate_cred(db: &DatabaseConnection, api_key: &str) -> Result<ApiUser, Error> {
        let token_data = verify_and_decode_jwt::<ApiTokenClaims>(api_key, &SETTINGS.read().secrets.api_key_signing_secret, None)?;

        let api_user = api_user::Entity::find_by_id(token_data.claims.sub).one(db).await?;
        if api_user.is_none() {
            return Err(Error::Authenticate(AuthenticateError::InvalidApiCredentials));
        }

        let api_user = api_user.unwrap();
        if api_user.expires.timestamp() <= chrono::Local::now().timestamp() {
            return Err(Error::Authenticate(AuthenticateError::InvalidApiCredentials));
        }

        if let Some(locked_at) = api_user.locked_at {
            if locked_at.timestamp() <= chrono::Local::now().timestamp() {
                return Err(Error::Authenticate(AuthenticateError::InvalidApiCredentials));
            }
        }

        Ok(Self::from(api_user))
    }

    pub fn has_access(&self, role: ApiUserScope, access: ApiUserAccess) -> bool {
        if self.role.has_access(role) && self.access.has_access(access) {
            return true;
        }
        false
    }

    pub fn is_master_realm_admin(&self) -> bool {
        self.realm_id == SETTINGS.read().default_cred.realm_id
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshTokenClaims {
    pub exp: usize,  // Expiration time (as UTC timestamp). validate_exp defaults to true in validation
    pub iat: usize,  // Issued at (as UTC timestamp)
    pub iss: String, // Issuer
    pub sid: Uuid,   // Session ID
    pub sub: Uuid,   // Subject --> Refresh Token ID
    pub rli: Uuid,   // Realm ID
    pub cli: Uuid,   // Client ID
}

impl RefreshTokenClaims {
    pub fn from(refresh_token: &refresh_token::Model, client: &client::Model) -> Self {
        Self {
            exp: chrono::Local::now().timestamp() as usize + client.refresh_token_lifetime as usize,
            iat: chrono::Local::now().timestamp() as usize,
            sub: refresh_token.id,
            sid: refresh_token.user_id,
            iss: SETTINGS.read().server.host.clone(),
            cli: client.id,
            rli: refresh_token.realm_id,
        }
    }

    pub fn create_token(&self, secret: &str) -> Result<String, jsonwebtoken::errors::Error> {
        let encoding_key = EncodingKey::from_secret(secret.as_ref());
        jsonwebtoken::encode(&HEADER, &self, &encoding_key)
    }
}

pub fn verify_and_decode_jwt<T>(token: &str, secret: &str, validation: Option<&Validation>) -> Result<TokenData<T>, JwtError>
where
    T: DeserializeOwned,
{
    let decoding_key = DecodingKey::from_secret(secret.as_ref());
    let validation = validation.unwrap_or(&VALIDATION);
    jsonwebtoken::decode::<T>(token, &decoding_key, validation)
}
