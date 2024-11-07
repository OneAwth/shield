use entity::sea_orm_active_enums::{ApiUserAccess, ApiUserScope};
use sea_orm::prelude::Uuid;

use crate::packages::{api_token::ApiUser, jwt_token::JwtUser};

use super::default_resource_checker::is_default_realm;

pub fn is_master_realm_admin(user: &JwtUser) -> bool {
    let resource = match &user.resource {
        Some(resource) => resource,
        None => return false,
    };
    let role = resource.identifiers.get("role");
    let realm = resource.identifiers.get("realm");
    if role.is_none() || realm.is_none() {
        return false;
    }

    let is_admin = role.unwrap() == "admin";
    let is_master_realm = is_default_realm(resource.identifiers.get("realm").unwrap().parse::<Uuid>().unwrap());

    is_admin && is_master_realm
}

pub fn is_current_realm_admin(user: &JwtUser, realm_id: &str) -> bool {
    user.resource.as_ref().is_some_and(|x| {
        x.identifiers
            .get("realm")
            .is_some_and(|y| y == realm_id && x.identifiers.get("role").is_some_and(|z| z == "admin"))
    })
}

pub async fn has_access_to_api_cred(api_user: &ApiUser, role: ApiUserScope, access: ApiUserAccess) -> bool {
    if !api_user.role.has_access(role) {
        return false;
    }

    api_user.access.has_access(access)
}
