//! Profile web controller — view + update own profile (authenticated).

use std::sync::Arc;

use rocket::form::Form;
use rocket::fs::TempFile;
use rocket::http::CookieJar;
use rocket::request::FlashMessage;
use rocket::response::{Flash, Redirect};
use rocket::tokio::io::AsyncReadExt;
use rocket::State;
use rocket_dyn_templates::Template;
use sea_orm::DatabaseConnection;
use serde_json::json;

use crate::config::storage;
use crate::errors::{AppError, AppResult};
use crate::guards::CurrentUser;
use crate::helpers::view::render_view;
use crate::modules::access::controllers::web::{chrome, merge};
use crate::modules::profile::service::{IProfileService, ProfileInput};
use crate::security::csrf::{ensure_token, CsrfProtected};

const INDEX_URL: &str = "/admin/v1/profile";

#[derive(rocket::FromForm, Debug, Default)]
pub struct ProfileForm<'r> {
    pub code: Option<String>,
    pub name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub timezone: Option<String>,
    pub status: Option<String>,
    pub password: Option<String>,
    pub password_confirmation: Option<String>,
    /// Optional avatar upload (multipart `picture`). Empty when no file is chosen.
    pub picture: Option<TempFile<'r>>,
}

/// Max avatar size — matches the fleet 2 MB upload standard (and the Rocket `file` limit).
const MAX_PICTURE_SIZE: usize = 2 * 1024 * 1024;

/// Read + validate an uploaded avatar and persist it to storage under `user/<id>.<ext>`.
/// Returns the stored object key, or `None` when no file was submitted. Mirrors the media
/// module: magic-byte image validation + driver-agnostic `storage::put`.
async fn store_picture(user_id: &str, file: Option<TempFile<'_>>) -> AppResult<Option<String>> {
    let Some(file) = file else { return Ok(None) };
    if file.len() == 0 {
        return Ok(None);
    }
    let mut reader = file
        .open()
        .await
        .map_err(|e| AppError::internal(format!("read upload: {e}")))?;
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .await
        .map_err(|e| AppError::internal(format!("read upload: {e}")))?;
    if bytes.len() > MAX_PICTURE_SIZE {
        return Err(AppError::bad_request("File size exceeds 2MB limit."));
    }
    let kind = infer::get(&bytes)
        .filter(|k| {
            k.matcher_type() == infer::MatcherType::Image
                && matches!(k.extension(), "jpg" | "jpeg" | "png" | "gif" | "webp")
        })
        .ok_or_else(|| AppError::bad_request("Unsupported or invalid image file"))?;
    let key = format!("user/{user_id}.{}", kind.extension());
    storage::put(&key, &bytes).await?;
    Ok(Some(key))
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
    form: Form<ProfileForm<'_>>,
) -> Flash<Redirect> {
    let mut f = form.into_inner();
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
    let picture = match store_picture(&user.id, f.picture.take()).await {
        Ok(p) => p,
        Err(e) => return Flash::error(Redirect::to(INDEX_URL), e.message().to_string()),
    };
    let input = ProfileInput {
        code: f.code.filter(|c| !c.trim().is_empty()),
        name,
        email,
        phone: f.phone.filter(|p| !p.trim().is_empty()),
        timezone: f.timezone.filter(|t| !t.trim().is_empty()),
        status: f.status.filter(|s| !s.trim().is_empty()),
        password: if pw.is_empty() { None } else { Some(pw) },
        picture,
    };
    match svc.update(db.inner(), &user.id, input).await {
        Ok(_) => Flash::success(Redirect::to(INDEX_URL), "Update Profile Success."),
        Err(e) => Flash::error(Redirect::to(INDEX_URL), e.message().to_string()),
    }
}

pub fn routes() -> Vec<rocket::Route> {
    routes![index, update]
}
