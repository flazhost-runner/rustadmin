//! Setting web controller — theme switcher + site config form (singleton).

use std::sync::Arc;

use rocket::form::Form;
use rocket::http::CookieJar;
use rocket::request::FlashMessage;
use rocket::response::content::RawHtml;
use rocket::response::{Flash, Redirect};
use rocket::State;
use rocket_dyn_templates::Template;
use sea_orm::DatabaseConnection;
use serde_json::{json, Value};

use crate::config::fe_templates::DEFAULT_FE_TEMPLATE;
use crate::errors::AppError;
use crate::guards::Authorized;
use crate::helpers::view::{nav_for, render_view};
use crate::modules::home::services::IFeCatalogService;
use crate::modules::setting::services::{ISettingService, SettingInput};
use crate::security::csrf::{ensure_token, CsrfProtected};

const INDEX_URL: &str = "/admin/v1/setting";

fn chrome(user: &crate::guards::CurrentUser, csrf: &str) -> Value {
    json!({
        "auth": { "name": user.name, "picture": null },
        "nav": nav_for(user.is_admin, &user.perms),
        "csrf_token": csrf,
        "active": "setting",
    })
}

#[derive(rocket::FromForm, Debug, Default)]
pub struct SettingForm {
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

impl From<SettingForm> for SettingInput {
    fn from(f: SettingForm) -> Self {
        SettingInput {
            initial: f.initial,
            name: f.name,
            description: f.description,
            icon: f.icon,
            logo: f.logo,
            login_image: f.login_image,
            phone: f.phone,
            address: f.address,
            email: f.email,
            copyright: f.copyright,
            theme: f.theme,
            fe_template: f.fe_template,
        }
    }
}

#[get("/setting?<fe_page>&<fe_search>&<fe_category>")]
#[allow(clippy::too_many_arguments)]
pub async fn index(
    auth: Authorized,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn ISettingService>>,
    catalog: &State<Arc<dyn IFeCatalogService>>,
    cookies: &CookieJar<'_>,
    flash: Option<FlashMessage<'_>>,
    fe_page: Option<u64>,
    fe_search: Option<String>,
    fe_category: Option<String>,
) -> Result<Template, AppError> {
    let setting = svc.get(db.inner()).await?;
    let csrf = ensure_token(cookies);
    let flash_v = match &flash {
        Some(m) => json!({ "key": m.kind(), "message": m.message() }),
        None => json!({}),
    };
    let active = setting
        .fe_template
        .clone()
        .unwrap_or_else(|| DEFAULT_FE_TEMPLATE.to_string());
    let cat = catalog
        .paginate(
            fe_search.as_deref(),
            fe_category.as_deref(),
            fe_page,
            &active,
        )
        .await;
    let fe_categories = catalog.categories().await;
    let mut page = json!({
        "setting": setting,
        "flash": flash_v,
        "fe_catalog": cat.rows,
        "fe_meta": cat.meta,
        "fe_pages": cat.pages,
        "fe_active": active,
        "fe_categories": fe_categories,
        "filter": {
            "q_name": fe_search.clone().unwrap_or_default(),
            "q_category": fe_category.clone().unwrap_or_default(),
        },
        "fe_search": fe_search.unwrap_or_default(),
        "fe_category": fe_category.unwrap_or_default(),
    });
    // merge chrome
    if let (Value::Object(b), Value::Object(c)) = (&mut page, chrome(&auth.0, &csrf)) {
        for (k, v) in c {
            b.insert(k, v);
        }
    }
    Ok(render_view("be/default/setting/index", page, None))
}

#[put("/setting/update", data = "<form>")]
pub async fn update(
    _auth: Authorized,
    _csrf: CsrfProtected,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn ISettingService>>,
    catalog: &State<Arc<dyn IFeCatalogService>>,
    form: Form<SettingForm>,
) -> Flash<Redirect> {
    let input: SettingInput = form.into_inner().into();
    let fe = input.fe_template.clone();
    match svc.update(db.inner(), input).await {
        Ok(_) => {
            // Download + cache the chosen frontend template on Save (PORTING_GUIDE: unduh saat Save).
            if let Some(slug) = fe {
                let _ = catalog.ensure(&slug).await;
            }
            Flash::success(Redirect::to(INDEX_URL), "Save Setting Success.")
        }
        Err(e) => Flash::error(Redirect::to(INDEX_URL), e.message().to_string()),
    }
}

/// Proxy a frontend-template preview (anti-SSRF; namespace `setting`, NOT a separate module).
#[get("/setting/fe-preview/<slug>")]
pub async fn fe_preview(
    _auth: Authorized,
    catalog: &State<Arc<dyn IFeCatalogService>>,
    slug: &str,
) -> Result<RawHtml<String>, AppError> {
    Ok(RawHtml(catalog.preview_html(slug).await?))
}

pub fn routes() -> Vec<rocket::Route> {
    routes![index, update, fe_preview]
}
