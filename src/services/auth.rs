use std::sync::Arc;

use crate::{
    mappers::auth::LoginResponse,
    middleware::session_info_extractor::SessionInfo,
    packages::{
        api_token::RefreshTokenClaims,
        db::AppState,
        errors::{AuthenticateError, Error, NotFoundError},
        jwt_token::create,
        settings::SETTINGS,
    },
};
use chrono::{self, Duration, Utc};
use entity::{client, refresh_token, resource, resource_group, session, user};
use sea_orm::{
    prelude::Uuid, ActiveModelTrait, ColumnTrait, DatabaseConnection, DatabaseTransaction, DbErr, EntityTrait, QueryFilter, Set, TransactionTrait,
};
use tracing::debug;

pub async fn handle_refresh_token(
    txn: &DatabaseTransaction,
    refresh_token: &refresh_token::Model,
    client: &client::Model,
) -> Result<RefreshTokenClaims, Error> {
    let refresh_token_model = if refresh_token.re_used_count >= client.refresh_token_reuse_limit {
        refresh_token::Entity::delete_by_id(refresh_token.id).exec(txn).await?;
        let model = refresh_token::ActiveModel {
            id: Set(Uuid::now_v7()),
            user_id: Set(refresh_token.user_id),
            client_id: Set(Some(client.id)),
            realm_id: Set(client.realm_id),
            re_used_count: Set(0),
            locked_at: Set(None),
            expires: Set((Utc::now() + Duration::seconds(client.refresh_token_lifetime as i64)).into()),
            ..Default::default()
        };
        model.insert(txn).await?
    } else {
        let model = refresh_token::ActiveModel {
            id: Set(refresh_token.id),
            user_id: Set(refresh_token.user_id),
            client_id: Set(refresh_token.client_id),
            realm_id: Set(refresh_token.realm_id),
            re_used_count: Set(refresh_token.re_used_count + 1),
            locked_at: Set(None),
            expires: Set((Utc::now() + Duration::seconds(client.refresh_token_lifetime as i64)).into()),
            ..Default::default()
        };
        model.update(txn).await?
    };

    Ok(RefreshTokenClaims::from(&refresh_token_model, client))
}

pub async fn get_active_session_by_id(db: &DatabaseConnection, id: Uuid) -> Result<session::Model, Error> {
    let session = session::Entity::find_by_id(id).one(db).await?;
    if session.is_none() {
        debug!("No session found");
        return Err(Error::NotFound(NotFoundError::SessionNotFound));
    }

    let session = session.unwrap();
    Ok(session)
}

pub async fn get_active_sessions_by_user_and_client_id(
    db: &DatabaseConnection,
    user_id: Uuid,
    client_id: Uuid,
) -> Result<Vec<session::Model>, Error> {
    let sessions = session::Entity::find()
        .filter(session::Column::UserId.eq(user_id))
        .filter(session::Column::ClientId.eq(client_id))
        .filter(session::Column::Expires.gt(chrono::Local::now()))
        .all(db)
        .await?;
    Ok(sessions)
}

pub async fn create_session_and_refresh_token(
    state: Arc<AppState>,
    user: user::Model,
    client: client::Model,
    resource_groups: resource_group::Model,
    session_info: Arc<SessionInfo>,
) -> Result<LoginResponse, Error> {
    Ok(state
        .db
        .transaction(|txn| {
            Box::pin(async move {
                let result: Result<LoginResponse, Error> = async {
                    let refresh_token_model = if client.use_refresh_token {
                        let model = refresh_token::ActiveModel {
                            id: Set(Uuid::now_v7()),
                            user_id: Set(user.id),
                            client_id: Set(Some(client.id)),
                            realm_id: Set(client.realm_id),
                            re_used_count: Set(0),
                            expires: Set((Utc::now() + Duration::seconds(client.refresh_token_lifetime as i64)).into()),
                            locked_at: Set(None),
                            ..Default::default()
                        };
                        Some(model.insert(txn).await?)
                    } else {
                        None
                    };

                    let session = create_session(
                        &client,
                        &user,
                        resource_groups,
                        session_info,
                        refresh_token_model.as_ref().map(|x| x.id),
                        txn,
                    )
                    .await?;

                    let refresh_token = if let Some(refresh_token) = refresh_token_model {
                        let claims = RefreshTokenClaims::from(&refresh_token, &client);
                        Some(claims.create_token(&SETTINGS.read().secrets.signing_key).unwrap())
                    } else {
                        None
                    };

                    Ok(LoginResponse {
                        access_token: session.access_token,
                        realm_id: user.realm_id,
                        user,
                        session_id: session.session_id,
                        client_id: client.id,
                        refresh_token,
                    })
                }
                .await;

                result.map_err(|e| DbErr::Custom(e.to_string()))
            })
        })
        .await?)
}

