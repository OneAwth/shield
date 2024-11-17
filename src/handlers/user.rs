use std::sync::Arc;

use crate::mappers::user::{
    AddResourceRequest, CreateUserRequest, ForgotPasswordRequest, ForgotPasswordResponse, InitiateForgotPasswordResponse,
    SendEmailVerificationRequest, SendEmailVerificationResponse, UpdateResourceGroupRequest, UpdateResourceRequest, UpdateUserRequest, UserResponse,
    VerifyEmailRequest, VerifyEmailResponse,
};
use crate::mappers::DeleteResponse;
use crate::packages::api_token::ApiUser;
use crate::services::user::{
    change_password, initiate_forgot_password_service, insert_user, send_email_verification_service, update_user_by_id, verify_user_email,
};
use crate::utils::default_resource_checker::{is_default_resource, is_default_resource_group, is_default_user};
use axum::extract::Path;
use axum::{Extension, Form, Json};
use chrono::Utc;
use entity::sea_orm_active_enums::{ApiUserAccess, ApiUserScope};
use entity::{resource, resource_group, user};
use futures::future::try_join_all;
use sea_orm::prelude::Uuid;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use tracing::debug;

use crate::packages::db::AppState;
use crate::packages::errors::{AuthenticateError, Error};

pub async fn create_user(
    api_user: ApiUser,
    Extension(state): Extension<Arc<AppState>>,
    Path(realm_id): Path<Uuid>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<Json<UserResponse>, Error> {
    if api_user.has_access(ApiUserScope::Client, ApiUserAccess::Write) {
        let user = insert_user(&state.db, realm_id, payload).await?;
        Ok(Json(UserResponse::from(user)))
    } else {
        Err(Error::Authenticate(AuthenticateError::ActionForbidden))
    }
}

pub async fn get_users(
    user: ApiUser,
    Extension(state): Extension<Arc<AppState>>,
    Path(realm_id): Path<Uuid>,
) -> Result<Json<Vec<UserResponse>>, Error> {
    if !user.has_access(ApiUserScope::Realm, ApiUserAccess::Read) {
        return Err(Error::Authenticate(AuthenticateError::NoResource));
    }

    if user.realm_id.ne(&realm_id) {
        return Err(Error::Authenticate(AuthenticateError::ActionForbidden));
    }

    let users = user::Entity::find().filter(user::Column::RealmId.eq(realm_id)).all(&state.db).await?;
    if users.is_empty() {
        return Err(Error::not_found());
    }

    Ok(Json(users.into_iter().map(UserResponse::from).collect()))
}

pub async fn get_user(
    user: ApiUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((realm_id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<UserResponse>, Error> {
    if !user.has_access(ApiUserScope::Realm, ApiUserAccess::Read) {
        return Err(Error::Authenticate(AuthenticateError::NoResource));
    }

    if user.realm_id.ne(&realm_id) {
        return Err(Error::Authenticate(AuthenticateError::ActionForbidden));
    }

    let user = user::Entity::find()
        .filter(user::Column::Id.eq(user_id))
        .filter(user::Column::RealmId.eq(realm_id))
        .one(&state.db)
        .await?;

    if user.is_none() {
        return Err(Error::not_found());
    }

    Ok(Json(UserResponse::from(user.unwrap())))
}

pub async fn update_user(
    user: ApiUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((realm_id, user_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<UpdateUserRequest>,
) -> Result<Json<UserResponse>, Error> {
    if !user.has_access(ApiUserScope::Client, ApiUserAccess::Update) {
        return Err(Error::Authenticate(AuthenticateError::NoResource));
    }

    if user.realm_id.ne(&realm_id) {
        return Err(Error::Authenticate(AuthenticateError::ActionForbidden));
    }

    update_user_by_id(&state.db, realm_id, user_id, payload).await
}

pub async fn delete_user(
    user: ApiUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((realm_id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<DeleteResponse>, Error> {
    if !user.has_access(ApiUserScope::Realm, ApiUserAccess::Delete) {
        return Err(Error::Authenticate(AuthenticateError::NoResource));
    }

    if is_default_user(user_id) {
        return Err(Error::cannot_perform_operation("Cannot delete the default user"));
    }

    if user.realm_id.ne(&realm_id) {
        return Err(Error::Authenticate(AuthenticateError::ActionForbidden));
    }

    let result = user::Entity::delete(user::ActiveModel {
        id: Set(user_id),
        realm_id: Set(realm_id),
        ..Default::default()
    })
    .exec(&state.db)
    .await?;

    Ok(Json(DeleteResponse {
        ok: result.rows_affected == 1,
    }))
}

pub async fn get_resource_groups(
    user: ApiUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((realm_id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<resource_group::Model>>, Error> {
    if !user.has_access(ApiUserScope::Realm, ApiUserAccess::Read) {
        return Err(Error::Authenticate(AuthenticateError::NoResource));
    }

    if user.realm_id.ne(&realm_id) {
        return Err(Error::Authenticate(AuthenticateError::ActionForbidden));
    }

    let resource_groups = resource_group::Entity::find()
        .filter(resource_group::Column::RealmId.eq(realm_id))
        .filter(resource_group::Column::UserId.eq(user_id))
        .all(&state.db)
        .await?;
    Ok(Json(resource_groups))
}

pub async fn get_resource_group(
    user: ApiUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((realm_id, user_id, resource_group_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Json<resource_group::Model>, Error> {
    if !user.has_access(ApiUserScope::Realm, ApiUserAccess::Read) {
        return Err(Error::Authenticate(AuthenticateError::NoResource));
    }

    if user.realm_id.ne(&realm_id) {
        return Err(Error::Authenticate(AuthenticateError::ActionForbidden));
    }

    let resource_group = resource_group::Entity::find()
        .filter(resource_group::Column::Id.eq(resource_group_id))
        .filter(resource_group::Column::UserId.eq(user_id))
        .filter(resource_group::Column::RealmId.eq(realm_id))
        .one(&state.db)
        .await?;

    if resource_group.is_none() {
        return Err(Error::not_found());
    }

    Ok(Json(resource_group.unwrap()))
}

pub async fn update_resource_group(
    user: ApiUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((realm_id, _, resource_group_id)): Path<(Uuid, Uuid, Uuid)>,
    Json(payload): Json<UpdateResourceGroupRequest>,
) -> Result<Json<resource_group::Model>, Error> {
    if !user.has_access(ApiUserScope::Realm, ApiUserAccess::Update) {
        return Err(Error::Authenticate(AuthenticateError::NoResource));
    }

    if is_default_resource_group(resource_group_id) {
        return Err(Error::cannot_perform_operation("Cannot update the default resource group"));
    }

    if user.realm_id.ne(&realm_id) {
        return Err(Error::Authenticate(AuthenticateError::ActionForbidden));
    }

    let resource_group = resource_group::Entity::find()
        .filter(resource_group::Column::Id.eq(resource_group_id))
        .filter(resource_group::Column::RealmId.eq(realm_id))
        .one(&state.db)
        .await?;
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
}

pub async fn delete_resource_group(
    user: ApiUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((realm_id, user_id, resource_group_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Json<DeleteResponse>, Error> {
    if !user.has_access(ApiUserScope::Realm, ApiUserAccess::Delete) {
        return Err(Error::Authenticate(AuthenticateError::NoResource));
    }

    if user.realm_id.ne(&realm_id) {
        return Err(Error::Authenticate(AuthenticateError::ActionForbidden));
    }

    if is_default_resource_group(resource_group_id) {
        return Err(Error::cannot_perform_operation("Cannot delete the default resource group"));
    }

    let result = resource_group::Entity::delete(resource_group::ActiveModel {
        id: Set(resource_group_id),
        user_id: Set(user_id),
        realm_id: Set(realm_id),
        ..Default::default()
    })
    .exec(&state.db)
    .await?;

    Ok(Json(DeleteResponse {
        ok: result.rows_affected == 1,
    }))
}

pub async fn get_resources(
    user: ApiUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((realm_id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<resource::Model>>, Error> {
    if !user.has_access(ApiUserScope::Realm, ApiUserAccess::Read) {
        return Err(Error::Authenticate(AuthenticateError::NoResource));
    }
    if user.realm_id.ne(&realm_id) {
        return Err(Error::Authenticate(AuthenticateError::ActionForbidden));
    }

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
}

pub async fn add_resources(
    user: ApiUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((realm_id, user_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<AddResourceRequest>,
) -> Result<Json<Vec<resource::Model>>, Error> {
    if !user.has_access(ApiUserScope::Realm, ApiUserAccess::Write) {
        return Err(Error::Authenticate(AuthenticateError::NoResource));
    }

    if user.realm_id.ne(&realm_id) {
        return Err(Error::Authenticate(AuthenticateError::ActionForbidden));
    }

    if is_default_user(user_id) {
        return Err(Error::cannot_perform_operation("Cannot add resources to the default user"));
    }

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
}

pub async fn update_resource(
    user: ApiUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((realm_id, _, resource_id)): Path<(Uuid, Uuid, Uuid)>,
    Json(payload): Json<UpdateResourceRequest>,
) -> Result<Json<resource::Model>, Error> {
    if !user.has_access(ApiUserScope::Realm, ApiUserAccess::Update) {
        return Err(Error::Authenticate(AuthenticateError::NoResource));
    }

    if user.realm_id.ne(&realm_id) {
        return Err(Error::Authenticate(AuthenticateError::ActionForbidden));
    }

    if is_default_resource(resource_id) {
        return Err(Error::cannot_perform_operation("Cannot update the default resource"));
    }

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
}

pub async fn delete_resource(
    user: ApiUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((realm_id, _, resource_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Json<DeleteResponse>, Error> {
    if !user.has_access(ApiUserScope::Realm, ApiUserAccess::Delete) {
        return Err(Error::Authenticate(AuthenticateError::NoResource));
    }

    if user.realm_id.ne(&realm_id) {
        return Err(Error::Authenticate(AuthenticateError::ActionForbidden));
    }

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
}

pub async fn send_email_verification(
    user: ApiUser,
    Extension(state): Extension<Arc<AppState>>,
    Path(realm_id): Path<Uuid>,
    Form(payload): Form<SendEmailVerificationRequest>,
) -> Result<Json<SendEmailVerificationResponse>, Error> {
    if !user.has_access(ApiUserScope::Client, ApiUserAccess::Write) {
        debug!("No allowed access");
        return Err(Error::Authenticate(AuthenticateError::ActionForbidden));
    }

    if user.realm_id.ne(&realm_id) {
        return Err(Error::Authenticate(AuthenticateError::ActionForbidden));
    };

    send_email_verification_service(&state.db, payload).await
}

pub async fn verify_email(
    user: ApiUser,
    Extension(state): Extension<Arc<AppState>>,
    Path(realm_id): Path<Uuid>,
    Form(payload): Form<VerifyEmailRequest>,
) -> Result<Json<VerifyEmailResponse>, Error> {
    if !user.has_access(ApiUserScope::Client, ApiUserAccess::Write) {
        debug!("No allowed access");
        return Err(Error::Authenticate(AuthenticateError::ActionForbidden));
    }

    if user.realm_id.ne(&realm_id) {
        return Err(Error::Authenticate(AuthenticateError::ActionForbidden));
    };

    verify_user_email(&state.db, payload).await
}

pub async fn initiate_forgot_password(
    user: ApiUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((realm_id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<InitiateForgotPasswordResponse>, Error> {
    if !user.has_access(ApiUserScope::Client, ApiUserAccess::Write) {
        debug!("No allowed access");
        return Err(Error::Authenticate(AuthenticateError::ActionForbidden));
    }

    if user.realm_id.ne(&realm_id) {
        return Err(Error::Authenticate(AuthenticateError::ActionForbidden));
    };

    initiate_forgot_password_service(&state.db, realm_id, user_id).await
}

pub async fn forgot_password(
    user: ApiUser,
    Extension(state): Extension<Arc<AppState>>,
    Path((realm_id, user_id)): Path<(Uuid, Uuid)>,
    Form(payload): Form<ForgotPasswordRequest>,
) -> Result<Json<ForgotPasswordResponse>, Error> {
    if !user.has_access(ApiUserScope::Client, ApiUserAccess::Write) {
        debug!("No allowed access");
        return Err(Error::Authenticate(AuthenticateError::ActionForbidden));
    }

    if user.realm_id.ne(&realm_id) {
        return Err(Error::Authenticate(AuthenticateError::ActionForbidden));
    };

    change_password(&state.db, realm_id, user_id, payload).await
}
