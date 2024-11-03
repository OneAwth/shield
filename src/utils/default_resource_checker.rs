use sea_orm::prelude::Uuid;

use crate::packages::settings::SETTINGS;

pub fn is_default_realm(realm_id: Uuid) -> bool {
    realm_id == SETTINGS.read().default_cred.realm_id
}

pub fn is_default_client(client_id: Uuid) -> bool {
    client_id == SETTINGS.read().default_cred.client_id
}

pub fn is_default_user(user_id: Uuid) -> bool {
    user_id == SETTINGS.read().default_cred.master_admin_user_id
}

pub fn is_default_resource_group_key(resource_group_key: Uuid) -> bool {
    resource_group_key == SETTINGS.read().default_cred.resource_group_key
}

pub fn is_default_resource(resource_id: Uuid) -> bool {
    SETTINGS.read().default_cred.resource_ids.contains(&resource_id)
}
