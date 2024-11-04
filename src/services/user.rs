use crate::{
    mappers::auth::CreateUserRequest,
    packages::errors::{AuthenticateError, Error, NotFoundError},
    utils::hash::generate_password_hash,
};
use entity::{group, resource, user, user_group};
use futures::future::join_all;
use sea_orm::{prelude::Uuid, ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, TransactionTrait};
use tracing::debug;

pub async fn insert_user(db: &DatabaseConnection, realm_id: Uuid, client_id: Uuid, payload: CreateUserRequest) -> Result<user::Model, Error> {
    let password_hash = generate_password_hash(payload.password).await?;

    let txn = db.begin().await?;
    let group_id: Option<Uuid> = match payload.resource.group {
        Some(group) => {
            let existing_group = group::Entity::find()
                .filter(group::Column::Name.eq(&group.name))
                .filter(group::Column::ClientId.eq(client_id))
                .filter(group::Column::RealmId.eq(realm_id))
                .one(&txn)
                .await?;
            if existing_group.is_some() {
                debug!("Group already exists");
                Some(existing_group.unwrap().id)
            } else {
                let group_model = group::ActiveModel {
                    id: Set(Uuid::now_v7()),
                    name: Set(group.name),
                    description: Set(group.description),
                    role: Set(group.role),
                    access: Set(group.access),
                    locked_at: Set(None),
                    client_id: Set(client_id),
                    realm_id: Set(realm_id),
                    ..Default::default()
                };
                let group = group_model.insert(&txn).await?;
                Some(group.id)
            }
        }
        None => None,
    };

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
    let user = user_model.insert(&txn).await?;

    if let Some(group_id) = group_id {
        let user_group_model = user_group::ActiveModel {
            user_id: Set(user.id),
            group_id: Set(group_id),
            ..Default::default()
        };
        user_group_model.insert(&txn).await?;
    }

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
                is_default: Set(Some(true)),
                ..Default::default()
            };
            resource.insert(&txn)
        })
        .collect();

    join_all(futures).await;
    txn.commit().await?;

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
