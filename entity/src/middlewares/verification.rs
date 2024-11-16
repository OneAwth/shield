use crate::models::verification;
use async_trait::async_trait;
use sea_orm::{entity::prelude::*, sqlx::types::chrono::Utc};

#[async_trait]
impl ActiveModelBehavior for verification::ActiveModel {
    /// Will be triggered before insert / update
    async fn before_save<C>(mut self, db: &C, _insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        // Perform session cleanup: delete all sessions that have expired
        verification::Entity::delete_many()
            .filter(verification::Column::Expires.lt(Utc::now()))
            .exec(db)
            .await?;

        Ok(self)
    }
}
