use crate::packages::errors::Error;
use entity::{client, resource, user};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use uuid::Uuid;

pub async fn is_client_belongs_to_realm(db: &DatabaseConnection, client_id: &Uuid, realm_id: &Uuid) -> Result<bool, Error> {
    let client = client::Entity::find_by_id(*client_id).one(db).await?;

    if client.is_none() {
        return Ok(false);
    }
    let client = client.unwrap();

    Ok(client.realm_id == *realm_id)
}

pub async fn is_resource_belongs_to_realm(db: &DatabaseConnection, realm_id: &Uuid, resource_id: &Uuid) -> Result<bool, Error> {
    let resource = resource::Entity::find_by_id(*resource_id).one(db).await?;

    if resource.is_none() {
        return Ok(false);
    }
    let resource = resource.unwrap();

    Ok(resource.group_key == *realm_id)
}

pub async fn is_user_belongs_to_realm(db: &DatabaseConnection, user_id: &Uuid, realm_id: &Uuid) -> Result<bool, Error> {
    let user = user::Entity::find_active_by_id(db, *user_id).await?;

    if user.is_none() {
        return Ok(false);
    }
    let user = user.unwrap();

    Ok(user.realm_id == *realm_id)
}

pub async fn is_resource_group_belongs_to_realm(db: &DatabaseConnection, resource_group_key: &Uuid, realm_id: &Uuid) -> Result<bool, Error> {
    let resource_group = resource::Entity::find()
        .filter(resource::Column::GroupKey.eq(*resource_group_key))
        .one(db)
        .await?;

    if resource_group.is_none() {
        return Ok(false);
    }
    let resource_group = resource_group.unwrap();
    let user = user::Entity::find_active_by_id(db, resource_group.user_id).await?;
    if user.is_none() {
        return Ok(false);
    }
    let user = user.unwrap();

    Ok(user.realm_id == *realm_id)
}
