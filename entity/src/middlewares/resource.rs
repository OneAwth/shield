use crate::{models::resource, utils::check_locked_at_constraint};
use async_trait::async_trait;
use sea_orm::{entity::prelude::*, ActiveValue};

#[async_trait]
impl ActiveModelBehavior for resource::ActiveModel {
    /// Will be triggered before insert / update
    async fn before_save<C>(mut self, db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        if let ActiveValue::Set(ref locked_at) = self.locked_at {
            check_locked_at_constraint(locked_at)?;
        }

        if let ActiveValue::Set(is_default) = &self.is_default {
            let user_id = self.user_id.clone().unwrap();
            let client_id = self.client_id.clone().unwrap();
            let group_key = self.group_key.clone().unwrap();

            // Count existing default groups excluding current group
            let default_count = resource::Entity::find()
                .filter(resource::Column::UserId.eq(user_id))
                .filter(resource::Column::ClientId.eq(client_id))
                .filter(resource::Column::IsDefault.eq(true))
                .filter(resource::Column::GroupKey.ne(group_key))
                .count(db)
                .await?;

            match (is_default, insert, default_count) {
                // Case 1: Setting to default true
                (Some(true), _, _) => {
                    // Set all other groups to non-default
                    resource::Entity::update_many()
                        .col_expr(resource::Column::IsDefault, Expr::value(false))
                        .filter(resource::Column::UserId.eq(user_id))
                        .filter(resource::Column::ClientId.eq(client_id))
                        .filter(resource::Column::GroupKey.ne(group_key))
                        .exec(db)
                        .await?;
                }
                // Case 2: Setting to default false/null during insert
                (Some(false) | None, true, 0) => {
                    // Force default to true if this is the first group
                    self.is_default = ActiveValue::Set(Some(true));
                }
                // Case 3: Setting to default false/null during update
                (Some(false) | None, false, 0) => {
                    // Check if this was the default group
                    let was_default = resource::Entity::find()
                        .filter(resource::Column::UserId.eq(user_id))
                        .filter(resource::Column::ClientId.eq(client_id))
                        .filter(resource::Column::GroupKey.eq(group_key))
                        .one(db)
                        .await?
                        .map(|m| m.is_default)
                        .unwrap_or(None)
                        .unwrap_or(false);

                    if was_default {
                        // Cannot remove default status if this was the only default
                        return Err(DbErr::Custom("Cannot remove default status from the only default group".to_string()));
                    }
                }
                // Case 4: All other cases are fine
                _ => {}
            }
        }

        Ok(self)
    }
}
