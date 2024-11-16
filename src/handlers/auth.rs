use axum_extra::either::Either;
use entity::{
    sea_orm_active_enums::{ApiUserAccess, ApiUserScope},
    session,
};

use sea_orm::{prelude::Uuid, ColumnTrait, EntityTrait, QueryFilter, TransactionTrait};
use std::sync::Arc;

use crate::{
    mappers::auth::{
        Credentials, IntrospectRequest, IntrospectResponse, LoginResponse, LogoutRequest, LogoutResponse, RefreshTokenRequest, RefreshTokenResponse,
    },
    middleware::session_info_extractor::SessionInfo,
    packages::{
        api_token::{verify_and_decode_jwt, ApiUser, RefreshTokenClaims},
        db::AppState,
        errors::{AuthenticateError, Error},
        jwt_token::{decode, JwtUser},
        settings::SETTINGS,
    },
    services::{
        auth::{
            create_session, create_session_and_refresh_token, get_active_refresh_token_by_id, get_active_resource_by_gu,
            get_active_resource_group_by_rcu, get_active_session_by_id, get_active_sessions_by_user_and_client_id, handle_refresh_token,
        },
        client::get_active_client_by_id,
        user::{get_active_user_and_resource_groups, get_active_user_by_id},
    },
    utils::role_checker::has_access_to_api_cred,
};
use axum::{extract::Path, Extension, Form, Json};
use tracing::debug;

pub async fn login(
    api_user: ApiUser,
    Extension(state): Extension<Arc<AppState>>,
    Extension(session_info): Extension<Arc<SessionInfo>>,
    Path((realm_id, client_id)): Path<(Uuid, Uuid)>,
    Form(payload): Form<Credentials>,
) -> Result<Json<LoginResponse>, Error> {
    debug!("🚀 Login request received! {:#?}", session_info);

    if !api_user.has_access(ApiUserScope::Client, ApiUserAccess::Write) {
        debug!("No allowed access");
        return Err(Error::Authenticate(AuthenticateError::ActionForbidden));
    }

    let (user, resource_groups) = get_active_user_and_resource_groups(&state.db, Either::E1(payload.username), realm_id, client_id).await?;

    if !user.verify_password(&payload.password) {
        debug!("Wrong password");
        return Err(Error::Authenticate(AuthenticateError::WrongCredentials));
    }

    let client = get_active_client_by_id(&state.db, client_id).await?;
    let sessions = get_active_sessions_by_user_and_client_id(&state.db, user.id, client.id).await?;

    if sessions.len() >= client.max_concurrent_sessions as usize {
        debug!("Client has reached max concurrent sessions");
        return Err(Error::Authenticate(AuthenticateError::MaxConcurrentSessions));
    }

    let login_response = create_session_and_refresh_token(state, user, client, resource_groups, session_info).await?;
    Ok(Json(login_response))
}

pub async fn logout_current_session(user: JwtUser, Extension(state): Extension<Arc<AppState>>) -> Result<Json<LogoutResponse>, Error> {
    let result = session::Entity::delete_by_id(user.sid).exec(&state.db).await?;
    Ok(Json(LogoutResponse {
        ok: result.rows_affected == 1,
        user_id: user.sub,
        session_id: user.sid,
    }))
}

pub async fn logout(
    api_user: ApiUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((_, _)): Path<(Uuid, Uuid)>,
    Form(payload): Form<LogoutRequest>,
) -> Result<Json<LogoutResponse>, Error> {
    if !api_user.has_access(ApiUserScope::Client, ApiUserAccess::Write) {
        debug!("No allowed access");
        return Err(Error::Authenticate(AuthenticateError::ActionForbidden));
    }

    match payload.access_token {
        Some(access_token) => {
            let claims = decode(&access_token, &SETTINGS.read().secrets.signing_key)
                .map_err(|_| AuthenticateError::InvalidToken)?
                .claims;

            let result = session::Entity::delete_by_id(claims.sid).exec(&state.db).await?;
            Ok(Json(LogoutResponse {
                ok: result.rows_affected == 1,
                user_id: claims.sub,
                session_id: claims.sid,
            }))
        }
        None => match payload.refresh_token {
            Some(refresh_token) => {
                let claims = decode(&refresh_token, &SETTINGS.read().secrets.signing_key)
                    .map_err(|_| AuthenticateError::InvalidToken)?
                    .claims;

                let result = session::Entity::delete_by_id(claims.sid).exec(&state.db).await?;
                Ok(Json(LogoutResponse {
                    ok: result.rows_affected == 1,
                    user_id: claims.sub,
                    session_id: claims.sid,
                }))
            }
            None => Err(Error::Authenticate(AuthenticateError::NoResource)),
        },
    }
}

