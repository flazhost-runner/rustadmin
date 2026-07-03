//! Permission web controller — CRUD; the index lazily syncs permissions from the route
//! registry (mirrors NodeAdmin's lazy `getAllRegisteredRoute`).

use std::sync::Arc;

use rocket::form::Form;
use rocket::http::CookieJar;
use rocket::request::FlashMessage;
use rocket::response::{Flash, Redirect};
use rocket::State;
use rocket_dyn_templates::Template;
use sea_orm::DatabaseConnection;
use serde_json::{json, Value};

use crate::errors::AppError;
use crate::guards::Authorized;
use crate::helpers::flash;
use crate::helpers::pagination::page_window;
use crate::helpers::view::render_view;
use crate::modules::access::services::permission_service::{IPermissionService, PermissionFilter};
use crate::modules::access::validators::permission::PermissionForm;
use crate::security::csrf::{ensure_token, CsrfProtected};

use super::user::SelectionForm;
use super::{chrome, filter_query, merge};

const INDEX_URL: &str = "/admin/v1/access/permission";
const CREATE_URL: &str = "/admin/v1/access/permission/create";

#[derive(rocket::FromForm, Debug, Default)]
pub struct PermissionQuery {
    pub q_page: Option<u64>,
    pub q_page_size: Option<u64>,
    pub q_name: Option<String>,
    pub q_guard: Option<String>,
    pub q_method: Option<String>,
    pub q_status: Option<String>,
    pub q_desc: Option<String>,
}

impl PermissionQuery {
    fn to_filter(&self) -> PermissionFilter {
        PermissionFilter {
            page: self.q_page,
            page_size: self.q_page_size,
            name: self.q_name.clone(),
            guard: self.q_guard.clone(),
            method: self.q_method.clone(),
            status: self.q_status.clone(),
            desc: self.q_desc.clone(),
        }
    }
    fn as_value(&self) -> Value {
        json!({
            "q_name": self.q_name.clone().unwrap_or_default(),
            "q_guard": self.q_guard.clone().unwrap_or_default(),
            "q_method": self.q_method.clone().unwrap_or_default(),
            "q_status": self.q_status.clone().unwrap_or_default(),
            "q_desc": self.q_desc.clone().unwrap_or_default(),
            "q_page_size": self.q_page_size.map(|v| v.to_string()).unwrap_or_default(),
        })
    }
    fn base_query(&self) -> String {
        let ps = self.q_page_size.map(|v| v.to_string());
        filter_query(&[
            ("q_page_size", ps.as_deref()),
            ("q_name", self.q_name.as_deref()),
            ("q_guard", self.q_guard.as_deref()),
            ("q_method", self.q_method.as_deref()),
            ("q_status", self.q_status.as_deref()),
            ("q_desc", self.q_desc.as_deref()),
        ])
    }
}

fn fv(f: &Option<FlashMessage<'_>>) -> Value {
    match f {
        Some(m) => json!({ "key": m.kind(), "message": m.message() }),
        None => json!({}),
    }
}

#[get("/access/permission?<f..>")]
pub async fn index(
    auth: Authorized,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IPermissionService>>,
    cookies: &CookieJar<'_>,
    flash: Option<FlashMessage<'_>>,
    f: PermissionQuery,
) -> Result<Template, AppError> {
    // lazy auto-sync from the named-route registry (idempotent)
    svc.sync_from_registry(db.inner()).await?;

    let idx = svc.index(db.inner(), &f.to_filter()).await?;
    let csrf = ensure_token(cookies);
    let pages = page_window(idx.meta.page, idx.meta.total_pages);
    let mut page = json!({
        "datas": idx.rows, "meta": idx.meta, "pages": pages,
        "filter": f.as_value(), "base_query": f.base_query(), "flash": fv(&flash),
    });
    merge(&mut page, chrome(&auth.0, &csrf, "permission"));
    Ok(render_view(
        "be/default/access/permission/index",
        page,
        None,
    ))
}

#[get("/access/permission/create")]
pub async fn create(auth: Authorized, cookies: &CookieJar<'_>) -> Result<Template, AppError> {
    let csrf = ensure_token(cookies);
    let (errors, old) = flash::take(cookies);
    let mut page = json!({ "errors": errors, "old": old });
    merge(&mut page, chrome(&auth.0, &csrf, "permission"));
    Ok(render_view(
        "be/default/access/permission/create",
        page,
        None,
    ))
}

#[post("/access/permission/store", data = "<form>")]
pub async fn store(
    _auth: Authorized,
    _csrf: CsrfProtected,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IPermissionService>>,
    cookies: &CookieJar<'_>,
    form: Form<PermissionForm>,
) -> Flash<Redirect> {
    match form.into_inner().validate() {
        Err(fe) => {
            flash::stash(cookies, &fe.errors, &fe.old);
            Flash::error(Redirect::to(CREATE_URL), "Please fix the errors below")
        }
        Ok(input) => match svc.store(db.inner(), input).await {
            Ok(_) => Flash::success(Redirect::to(INDEX_URL), "Create Permission Success."),
            Err(e) => Flash::error(Redirect::to(CREATE_URL), e.message().to_string()),
        },
    }
}

#[get("/access/permission/<id>/edit")]
pub async fn edit(
    auth: Authorized,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IPermissionService>>,
    cookies: &CookieJar<'_>,
    id: &str,
) -> Result<Template, AppError> {
    let data = svc.find(db.inner(), id).await?;
    let csrf = ensure_token(cookies);
    let (errors, old) = flash::take(cookies);
    let mut page = json!({ "data": data, "errors": errors, "old": old });
    merge(&mut page, chrome(&auth.0, &csrf, "permission"));
    Ok(render_view("be/default/access/permission/edit", page, None))
}

#[put("/access/permission/<id>/update", data = "<form>")]
pub async fn update(
    _auth: Authorized,
    _csrf: CsrfProtected,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IPermissionService>>,
    cookies: &CookieJar<'_>,
    id: &str,
    form: Form<PermissionForm>,
) -> Flash<Redirect> {
    let edit_url = format!("/admin/v1/access/permission/{id}/edit");
    match form.into_inner().validate() {
        Err(fe) => {
            flash::stash(cookies, &fe.errors, &fe.old);
            Flash::error(Redirect::to(edit_url), "Please fix the errors below")
        }
        Ok(input) => match svc.update(db.inner(), id, input).await {
            Ok(_) => Flash::success(Redirect::to(INDEX_URL), "Update Permission Success."),
            Err(e) => Flash::error(Redirect::to(edit_url), e.message().to_string()),
        },
    }
}

#[delete("/access/permission/<id>/delete")]
pub async fn delete(
    _auth: Authorized,
    _csrf: CsrfProtected,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IPermissionService>>,
    id: &str,
) -> Flash<Redirect> {
    match svc.delete(db.inner(), id).await {
        Ok(_) => Flash::success(Redirect::to(INDEX_URL), "Delete Permission Success."),
        Err(e) => Flash::error(Redirect::to(INDEX_URL), e.message().to_string()),
    }
}

#[post("/access/permission/delete_selected", data = "<form>")]
pub async fn delete_selected(
    _auth: Authorized,
    _csrf: CsrfProtected,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IPermissionService>>,
    form: Form<SelectionForm>,
) -> Flash<Redirect> {
    match svc
        .delete_selected(db.inner(), form.into_inner().selected)
        .await
    {
        Ok(_) => Flash::success(Redirect::to(INDEX_URL), "Delete Permission Success."),
        Err(e) => Flash::error(Redirect::to(INDEX_URL), e.message().to_string()),
    }
}
