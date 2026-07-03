//! Role web controller — CRUD + per-role permission assignment (separate page).

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
use crate::modules::access::services::role_service::{IRoleService, PermAssignFilter, RoleFilter};
use crate::modules::access::validators::role::RoleForm;
use crate::security::csrf::{ensure_token, CsrfProtected};

use super::user::SelectionForm;
use super::{chrome, filter_query, merge};

const INDEX_URL: &str = "/admin/v1/access/role";
const CREATE_URL: &str = "/admin/v1/access/role/create";

#[derive(rocket::FromForm, Debug, Default)]
pub struct RoleQuery {
    pub q_page: Option<u64>,
    pub q_page_size: Option<u64>,
    pub q_name: Option<String>,
    pub q_status: Option<String>,
    pub q_desc: Option<String>,
}

impl RoleQuery {
    fn to_filter(&self) -> RoleFilter {
        RoleFilter {
            page: self.q_page,
            page_size: self.q_page_size,
            name: self.q_name.clone(),
            status: self.q_status.clone(),
            desc: self.q_desc.clone(),
        }
    }
    fn as_value(&self) -> Value {
        json!({
            "q_name": self.q_name.clone().unwrap_or_default(),
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

#[get("/access/role?<f..>")]
pub async fn index(
    auth: Authorized,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IRoleService>>,
    cookies: &CookieJar<'_>,
    flash: Option<FlashMessage<'_>>,
    f: RoleQuery,
) -> Result<Template, AppError> {
    let idx = svc.index(db.inner(), &f.to_filter()).await?;
    let csrf = ensure_token(cookies);
    let pages = page_window(idx.meta.page, idx.meta.total_pages);
    let mut page = json!({
        "datas": idx.rows, "meta": idx.meta, "pages": pages,
        "filter": f.as_value(), "base_query": f.base_query(), "flash": fv(&flash),
    });
    merge(&mut page, chrome(&auth.0, &csrf, "role"));
    Ok(render_view("be/default/access/roles/index", page, None))
}

#[get("/access/role/create")]
pub async fn create(auth: Authorized, cookies: &CookieJar<'_>) -> Result<Template, AppError> {
    let csrf = ensure_token(cookies);
    let (errors, old) = flash::take(cookies);
    let mut page = json!({ "errors": errors, "old": old });
    merge(&mut page, chrome(&auth.0, &csrf, "role"));
    Ok(render_view("be/default/access/roles/create", page, None))
}

#[post("/access/role/store", data = "<form>")]
pub async fn store(
    _auth: Authorized,
    _csrf: CsrfProtected,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IRoleService>>,
    cookies: &CookieJar<'_>,
    form: Form<RoleForm>,
) -> Flash<Redirect> {
    match form.into_inner().validate() {
        Err(fe) => {
            flash::stash(cookies, &fe.errors, &fe.old);
            Flash::error(Redirect::to(CREATE_URL), "Please fix the errors below")
        }
        Ok(input) => match svc.store(db.inner(), input).await {
            Ok(_) => Flash::success(Redirect::to(INDEX_URL), "Create Role Success."),
            Err(e) => Flash::error(Redirect::to(CREATE_URL), e.message().to_string()),
        },
    }
}

#[get("/access/role/<id>/edit")]
pub async fn edit(
    auth: Authorized,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IRoleService>>,
    cookies: &CookieJar<'_>,
    id: &str,
) -> Result<Template, AppError> {
    let data = svc.find(db.inner(), id).await?;
    let csrf = ensure_token(cookies);
    let (errors, old) = flash::take(cookies);
    let mut page = json!({ "data": data, "errors": errors, "old": old });
    merge(&mut page, chrome(&auth.0, &csrf, "role"));
    Ok(render_view("be/default/access/roles/edit", page, None))
}

#[put("/access/role/<id>/update", data = "<form>")]
pub async fn update(
    _auth: Authorized,
    _csrf: CsrfProtected,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IRoleService>>,
    cookies: &CookieJar<'_>,
    id: &str,
    form: Form<RoleForm>,
) -> Flash<Redirect> {
    let edit_url = format!("/admin/v1/access/role/{id}/edit");
    match form.into_inner().validate() {
        Err(fe) => {
            flash::stash(cookies, &fe.errors, &fe.old);
            Flash::error(Redirect::to(edit_url), "Please fix the errors below")
        }
        Ok(input) => match svc.update(db.inner(), id, input).await {
            Ok(_) => Flash::success(Redirect::to(INDEX_URL), "Update Role Success."),
            Err(e) => Flash::error(Redirect::to(edit_url), e.message().to_string()),
        },
    }
}

#[delete("/access/role/<id>/delete")]
pub async fn delete(
    _auth: Authorized,
    _csrf: CsrfProtected,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IRoleService>>,
    id: &str,
) -> Flash<Redirect> {
    match svc.delete(db.inner(), id).await {
        Ok(_) => Flash::success(Redirect::to(INDEX_URL), "Delete Role Success."),
        Err(e) => Flash::error(Redirect::to(INDEX_URL), e.message().to_string()),
    }
}

#[post("/access/role/delete_selected", data = "<form>")]
pub async fn delete_selected(
    _auth: Authorized,
    _csrf: CsrfProtected,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IRoleService>>,
    form: Form<SelectionForm>,
) -> Flash<Redirect> {
    match svc
        .delete_selected(db.inner(), form.into_inner().selected)
        .await
    {
        Ok(_) => Flash::success(Redirect::to(INDEX_URL), "Delete Role Success."),
        Err(e) => Flash::error(Redirect::to(INDEX_URL), e.message().to_string()),
    }
}

// ---------- per-role permission management ----------

#[derive(rocket::FromForm, Debug, Default)]
pub struct PermQuery {
    pub q_page: Option<u64>,
    pub q_page_size: Option<u64>,
    pub q_name: Option<String>,
    pub q_status: Option<String>,
    pub q_desc: Option<String>,
}

#[get("/access/role/<id>/permission?<f..>")]
pub async fn permission(
    auth: Authorized,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IRoleService>>,
    cookies: &CookieJar<'_>,
    flash: Option<FlashMessage<'_>>,
    id: &str,
    f: PermQuery,
) -> Result<Template, AppError> {
    let filter = PermAssignFilter {
        page: f.q_page,
        page_size: f.q_page_size,
        name: f.q_name.clone(),
        status: f.q_status.clone(),
        desc: f.q_desc.clone(),
    };
    let idx = svc.list_permissions(db.inner(), id, &filter).await?;
    let csrf = ensure_token(cookies);
    let pages = page_window(idx.meta.page, idx.meta.total_pages);
    let base_query = {
        let ps = f.q_page_size.map(|v| v.to_string());
        filter_query(&[
            ("q_page_size", ps.as_deref()),
            ("q_name", f.q_name.as_deref()),
            ("q_status", f.q_status.as_deref()),
            ("q_desc", f.q_desc.as_deref()),
        ])
    };
    let mut page = json!({
        "role": idx.role, "datas": idx.rows, "meta": idx.meta, "pages": pages,
        "filter": {
            "q_name": f.q_name.clone().unwrap_or_default(),
            "q_status": f.q_status.clone().unwrap_or_default(),
            "q_desc": f.q_desc.clone().unwrap_or_default(),
            "q_page_size": f.q_page_size.map(|v| v.to_string()).unwrap_or_default(),
        },
        "base_query": base_query, "flash": fv(&flash),
    });
    merge(&mut page, chrome(&auth.0, &csrf, "role"));
    Ok(render_view(
        "be/default/access/roles/permission",
        page,
        None,
    ))
}

#[get("/access/role/<id>/permission/<permission_id>/assign")]
pub async fn assign(
    _auth: Authorized,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IRoleService>>,
    id: &str,
    permission_id: &str,
) -> Flash<Redirect> {
    let back = format!("/admin/v1/access/role/{id}/permission");
    match svc.assign(db.inner(), id, permission_id).await {
        Ok(_) => Flash::success(Redirect::to(back), "Assign Permission Success."),
        Err(e) => Flash::error(Redirect::to(back), e.message().to_string()),
    }
}

#[get("/access/role/<id>/permission/<permission_id>/unassign")]
pub async fn unassign(
    _auth: Authorized,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IRoleService>>,
    id: &str,
    permission_id: &str,
) -> Flash<Redirect> {
    let back = format!("/admin/v1/access/role/{id}/permission");
    match svc.unassign(db.inner(), id, permission_id).await {
        Ok(_) => Flash::success(Redirect::to(back), "Unassign Permission Success."),
        Err(e) => Flash::error(Redirect::to(back), e.message().to_string()),
    }
}

#[post("/access/role/<id>/permission/assign_selected", data = "<form>")]
pub async fn assign_selected(
    _auth: Authorized,
    _csrf: CsrfProtected,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IRoleService>>,
    id: &str,
    form: Form<SelectionForm>,
) -> Flash<Redirect> {
    let back = format!("/admin/v1/access/role/{id}/permission");
    match svc
        .assign_selected(db.inner(), id, form.into_inner().selected)
        .await
    {
        Ok(_) => Flash::success(Redirect::to(back), "Assign Permission Success."),
        Err(e) => Flash::error(Redirect::to(back), e.message().to_string()),
    }
}

#[post("/access/role/<id>/permission/unassign_selected", data = "<form>")]
pub async fn unassign_selected(
    _auth: Authorized,
    _csrf: CsrfProtected,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IRoleService>>,
    id: &str,
    form: Form<SelectionForm>,
) -> Flash<Redirect> {
    let back = format!("/admin/v1/access/role/{id}/permission");
    match svc
        .unassign_selected(db.inner(), id, form.into_inner().selected)
        .await
    {
        Ok(_) => Flash::success(Redirect::to(back), "Unassign Permission Success."),
        Err(e) => Flash::error(Redirect::to(back), e.message().to_string()),
    }
}
