//! Components showcase controller — renders the live component reference (RBAC-gated).

use rocket::http::CookieJar;
use rocket_dyn_templates::Template;
use serde_json::json;

use crate::errors::AppError;
use crate::guards::Authorized;
use crate::helpers::view::render_view;
use crate::modules::access::controllers::web::{chrome, merge};
use crate::security::csrf::ensure_token;

#[get("/components")]
pub async fn index(auth: Authorized, cookies: &CookieJar<'_>) -> Result<Template, AppError> {
    let csrf = ensure_token(cookies);
    let mut page = json!({});
    merge(&mut page, chrome(&auth.0, &csrf, "components"));
    Ok(render_view("be/default/components/index", page, None))
}

pub fn routes() -> Vec<rocket::Route> {
    routes![index]
}
