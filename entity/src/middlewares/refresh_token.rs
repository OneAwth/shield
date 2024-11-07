use crate::{models::refresh_token, utils::check_locked_at_constraint};
use async_trait::async_trait;
use sea_orm::{entity::prelude::*, sqlx::types::chrono::Utc, ActiveValue};

#[async_trait]
impl ActiveModelBehavior for refresh_token::ActiveModel {
    /// Will be triggered before insert / update
    async fn before_save<C>(mut self, _db: &C, _insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        if let ActiveValue::Set(ref locked_at) = self.locked_at {
            check_locked_at_constraint(locked_at)?
        }

        if let ActiveValue::Set(ref expires) = self.expires {
            if expires < &Utc::now().fixed_offset() {
                return Err(DbErr::Custom("Expires must be greater than created_at".to_owned()));
            }
        }

        Ok(self)
    }
}
