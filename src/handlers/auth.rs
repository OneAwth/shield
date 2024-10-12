use chrono::Utc;
use entity::{
    client, refresh_token, resource, resource_group,
    sea_orm_active_enums::{ApiUserAccess, ApiUserRole},
    session, user,
};

use sea_orm::{prelude::Uuid, ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, Set};
use std::sync::Arc;

use crate::{
    mappers::auth::{
        CreateUserRequest, IntrospectRequest, IntrospectResponse, LogoutRequest, LogoutResponse, RefreshTokenRequest, RefreshTokenResponse,
    },
    middleware::session_info_extractor::SessionInfo,
    packages::{
        api_token::{decode_refresh_token, ApiUser, RefreshTokenClaims},
        db::AppState,
        errors::{AuthenticateError, Error},
        jwt_token::{create, decode, JwtUser},
        settings::SETTINGS,
    },
    services::user::insert_user,
    utils::role_checker::{has_access_to_api_cred, is_current_realm_admin, is_master_realm_admin},
};
use axum::{extract::Path, Extension, Json};
use serde::{Deserialize, Serialize};
use tracing::debug;

#[derive(Deserialize)]
pub struct Credentials {
    email: String,
    password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    access_token: String,
    user: user::Model,
    session_id: Uuid,
    realm_id: Uuid,
    client_id: Uuid,
}

