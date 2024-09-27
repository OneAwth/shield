use crate::packages::errors::AuthenticateError;
use crate::packages::errors::Error;
use crate::packages::settings::SETTINGS;
use crate::packages::token;
use crate::packages::token::TokenUser;

use axum::{async_trait, extract::FromRequestParts, http::request::Parts, RequestPartsExt};

use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};

#[async_trait]
impl<S> FromRequestParts<S> for TokenUser
where
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| AuthenticateError::InvalidToken)?;

        let secret = &SETTINGS.secrets.signing_key;
        let token_data = token::decode(bearer.token(), secret).map_err(|_| AuthenticateError::InvalidToken)?;

        Ok(TokenUser::from_claim(token_data.claims))
    }
}
