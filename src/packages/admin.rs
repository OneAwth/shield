use chrono::{Duration, Utc};
use entity::{
    api_user, client, group, realm, resource,
    sea_orm_active_enums::{ApiUserAccess, ApiUserRole, GroupAccess, GroupRole},
    user,
};
use futures::future;
use sea_orm::{
    prelude::Uuid, ActiveModelTrait, ColumnTrait, DatabaseConnection, DatabaseTransaction, EntityTrait, QueryFilter, Set, TransactionError,
    TransactionTrait,
};
use std::{
    fs::{create_dir_all, File},
    io::Write,
    path::Path,
};

use tracing::info;

use crate::{
    packages::settings::{Settings, SETTINGS},
    utils::{hash::generate_password_hash, helpers::default_cred::DefaultCred},
};

use super::{api_token::ApiUser, db::AppState, errors::Error};

pub async fn setup(state: &AppState) -> Result<bool, TransactionError<Error>> {
    info!("Checking ADMIN availability!");
    let admin_email = SETTINGS.read().admin.email.clone();
    let is_admin_user_exists = user::Entity::find().filter(user::Column::Email.eq(admin_email)).one(&state.db).await?;

    if is_admin_user_exists.is_some() {
        info!("DB has been already initialized!");
        info!("Starting the server...");
        Ok(false)
    } else {
        info!("DB has not been initialized!");
        info!("⌛ Initializing the DB...");

        initialize_db(&state.db).await?;
        info!("Admin initialization complete.");
        Settings::reload().expect("Failed to reload settings");
        Ok(true)
    }
}

async fn initialize_db(conn: &DatabaseConnection) -> Result<(), TransactionError<Error>> {
    conn.transaction(|txn| {
        Box::pin(async move {
            let realm = create_master_realm(txn).await?;
            let client = create_default_client(txn, realm.id).await?;
            let user = create_admin_user(txn, realm.id).await?;
            let api_user = create_api_user(txn, realm.id, client.id).await?;
            let resource_assignment_result = assign_resource_to_admin(txn, realm.id, client.id, user.id).await?;

            let default_cred = DefaultCred {
                realm_id: realm.id,
                client_id: client.id,
                master_admin_user_id: user.id,
                master_api_key: ApiUser::create_token(api_user, &SETTINGS.read().secrets.api_key_signing_secret).expect("Failed to create api key"),
                resource_group_key: resource_assignment_result.group_key,
                group_id: resource_assignment_result.group_id,
                resource_ids: resource_assignment_result.resource_ids,
            };

            write_default_cred(default_cred)?;
            Ok(())
        })
    })
    .await
}

async fn create_master_realm(conn: &DatabaseTransaction) -> Result<realm::Model, Error> {
    let realm_model = realm::ActiveModel {
        id: Set(Uuid::now_v7()),
        name: Set("Master".to_owned()),
        ..Default::default()
    };
    let inserted_realm = realm_model.insert(conn).await?;
    info!("✅ 1/6: Master realm created");

    Ok(inserted_realm)
}

async fn create_default_client(conn: &DatabaseTransaction, realm_id: Uuid) -> Result<client::Model, Error> {
    let client_model = client::ActiveModel {
        id: Set(Uuid::now_v7()),
        name: Set("client".to_owned()),
        realm_id: Set(realm_id),
        ..Default::default()
    };
    let inserted_client = client_model.insert(conn).await?;
    info!("✅ 2/6: Default client created");

    Ok(inserted_client)
}

async fn create_admin_user(conn: &DatabaseTransaction, realm_id: Uuid) -> Result<user::Model, Error> {
    let admin = SETTINGS.read().admin.clone();
    let pw_hash = generate_password_hash(admin.password).await?;
    let user_model = user::ActiveModel {
        id: Set(Uuid::now_v7()),
        email: Set(admin.email.to_owned()),
        password_hash: Set(Some(pw_hash)),
        realm_id: Set(realm_id),
        first_name: Set(admin.email.to_owned()),
        is_temp_password: Set(false),
        ..Default::default()
    };
    let inserted_user = user_model.insert(conn).await?;
    info!("✅ 3/6: Admin user created");

    Ok(inserted_user)
}

async fn create_api_user(conn: &DatabaseTransaction, realm_id: Uuid, client_id: Uuid) -> Result<api_user::Model, Error> {
    let api_user_model = api_user::ActiveModel {
        id: Set(Uuid::now_v7()),
        name: Set("master_realm_default_api_user".to_owned()),
        description: Set(Some("This api user has been created at the time of system initialization.".to_owned())),
        realm_id: Set(realm_id),
        client_id: Set(client_id),
        role: Set(ApiUserRole::RealmAdmin),
        access: Set(ApiUserAccess::Admin),
        expires: Set((Utc::now() + Duration::days(30)).into()),
        ..Default::default()
    };
    let inserted_api_user = api_user_model.insert(conn).await?;
    info!("✅ 4/6: Default api user created");

    Ok(inserted_api_user)
}

struct ResourceAssignmentResult {
    group_key: Uuid,
    group_id: Uuid,
    resource_ids: Vec<Uuid>,
}

async fn assign_resource_to_admin(
    conn: &DatabaseTransaction,
    realm_id: Uuid,
    client_id: Uuid,
    user_id: Uuid,
) -> Result<ResourceAssignmentResult, Error> {
    let group_model = group::ActiveModel {
        id: Set(Uuid::now_v7()),
        realm_id: Set(realm_id),
        client_id: Set(client_id),
        name: Set("default_group".to_owned()),
        role: Set(GroupRole::User),
        access: Set(GroupAccess::Admin),
        ..Default::default()
    };
    let group = group_model.insert(conn).await?;
    info!("✅ 5/6: Default group created");

    let group_key = Uuid::now_v7();
    let resource_model = resource::ActiveModel {
        id: Set(Uuid::now_v7()),
        user_id: Set(user_id),
        client_id: Set(client_id),
        group_key: Set(group_key),
        name: Set("role".to_owned()),
        value: Set("admin".to_owned()),
        description: Set(Some("This role has been created at the time of initialization.".to_owned())),
        ..Default::default()
    };

    let new_resource_2 = resource::ActiveModel {
        id: Set(Uuid::now_v7()),
        user_id: Set(user_id),
        client_id: Set(client_id),
        group_key: Set(group_key),
        name: Set("realm".to_owned()),
        value: Set(realm_id.to_string()),
        description: Set(Some("This role has been created at the time of initialization.".to_owned())),
        ..Default::default()
    };
    let (inserted_resource, inserted_resource_2) = future::try_join(resource_model.insert(conn), new_resource_2.insert(conn)).await?;
    info!("✅ 6/6: Default resource created");
    Ok(ResourceAssignmentResult {
        group_key,
        group_id: group.id,
        resource_ids: vec![inserted_resource.id, inserted_resource_2.id],
    })
}

fn write_default_cred(default_cred: DefaultCred) -> Result<(), Error> {
    info!("🗝️ Please note these credentials!");
    info!("{:#?}", default_cred);

    let file_path = "./logs/default_cred.json";
    let path = Path::new(file_path);
    if let Some(parent_dir) = path.parent() {
        create_dir_all(parent_dir)?;
    } else {
        panic!("Invalid file path");
    }

    let json = serde_json::to_string_pretty(&default_cred)?;
    let mut file = File::create(file_path)?;
    file.write_all(json.as_bytes())?;

    info!("📝 However above credentials have been '/logs/default_cred.json' file.");
    Ok(())
}
