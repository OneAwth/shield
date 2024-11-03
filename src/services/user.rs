use crate::{
    mappers::auth::CreateUserRequest,
    packages::errors::{AuthenticateError, Error, NotFoundError},
    utils::hash::generate_password_hash,
};
use entity::{resource, user};
use futures::future::join_all;
use sea_orm::{prelude::Uuid, ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use tracing::debug;

pub async fn insert_user(db: &DatabaseConnection, realm_id: Uuid, client_id: Uuid, payload: CreateUserRequest) -> Result<user::Model, Error> {
    let password_hash = generate_password_hash(payload.password).await?;
    let user_model = user::ActiveModel {
        id: Set(Uuid::now_v7()),
        realm_id: Set(realm_id),
        email: Set(payload.email),
        password_hash: Set(Some(password_hash)),
        first_name: Set(payload.first_name),
        last_name: Set(payload.last_name),
        phone: Set(payload.phone),
        image: Set(payload.image),
        ..Default::default()
    };

    let user = user_model.insert(db).await?;

    let group_key = Uuid::now_v7();
    let futures: Vec<_> = payload
        .resource
        .identifiers
        .iter()
        .map(|(name, value)| {
            let resource = resource::ActiveModel {
                id: Set(Uuid::now_v7()),
                group_key: Set(group_key),
                user_id: Set(user.id),
                client_id: Set(client_id),
                name: Set(name.to_string()),
                value: Set(value.to_string()),
                ..Default::default()
            };
            resource.insert(db)
        })
        .collect();

    join_all(futures).await;

    Ok(user)
}

pub async fn get_active_user_by_id(db: &DatabaseConnection, id: Uuid) -> Result<user::Model, Error> {
    let user = user::Entity::find_by_id(id).one(db).await?;
    if user.is_none() {
        debug!("No user found");
        return Err(Error::NotFound(NotFoundError::UserNotFound));
    }

    let user = user.unwrap();
    if user.locked_at.is_some() {
        debug!("User is locked");
        return Err(Error::Authenticate(AuthenticateError::Locked));
    }
    Ok(user)
}

pub async fn get_active_user_by_email(db: &DatabaseConnection, email: String, realm_id: Uuid) -> Result<user::Model, Error> {
    let user = user::Entity::find()
        .filter(user::Column::RealmId.eq(realm_id))
        .filter(user::Column::Email.eq(email))
        .one(db)
        .await?;
    if user.is_none() {
        debug!("No user found");
        return Err(Error::NotFound(NotFoundError::UserNotFound));
    }

    let user = user.unwrap();
    if user.locked_at.is_some() {
        debug!("User is locked");
        return Err(Error::Authenticate(AuthenticateError::Locked));
    }
    Ok(user)
}
