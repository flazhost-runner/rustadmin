//! Public landing controller. `/` renders the home directly (no redirect); `/home` is an
//! explicit alias. The active template decides rendering: the pinned default → the native
//! rich `fe/default` view (bound to Setting); any other slug → its proxied HTML.

use std::sync::Arc;

use rocket::response::content::RawHtml;
use rocket::State;
use rocket_dyn_templates::Template;
use serde_json::json;

use crate::config::fe_templates::DEFAULT_FE_TEMPLATE;
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

/// Landing view-model bound from the cached Setting with safe fallbacks
/// (parity: NodeAdmin/GoAdmin `HomeService.Landing`) — the view stays flat.
fn landing_ctx() -> serde_json::Value {
    let setting = crate::site::setting().unwrap_or(serde_json::Value::Null);
    let get = |key: &str| {
        setting
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    let or = |value: String, default: &str| {
        if value.is_empty() {
            default.to_string()
        } else {
            value
        }
    };
    json!({
        "app_name": or(get("name"), "RustAdmin"),
        "description": get("description"),
        "logo": get("logo"),
        "email": get("email"),
        "phone": get("phone"),
        "address": get("address"),
        "copyright": or(get("copyright"), "© RustAdmin"),
    })
}

fn native_landing() -> Landing {
    Landing::Native(render_view(
        "fe/default/index",
        json!({ "landing": landing_ctx() }),
        None,
    ))
}

async fn render_landing(catalog: &Arc<dyn IFeCatalogService>) -> Landing {
    let slug = active_slug();
    // default → native bundled v6 landing (render_view injects the cached `setting`);
    // any other slug → its real downloaded HTML; a failed download falls back to the
    // native view so the landing never errors (parity GoAdmin HomeController).
    match catalog.active_html(&slug).await {
        Ok(Some(html)) => Landing::Raw(RawHtml(html)),
        Ok(None) | Err(_) => native_landing(),
    }
}

#[get("/")]
pub async fn root(catalog: &State<Arc<dyn IFeCatalogService>>) -> Landing {
    render_landing(catalog.inner()).await
}

#[get("/home")]
pub async fn index(catalog: &State<Arc<dyn IFeCatalogService>>) -> Landing {
    render_landing(catalog.inner()).await
}

pub fn routes() -> Vec<rocket::Route> {
    routes![root, index]
}
