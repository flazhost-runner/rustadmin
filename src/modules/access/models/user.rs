//! `users` entity. `id` is a varchar(36) UUID string (no auto-increment).

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    #[sea_orm(unique)]
    pub code: String,
    pub name: String,
    pub phone: Option<String>,
    #[sea_orm(unique)]
    pub email: String,
    pub email_verified_at: Option<DateTime>,
    pub password: String,
    pub password_otp: Option<String>,
    pub password_otp_expires: Option<i64>,
    pub status: String,
    pub picture: Option<String>,
    pub blocked: bool,
    pub blocked_reason: Option<String>,
    pub timezone: Option<String>,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
