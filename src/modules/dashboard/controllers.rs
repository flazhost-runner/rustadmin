//! Dashboard web controller — counts + render. Read-only; authenticated (CurrentUser).

use rocket::http::CookieJar;
use rocket::State;
use rocket_dyn_templates::Template;
use sea_orm::{DatabaseConnection, EntityTrait, PaginatorTrait};
use serde_json::json;

use crate::errors::AppError;
use crate::guards::CurrentUser;
use crate::helpers::view::render_view;
use crate::modules::access::controllers::web::{chrome, merge};
use crate::modules::access::models::{permission, role, user};
use crate::security::csrf::ensure_token;

#[get("/dashboard")]
pub async fn index(
    user: CurrentUser,
    db: &State<DatabaseConnection>,
    cookies: &CookieJar<'_>,
) -> Result<Template, AppError> {
    let users = user::Entity::find().count(db.inner()).await?;
    let roles = role::Entity::find().count(db.inner()).await?;
    let permissions = permission::Entity::find().count(db.inner()).await?;
    let csrf = ensure_token(cookies);

    let now = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
    let mut page = json!({
        "stats": { "users": users, "roles": roles, "permissions": permissions },
        "now": now,
    });
    merge(&mut page, chrome(&user, &csrf, "dashboard"));
    Ok(render_view("be/default/dashboard/index", page, None))
}

pub fn routes() -> Vec<rocket::Route> {
    routes![index]
}
