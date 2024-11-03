use jsonwebtoken::{errors::Error, DecodingKey, EncodingKey, Header, TokenData, Validation};
use once_cell::sync::Lazy;
use sea_orm::prelude::Uuid;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use entity::{client, resource, session, user};

use super::settings::SETTINGS;

type TokenResult = Result<TokenData<Claims>, Error>;

static VALIDATION: Lazy<Validation> = Lazy::new(Validation::default);
static HEADER: Lazy<Header> = Lazy::new(Header::default);

#[derive(Debug, Serialize, Deserialize)]
pub struct Resource {
    pub client_id: Uuid,
    pub client_name: String,
    pub group_key: Uuid,
    pub identifiers: HashMap<String, String>,
}

impl Resource {
    fn from(client: &client::Model, resources: Vec<resource::Model>) -> Self {
        let resource_group_key = resources.first().as_ref().unwrap().group_key;
        let mut identifiers = HashMap::new();
        for resource in resources {
            identifiers.insert(resource.name, resource.value);
        }

        Self {
            client_id: client.id,
            client_name: client.name.clone(),
            group_key: resource_group_key,
            identifiers,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JwtUser {
    pub sub: Uuid,
    pub sid: Uuid,
    #[serde(rename = "firstName")]
    pub first_name: String,
    #[serde(rename = "lastName")]
    pub last_name: String,
    pub email: String,
    pub phone: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource: Option<Resource>,
}

impl JwtUser {
    fn from(user: user::Model, client: &client::Model, resources: Vec<resource::Model>, session: &session::Model) -> Self {
        Self {
            sub: user.id,
            sid: session.id,
            first_name: user.first_name.clone(),
            last_name: user.last_name.unwrap_or_else(|| "".into()),
            email: user.email.clone(),
            phone: user.phone.unwrap_or_else(|| "".into()),
            resource: Some(Resource::from(client, resources)),
        }
    }

    pub fn from_claim(claims: Claims) -> Self {
        Self {
            sub: claims.sub,
            sid: claims.sid,
            first_name: claims.first_name,
            last_name: claims.last_name,
            email: claims.email,
            phone: claims.phone,
            resource: claims.resource,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub exp: usize,  // Expiration time (as UTC timestamp). validate_exp defaults to true in validation
    pub iat: usize,  // Issued at (as UTC timestamp)
    pub sub: Uuid,   // Subject
    pub sid: Uuid,   // Session ID
    pub iss: String, // Issuer
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: String,
    pub resource: Option<Resource>,
}

impl Claims {
    pub fn new(user: user::Model, client: &client::Model, resources: Vec<resource::Model>, session: &session::Model) -> Self {
        let user = JwtUser::from(user, client, resources, session);

        Self {
            exp: session.expires.timestamp() as usize,
            iat: chrono::Local::now().timestamp() as usize,
            sub: user.sub,
            sid: user.sid,
            iss: SETTINGS.read().server.host.clone(),
            first_name: user.first_name,
            last_name: user.last_name,
            email: user.email,
            phone: user.phone,
            resource: user.resource,
        }
    }
}

pub fn create(
    user: user::Model,
    client: &client::Model,
    resources: Vec<resource::Model>,
    session: &session::Model,
    secret: &str,
) -> Result<String, Error> {
    let encoding_key = EncodingKey::from_secret(secret.as_ref());
    let claims = Claims::new(user, client, resources, session);

    jsonwebtoken::encode(&HEADER, &claims, &encoding_key)
}

pub fn decode(token: &str, secret: &str) -> TokenResult {
    let decoding_key = DecodingKey::from_secret(secret.as_ref());

    jsonwebtoken::decode::<Claims>(token, &decoding_key, &VALIDATION)
}
