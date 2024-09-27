use jsonwebtoken::{errors::Error, DecodingKey, EncodingKey, Header, TokenData, Validation};
use once_cell::sync::Lazy;
use sea_orm::prelude::Uuid;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::database::{
    client::Model as ClientModel, resource::Model as ResourceModel, resource_group::Model as ResourceGroupModel, user::Model as UserModel,
};

type TokenResult = Result<TokenData<Claims>, Error>;

static VALIDATION: Lazy<Validation> = Lazy::new(Validation::default);
static HEADER: Lazy<Header> = Lazy::new(Header::default);

#[derive(Debug, Serialize, Deserialize)]
pub struct Resource {
    pub client_id: Uuid,
    pub client_name: String,
    pub group_name: String,
    pub identifiers: HashMap<String, String>,
}

impl Resource {
    fn from(client: ClientModel, resource_group: ResourceGroupModel, resources: Vec<ResourceModel>) -> Self {
        let mut identifiers = HashMap::new();
        for resource in resources {
            identifiers.insert(resource.name, resource.value);
        }

        Self {
            client_id: client.id,
            client_name: client.name,
            group_name: resource_group.name,
            identifiers,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenUser {
    pub sub: Uuid,
    #[serde(rename = "firstName")]
    pub first_name: String,
    #[serde(rename = "lastName")]
    pub last_name: String,
    pub email: String,
    pub phone: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource: Option<Resource>,
}

impl TokenUser {
    fn from(user: UserModel, client: ClientModel, resource_group: ResourceGroupModel, resources: Vec<ResourceModel>) -> Self {
        Self {
            sub: user.id,
            first_name: user.first_name.clone(),
            last_name: user.last_name.unwrap_or_else(|| "".into()),
            email: user.email.clone(),
            phone: user.phone.unwrap_or_else(|| "".into()),
            resource: Some(Resource::from(client, resource_group, resources)),
        }
    }

    pub fn from_claim(claims: Claims) -> Self {
        Self {
            sub: claims.sub,
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
    pub exp: usize, // Expiration time (as UTC timestamp). validate_exp defaults to true in validation
    pub iat: usize, // Issued at (as UTC timestamp)
    pub sub: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: String,
    pub resource: Option<Resource>,
}

impl Claims {
    pub fn new(user: UserModel, client: ClientModel, resource_group: ResourceGroupModel, resources: Vec<ResourceModel>) -> Self {
        let user = TokenUser::from(user, client, resource_group, resources);
        Self {
            exp: (chrono::Local::now() + chrono::Duration::days(30)).timestamp() as usize,
            iat: chrono::Local::now().timestamp() as usize,
            sub: user.sub,
            first_name: user.first_name,
            last_name: user.last_name,
            email: user.email,
            phone: user.phone,
            resource: user.resource,
        }
    }
}

pub fn create(
    user: UserModel,
    client: ClientModel,
    resource_group: ResourceGroupModel,
    resources: Vec<ResourceModel>,
    secret: &str,
) -> Result<String, Error> {
    let encoding_key = EncodingKey::from_secret(secret.as_ref());
    let claims = Claims::new(user, client, resource_group, resources);

    jsonwebtoken::encode(&HEADER, &claims, &encoding_key)
}

pub fn decode(token: &str, secret: &str) -> TokenResult {
    let decoding_key = DecodingKey::from_secret(secret.as_ref());

    jsonwebtoken::decode::<Claims>(token, &decoding_key, &VALIDATION)
}
