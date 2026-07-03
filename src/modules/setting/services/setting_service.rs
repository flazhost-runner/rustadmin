//! Setting service — singleton load/update with global cache refresh + HTML sanitization.
//! Mirrors NodeAdmin SettingService + settingCache (invalidate-on-update).

use async_trait::async_trait;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel};
use serde_json::json;
use uuid::Uuid;

use crate::config::fe_templates::DEFAULT_FE_TEMPLATE;
use crate::config::themes::DEFAULT_THEME;
use crate::errors::AppResult;
use crate::modules::setting::models::setting;

#[derive(Debug, Default, Clone)]
pub struct SettingInput {
    pub initial: Option<String>,
    pub name: Option<String>,
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
}

#[async_trait]
pub trait ISettingService: Send + Sync {
    /// Load the singleton (creating defaults if missing) and refresh the global cache.
    async fn get(&self, db: &sea_orm::DatabaseConnection) -> AppResult<setting::Model>;
    /// Update the singleton (sanitizing rich-text `description`) and refresh the cache.
    async fn update(&self, db: &sea_orm::DatabaseConnection, input: SettingInput) -> AppResult<()>;
}

pub struct SettingService;

impl SettingService {
    fn recache(model: &setting::Model) {
        let theme = model
            .theme
            .clone()
            .unwrap_or_else(|| DEFAULT_THEME.to_string());
        let value = serde_json::to_value(model).unwrap_or_else(|_| json!({}));
        crate::site::set(theme, value);
    }
}

#[async_trait]
impl ISettingService for SettingService {
    async fn get(&self, db: &sea_orm::DatabaseConnection) -> AppResult<setting::Model> {
        if let Some(existing) = setting::Entity::find().one(db).await? {
            Self::recache(&existing);
            return Ok(existing);
        }
        // create a default singleton
        let model = setting::ActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            name: Set(Some("RustAdmin".into())),
            theme: Set(Some(DEFAULT_THEME.into())),
            fe_template: Set(Some(DEFAULT_FE_TEMPLATE.into())),
            ..Default::default()
        }
        .insert(db)
        .await?;
        Self::recache(&model);
        Ok(model)
    }

    async fn update(&self, db: &sea_orm::DatabaseConnection, input: SettingInput) -> AppResult<()> {
        let current = self.get(db).await?;
        let mut am = current.into_active_model();
        am.initial = Set(input.initial);
        am.name = Set(input.name);
        // sanitize rich-text HTML on save (rendered raw in the landing)
        am.description = Set(input.description.map(|d| ammonia::clean(&d)));
        am.icon = Set(input.icon);
        am.logo = Set(input.logo);
        am.login_image = Set(input.login_image);
        am.phone = Set(input.phone);
        am.address = Set(input.address);
        am.email = Set(input.email);
        am.copyright = Set(input.copyright);
        if let Some(theme) = input.theme {
            am.theme = Set(Some(theme));
        }
        if let Some(fe) = input.fe_template {
            am.fe_template = Set(Some(fe));
        }
        let saved = am.update(db).await?;
        Self::recache(&saved);
        Ok(())
    }
}
