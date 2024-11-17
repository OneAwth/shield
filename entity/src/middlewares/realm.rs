use crate::{
    client,
    models::{client::Entity as Client, realm::ActiveModel},
    utils::check_locked_at_constraint,
};
use async_trait::async_trait;
use sea_orm::{
    entity::prelude::*,
    sqlx::types::chrono::{DateTime, FixedOffset},
    ActiveValue, EntityTrait, QueryFilter,
};
use slug::slugify;

#[async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(mut self, db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        if let ActiveValue::Set(ref locked_at) = self.locked_at {
            check_locked_at_constraint(locked_at)?;
            if !insert {
                if let ActiveValue::Set(realm_id) = self.id {
                    update_clients_lock_status(db, realm_id, locked_at).await?;
                }
            }
        }

        if let ActiveValue::Set(is_account_activation_required) = self.is_account_activation_required {
            if !insert {
                if let ActiveValue::Set(realm_id) = self.id {
                    update_clients_is_account_activation_required_status(db, realm_id, is_account_activation_required).await?;
                }
            }
        }

        if let ActiveValue::Set(ref name) = self.name {
            let slug = slugify(name);
            self.slug = ActiveValue::Set(slug);
        }

        Ok(self)
    }
}

async fn update_clients_lock_status<C>(db: &C, realm_id: Uuid, locked_at: &Option<DateTime<FixedOffset>>) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    if locked_at.is_some() {
        Client::update_many()
            .filter(client::Column::RealmId.eq(realm_id))
            .filter(client::Column::LockedAt.is_null())
            .set(client::ActiveModel {
                locked_at: ActiveValue::Set(*locked_at),
                ..Default::default()
            })
            .exec(db)
            .await?;
    }
    Ok(())
}

async fn update_clients_is_account_activation_required_status<C>(db: &C, realm_id: Uuid, is_account_activation_required: bool) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    if is_account_activation_required {
        Client::update_many()
            .filter(client::Column::RealmId.eq(realm_id))
            .filter(client::Column::IsAccountActivationRequired.eq(false))
            .set(client::ActiveModel {
                is_account_activation_required: ActiveValue::Set(is_account_activation_required),
                ..Default::default()
            })
            .exec(db)
            .await?;
    }

    Ok(())
}
