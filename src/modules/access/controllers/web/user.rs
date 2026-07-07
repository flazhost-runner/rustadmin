//! User web controller (canonical index table + create/edit forms + method-override CRUD).

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
use serde_json::{json, Value};

use crate::errors::AppError;
use crate::guards::Authorized;
use crate::helpers::flash;
use crate::helpers::pagination::page_window;
use crate::helpers::view::render_view;
use crate::modules::access::services::user_service::{IUserService, UserFilter};
use crate::modules::access::validators::user::UserForm;
use crate::security::csrf::{ensure_token, CsrfProtected};

use super::{chrome, filter_query, merge};

const INDEX_URL: &str = "/admin/v1/access/user";
const CREATE_URL: &str = "/admin/v1/access/user/create";

/// Per-column filter query params (`q_*`).
#[derive(rocket::FromForm, Debug, Default)]
pub struct UserQuery {
    pub q_page: Option<u64>,
    pub q_page_size: Option<u64>,
    pub q_code: Option<String>,
    pub q_name: Option<String>,
    pub q_phone: Option<String>,
    pub q_email: Option<String>,
    pub q_status: Option<String>,
    pub q_role: Option<String>,
}

impl UserQuery {
    fn to_filter(&self) -> UserFilter {
        UserFilter {
            page: self.q_page,
            page_size: self.q_page_size,
            code: self.q_code.clone(),
            name: self.q_name.clone(),
            phone: self.q_phone.clone(),
            email: self.q_email.clone(),
            status: self.q_status.clone(),
            role: self.q_role.clone(),
        }
    }

    fn as_value(&self) -> Value {
        json!({
            "q_code": self.q_code.clone().unwrap_or_default(),
            "q_name": self.q_name.clone().unwrap_or_default(),
            "q_phone": self.q_phone.clone().unwrap_or_default(),
            "q_email": self.q_email.clone().unwrap_or_default(),
            "q_status": self.q_status.clone().unwrap_or_default(),
            "q_role": self.q_role.clone().unwrap_or_default(),
            "q_page_size": self.q_page_size.map(|v| v.to_string()).unwrap_or_default(),
        })
    }

    fn base_query(&self) -> String {
        let ps = self.q_page_size.map(|v| v.to_string());
        filter_query(&[
            ("q_page_size", ps.as_deref()),
            ("q_code", self.q_code.as_deref()),
            ("q_name", self.q_name.as_deref()),
            ("q_phone", self.q_phone.as_deref()),
            ("q_email", self.q_email.as_deref()),
            ("q_status", self.q_status.as_deref()),
            ("q_role", self.q_role.as_deref()),
        ])
    }
}

#[derive(rocket::FromForm)]
pub struct SelectionForm {
    pub selected: Vec<String>,
}

fn flash_value(flash: &Option<FlashMessage<'_>>) -> Value {
    match flash {
        Some(f) => json!({ "key": f.kind(), "message": f.message() }),
        None => json!({}),
    }
}

#[get("/access/user?<f..>")]
pub async fn index(
    auth: Authorized,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IUserService>>,
    cookies: &CookieJar<'_>,
    flash: Option<FlashMessage<'_>>,
    f: UserQuery,
) -> Result<Template, AppError> {
    let idx = svc.index(db.inner(), &f.to_filter()).await?;
    let csrf = ensure_token(cookies);
    let pages = page_window(idx.meta.page, idx.meta.total_pages);

    let mut page = json!({
        "datas": idx.rows,
        "meta": idx.meta,
        "pages": pages,
        "roles": idx.roles,
        "filter": f.as_value(),
        "base_query": f.base_query(),
        "flash": flash_value(&flash),
    });
    merge(&mut page, chrome(&auth.0, &csrf, "user"));
    Ok(render_view("be/default/access/users/index", page, None))
}

#[get("/access/user/create")]
pub async fn create(
    auth: Authorized,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IUserService>>,
    cookies: &CookieJar<'_>,
) -> Result<Template, AppError> {
    let roles = svc.all_roles(db.inner()).await?;
    let csrf = ensure_token(cookies);
    let (errors, old) = flash::take(cookies);

    let mut page = json!({
        "roles": roles,
        "errors": errors,
        "old": old,
        "timezones": timezones(),
    });
    merge(&mut page, chrome(&auth.0, &csrf, "user"));
    Ok(render_view("be/default/access/users/create", page, None))
}