pub async fn logout_my_all_sessions(
    user: JwtUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((_, client_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<LogoutResponse>, Error> {
    let result = session::Entity::delete_many()
        .filter(session::Column::ClientId.eq(client_id))
        .filter(session::Column::UserId.eq(user.sub))
        .exec(&state.db)
        .await?;
    Ok(Json(LogoutResponse {
        ok: result.rows_affected > 0,
        user_id: user.sub,
        session_id: user.sid,
    }))
}

pub async fn logout_all(
    api_user: ApiUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((_, client_id)): Path<(Uuid, Uuid)>,
    Form(payload): Form<LogoutRequest>,
) -> Result<Json<LogoutResponse>, Error> {
    if !api_user.has_access(ApiUserScope::Client, ApiUserAccess::Write) {
        debug!("No allowed access");
        return Err(Error::Authenticate(AuthenticateError::ActionForbidden));
    }

    match payload.access_token {
        Some(access_token) => {
            let claims = decode(&access_token, &SETTINGS.read().secrets.signing_key)
                .map_err(|_| AuthenticateError::InvalidToken)?
                .claims;

            let result = session::Entity::delete_many()
                .filter(session::Column::ClientId.eq(client_id))
                .filter(session::Column::UserId.eq(claims.sub))
                .exec(&state.db)
                .await?;
            Ok(Json(LogoutResponse {
                ok: result.rows_affected > 0,
                user_id: claims.sub,
                session_id: claims.sid,
            }))
        }
        None => match payload.refresh_token {
            Some(refresh_token) => {
                let claims = decode(&refresh_token, &SETTINGS.read().secrets.signing_key)
                    .map_err(|_| AuthenticateError::InvalidToken)?
                    .claims;
                let result = session::Entity::delete_many()
                    .filter(session::Column::ClientId.eq(client_id))
                    .filter(session::Column::UserId.eq(claims.sub))
                    .exec(&state.db)
                    .await?;
                Ok(Json(LogoutResponse {
                    ok: result.rows_affected > 0,
                    user_id: claims.sub,
                    session_id: claims.sid,
                }))
            }
            None => Err(Error::Authenticate(AuthenticateError::NoResource)),
        },
    }
}

pub async fn introspect(
    api_user: ApiUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((realm_id, client_id)): Path<(Uuid, Uuid)>,
    Form(payload): Form<IntrospectRequest>,
) -> Result<Json<IntrospectResponse>, Error> {
    if !api_user.has_access(ApiUserScope::Client, ApiUserAccess::Read) {
        debug!("No allowed access");
        return Err(Error::Authenticate(AuthenticateError::ActionForbidden));
    }

    let token_data = decode(&payload.access_token, &SETTINGS.read().secrets.signing_key)?;

    if token_data.claims.resource.is_none() || token_data.claims.resource.is_some() && token_data.claims.resource.unwrap().client_id != client_id {
        return Err(Error::Authenticate(AuthenticateError::ActionForbidden));
    }

    let session = get_active_session_by_id(&state.db, token_data.claims.sid).await?;
    let user = get_active_user_by_id(&state.db, session.user_id).await?;
    let client = get_active_client_by_id(&state.db, session.client_id).await?;
    let resource_group = get_active_resource_group_by_rcu(&state.db, realm_id, session.client_id, session.user_id).await?;
    let resources = get_active_resource_by_gu(&state.db, resource_group.id, session.user_id).await?;

    Ok(Json(IntrospectResponse {
        active: true,
        client_id: client.id,
        first_name: user.first_name.to_string(),
        last_name: Some(user.last_name.unwrap_or("".to_string())),
        sub: user.id,
        token_type: "bearer".to_string(),
        exp: token_data.claims.exp,
        iat: token_data.claims.iat,
        iss: SETTINGS.read().server.host.clone(),
        client_name: client.name,
        resource_group: resource_group.name,
        resources: resources.iter().map(|r| r.name.clone()).collect::<Vec<String>>(),
    }))
}

pub async fn refresh_token(
    user: ApiUser,
    Extension(state): Extension<Arc<AppState>>,
    Extension(session_info): Extension<Arc<SessionInfo>>,
    Path((realm_id, client_id)): Path<(Uuid, Uuid)>,
    Form(payload): Form<RefreshTokenRequest>,
) -> Result<Json<RefreshTokenResponse>, Error> {
    if !has_access_to_api_cred(&user, ApiUserScope::Client, ApiUserAccess::Write).await {
        debug!("No allowed access");
        return Err(Error::Authenticate(AuthenticateError::ActionForbidden));
    }

    let token_data = verify_and_decode_jwt::<RefreshTokenClaims>(&payload.refresh_token, &SETTINGS.read().secrets.signing_key, None)?;
    if token_data.claims.rli != realm_id || token_data.claims.cli != client_id {
        return Err(Error::Authenticate(AuthenticateError::ActionForbidden));
    }

    let refresh_token = get_active_refresh_token_by_id(&state.db, token_data.claims.sub).await?;
    let client = get_active_client_by_id(&state.db, token_data.claims.cli).await?;
    let (user, resource_groups) =
        get_active_user_and_resource_groups(&state.db, Either::E2(refresh_token.user_id), client.realm_id, client.id).await?;

    debug!("Before transaction calls");
    Ok(state
        .db
        .transaction(|txn| {
            Box::pin(async move {
                let refresh_token_claims = handle_refresh_token(txn, &refresh_token, &client).await.unwrap();
                let session = create_session(&client, &user, resource_groups, session_info, Some(refresh_token_claims.sub), txn)
                    .await
                    .unwrap();
                let refresh_token = refresh_token_claims.create_token(&SETTINGS.read().secrets.signing_key).unwrap();
                Ok(Json(RefreshTokenResponse {
                    access_token: session.access_token.clone(),
                    refresh_token,
                    expires_in: token_data.claims.exp - chrono::Local::now().timestamp() as usize,
                }))
            })
        })
        .await?)
}
