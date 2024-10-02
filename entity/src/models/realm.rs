//! `SeaORM` Entity, @generated by sea-orm-codegen 1.0.1

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "realm")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(unique)]
    pub name: String,
    #[sea_orm(unique)]
    pub slug: String,
    pub max_concurrent_sessions: Option<i32>,
    pub session_lifetime: i32,
    pub refresh_token_lifetime: i32,
    pub refresh_token_reuse_limit: i32,
    pub locked_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::api_user::Entity")]
    ApiUser,
    #[sea_orm(has_many = "super::client::Entity")]
    Client,
    #[sea_orm(has_many = "super::resource_group::Entity")]
    ResourceGroup,
    #[sea_orm(has_many = "super::user::Entity")]
    User,
}

impl Related<super::api_user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ApiUser.def()
    }
}

impl Related<super::client::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Client.def()
    }
}

impl Related<super::resource_group::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ResourceGroup.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}
