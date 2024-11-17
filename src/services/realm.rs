use chrono::Utc;
use sea_orm::{prelude::Uuid, ActiveModelTrait, DatabaseConnection, DeleteResult, EntityTrait, Set};

use crate::{
    mappers::realm::UpdateRealmRequest,
    packages::errors::{AuthenticateError, Error},
    utils::default_resource_checker::is_default_realm,
};
use entity::realm;

pub async fn get_all_realms(db: &DatabaseConnection) -> Result<Vec<realm::Model>, Error> {
    Ok(realm::Entity::find().all(db).await?)
}

pub async fn get_realm_by_id(db: &DatabaseConnection, id: Uuid) -> Result<Option<realm::Model>, Error> {
    Ok(realm::Entity::find_by_id(id).one(db).await?)
}

pub async fn insert_realm(db: &DatabaseConnection, name: String) -> Result<realm::Model, Error> {
    let realm = realm::ActiveModel {
        id: Set(Uuid::now_v7()),
        name: Set(name),
        ..Default::default()
    };
    Ok(realm.insert(db).await?)
}

pub async fn update_realm_by_id(db: &DatabaseConnection, id: Uuid, payload: UpdateRealmRequest) -> Result<realm::Model, Error> {
    if is_default_realm(id) && payload.lock == Some(true) {
        return Err(Error::cannot_perform_operation("Cannot lock the default realm"));
    }

    let realm = get_realm_by_id(db, id).await?;
    match realm {
        Some(realm) => {
            let locked_at = match payload.lock {
                Some(true) => Some(realm.locked_at.unwrap_or_else(|| Utc::now().into())),
                Some(false) => None,
                None => realm.locked_at,
            };

            let updated_realm = realm::ActiveModel {
                id: Set(realm.id),

                name: Set(match payload.name {
                    Some(name) => name,
                    None => realm.name,
                }),
                max_concurrent_sessions: Set(match payload.max_concurrent_sessions {
                    Some(max_concurrent_sessions) => Some(max_concurrent_sessions),
                    None => realm.max_concurrent_sessions,
                }),
                session_lifetime: Set(match payload.session_lifetime {
                    Some(session_lifetime) => session_lifetime,
                    None => realm.session_lifetime,
                }),
                refresh_token_lifetime: Set(match payload.refresh_token_lifetime {
                    Some(refresh_token_lifetime) => refresh_token_lifetime,
                    None => realm.refresh_token_lifetime,
                }),
                refresh_token_reuse_limit: Set(match payload.refresh_token_reuse_limit {
                    Some(refresh_token_reuse_limit) => refresh_token_reuse_limit,
                    None => realm.refresh_token_reuse_limit,
                }),
                is_account_activation_required: Set(match payload.is_account_activation_required {
                    Some(is_account_activation_required) => is_account_activation_required,
                    None => realm.is_account_activation_required,
                }),
                locked_at: Set(locked_at),
                ..Default::default()
            };
            let updated_realm = updated_realm.update(db).await?;
            Ok(updated_realm)
        }
        None => Err(Error::Authenticate(AuthenticateError::NoResource)),
    }
}

pub async fn delete_realm_by_id(db: &DatabaseConnection, id: Uuid) -> Result<DeleteResult, Error> {
    Ok(realm::Entity::delete_by_id(id).exec(db).await?)
}
