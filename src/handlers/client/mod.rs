use std::sync::Arc;
pub mod api_user;

use crate::{
    mappers::{
        client::{CreateClientRequest, UpdateClientRequest},
        DeleteResponse,
    },
    packages::{
        api_token::ApiUser,
        db::AppState,
        errors::{AuthenticateError, Error},
    },
    services::client::{delete_client_by_id, get_all_clients, get_client_by_id, insert_client, update_client_by_id},
    utils::default_resource_checker::is_default_client,
};
use axum::{extract::Path, Extension, Json};
use entity::{
    client,
    sea_orm_active_enums::{ApiUserAccess, ApiUserRole},
};
use sea_orm::prelude::Uuid;

pub async fn get_clients(
    api_user: ApiUser,
    Extension(state): Extension<Arc<AppState>>,
    Path(realm_id): Path<Uuid>,
) -> Result<Json<Vec<client::Model>>, Error> {
    if !api_user.has_access(ApiUserRole::RealmAdmin, ApiUserAccess::Read) {
        return Err(Error::Authenticate(AuthenticateError::NoResource));
    }

    let clients = get_all_clients(&state.db, realm_id).await?;
    Ok(Json(clients))
}

pub async fn get_client(
    api_user: ApiUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((realm_id, client_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<client::Model>, Error> {
    if !api_user.has_access(ApiUserRole::ClientAdmin, ApiUserAccess::Read) {
        return Err(Error::Authenticate(AuthenticateError::NoResource));
    }

    let client = get_client_by_id(&state.db, client_id).await?;
    match client {
        Some(client) => {
            if client.realm_id != realm_id {
                return Err(Error::Authenticate(AuthenticateError::NoResource));
            }
            Ok(Json(client))
        }
        None => Err(Error::Authenticate(AuthenticateError::NoResource)),
    }
}

pub async fn create_client(
    api_user: ApiUser,
    Extension(state): Extension<Arc<AppState>>,
    Path(realm_id): Path<Uuid>,
    Json(payload): Json<CreateClientRequest>,
) -> Result<Json<client::Model>, Error> {
    if !api_user.has_access(ApiUserRole::RealmAdmin, ApiUserAccess::Admin) {
        return Err(Error::Authenticate(AuthenticateError::NoResource));
    }

    let client = insert_client(&state.db, realm_id, payload).await?;
    Ok(Json(client))
}

pub async fn update_client(
    api_user: ApiUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((realm_id, client_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<UpdateClientRequest>,
) -> Result<Json<client::Model>, Error> {
    if !api_user.has_access(ApiUserRole::ClientAdmin, ApiUserAccess::Update) {
        return Err(Error::Authenticate(AuthenticateError::NoResource));
    }

    let client = update_client_by_id(&state.db, realm_id, client_id, payload).await?;
    Ok(Json(client))
}

pub async fn delete_client(
    api_user: ApiUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((realm_id, client_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<DeleteResponse>, Error> {
    if !api_user.is_master_realm_admin() {
        return Err(Error::Authenticate(AuthenticateError::ActionForbidden));
    }

    if is_default_client(client_id) {
        return Err(Error::cannot_perform_operation("Cannot delete the default client"));
    }

    let client = delete_client_by_id(&state.db, realm_id, client_id).await?;
    Ok(Json(DeleteResponse {
        ok: client.rows_affected == 1,
    }))
}