pub async fn create_session(
    client: &client::Model,
    user: &user::Model,
    resource_groups: resource_group::Model,
    session_info: Arc<SessionInfo>,
    refresh_token_id: Option<Uuid>,
    db: &DatabaseTransaction,
) -> Result<LoginResponse, Error> {
    // Fetch resources
    let resources = resource::Entity::find()
        .filter(resource::Column::GroupId.eq(resource_groups.id))
        .filter(resource::Column::LockedAt.is_null())
        .all(db)
        .await?;

    // TODO: if resource_groups_id is Some and resources are empty then return error else continue
    if resources.is_empty() {
        debug!("No resources found");
        return Err(Error::Authenticate(AuthenticateError::Locked));
    }

    let session_model = session::ActiveModel {
        id: Set(Uuid::now_v7()),
        user_id: Set(user.id),
        client_id: Set(client.id),
        ip_address: Set(session_info.ip_address.to_string()),
        user_agent: Set(Some(session_info.user_agent.to_string())),
        browser: Set(Some(session_info.browser.to_string())),
        browser_version: Set(Some(session_info.browser_version.to_string())),
        operating_system: Set(Some(session_info.operating_system.to_string())),
        device_type: Set(Some(session_info.device_type.to_string())),
        country_code: Set(session_info.country_code.to_string()),
        refresh_token_id: Set(refresh_token_id),
        expires: Set((chrono::Utc::now() + chrono::Duration::seconds(client.session_lifetime as i64)).into()),
        ..Default::default()
    };
    let session = session_model.insert(db).await?;

    let access_token = create(
        user.clone(),
        client,
        resource_groups,
        resources,
        &session,
        &SETTINGS.read().secrets.signing_key,
    )
    .unwrap();

    Ok(LoginResponse {
        access_token,
        realm_id: user.realm_id,
        user: user.clone(),
        session_id: session.id,
        client_id: client.id,
        refresh_token: None,
    })
}

pub async fn get_active_resource_group_by_rcu(
    db: &DatabaseConnection,
    realm_id: Uuid,
    client_id: Uuid,
    user_id: Uuid,
) -> Result<resource_group::Model, Error> {
    let resource_group = resource_group::Entity::find()
        .filter(resource_group::Column::RealmId.eq(realm_id))
        .filter(resource_group::Column::ClientId.eq(client_id))
        .filter(resource_group::Column::UserId.eq(user_id))
        .one(db)
        .await?;
    if resource_group.is_none() {
        debug!("No resource group found");
        return Err(Error::NotFound(NotFoundError::ResourceGroupNotFound));
    }

    let resource_group = resource_group.unwrap();
    if resource_group.locked_at.is_some() {
        debug!("Resource group is locked");
        return Err(Error::Authenticate(AuthenticateError::Locked));
    }
    Ok(resource_group)
}

pub async fn get_active_resource_by_gu(db: &DatabaseConnection, group_id: Uuid, _user_id: Uuid) -> Result<Vec<resource::Model>, Error> {
    let resource = resource::Entity::find()
        .filter(resource::Column::GroupId.eq(group_id))
        .filter(resource::Column::LockedAt.is_null())
        .all(db)
        .await?;
    if resource.is_empty() {
        debug!("No resource found");
        return Err(Error::NotFound(NotFoundError::ResourceNotFound));
    }

    Ok(resource)
}

pub async fn get_active_refresh_token_by_id(db: &DatabaseConnection, id: Uuid) -> Result<refresh_token::Model, Error> {
    let refresh_token = refresh_token::Entity::find_by_id(id).one(db).await?;
    if refresh_token.is_none() {
        debug!("No refresh token found");
        return Err(Error::not_found());
    }

    let refresh_token = refresh_token.unwrap();
    if refresh_token.locked_at.is_some() {
        debug!("Refresh token is locked");
        return Err(Error::Authenticate(AuthenticateError::Locked));
    }
    Ok(refresh_token)
}
