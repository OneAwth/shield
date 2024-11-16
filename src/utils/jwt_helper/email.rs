use entity::sea_orm_active_enums::VerificationType;
use jsonwebtoken::{errors::Error, DecodingKey, EncodingKey, Header, TokenData, Validation};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::packages::settings::SETTINGS;

static VALIDATION: Lazy<Validation> = Lazy::new(Validation::default);
static HEADER: Lazy<Header> = Lazy::new(Header::default);

#[derive(Deserialize, Serialize)]
pub struct Claims {
    pub sub: Uuid,
    pub vefification_type: VerificationType,
    pub exp: usize,
}

pub struct CreateEmailVerificationTokenArgs {
    pub sub: Uuid,
    pub exp: usize,
    pub secret: String,
}

impl Default for CreateEmailVerificationTokenArgs {
    fn default() -> Self {
        Self {
            sub: Uuid::nil(),
            exp: chrono::Local::now().timestamp() as usize + 60 * 60,
            secret: SETTINGS.read().secrets.signing_key.to_owned(),
        }
    }
}

pub struct VerifyEmailTokenArgs {
    pub token: String,
    pub secret: String,
}

impl Default for VerifyEmailTokenArgs {
    fn default() -> Self {
        Self {
            token: "".to_string(),
            secret: SETTINGS.read().secrets.signing_key.to_owned(),
        }
    }
}

impl Claims {
    pub fn new(sub: Uuid, verification_type: VerificationType, exp: usize) -> Self {
        Self {
            sub,
            vefification_type: verification_type,
            exp,
        }
    }
}

pub fn create_email_verification_token(args: CreateEmailVerificationTokenArgs) -> Result<String, Error> {
    let CreateEmailVerificationTokenArgs { sub, exp, secret } = args;
    let encoding_key = EncodingKey::from_secret(secret.as_ref());
    let claims = Claims::new(sub, VerificationType::Email, exp);
    jsonwebtoken::encode(&HEADER, &claims, &encoding_key)
}

pub fn validate_email_token(args: VerifyEmailTokenArgs) -> Result<TokenData<Claims>, Error> {
    let decoding_key = DecodingKey::from_secret(args.secret.as_ref());
    let validation = VALIDATION.clone();
    jsonwebtoken::decode::<Claims>(&args.token, &decoding_key, &validation)
}
