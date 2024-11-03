use crate::{models::resource, utils::check_locked_at_constraint};
use async_trait::async_trait;
use sea_orm::{entity::prelude::*, ActiveValue};

#[async_trait]
impl ActiveModelBehavior for resource::ActiveModel {
    /// Will be triggered before insert / update
    async fn before_save<C>(mut self, db: &C, _insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        if let ActiveValue::Set(ref locked_at) = self.locked_at {
            check_locked_at_constraint(locked_at)?
        }

        if let ActiveValue::Set(is_default) = self.is_default {
            if is_default.unwrap() {
                // Set all other resource groups for the same user and client to non-default
                resource::Entity::update_many()
                    .col_expr(resource::Column::IsDefault, Expr::value(false))
                    .filter(resource::Column::UserId.eq(self.user_id.clone().unwrap()))
                    .filter(resource::Column::ClientId.eq(self.client_id.clone().unwrap()))
                    .filter(resource::Column::GroupKey.ne(self.group_key.clone().unwrap()))
                    .exec(db)
                    .await?;
            } else {
                // Check if this was the only default group
                let default_exists = resource::Entity::find()
                    .filter(resource::Column::UserId.eq(self.user_id.clone().unwrap()))
                    .filter(resource::Column::ClientId.eq(self.client_id.clone().unwrap()))
                    .filter(resource::Column::IsDefault.eq(true))
                    .filter(resource::Column::GroupKey.ne(self.group_key.clone().unwrap()))
                    .one(db)
                    .await?
                    .is_some();

                // If no other default exists, force this group to be default
                if !default_exists {
                    self.is_default = ActiveValue::Set(Some(true));
                }
            }
        }

        Ok(self)
    }
}
