use std::sync::Arc;

use crate::packages::api_token::ApiUser;
use crate::packages::db::AppState;
use crate::packages::errors::AuthenticateError;
use crate::packages::errors::Error;

use axum::{async_trait, extract::FromRequestParts, http::request::Parts};

#[async_trait]
impl<S> FromRequestParts<S> for ApiUser
where
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let state = parts.extensions.get::<Arc<AppState>>().expect("AppState not found");

        if let Some(api_key) = parts.headers.get("Api-Key").and_then(|v| v.to_str().ok()) {
            return ApiUser::validate_cred(&state.db, api_key).await;
        }
        Err(Error::Authenticate(AuthenticateError::InvalidApiCredentials))
    }
}
