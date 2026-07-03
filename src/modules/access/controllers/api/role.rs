//! Role API controller — verbose CRUD + per-role permission management (5 routes, mirror web).

use std::sync::Arc;

use rocket::http::Status;
use rocket::serde::json::{json, Json};
use rocket::State;
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use serde_json::Value;

use crate::errors::AppError;
use crate::guards::Authorized;
use crate::modules::access::services::role_service::{
    IRoleService, PermAssignFilter, RoleFilter, RoleInput,
};

type ApiResult = Result<(Status, Json<Value>), AppError>;

#[derive(Debug, Deserialize, Default)]
pub struct RoleBody {
    pub name: Option<String>,
    pub status: Option<String>,
    pub desc: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SelectedBody {
    #[serde(default)]
    pub selected: Vec<String>,
}

fn to_input(b: RoleBody) -> Result<RoleInput, AppError> {
    let name = b
        .name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::validation("name is required"))?
        .to_string();
    Ok(RoleInput {
        name,
        status: b.status,
        desc: b.desc,
    })
}

#[get("/access/role?<q_page>&<q_page_size>&<q_name>&<q_status>&<q_desc>")]
#[allow(clippy::too_many_arguments)]
pub async fn index(
    _auth: Authorized,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IRoleService>>,
    q_page: Option<u64>,
    q_page_size: Option<u64>,
    q_name: Option<String>,
    q_status: Option<String>,
    q_desc: Option<String>,
) -> ApiResult {
    let filter = RoleFilter {
        page: q_page,
        page_size: q_page_size,
        name: q_name,
        status: q_status,
        desc: q_desc,
    };
    let idx = svc.index(db.inner(), &filter).await?;
    Ok((
        Status::Ok,
        Json(json!({
            "status": true, "message": "OK",
            "data": {
                "datas": idx.rows,
                "paginate_data": {
                    "total_data": idx.meta.total,
                    "current_page": idx.meta.page,
                    "page_size": idx.meta.page_size,
                    "total_page": idx.meta.total_pages,
                }
            }
        })),
    ))
}

#[post("/access/role/store", data = "<body>")]
pub async fn store(
    _auth: Authorized,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IRoleService>>,
    body: Json<RoleBody>,
) -> ApiResult {
    let id = svc.store(db.inner(), to_input(body.into_inner())?).await?;
    Ok((
        Status::Created,
        Json(json!({ "status": true, "data": { "id": id } })),
    ))
}

#[get("/access/role/<id>/edit")]
pub async fn edit(
    _auth: Authorized,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IRoleService>>,
    id: &str,
) -> ApiResult {
    let role = svc.find(db.inner(), id).await?;
    Ok((Status::Ok, Json(json!({ "status": true, "data": role }))))
}

#[put("/access/role/<id>/update", data = "<body>")]
pub async fn update(
    _auth: Authorized,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IRoleService>>,
    id: &str,
    body: Json<RoleBody>,
) -> ApiResult {
    svc.update(db.inner(), id, to_input(body.into_inner())?)
        .await?;
    Ok((
        Status::Ok,
        Json(json!({ "status": true, "message": "Updated" })),
    ))
}

#[delete("/access/role/<id>/delete")]
pub async fn delete(
    _auth: Authorized,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IRoleService>>,
    id: &str,
) -> ApiResult {
    svc.delete(db.inner(), id).await?;
    Ok((
        Status::Ok,
        Json(json!({ "status": true, "message": "Deleted" })),
    ))
}

#[post("/access/role/delete_selected", data = "<body>")]
pub async fn delete_selected(
    _auth: Authorized,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IRoleService>>,
    body: Json<SelectedBody>,
) -> ApiResult {
    svc.delete_selected(db.inner(), body.into_inner().selected)
        .await?;
    Ok((
        Status::Ok,
        Json(json!({ "status": true, "message": "Deleted" })),
    ))
}

// ---- per-role permission management (symmetric to web) ----

#[get("/access/role/<id>/permission?<q_page>&<q_page_size>&<q_name>&<q_status>&<q_desc>")]
#[allow(clippy::too_many_arguments)]
pub async fn permission(
    _auth: Authorized,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IRoleService>>,
    id: &str,
    q_page: Option<u64>,
    q_page_size: Option<u64>,
    q_name: Option<String>,
    q_status: Option<String>,
    q_desc: Option<String>,
) -> ApiResult {
    let filter = PermAssignFilter {
        page: q_page,
        page_size: q_page_size,
        name: q_name,
        status: q_status,
        desc: q_desc,
    };
    let idx = svc.list_permissions(db.inner(), id, &filter).await?;
    Ok((
        Status::Ok,
        Json(json!({
            "status": true, "message": "OK",
            "data": {
                "datas": idx.rows,
                "paginate_data": {
                    "total_data": idx.meta.total,
                    "current_page": idx.meta.page,
                    "page_size": idx.meta.page_size,
                    "total_page": idx.meta.total_pages,
                }
            }
        })),
    ))
}

#[get("/access/role/<id>/permission/<permission_id>/assign")]
pub async fn assign(
    _auth: Authorized,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IRoleService>>,
    id: &str,
    permission_id: &str,
) -> ApiResult {
    svc.assign(db.inner(), id, permission_id).await?;
    Ok((
        Status::Ok,
        Json(json!({ "status": true, "message": "Assigned" })),
    ))
}

#[get("/access/role/<id>/permission/<permission_id>/unassign")]
pub async fn unassign(
    _auth: Authorized,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IRoleService>>,
    id: &str,
    permission_id: &str,
) -> ApiResult {
    svc.unassign(db.inner(), id, permission_id).await?;
    Ok((
        Status::Ok,
        Json(json!({ "status": true, "message": "Unassigned" })),
    ))
}

#[post("/access/role/<id>/permission/assign_selected", data = "<body>")]
pub async fn assign_selected(
    _auth: Authorized,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IRoleService>>,
    id: &str,
    body: Json<SelectedBody>,
) -> ApiResult {
    svc.assign_selected(db.inner(), id, body.into_inner().selected)
        .await?;
    Ok((
        Status::Ok,
        Json(json!({ "status": true, "message": "Assigned" })),
    ))
}

#[post("/access/role/<id>/permission/unassign_selected", data = "<body>")]
pub async fn unassign_selected(
    _auth: Authorized,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IRoleService>>,
    id: &str,
    body: Json<SelectedBody>,
) -> ApiResult {
    svc.unassign_selected(db.inner(), id, body.into_inner().selected)
        .await?;
    Ok((
        Status::Ok,
        Json(json!({ "status": true, "message": "Unassigned" })),
    ))
}
