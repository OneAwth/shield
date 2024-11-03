use chrono::Utc;
use sea_orm::{prelude::Uuid, ActiveModelTrait, ColumnTrait, DatabaseConnection, DeleteResult, EntityTrait, QueryFilter, Set};
use tracing::debug;

use crate::{
    mappers::client::{CreateClientRequest, UpdateClientRequest},
    packages::errors::{AuthenticateError, Error, NotFoundError},
    utils::default_resource_checker::is_default_client,
};
use entity::client;

pub async fn get_all_clients(db: &DatabaseConnection, realm_id: Uuid) -> Result<Vec<client::Model>, Error> {
    Ok(client::Entity::find().filter(client::Column::RealmId.eq(realm_id)).all(db).await?)
}

pub async fn get_client_by_id(db: &DatabaseConnection, client_id: Uuid) -> Result<Option<client::Model>, Error> {
    Ok(client::Entity::find_by_id(client_id).one(db).await?)
}

pub async fn insert_client(db: &DatabaseConnection, payload: CreateClientRequest) -> Result<client::Model, Error> {
    let client = client::ActiveModel {
        id: Set(Uuid::now_v7()),
        name: Set(payload.name.to_owned()),
        realm_id: Set(payload.realm_id),
        ..Default::default()
    };
    Ok(client.insert(db).await?)
}

pub async fn update_client_by_id(db: &DatabaseConnection, client_id: Uuid, payload: UpdateClientRequest) -> Result<client::Model, Error> {
    if is_default_client(client_id) && payload.lock == Some(true) {
        return Err(Error::cannot_perform_operation("Cannot lock the default client"));
    }

    let client = get_client_by_id(db, client_id).await?;
    match client {
        Some(client) => {
            let locked_at = match payload.lock {
                Some(true) => Some(client.locked_at.unwrap_or_else(|| Utc::now().into())),
                Some(false) => None,
                None => client.locked_at,
            };

            let updated_client = client::ActiveModel {
                id: Set(client.id),
                realm_id: Set(client.realm_id),
                name: Set(match payload.name {
                    Some(name) => name,
                    None => client.name,
                }),
                max_concurrent_sessions: Set(match payload.max_concurrent_sessions {
                    Some(max_concurrent_sessions) => max_concurrent_sessions,
                    None => client.max_concurrent_sessions,
                }),
                session_lifetime: Set(match payload.session_lifetime {
                    Some(session_lifetime) => session_lifetime,
                    None => client.session_lifetime,
                }),
                refresh_token_lifetime: Set(match payload.refresh_token_lifetime {
                    Some(refresh_token_lifetime) => refresh_token_lifetime,
                    None => client.refresh_token_lifetime,
                }),
                refresh_token_reuse_limit: Set(match payload.refresh_token_reuse_limit {
                    Some(refresh_token_reuse_limit) => refresh_token_reuse_limit,
                    None => client.refresh_token_reuse_limit,
                }),
                locked_at: Set(locked_at),
                ..Default::default()
            };
            let updated_client = updated_client.update(db).await?;
            Ok(updated_client)
        }
        None => Err(Error::Authenticate(AuthenticateError::NoResource)),
    }
}

pub async fn delete_client_by_id(db: &DatabaseConnection, id: Uuid) -> Result<DeleteResult, Error> {
    Ok(client::Entity::delete_by_id(id).exec(db).await?)
}

pub async fn get_active_client_by_id(db: &DatabaseConnection, client_id: Uuid) -> Result<client::Model, Error> {
    let client = client::Entity::find_by_id(client_id).one(db).await?;
    if client.is_none() {
        debug!("No client found");
        return Err(Error::NotFound(NotFoundError::ClientNotFound));
    }

    let client = client.unwrap();
    if client.locked_at.is_some() {
        debug!("Client is locked");
        return Err(Error::Authenticate(AuthenticateError::Locked));
    }
    Ok(client)
}
