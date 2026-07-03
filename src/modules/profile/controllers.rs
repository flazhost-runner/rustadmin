//! Profile web controller — view + update own profile (authenticated).

use std::sync::Arc;

use rocket::form::Form;
use rocket::http::CookieJar;
use rocket::request::FlashMessage;
use rocket::response::{Flash, Redirect};
use rocket::State;
use rocket_dyn_templates::Template;
use sea_orm::DatabaseConnection;
use serde_json::json;

use crate::errors::AppError;
use crate::guards::CurrentUser;
use crate::helpers::view::render_view;
use crate::modules::access::controllers::web::{chrome, merge};
use crate::modules::profile::service::{IProfileService, ProfileInput};
use crate::security::csrf::{ensure_token, CsrfProtected};

const INDEX_URL: &str = "/admin/v1/profile";

#[derive(rocket::FromForm, Debug, Default)]
pub struct ProfileForm {
    pub code: Option<String>,
    pub name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub timezone: Option<String>,
    pub status: Option<String>,
    pub password: Option<String>,
    pub password_confirmation: Option<String>,
}

/// Timezone options for the profile form (matches the user form).
fn timezones() -> Vec<&'static str> {
    vec![
        "UTC",
        "Asia/Jakarta",
        "Asia/Singapore",
        "Asia/Tokyo",
        "Europe/London",
        "Europe/Paris",
        "America/New_York",
        "America/Los_Angeles",
    ]
}

#[get("/profile")]
pub async fn index(
    user: CurrentUser,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IProfileService>>,
    cookies: &CookieJar<'_>,
    flash: Option<FlashMessage<'_>>,
) -> Result<Template, AppError> {
    let data = svc.get(db.inner(), &user.id).await?;
    let csrf = ensure_token(cookies);
    let flash_v = match &flash {
        Some(m) => json!({ "key": m.kind(), "message": m.message() }),
        None => json!({}),
    };
    let mut page = json!({ "data": data, "flash": flash_v, "timezones": timezones() });
    merge(&mut page, chrome(&user, &csrf, ""));
    Ok(render_view("be/default/profile/profile", page, None))
}

#[put("/profile/update", data = "<form>")]
pub async fn update(
    user: CurrentUser,
    _csrf: CsrfProtected,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IProfileService>>,
    form: Form<ProfileForm>,
) -> Flash<Redirect> {
    let f = form.into_inner();
    let name = f.name.as_deref().unwrap_or("").trim().to_string();
    let email = f.email.as_deref().unwrap_or("").trim().to_string();
    if name.is_empty() || email.is_empty() {
        return Flash::error(Redirect::to(INDEX_URL), "Name and email are required");
    }
    let pw = f.password.as_deref().unwrap_or("").trim().to_string();
    if !pw.is_empty() && pw != f.password_confirmation.as_deref().unwrap_or("").trim() {
        return Flash::error(
            Redirect::to(INDEX_URL),
            "Password confirmation does not match",
        );
    }
    let input = ProfileInput {
        code: f.code.filter(|c| !c.trim().is_empty()),
        name,
        email,
        phone: f.phone.filter(|p| !p.trim().is_empty()),
        timezone: f.timezone.filter(|t| !t.trim().is_empty()),
        status: f.status.filter(|s| !s.trim().is_empty()),
        password: if pw.is_empty() { None } else { Some(pw) },
    };
    match svc.update(db.inner(), &user.id, input).await {
        Ok(_) => Flash::success(Redirect::to(INDEX_URL), "Update Profile Success."),
        Err(e) => Flash::error(Redirect::to(INDEX_URL), e.message().to_string()),
    }
}

pub fn routes() -> Vec<rocket::Route> {
    routes![index, update]
}
