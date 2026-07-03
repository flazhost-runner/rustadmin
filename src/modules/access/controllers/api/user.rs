//! User API controller (verbose JSON CRUD, JWT-authorized).

use std::sync::Arc;

use rocket::http::Status;
use rocket::serde::json::{json, Json};
use rocket::State;
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use serde_json::Value;

use crate::errors::AppError;
use crate::guards::Authorized;
use crate::modules::access::services::user_service::{
    IUserService, StoreUserInput, UpdateUserInput, UserFilter,
};

type ApiResult = Result<(Status, Json<Value>), AppError>;

#[derive(Debug, Deserialize, Default)]
pub struct UserBody {
    pub code: Option<String>,
    pub name: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub timezone: Option<String>,
    pub password: Option<String>,
    pub status: Option<String>,
    pub blocked: Option<bool>,
    pub blocked_reason: Option<String>,
    #[serde(default)]
    pub roles: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct SelectedBody {
    #[serde(default)]
    pub selected: Vec<String>,
}

#[get(
    "/access/user?<q_page>&<q_page_size>&<q_code>&<q_name>&<q_phone>&<q_email>&<q_status>&<q_role>"
)]
#[allow(clippy::too_many_arguments)]
pub async fn index(
    _auth: Authorized,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IUserService>>,
    q_page: Option<u64>,
    q_page_size: Option<u64>,
    q_code: Option<String>,
    q_name: Option<String>,
    q_phone: Option<String>,
    q_email: Option<String>,
    q_status: Option<String>,
    q_role: Option<String>,
) -> ApiResult {
    let filter = UserFilter {
        page: q_page,
        page_size: q_page_size,
        code: q_code,
        name: q_name,
        phone: q_phone,
        email: q_email,
        status: q_status,
        role: q_role,
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

#[post("/access/user/store", data = "<body>")]
pub async fn store(
    _auth: Authorized,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IUserService>>,
    body: Json<UserBody>,
) -> ApiResult {
    let input = to_store(body.into_inner())?;
    let id = svc.store(db.inner(), input).await?;
    Ok((
        Status::Created,
        Json(json!({ "status": true, "message": "Created", "data": { "id": id } })),
    ))
}

#[get("/access/user/<id>/edit")]
pub async fn edit(
    _auth: Authorized,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IUserService>>,
    id: &str,
) -> ApiResult {
    let (user, role_ids, _roles) = svc.edit(db.inner(), id).await?;
    Ok((
        Status::Ok,
        Json(json!({ "status": true, "data": { "user": user, "role_ids": role_ids } })),
    ))
}

#[put("/access/user/<id>/update", data = "<body>")]
pub async fn update(
    _auth: Authorized,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IUserService>>,
    id: &str,
    body: Json<UserBody>,
) -> ApiResult {
    let input = to_update(body.into_inner())?;
    svc.update(db.inner(), id, input).await?;
    Ok((
        Status::Ok,
        Json(json!({ "status": true, "message": "Updated" })),
    ))
}

#[delete("/access/user/<id>/delete")]
pub async fn delete(
    _auth: Authorized,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IUserService>>,
    id: &str,
) -> ApiResult {
    svc.delete(db.inner(), id).await?;
    Ok((
        Status::Ok,
        Json(json!({ "status": true, "message": "Deleted" })),
    ))
}

#[post("/access/user/delete_selected", data = "<body>")]
pub async fn delete_selected(
    _auth: Authorized,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IUserService>>,
    body: Json<SelectedBody>,
) -> ApiResult {
    svc.delete_selected(db.inner(), body.into_inner().selected)
        .await?;
    Ok((
        Status::Ok,
        Json(json!({ "status": true, "message": "Deleted" })),
    ))
}

fn to_store(b: UserBody) -> Result<StoreUserInput, AppError> {
    let code = req(&b.code, "code")?;
    let name = req(&b.name, "name")?;
    let email = req(&b.email, "email")?;
    let password = req(&b.password, "password")?;
    if password.len() < 8 {
        return Err(AppError::validation(
            "Password must be at least 8 characters",
        ));
    }
    Ok(StoreUserInput {
        code,
        name,
        phone: b.phone,
        email,
        timezone: b.timezone,
        password,
        status: b.status,
        blocked: b.blocked.unwrap_or(false),
        blocked_reason: b.blocked_reason,
        picture: None,
        roles: b.roles,
    })
}

fn to_update(b: UserBody) -> Result<UpdateUserInput, AppError> {
    Ok(UpdateUserInput {
        code: req(&b.code, "code")?,
        name: req(&b.name, "name")?,
        phone: b.phone,
        email: req(&b.email, "email")?,
        timezone: b.timezone,
        password: b.password.filter(|p| !p.is_empty()),
        status: b.status,
        blocked: b.blocked.unwrap_or(false),
        blocked_reason: b.blocked_reason,
        picture: None,
        roles: b.roles,
    })
}

fn req(opt: &Option<String>, field: &str) -> Result<String, AppError> {
    match opt.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(v) => Ok(v.to_string()),
        None => Err(AppError::validation(format!("{field} is required"))),
    }
}
