use std::sync::Arc;

use axum::{extract::Path, Extension, Json};
use entity::{
    realm,
    sea_orm_active_enums::{ApiUserAccess, ApiUserScope},
};
use sea_orm::prelude::Uuid;

use crate::{
    mappers::{
        realm::{CreateRealmRequest, UpdateRealmRequest},
        DeleteResponse,
    },
    packages::{
        api_token::ApiUser,
        db::AppState,
        errors::{AuthenticateError, Error},
    },
    services::realm::{delete_realm_by_id, get_all_realms, get_realm_by_id, insert_realm, update_realm_by_id},
    utils::default_resource_checker::is_default_realm,
};

pub async fn get_realms(api_user: ApiUser, Extension(state): Extension<Arc<AppState>>) -> Result<Json<Vec<realm::Model>>, Error> {
    if !api_user.is_master_realm_admin() {
        return Err(Error::Authenticate(AuthenticateError::NoResource));
    }

    let realms = get_all_realms(&state.db).await?;
    Ok(Json(realms))
}

pub async fn get_realm(
    api_user: ApiUser,
    Extension(state): Extension<Arc<AppState>>,
    Path(realm_id): Path<Uuid>,
) -> Result<Json<realm::Model>, Error> {
    if !api_user.has_access(ApiUserScope::Client, ApiUserAccess::Read) {
        return Err(Error::Authenticate(AuthenticateError::NoResource));
    }

    let fetched_realm = get_realm_by_id(&state.db, realm_id).await?;
    match fetched_realm {
        Some(fetched_realm) => Ok(Json(fetched_realm)),
        None => Err(Error::not_found()),
    }
}

pub async fn create_realm(
    api_user: ApiUser,
    Extension(state): Extension<Arc<AppState>>,
    Json(payload): Json<CreateRealmRequest>,
) -> Result<Json<realm::Model>, Error> {
    if !api_user.is_master_realm_admin() {
        return Err(Error::Authenticate(AuthenticateError::NoResource));
    }

    let realm = insert_realm(&state.db, payload.name).await?;
    Ok(Json(realm))
}

pub async fn update_realm(
    api_user: ApiUser,
    Extension(state): Extension<Arc<AppState>>,
    Path(realm_id): Path<Uuid>,
    Json(payload): Json<UpdateRealmRequest>,
) -> Result<Json<realm::Model>, Error> {
    if !api_user.has_access(ApiUserScope::Realm, ApiUserAccess::Update) {
        return Err(Error::Authenticate(AuthenticateError::NoResource));
    }

    let realm = update_realm_by_id(&state.db, realm_id, payload).await?;
    Ok(Json(realm))
}

pub async fn delete_realm(
    api_user: ApiUser,
    Extension(state): Extension<Arc<AppState>>,
    Path(realm_id): Path<Uuid>,
) -> Result<Json<DeleteResponse>, Error> {
    if !api_user.is_master_realm_admin() {
        return Err(Error::Authenticate(AuthenticateError::NoResource));
    }

    if is_default_realm(realm_id) {
        return Err(Error::cannot_perform_operation("Cannot delete the default realm"));
    }

    let result = delete_realm_by_id(&state.db, realm_id).await?;
    Ok(Json(DeleteResponse {
        ok: result.rows_affected == 1,
    }))
}
