use crate::{
    mappers::user::{
        CreateUserRequest, ForgotPasswordRequest, ForgotPasswordResponse, InitiateForgotPasswordResponse, SendEmailVerificationRequest,
        SendEmailVerificationResponse, VerifyEmailRequest, VerifyEmailResponse,
    },
    packages::{
        errors::{AuthenticateError, Error, NotFoundError},
        mail::postman::send_verification_email,
    },
    utils::{
        hash::generate_password_hash,
        jwt_helper::{
            email::{create_email_verification_token, validate_email_token, CreateEmailVerificationTokenArgs, VerifyEmailTokenArgs},
            forgot_password::{
                create_forgot_password_token, validate_forgot_password_token, CreateForgotPasswordTokenArgs, VerifyForgotPasswordTokenArgs,
            },
        },
    },
};
use axum::Json;
use axum_extra::either::Either;
use chrono::Utc;
use entity::{resource, resource_group, sea_orm_active_enums::VerificationType, user, verification};
use futures::future::join_all;
use sea_orm::{prelude::Uuid, ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, TransactionTrait};
use tracing::debug;

pub async fn insert_user(db: &DatabaseConnection, realm_id: Uuid, payload: CreateUserRequest) -> Result<user::Model, Error> {
    let txn = db.begin().await?;
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

    let user = user_model.insert(&txn).await?;

    let resource_group = resource_group::ActiveModel {
        id: Set(Uuid::now_v7()),
        realm_id: Set(user.realm_id),
        client_id: Set(payload.resource.group.client_id),
        user_id: Set(user.id),
        name: Set(payload.resource.group.name),
        ..Default::default()
    };
    let resource_group = resource_group.insert(&txn).await?;

    let futures: Vec<_> = payload
        .resource
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

pub async fn get_active_user_and_resource_groups(
    db: &DatabaseConnection,
    user_identifier: Either<String, Uuid>,
    realm_id: Uuid,
    client_id: Uuid,
) -> Result<(user::Model, resource_group::Model), Error> {
    let mut query = user::Entity::find();
    query = match user_identifier {
        Either::E1(email) => query.filter(user::Column::Email.eq(email)),
        Either::E2(user_id) => query.filter(user::Column::Id.eq(user_id)),
    };

    let user_with_resource_groups = query
        .find_also_related(resource_group::Entity)
        .filter(resource_group::Column::RealmId.eq(realm_id))
        .filter(resource_group::Column::ClientId.eq(client_id))
        .one(db)
        .await?;

    if user_with_resource_groups.is_none() {
        debug!("No matching data found");
        return Err(Error::not_found());
    }

    let (user, resource_groups) = user_with_resource_groups.unwrap();

    if user.locked_at.is_some() {
        debug!("User is locked");
        return Err(Error::Authenticate(AuthenticateError::Locked));
    }

    if resource_groups.is_none() {
        debug!("No matching resource group found");
        return Err(Error::not_found());
    }

    let resource_groups = resource_groups.unwrap();
    if resource_groups.locked_at.is_some() {
        debug!("Resource group is locked");
        return Err(Error::Authenticate(AuthenticateError::Locked));
    }

    Ok((user, resource_groups))
}

pub async fn send_email_verification_service(
    db: &DatabaseConnection,
    data: SendEmailVerificationRequest,
) -> Result<Json<SendEmailVerificationResponse>, Error> {
    let user = user::Entity::find_by_id(data.user_id).one(db).await?;
    if user.is_none() {
        return Err(Error::Authenticate(AuthenticateError::InvalidToken));
    }
    let user = user.unwrap();
    if user.locked_at.is_some() {
        return Err(Error::Authenticate(AuthenticateError::Locked));
    }

    let verification = verification::ActiveModel {
        id: Set(Uuid::now_v7()),
        user_id: Set(user.id),
        r#type: Set(VerificationType::Email),
        expires: Set((chrono::Utc::now() + chrono::Duration::seconds(60 * 60)).into()),
    };
    let verification = verification.insert(db).await?;

    let token = create_email_verification_token(CreateEmailVerificationTokenArgs {
        sub: verification.id,
        exp: verification.expires.timestamp() as usize,
        ..Default::default()
    })?;

    let name = format!("{} {}", user.first_name, user.last_name.unwrap_or_default());
    send_verification_email(vec![&user.email], &name, &token).await.unwrap();

    Ok(Json(SendEmailVerificationResponse { ok: true }))
}

pub async fn verify_user_email(db: &DatabaseConnection, data: VerifyEmailRequest) -> Result<Json<VerifyEmailResponse>, Error> {
    let token_data = validate_email_token(VerifyEmailTokenArgs {
        token: data.token,
        ..Default::default()
    })?;
    if token_data.claims.vefification_type != VerificationType::Email {
        return Err(Error::Authenticate(AuthenticateError::InvalidToken));
    }

    let data = verification::Entity::find_by_id(token_data.claims.sub).one(db).await?;
    if data.is_none() {
        return Err(Error::Authenticate(AuthenticateError::InvalidToken));
    }

    let data = data.unwrap();
    if data.expires.timestamp() <= chrono::Local::now().timestamp() {
        return Err(Error::Authenticate(AuthenticateError::InvalidToken));
    }

    let user = user::Entity::find_by_id(data.user_id).one(db).await?;
    if user.is_none() {
        return Err(Error::Authenticate(AuthenticateError::InvalidToken));
    }
    if user.as_ref().unwrap().locked_at.is_some() {
        return Err(Error::Authenticate(AuthenticateError::Locked));
    }

    let mut user: user::ActiveModel = user.unwrap().into();
    user.email_verified_at = Set(Some(Utc::now().into()));
    user.update(db).await?;

    verification::Entity::delete_by_id(token_data.claims.sub).exec(db).await?;

    Ok(Json(VerifyEmailResponse { ok: true }))
}

pub async fn initiate_forgot_password_service(
    db: &DatabaseConnection,
    realm_id: Uuid,
    user_id: Uuid,
) -> Result<Json<InitiateForgotPasswordResponse>, Error> {
    let user = user::Entity::find_by_id(user_id).one(db).await?;
    if user.is_none() {
        return Err(Error::NotFound(NotFoundError::UserNotFound));
    }

    let user = user.unwrap();
    if user.locked_at.is_some() {
        return Err(Error::Authenticate(AuthenticateError::Locked));
    }

    if user.realm_id != realm_id {
        return Err(Error::Authenticate(AuthenticateError::ActionForbidden));
    }

    let verification = verification::Entity::find()
        .filter(verification::Column::UserId.eq(user.id))
        .filter(verification::Column::Type.eq(VerificationType::ForgotPassword))
        .one(db)
        .await?;

    let verification = match verification {
        Some(verification) => {
            let mut active_model: verification::ActiveModel = verification.into();
            active_model.expires = Set((chrono::Utc::now() + chrono::Duration::seconds(60 * 60)).into());
            active_model.update(db).await?
        }
        None => {
            let model = verification::ActiveModel {
                id: Set(Uuid::now_v7()),
                user_id: Set(user.id),
                r#type: Set(VerificationType::ForgotPassword),
                expires: Set((chrono::Utc::now() + chrono::Duration::seconds(60 * 60)).into()),
            };
            model.insert(db).await?
        }
    };

    let token = create_forgot_password_token(CreateForgotPasswordTokenArgs {
        sub: verification.id,
        exp: verification.expires.timestamp() as usize,
        user_id: user.id,
        ..Default::default()
    })?;
    Ok(Json(InitiateForgotPasswordResponse {
        ok: true,
        token,
        expires_at: verification.expires.timestamp() as usize,
    }))
}

pub async fn change_password(
    db: &DatabaseConnection,
    realm_id: Uuid,
    user_id: Uuid,
    payload: ForgotPasswordRequest,
) -> Result<Json<ForgotPasswordResponse>, Error> {
    let token_data = validate_forgot_password_token(VerifyForgotPasswordTokenArgs {
        token: payload.token,
        ..Default::default()
    })?;

    if token_data.claims.user_id != user_id {
        return Err(Error::Authenticate(AuthenticateError::InvalidToken));
    }

    let user = user::Entity::find_by_id(user_id).one(db).await?;
    if user.is_none() {
        return Err(Error::NotFound(NotFoundError::UserNotFound));
    }

    let user = user.unwrap();
    if user.locked_at.is_some() {
        return Err(Error::Authenticate(AuthenticateError::Locked));
    }

    if user.realm_id != realm_id {
        return Err(Error::Authenticate(AuthenticateError::ActionForbidden));
    }

    if payload.password != payload.password_confirmation {
        return Err(Error::cannot_perform_operation("Passwords do not match"));
    }

    let verification_data = verification::Entity::find_by_id(token_data.claims.sub).one(db).await?;
    if verification_data.is_none() {
        return Err(Error::Authenticate(AuthenticateError::InvalidToken));
    }

    let verification_data = verification_data.unwrap();
    if verification_data.expires.timestamp() <= chrono::Local::now().timestamp() {
        return Err(Error::Authenticate(AuthenticateError::InvalidToken));
    }

    if verification_data.r#type != VerificationType::ForgotPassword {
        return Err(Error::Authenticate(AuthenticateError::InvalidToken));
    }

    let mut user: user::ActiveModel = user.into();
    user.password_hash = Set(Some(generate_password_hash(payload.password).await?));
    user.update(db).await?;

    Ok(Json(ForgotPasswordResponse { ok: true }))
}
