//! Profile service — load/update the current user's own profile (roles untouched).

use async_trait::async_trait;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, IntoActiveModel};

use crate::errors::{AppError, AppResult};
use crate::modules::access::models::user;

#[derive(Debug, Default, Clone)]
pub struct ProfileInput {
    pub code: Option<String>,
    pub name: String,
    pub email: String,
    pub phone: Option<String>,
    pub timezone: Option<String>,
    pub status: Option<String>,
    /// Empty/None = keep existing password.
    pub password: Option<String>,
    /// Storage key of a newly-uploaded avatar (e.g. `user/<id>.png`); None = keep existing.
    pub picture: Option<String>,
}

#[async_trait]
pub trait IProfileService: Send + Sync {
    async fn get(&self, db: &DatabaseConnection, user_id: &str) -> AppResult<user::Model>;
    async fn update(
        &self,
        db: &DatabaseConnection,
        user_id: &str,
        input: ProfileInput,
    ) -> AppResult<()>;
}

pub struct ProfileService;

#[async_trait]
impl IProfileService for ProfileService {
    async fn get(&self, db: &DatabaseConnection, user_id: &str) -> AppResult<user::Model> {
        user::Entity::find_by_id(user_id.to_string())
            .one(db)
            .await?
            .ok_or_else(|| AppError::not_found("User not found"))
    }

    async fn update(
        &self,
        db: &DatabaseConnection,
        user_id: &str,
        input: ProfileInput,
    ) -> AppResult<()> {
        let existing = self.get(db, user_id).await?;
        let mut am = existing.into_active_model();
        if let Some(code) = input.code.filter(|c| !c.trim().is_empty()) {
            am.code = Set(code);
        }
        am.name = Set(input.name);
        am.email = Set(input.email);
        am.phone = Set(input.phone);
        if let Some(tz) = input.timezone {
            am.timezone = Set(Some(tz));
        }
        if let Some(status) = input.status.filter(|s| !s.trim().is_empty()) {
            am.status = Set(status);
        }
        if let Some(pw) = input.password.filter(|p| !p.is_empty()) {
            am.password = Set(bcrypt::hash(&pw, 10)?);
        }
        if let Some(picture) = input.picture.filter(|p| !p.trim().is_empty()) {
            am.picture = Set(Some(picture));
        }
        am.update(db).await?;
        Ok(())
    }
}
