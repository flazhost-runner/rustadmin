//! Public landing controller. `/` renders the home directly (no redirect); `/home` is an
//! explicit alias. The active template decides rendering: the pinned default → the native
//! rich `fe/default` view (bound to Setting); any other slug → its proxied HTML.

use std::sync::Arc;

use rocket::response::content::RawHtml;
use rocket::State;
use rocket_dyn_templates::Template;
use serde_json::json;

use crate::config::fe_templates::DEFAULT_FE_TEMPLATE;
use crate::errors::AppError;
use crate::helpers::view::render_view;
use crate::modules::home::services::IFeCatalogService;

/// Either the native landing template or proxied template HTML.
#[derive(Responder)]
#[allow(clippy::large_enum_variant)]
pub enum Landing {
    Native(Template),
    Raw(RawHtml<String>),
}

fn active_slug() -> String {
    crate::site::setting()
        .and_then(|s| {
            s.get("fe_template")
                .and_then(|v| v.as_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| DEFAULT_FE_TEMPLATE.to_string())
}

async fn render_landing(catalog: &Arc<dyn IFeCatalogService>) -> Result<Landing, AppError> {
    let slug = active_slug();
    // default → native rich landing (render_view injects the cached `setting`);
    // any other slug → its real downloaded HTML.
    match catalog.active_html(&slug).await? {
        None => Ok(Landing::Native(render_view(
            "fe/default/index",
            json!({}),
            None,
        ))),
        Some(html) => Ok(Landing::Raw(RawHtml(html))),
    }
}

#[get("/")]
pub async fn root(catalog: &State<Arc<dyn IFeCatalogService>>) -> Result<Landing, AppError> {
    render_landing(catalog.inner()).await
}

#[get("/home")]
pub async fn index(catalog: &State<Arc<dyn IFeCatalogService>>) -> Result<Landing, AppError> {
    render_landing(catalog.inner()).await
}

pub fn routes() -> Vec<rocket::Route> {
    routes![root, index]
}
