//! Permission API controller — verbose CRUD.

use std::sync::Arc;

use rocket::http::Status;
use rocket::serde::json::{json, Json};
use rocket::State;
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use serde_json::Value;

use crate::errors::AppError;
use crate::guards::Authorized;
use crate::modules::access::services::permission_service::{
    IPermissionService, PermissionFilter, PermissionInput,
};

type ApiResult = Result<(Status, Json<Value>), AppError>;

#[derive(Debug, Deserialize, Default)]
pub struct PermissionBody {
    pub name: Option<String>,
    pub guard_name: Option<String>,
    pub method: Option<String>,
    pub status: Option<String>,
    pub desc: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SelectedBody {
    #[serde(default)]
    pub selected: Vec<String>,
}

fn to_input(b: PermissionBody) -> Result<PermissionInput, AppError> {
    let name = b
        .name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::validation("name is required"))?
        .to_string();
    Ok(PermissionInput {
        name,
        guard_name: b.guard_name,
        method: b.method,
        status: b.status,
        desc: b.desc,
    })
}

#[get(
    "/access/permission?<q_page>&<q_page_size>&<q_name>&<q_guard>&<q_method>&<q_status>&<q_desc>"
)]
#[allow(clippy::too_many_arguments)]
pub async fn index(
    _auth: Authorized,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IPermissionService>>,
    q_page: Option<u64>,
    q_page_size: Option<u64>,
    q_name: Option<String>,
    q_guard: Option<String>,
    q_method: Option<String>,
    q_status: Option<String>,
    q_desc: Option<String>,
) -> ApiResult {
    svc.sync_from_registry(db.inner()).await?;
    let filter = PermissionFilter {
        page: q_page,
        page_size: q_page_size,
        name: q_name,
        guard: q_guard,
        method: q_method,
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

#[post("/access/permission/store", data = "<body>")]
pub async fn store(
    _auth: Authorized,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IPermissionService>>,
    body: Json<PermissionBody>,
) -> ApiResult {
    let id = svc.store(db.inner(), to_input(body.into_inner())?).await?;
    Ok((
        Status::Created,
        Json(json!({ "status": true, "data": { "id": id } })),
    ))
}

#[get("/access/permission/<id>/edit")]
pub async fn edit(
    _auth: Authorized,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IPermissionService>>,
    id: &str,
) -> ApiResult {
    let p = svc.find(db.inner(), id).await?;
    Ok((Status::Ok, Json(json!({ "status": true, "data": p }))))
}

#[put("/access/permission/<id>/update", data = "<body>")]
pub async fn update(
    _auth: Authorized,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IPermissionService>>,
    id: &str,
    body: Json<PermissionBody>,
) -> ApiResult {
    svc.update(db.inner(), id, to_input(body.into_inner())?)
        .await?;
    Ok((
        Status::Ok,
        Json(json!({ "status": true, "message": "Updated" })),
    ))
}

#[delete("/access/permission/<id>/delete")]
pub async fn delete(
    _auth: Authorized,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IPermissionService>>,
    id: &str,
) -> ApiResult {
    svc.delete(db.inner(), id).await?;
    Ok((
        Status::Ok,
        Json(json!({ "status": true, "message": "Deleted" })),
    ))
}

#[post("/access/permission/delete_selected", data = "<body>")]
pub async fn delete_selected(
    _auth: Authorized,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IPermissionService>>,
    body: Json<SelectedBody>,
) -> ApiResult {
    svc.delete_selected(db.inner(), body.into_inner().selected)
        .await?;
    Ok((
        Status::Ok,
        Json(json!({ "status": true, "message": "Deleted" })),
    ))
}
