use std::sync::Arc;

use crate::mappers::user::{AddResourceRequest, UpdateResourceGroupRequest, UpdateResourceRequest};
use crate::mappers::DeleteResponse;
use crate::packages::api_token::ApiUser;
use crate::packages::errors::BadRequestError;
use crate::utils::default_resource_checker::{is_default_resource, is_default_resource_group_key, is_default_user};
use crate::utils::helpers::key_combo_validator::{
    is_client_belongs_to_realm, is_resource_belongs_to_realm, is_resource_group_belongs_to_realm, is_user_belongs_to_realm,
};
use axum::extract::Path;
use axum::{Extension, Json};
use chrono::Utc;
use entity::sea_orm_active_enums::{ApiUserAccess, ApiUserRole};
use entity::{resource, user};
use futures::future::try_join_all;
use sea_orm::prelude::Uuid;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait};

use crate::packages::db::AppState;
use crate::{
    packages::{
        errors::{AuthenticateError, Error},
        jwt_token::JwtUser,
    },
    utils::role_checker::{is_current_realm_admin, is_master_realm_admin},
};

pub async fn get_users(
    user: JwtUser,
    Extension(state): Extension<Arc<AppState>>,
    Path(realm_id): Path<Uuid>,
) -> Result<Json<Vec<user::Model>>, Error> {
    if is_master_realm_admin(&user) || is_current_realm_admin(&user, &realm_id.to_string()) {
        let users = user::Entity::find().filter(user::Column::RealmId.eq(realm_id)).all(&state.db).await?;
        if users.is_empty() {
            return Err(Error::not_found());
        }
        Ok(Json(users))
    } else {
        Err(Error::Authenticate(AuthenticateError::NoResource))
    }
}

