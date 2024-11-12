use std::sync::Arc;

use crate::mappers::user::{AddResourceRequest, CreateUserRequest, UpdateResourceGroupRequest, UpdateResourceRequest};
use crate::mappers::DeleteResponse;
use crate::packages::api_token::ApiUser;
use crate::services::user::insert_user;
use crate::utils::default_resource_checker::{is_default_resource, is_default_resource_group, is_default_user};
use axum::extract::Path;
use axum::{Extension, Json};
use chrono::Utc;
use entity::sea_orm_active_enums::{ApiUserAccess, ApiUserScope};
use entity::{resource, resource_group, user};
use futures::future::try_join_all;
use sea_orm::prelude::Uuid;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

use crate::packages::db::AppState;
use crate::{
    packages::{
        errors::{AuthenticateError, Error},
        jwt_token::JwtUser,
    },
    utils::role_checker::{is_current_realm_admin, is_master_realm_admin},
};

pub async fn create_user(
    api_user: ApiUser,
    Extension(state): Extension<Arc<AppState>>,
    Path(realm_id): Path<Uuid>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<Json<user::Model>, Error> {
    if api_user.has_access(ApiUserScope::Client, ApiUserAccess::Write) {
        let user = insert_user(&state.db, realm_id, payload).await?;
        Ok(Json(user))
    } else {
        Err(Error::Authenticate(AuthenticateError::ActionForbidden))
    }
}

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
    user: JwtUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((realm_id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<resource_group::Model>>, Error> {
    if is_master_realm_admin(&user) || is_current_realm_admin(&user, &realm_id.to_string()) {
        let resource_groups = resource_group::Entity::find()
            .filter(resource_group::Column::RealmId.eq(realm_id))
            .filter(resource_group::Column::UserId.eq(user_id))
            .all(&state.db)
            .await?;
        Ok(Json(resource_groups))
    } else {
        Err(Error::Authenticate(AuthenticateError::ActionForbidden))
    }
}

pub async fn get_resource_group(
    user: JwtUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((realm_id, _, resource_group_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Json<resource_group::Model>, Error> {
    if is_master_realm_admin(&user) || is_current_realm_admin(&user, &realm_id.to_string()) {
        let resource_group = resource_group::Entity::find_by_id(resource_group_id).one(&state.db).await?;
        match resource_group {
            Some(resource_group) => Ok(Json(resource_group)),
            None => Err(Error::not_found()),
        }
    } else {
        Err(Error::Authenticate(AuthenticateError::ActionForbidden))
    }
}

pub async fn update_resource_group(
    user: JwtUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((realm_id, _, resource_group_id)): Path<(Uuid, Uuid, Uuid)>,
    Json(payload): Json<UpdateResourceGroupRequest>,
) -> Result<Json<resource_group::Model>, Error> {
    if is_master_realm_admin(&user) || is_current_realm_admin(&user, &realm_id.to_string()) {
        if is_default_resource_group(resource_group_id) {
            return Err(Error::cannot_perform_operation("Cannot update the default resource group"));
        }

        let resource_group = resource_group::Entity::find_by_id(resource_group_id).one(&state.db).await?;
        if resource_group.is_none() {
            return Err(Error::not_found());
        }

        let locked_at = match payload.lock {
            Some(true) => Some(resource_group.as_ref().unwrap().locked_at.unwrap_or_else(|| Utc::now().into())),
            Some(false) => None,
            None => resource_group.as_ref().unwrap().locked_at,
        };
        let is_default = match payload.is_default {
            Some(true) => Some(true),
            _ => Some(resource_group.as_ref().unwrap().is_default),
        };

        let resource_group = resource_group::ActiveModel {
            id: Set(resource_group_id),
            realm_id: Set(resource_group.as_ref().unwrap().realm_id),
            client_id: Set(resource_group.as_ref().unwrap().client_id),
            user_id: Set(resource_group.as_ref().unwrap().user_id),
            name: Set(payload.name),
            description: Set(payload.description),
            is_default: Set(is_default.unwrap()),
            locked_at: Set(locked_at),
            ..Default::default()
        };
        let resource_group = resource_group.update(&state.db).await?;
        Ok(Json(resource_group))
    } else {
        Err(Error::Authenticate(AuthenticateError::ActionForbidden))
    }
}

pub async fn delete_resource_group(
    user: JwtUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((realm_id, _, resource_group_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Json<DeleteResponse>, Error> {
    if is_master_realm_admin(&user) || is_current_realm_admin(&user, &realm_id.to_string()) {
        if is_default_resource_group(resource_group_id) {
            return Err(Error::cannot_perform_operation("Cannot delete the default resource group"));
        }

        let resource_group = resource_group::Entity::find_by_id(resource_group_id).one(&state.db).await?;
        if resource_group.is_none() {
            return Err(Error::not_found());
        }

        let result = resource_group::Entity::delete_by_id(resource_group_id).exec(&state.db).await?;
        Ok(Json(DeleteResponse {
            ok: result.rows_affected == 1,
        }))
    } else {
        Err(Error::Authenticate(AuthenticateError::ActionForbidden))
    }
}

pub async fn get_resources(
    user: JwtUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((realm_id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<resource::Model>>, Error> {
    if is_master_realm_admin(&user) || is_current_realm_admin(&user, &realm_id.to_string()) {
        let resource_groups = resource_group::Entity::find()
            .filter(resource_group::Column::RealmId.eq(realm_id))
            .filter(resource_group::Column::UserId.eq(user_id))
            .all(&state.db)
            .await?;

        let mut resource_group_ids = Vec::new();
        for resource_group in resource_groups {
            resource_group_ids.push(resource_group.id);
        }
        let resources = resource::Entity::find()
            .filter(resource::Column::GroupId.is_in(resource_group_ids))
            .all(&state.db)
            .await?;
        Ok(Json(resources))
    } else {
        Err(Error::Authenticate(AuthenticateError::ActionForbidden))
    }
}

pub async fn add_resources(
    user: JwtUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((realm_id, user_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<AddResourceRequest>,
) -> Result<Json<Vec<resource::Model>>, Error> {
    if is_master_realm_admin(&user) || is_current_realm_admin(&user, &realm_id.to_string()) {
        if payload.group_id.is_some() {
            let futures: Vec<_> = payload
                .identifiers
                .iter()
                .map(|(name, value)| {
                    let resource = resource::ActiveModel {
                        id: Set(Uuid::now_v7()),
                        group_id: Set(payload.group_id.unwrap()),
                        name: Set(name.to_string()),
                        value: Set(value.to_string()),
                        ..Default::default()
                    };
                    resource.insert(&state.db)
                })
                .collect();
            let resources = try_join_all(futures).await?;
            Ok(Json(resources))
        } else if payload.group_name.is_some() {
            let resource_groups = resource_group::Entity::find()
                .filter(resource_group::Column::RealmId.eq(realm_id))
                .filter(resource_group::Column::UserId.eq(user_id))
                .filter(resource_group::Column::Name.eq(payload.group_name))
                .one(&state.db)
                .await?;
            if resource_groups.is_none() {
                return Err(Error::not_found());
            }
            let resource_group = resource_groups.unwrap();

            let futures: Vec<_> = payload
                .identifiers
                .iter()
                .map(|(name, value)| {
                    let resource = resource::ActiveModel {
                        id: Set(Uuid::now_v7()),
                        group_id: Set(resource_group.id),
                        name: Set(name.to_string()),
                        value: Set(value.to_string()),
                        ..Default::default()
                    };
                    resource.insert(&state.db)
                })
                .collect();
            let resources = try_join_all(futures).await?;
            Ok(Json(resources))
        } else {
            Err(Error::cannot_perform_operation("Either group_name or group_id must be provided"))
        }
    } else {
        Err(Error::Authenticate(AuthenticateError::ActionForbidden))
    }
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
            group_id: Set(resource.unwrap().group_id),
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
    user: JwtUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((realm_id, _, resource_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Json<DeleteResponse>, Error> {
    if is_master_realm_admin(&user) || is_current_realm_admin(&user, &realm_id.to_string()) {
        if is_default_resource(resource_id) {
            return Err(Error::cannot_perform_operation("Cannot delete the default resource"));
        }

        let resource = resource::Entity::find_by_id(resource_id).one(&state.db).await?;
        if resource.is_none() {
            return Err(Error::not_found());
        }

        let result = resource::Entity::delete_by_id(resource_id).exec(&state.db).await?;
        Ok(Json(DeleteResponse {
            ok: result.rows_affected == 1,
        }))
    } else {
        Err(Error::Authenticate(AuthenticateError::ActionForbidden))
    }
}