pub async fn login(
    Extension(state): Extension<Arc<AppState>>,
    Extension(session_info): Extension<Arc<SessionInfo>>,
    Path((realm_id, client_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<Credentials>,
) -> Result<Json<LoginResponse>, Error> {
    debug!("🚀 Login request received! {:#?}", session_info);

    let user_with_resource_groups = user::Entity::find()
        .filter(user::Column::Email.eq(payload.email))
        .find_also_related(resource_group::Entity)
        .filter(resource_group::Column::RealmId.eq(realm_id))
        .filter(resource_group::Column::ClientId.eq(client_id))
        .one(&state.db)
        .await?;

    if user_with_resource_groups.is_none() {
        debug!("No matching data found");
        return Err(Error::not_found());
    }

    let (user, resource_groups) = user_with_resource_groups.unwrap();
    if !user.verify_password(&payload.password) {
        debug!("Wrong password");
        return Err(Error::Authenticate(AuthenticateError::WrongCredentials));
    }
    if user.locked_at.is_some() {
        debug!("User is locked");
        return Err(Error::Authenticate(AuthenticateError::Locked));
    }

    if resource_groups.is_none() {
        debug!("No matching resource group found");
        return Err(Error::not_found());
    }

    let resource_groups = resource_groups.unwrap();
    if resource_groups.locked_at.is_some() {
        debug!("Resource group is locked");
        return Err(Error::Authenticate(AuthenticateError::Locked));
    }

    // Fetch client separately
    let client = client::Entity::find_by_id(client_id).one(&state.db).await?.ok_or_else(|| {
        debug!("No client found");
        Error::not_found()
    })?;

    if client.locked_at.is_some() {
        debug!("Client is locked");
        return Err(Error::Authenticate(AuthenticateError::Locked));
    }

    create_session(client, user, resource_groups, session_info, None, state).await
}

async fn create_session(
    client: client::Model,
    user: user::Model,
    resource_groups: resource_group::Model,
    session_info: Arc<SessionInfo>,
    refresh_token_id: Option<Uuid>,
    state: Arc<AppState>,
) -> Result<Json<LoginResponse>, Error> {
    let sessions = session::Entity::find()
        .filter(session::Column::ClientId.eq(client.id))
        .filter(session::Column::UserId.eq(user.id))
        .filter(session::Column::Expires.gt(chrono::Utc::now()))
        .count(&state.db)
        .await?;

    if sessions >= client.max_concurrent_sessions as u64 {
        debug!("Client has reached max concurrent sessions");
        return Err(Error::Authenticate(AuthenticateError::MaxConcurrentSessions));
    }

    // Fetch resources
    let resources = resource::Entity::find()
        .filter(resource::Column::GroupId.eq(resource_groups.id))
        .filter(resource::Column::LockedAt.is_null())
        .all(&state.db)
        .await?;

    if resources.is_empty() {
        debug!("No resources found");
        return Err(Error::Authenticate(AuthenticateError::Locked));
    }

    let session_model = session::ActiveModel {
        id: Set(Uuid::now_v7()),
        user_id: Set(user.id),
        client_id: Set(client.id),
        ip_address: Set(session_info.ip_address.to_string()),
        user_agent: Set(Some(session_info.user_agent.to_string())),
        browser: Set(Some(session_info.browser.to_string())),
        browser_version: Set(Some(session_info.browser_version.to_string())),
        operating_system: Set(Some(session_info.operating_system.to_string())),
        device_type: Set(Some(session_info.device_type.to_string())),
        country_code: Set(session_info.country_code.to_string()),
        refresh_token_id: Set(refresh_token_id),
        expires: Set((Utc::now() + chrono::Duration::seconds(client.session_lifetime as i64)).into()),
        ..Default::default()
    };
    let session = session_model.insert(&state.db).await?;

    let access_token = create(
        user.clone(),
        &client,
        resource_groups,
        resources,
        &session,
        &SETTINGS.read().secrets.signing_key,
    )
    .unwrap();

    Ok(Json(LoginResponse {
        access_token,
        realm_id: user.realm_id,
        user,
        session_id: session.id,
        client_id: client.id,
    }))
}

pub async fn register(
    user: JwtUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((realm_id, client_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<Json<user::Model>, Error> {
    if is_master_realm_admin(&user) || is_current_realm_admin(&user, &realm_id.to_string()) {
        let user = insert_user(&state.db, realm_id, client_id, payload).await?;
        Ok(Json(user))
    } else {
        Err(Error::Authenticate(AuthenticateError::ActionForbidden))
    }
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
    user: JwtUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((realm_id, _)): Path<(Uuid, Uuid)>,
    Json(payload): Json<LogoutRequest>,
) -> Result<Json<LogoutResponse>, Error> {
    if is_master_realm_admin(&user) || is_current_realm_admin(&user, &realm_id.to_string()) {
        match payload.access_token {
            Some(access_token) => {
                let sid = decode(&access_token, &SETTINGS.read().secrets.signing_key)
                    .map_err(|_| AuthenticateError::InvalidToken)?
                    .claims
                    .sid;
                let result = session::Entity::delete_by_id(sid).exec(&state.db).await?;
                Ok(Json(LogoutResponse {
                    ok: result.rows_affected == 1,
                    user_id: user.sub,
                    session_id: user.sid,
                }))
            }
            None => match payload.refresh_token {
                Some(refresh_token) => {
                    let sid = decode(&refresh_token, &SETTINGS.read().secrets.signing_key)
                        .map_err(|_| AuthenticateError::InvalidToken)?
                        .claims
                        .sid;
                    let result = session::Entity::delete_by_id(sid).exec(&state.db).await?;
                    Ok(Json(LogoutResponse {
                        ok: result.rows_affected == 1,
                        user_id: user.sub,
                        session_id: user.sid,
                    }))
                }
                None => Err(Error::Authenticate(AuthenticateError::NoResource)),
            },
        }
    } else {
        Err(Error::Authenticate(AuthenticateError::ActionForbidden))
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
    user: JwtUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((realm_id, client_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<LogoutRequest>,
) -> Result<Json<LogoutResponse>, Error> {
    if is_master_realm_admin(&user) || is_current_realm_admin(&user, &realm_id.to_string()) {
        match payload.access_token {
            Some(access_token) => {
                let sub = decode(&access_token, &SETTINGS.read().secrets.signing_key)
                    .map_err(|_| AuthenticateError::InvalidToken)?
                    .claims
                    .sub;
                let result = session::Entity::delete_many()
                    .filter(session::Column::ClientId.eq(client_id))
                    .filter(session::Column::UserId.eq(sub))
                    .exec(&state.db)
                    .await?;
                Ok(Json(LogoutResponse {
                    ok: result.rows_affected > 0,
                    user_id: user.sub,
                    session_id: user.sid,
                }))
            }
            None => match payload.refresh_token {
                Some(refresh_token) => {
                    let sub = decode(&refresh_token, &SETTINGS.read().secrets.signing_key)
                        .map_err(|_| AuthenticateError::InvalidToken)?
                        .claims
                        .sub;
                    let result = session::Entity::delete_many()
                        .filter(session::Column::ClientId.eq(client_id))
                        .filter(session::Column::UserId.eq(sub))
                        .exec(&state.db)
                        .await?;
                    Ok(Json(LogoutResponse {
                        ok: result.rows_affected > 0,
                        user_id: user.sub,
                        session_id: user.sid,
                    }))
                }
                None => Err(Error::Authenticate(AuthenticateError::NoResource)),
            },
        }
    } else {
        Err(Error::Authenticate(AuthenticateError::ActionForbidden))
    }
}

pub async fn introspect(
    user: JwtUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((realm_id, client_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<IntrospectRequest>,
) -> Result<Json<IntrospectResponse>, Error> {
    if is_master_realm_admin(&user) || is_current_realm_admin(&user, &realm_id.to_string()) {
        let token_data = decode(&payload.access_token, &SETTINGS.read().secrets.signing_key).expect("Failed to decode token");

        if token_data.claims.resource.is_none() || token_data.claims.resource.is_some() && token_data.claims.resource.unwrap().client_id != client_id
        {
            return Err(Error::Authenticate(AuthenticateError::NoResource));
        }

        let session = session::Entity::find_by_id(token_data.claims.sid).one(&state.db).await?;
        match session {
            Some(session) => {
                let user = user::Entity::find_by_id(session.user_id)
                    .filter(user::Column::LockedAt.is_null())
                    .one(&state.db)
                    .await?;

                match user {
                    Some(user) => {
                        let client = client::Entity::find_by_id(session.client_id)
                            .filter(client::Column::LockedAt.is_null())
                            .one(&state.db)
                            .await?;

                        match client {
                            Some(client) => {
                                let resource_group = resource_group::Entity::find()
                                    .filter(resource_group::Column::RealmId.eq(realm_id))
                                    .filter(resource_group::Column::ClientId.eq(client.id))
                                    .filter(resource_group::Column::UserId.eq(user.id))
                                    .filter(resource_group::Column::LockedAt.is_null())
                                    .one(&state.db)
                                    .await?;

                                match resource_group {
                                    Some(resource_group) => {
                                        let resources = resource::Entity::find()
                                            .filter(resource::Column::GroupId.eq(resource_group.id))
                                            .filter(resource::Column::LockedAt.is_null())
                                            .all(&state.db)
                                            .await?;
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
                                    None => Err(Error::Authenticate(AuthenticateError::NoResource))?,
                                }
                            }
                            None => Err(Error::Authenticate(AuthenticateError::NoResource)),
                        }
                    }
                    None => Err(Error::Authenticate(AuthenticateError::NoResource)),
                }
            }
            None => Err(Error::Authenticate(AuthenticateError::NoResource)),
        }
    } else {
        Err(Error::Authenticate(AuthenticateError::ActionForbidden))
    }
}

pub async fn refresh_token(
    user: ApiUser,
    Extension(state): Extension<Arc<AppState>>,
    Extension(session_info): Extension<Arc<SessionInfo>>,
    Path((realm_id, client_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<RefreshTokenRequest>,
) -> Result<Json<RefreshTokenResponse>, Error> {
    if has_access_to_api_cred(&user, ApiUserRole::ClientAdmin, ApiUserAccess::Admin).await {
        let token_data = decode_refresh_token(&payload.refresh_token, &SETTINGS.read().secrets.signing_key).expect("Failed to decode token");
        if token_data.claims.rli != realm_id || token_data.claims.cli != client_id {
            return Err(Error::Authenticate(AuthenticateError::InvalidToken));
        }

        let refresh_token = refresh_token::Entity::find_active_by_id(&state.db, token_data.claims.sub).await?;
        if refresh_token.is_none() {
            return Err(Error::not_found());
        }
        let client = client::Entity::find_active_by_id(&state.db, token_data.claims.cli).await?;
        if client.is_none() {
            return Err(Error::Authenticate(AuthenticateError::InvalidToken));
        }

        let refresh_token = refresh_token.unwrap();
        let client = client.unwrap();

        let refresh_token_model = if refresh_token.re_used_count >= client.refresh_token_reuse_limit {
            let model = refresh_token::ActiveModel {
                id: Set(Uuid::now_v7()),
                user_id: Set(user.id),
                client_id: Set(Some(client_id)),
                realm_id: Set(realm_id),
                re_used_count: Set(0),
                locked_at: Set(None),
                ..Default::default()
            };
            model.insert(&state.db).await?
        } else {
            let model = refresh_token::ActiveModel {
                id: Set(refresh_token.id),
                user_id: Set(refresh_token.user_id),
                client_id: Set(refresh_token.client_id),
                realm_id: Set(refresh_token.realm_id),
                re_used_count: Set(refresh_token.re_used_count + 1),
                locked_at: Set(None),
                ..Default::default()
            };
            model.update(&state.db).await?
        };

        // Fetch user and resource groups
        let user_with_resource_groups = user::Entity::find()
            .filter(user::Column::Id.eq(refresh_token.user_id))
            .find_also_related(resource_group::Entity)
            .filter(resource_group::Column::RealmId.eq(realm_id))
            .filter(resource_group::Column::ClientId.eq(client_id))
            .one(&state.db)
            .await?;

        if user_with_resource_groups.is_none() {
            debug!("No matching data found");
            return Err(Error::not_found());
        }

        let (user, resource_groups) = user_with_resource_groups.unwrap();
        if user.locked_at.is_some() {
            debug!("User is locked");
            return Err(Error::Authenticate(AuthenticateError::Locked));
        }

        if resource_groups.is_none() {
            debug!("No matching resource group found");
            return Err(Error::not_found());
        }

        let resource_groups = resource_groups.unwrap();
        if resource_groups.locked_at.is_some() {
            debug!("Resource group is locked");
            return Err(Error::Authenticate(AuthenticateError::Locked));
        }

        let refresh_token_claims = RefreshTokenClaims::from(&refresh_token_model, &client);
        let session = create_session(client, user, resource_groups, session_info, Some(refresh_token.id), state).await?;
        let refresh_token = refresh_token_claims.create_token(&SETTINGS.read().secrets.signing_key).unwrap();
        Ok(Json(RefreshTokenResponse {
            access_token: session.access_token.clone(),
            refresh_token,
            expires_in: token_data.claims.exp - chrono::Local::now().timestamp() as usize,
        }))
    } else {
        Err(Error::Authenticate(AuthenticateError::ActionForbidden))
    }
}
