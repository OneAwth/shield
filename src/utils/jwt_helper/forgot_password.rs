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
    pub user_id: Uuid,
    pub exp: usize,
}

pub struct CreateForgotPasswordTokenArgs {
    pub sub: Uuid,
    pub exp: usize,
    pub user_id: Uuid,
    pub secret: String,
}

impl Default for CreateForgotPasswordTokenArgs {
    fn default() -> Self {
        Self {
            sub: Uuid::nil(),
            exp: chrono::Local::now().timestamp() as usize + 60 * 60,
            user_id: Uuid::nil(),
            secret: SETTINGS.read().secrets.signing_key.to_owned(),
        }
    }
}

pub struct VerifyForgotPasswordTokenArgs {
    pub token: String,
    pub secret: String,
}

impl Default for VerifyForgotPasswordTokenArgs {
    fn default() -> Self {
        Self {
            token: "".to_string(),
            secret: SETTINGS.read().secrets.signing_key.to_owned(),
        }
    }
}

impl Claims {
    pub fn new(sub: Uuid, verification_type: VerificationType, user_id: Uuid, exp: usize) -> Self {
        Self {
            sub,
            vefification_type: verification_type,
            user_id,
            exp,
        }
    }
}

pub fn create_forgot_password_token(args: CreateForgotPasswordTokenArgs) -> Result<String, Error> {
    let CreateForgotPasswordTokenArgs { sub, exp, secret, user_id } = args;
    let encoding_key = EncodingKey::from_secret(secret.as_ref());
    let claims = Claims::new(sub, VerificationType::ForgotPassword, user_id, exp);
    jsonwebtoken::encode(&HEADER, &claims, &encoding_key)
}

pub fn validate_forgot_password_token(args: VerifyForgotPasswordTokenArgs) -> Result<TokenData<Claims>, Error> {
    let decoding_key = DecodingKey::from_secret(args.secret.as_ref());
    let validation = VALIDATION.clone();
    jsonwebtoken::decode::<Claims>(&args.token, &decoding_key, &validation)
}