#[post("/access/user/store", data = "<form>")]
pub async fn store(
    _auth: Authorized,
    _csrf: CsrfProtected,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IUserService>>,
    cookies: &CookieJar<'_>,
    form: Form<UserForm<'_>>,
) -> Flash<Redirect> {
    let mut f = form.into_inner();
    let picture = f.picture.take();
    match f.validate_store() {
        Err(fe) => {
            flash::stash(cookies, &fe.errors, &fe.old);
            Flash::error(Redirect::to(CREATE_URL), "Please fix the errors below")
        }
        Ok(mut input) => {
            match upload_picture(picture).await {
                Ok(p) => input.picture = p,
                Err(e) => return Flash::error(Redirect::to(CREATE_URL), e.message().to_string()),
            }
            match svc.store(db.inner(), input).await {
                Ok(_) => Flash::success(Redirect::to(INDEX_URL), "Create User Success."),
                Err(e) => Flash::error(Redirect::to(CREATE_URL), e.message().to_string()),
            }
        }
    }
}

#[get("/access/user/<id>/edit")]
pub async fn edit(
    auth: Authorized,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IUserService>>,
    cookies: &CookieJar<'_>,
    id: &str,
) -> Result<Template, AppError> {
    let (user, role_ids, roles) = svc.edit(db.inner(), id).await?;
    let csrf = ensure_token(cookies);
    let (errors, old) = flash::take(cookies);

    let mut page = json!({
        "data": user,
        "role_ids": role_ids,
        "roles": roles,
        "errors": errors,
        "old": old,
        "timezones": timezones(),
    });
    merge(&mut page, chrome(&auth.0, &csrf, "user"));
    Ok(render_view("be/default/access/users/edit", page, None))
}

#[put("/access/user/<id>/update", data = "<form>")]
pub async fn update(
    _auth: Authorized,
    _csrf: CsrfProtected,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IUserService>>,
    cookies: &CookieJar<'_>,
    id: &str,
    form: Form<UserForm<'_>>,
) -> Flash<Redirect> {
    let edit_url = format!("/admin/v1/access/user/{id}/edit");
    let mut f = form.into_inner();
    let picture = f.picture.take();
    match f.validate_update() {
        Err(fe) => {
            flash::stash(cookies, &fe.errors, &fe.old);
            Flash::error(Redirect::to(edit_url), "Please fix the errors below")
        }
        Ok(mut input) => {
            match upload_picture(picture).await {
                Ok(p) => input.picture = p,
                Err(e) => return Flash::error(Redirect::to(edit_url), e.message().to_string()),
            }
            match svc.update(db.inner(), id, input).await {
                Ok(_) => Flash::success(Redirect::to(INDEX_URL), "Update User Success."),
                Err(e) => Flash::error(Redirect::to(edit_url), e.message().to_string()),
            }
        }
    }
}

#[delete("/access/user/<id>/delete")]
pub async fn delete(
    _auth: Authorized,
    _csrf: CsrfProtected,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IUserService>>,
    id: &str,
) -> Flash<Redirect> {
    match svc.delete(db.inner(), id).await {
        Ok(_) => Flash::success(Redirect::to(INDEX_URL), "Delete User Success."),
        Err(e) => Flash::error(Redirect::to(INDEX_URL), e.message().to_string()),
    }
}

#[post("/access/user/delete_selected", data = "<form>")]
pub async fn delete_selected(
    _auth: Authorized,
    _csrf: CsrfProtected,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IUserService>>,
    form: Form<SelectionForm>,
) -> Flash<Redirect> {
    match svc
        .delete_selected(db.inner(), form.into_inner().selected)
        .await
    {
        Ok(_) => Flash::success(Redirect::to(INDEX_URL), "Delete User Success."),
        Err(e) => Flash::error(Redirect::to(INDEX_URL), e.message().to_string()),
    }
}

/// Read + store the optional avatar (magic-byte validated by the media service).
/// Returns the stored URL, or `None` when no file was provided.
async fn upload_picture(file: Option<TempFile<'_>>) -> Result<Option<String>, AppError> {
    let Some(f) = file else {
        return Ok(None);
    };
    if f.len() == 0 {
        return Ok(None);
    }
    let mut reader = f
        .open()
        .await
        .map_err(|e| AppError::internal(format!("read upload: {e}")))?;
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .await
        .map_err(|e| AppError::internal(format!("read upload: {e}")))?;
    let data = crate::modules::media::service::upload(&bytes).await?;
    Ok(data.get("url").and_then(|u| u.as_str()).map(str::to_string))
}

/// A short timezone list for the form (full tz DB is overkill for the bootstrap).
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
