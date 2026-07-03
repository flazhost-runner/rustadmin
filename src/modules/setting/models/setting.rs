//! `settings` singleton entity. `description` is rich text (HTML, sanitized on save).

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "settings")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub initial: Option<String>,
    pub name: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub description: Option<String>,
    pub icon: Option<String>,
    pub logo: Option<String>,
    pub login_image: Option<String>,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub email: Option<String>,
    pub copyright: Option<String>,
    pub theme: Option<String>,
    pub fe_template: Option<String>,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