pub async fn get_user(
    user: JwtUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((realm_id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<user::Model>, Error> {
    if is_master_realm_admin(&user) || is_current_realm_admin(&user, &realm_id.to_string()) {
        let user = user::Entity::find_by_id(user_id).one(&state.db).await?;
        match user {
            Some(user) => Ok(Json(user)),
            None => Err(Error::Authenticate(AuthenticateError::NoResource)),
        }
    } else {
        Err(Error::Authenticate(AuthenticateError::NoResource))
    }
}

pub async fn delete_user(
    user: JwtUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((realm_id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<DeleteResponse>, Error> {
    if is_master_realm_admin(&user) || is_current_realm_admin(&user, &realm_id.to_string()) {
        if is_default_user(user_id) {
            return Err(Error::cannot_perform_operation("Cannot delete the default user"));
        }
        if user_id == user.sub {
            return Err(Error::cannot_perform_operation("Cannot delete the current user"));
        }

        let result = user::Entity::delete_by_id(user_id).exec(&state.db).await?;
        Ok(Json(DeleteResponse {
            ok: result.rows_affected == 1,
        }))
    } else {
        Err(Error::Authenticate(AuthenticateError::ActionForbidden))
    }
}

pub async fn get_resource_groups(
    api_user: ApiUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((realm_id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<resource::Model>>, Error> {
    if !api_user.has_access(ApiUserRole::ClientAdmin, ApiUserAccess::Read) {
        return Err(Error::Authenticate(AuthenticateError::ActionForbidden));
    }

    if !is_user_belongs_to_realm(&state.db, &user_id, &realm_id).await? {
        return Err(Error::BadRequest(BadRequestError::BadRealmClientCombo));
    }

    let resource_groups = resource::Entity::find()
        .filter(resource::Column::UserId.eq(user_id))
        .all(&state.db)
        .await?;
    Ok(Json(resource_groups))
}

pub async fn get_resource_group(
    api_user: ApiUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((realm_id, _, resource_group_key)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Json<Vec<resource::Model>>, Error> {
    if !api_user.has_access(ApiUserRole::ClientAdmin, ApiUserAccess::Read) {
        return Err(Error::Authenticate(AuthenticateError::ActionForbidden));
    }

    if !is_resource_group_belongs_to_realm(&state.db, &resource_group_key, &realm_id).await? {
        return Err(Error::BadRequest(BadRequestError::BadRealmClientCombo));
    }

    let resources = resource::Entity::find()
        .filter(resource::Column::GroupKey.eq(resource_group_key))
        .all(&state.db)
        .await?;
    if resources.is_empty() {
        return Err(Error::not_found());
    }
    Ok(Json(resources))
}

pub async fn update_resource_group(
    api_user: ApiUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((realm_id, _, resource_group_key)): Path<(Uuid, Uuid, Uuid)>,
    Json(payload): Json<UpdateResourceGroupRequest>,
) -> Result<Json<Vec<resource::Model>>, Error> {
    if !api_user.has_access(ApiUserRole::ClientAdmin, ApiUserAccess::Update) {
        return Err(Error::Authenticate(AuthenticateError::NoResource));
    }

    if is_default_resource_group_key(resource_group_key) {
        return Err(Error::cannot_perform_operation("Cannot update the default resource group"));
    }

    if !is_resource_group_belongs_to_realm(&state.db, &resource_group_key, &realm_id).await? {
        return Err(Error::BadRequest(BadRequestError::BadRealmClientCombo));
    }

    let resources = resource::Entity::find()
        .filter(resource::Column::GroupKey.eq(resource_group_key))
        .all(&state.db)
        .await?;
    if resources.is_empty() {
        return Err(Error::not_found());
    }

    let first_resource = resources.first().unwrap();
    let locked_at = match payload.lock {
        Some(true) => Some(first_resource.locked_at.unwrap_or_else(|| Utc::now().into())),
        Some(false) => None,
        None => first_resource.locked_at,
    };
    let is_default = match payload.is_default {
        Some(true) => Some(true),
        _ => first_resource.is_default,
    };

    let txn = state.db.begin().await?;

    let mut updated_resources = Vec::new();
    for (name, value) in payload.identifiers.iter() {
        let resource_model = if let Some(existing) = resources.iter().find(|r| r.name == *name) {
            // Update existing resource
            resource::ActiveModel {
                id: Set(existing.id),
                client_id: Set(existing.client_id),
                user_id: Set(existing.user_id),
                group_key: Set(resource_group_key),
                name: Set(name.clone()),
                value: Set(value.clone()),
                description: Set(existing.description.to_owned()),
                is_default: Set(is_default),
                locked_at: Set(locked_at),
                ..Default::default()
            }
            .update(&txn)
            .await?
        } else {
            continue;
        };
        updated_resources.push(resource_model);
    }

    txn.commit().await?;

    // Return all resources in the group after update
    let final_resources = resource::Entity::find()
        .filter(resource::Column::GroupKey.eq(resource_group_key))
        .all(&state.db)
        .await?;
    Ok(Json(final_resources))
    // let resource_model = resource::ActiveModel {
    //     // TODO: Fix this update may function
    //     id: Set(resource_data.id),
    //     client_id: Set(resource_data.client_id),
    //     user_id: Set(resource_data.user_id),
    //     name: Set(payload.name),
    //     value: Set(payload.value),
    //     description: Set(payload.description),
    //     is_default: Set(is_default.unwrap()),
    //     locked_at: Set(locked_at),
    //     ..Default::default()
    // };
    // let resource_group = resource_group.update(&state.db).await?;
}

pub async fn delete_resource_by_group(
    api_user: ApiUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((realm_id, _user_id, resource_group_key)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Json<DeleteResponse>, Error> {
    if !api_user.has_access(ApiUserRole::ClientAdmin, ApiUserAccess::Delete) {
        return Err(Error::Authenticate(AuthenticateError::NoResource));
    }

    if is_default_resource_group_key(resource_group_key) {
        return Err(Error::BadRequest(BadRequestError::CannotDeleteDefaultProperty));
    }

    if !is_resource_group_belongs_to_realm(&state.db, &resource_group_key, &realm_id).await? {
        return Err(Error::BadRequest(BadRequestError::BadRealmClientCombo));
    }

    let result = resource::Entity::delete_many()
        .filter(resource::Column::GroupKey.eq(resource_group_key))
        .exec(&state.db)
        .await?;
    Ok(Json(DeleteResponse {
        ok: result.rows_affected == 1,
    }))
}

pub async fn get_resources(
    api_user: ApiUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((_, user_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<resource::Model>>, Error> {
    if !api_user.has_access(ApiUserRole::ClientAdmin, ApiUserAccess::Read) {
        return Err(Error::Authenticate(AuthenticateError::ActionForbidden));
    }

    let resources = resource::Entity::find()
        .filter(resource::Column::UserId.eq(user_id))
        .all(&state.db)
        .await?;
    Ok(Json(resources))
}

pub async fn add_resources(
    api_user: ApiUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((realm_id, user_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<AddResourceRequest>,
) -> Result<Json<Vec<resource::Model>>, Error> {
    if !api_user.has_access(ApiUserRole::ClientAdmin, ApiUserAccess::Write) {
        return Err(Error::Authenticate(AuthenticateError::ActionForbidden));
    }

    if !is_client_belongs_to_realm(&state.db, &payload.client_id, &realm_id).await? {
        return Err(Error::BadRequest(BadRequestError::BadRealmClientCombo));
    }

    let futures: Vec<_> = payload
        .identifiers
        .iter()
        .map(|(name, value)| {
            let resource = resource::ActiveModel {
                id: Set(Uuid::now_v7()),
                user_id: Set(user_id),
                client_id: Set(payload.client_id),
                name: Set(name.to_string()),
                value: Set(value.to_string()),
                ..Default::default()
            };
            resource.insert(&state.db)
        })
        .collect();
    let resources = try_join_all(futures).await?;
    Ok(Json(resources))
}

pub async fn update_resource(
    user: JwtUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((realm_id, _, resource_id)): Path<(Uuid, Uuid, Uuid)>,
    Json(payload): Json<UpdateResourceRequest>,
) -> Result<Json<resource::Model>, Error> {
    if is_master_realm_admin(&user) || is_current_realm_admin(&user, &realm_id.to_string()) {
        if is_default_resource(resource_id) {
            return Err(Error::cannot_perform_operation("Cannot update the default resource"));
        }

        let resource = resource::Entity::find_by_id(resource_id).one(&state.db).await?;
        if resource.is_none() {
            return Err(Error::not_found());
        }

        let locked_at = match payload.lock {
            Some(true) => Some(resource.as_ref().unwrap().locked_at.unwrap_or_else(|| Utc::now().into())),
            Some(false) => None,
            None => None,
        };
        let resource = resource::ActiveModel {
            id: Set(resource_id),
            group_key: Set(resource.unwrap().group_key),
            name: Set(payload.name),
            value: Set(payload.value),
            description: Set(payload.description),
            locked_at: Set(locked_at),
            ..Default::default()
        };
        let resource = resource.update(&state.db).await?;
        Ok(Json(resource))
    } else {
        Err(Error::Authenticate(AuthenticateError::ActionForbidden))
    }
}

pub async fn delete_resource(
    api_user: ApiUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((realm_id, _, resource_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Json<DeleteResponse>, Error> {
    if !api_user.has_access(ApiUserRole::ClientAdmin, ApiUserAccess::Delete) {
        return Err(Error::Authenticate(AuthenticateError::NoResource));
    }

    if is_default_resource(resource_id) {
        return Err(Error::BadRequest(BadRequestError::CannotDeleteDefaultProperty));
    }

    if !is_resource_belongs_to_realm(&state.db, &realm_id, &resource_id).await? {
        return Err(Error::BadRequest(BadRequestError::BadRealmClientCombo));
    }

    let resource = resource::Entity::find_by_id(resource_id).one(&state.db).await?; // TODO: It can be removed
    if resource.is_none() {
        return Err(Error::not_found());
    }

    let result = resource::Entity::delete_by_id(resource_id).exec(&state.db).await?;
    Ok(Json(DeleteResponse {
        ok: result.rows_affected == 1,
    }))
}
