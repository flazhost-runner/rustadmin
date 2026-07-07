//! Media controller — AJAX file-manager endpoints (session + CSRF header).

use rocket::form::Form;
use rocket::fs::TempFile;
use rocket::http::Status;
use rocket::serde::json::{json, Json};
use rocket::tokio::io::AsyncReadExt;
use serde_json::Value;

use crate::errors::AppError;
use crate::guards::CurrentUser;
use crate::modules::media::service;
use crate::security::csrf::CsrfProtected;

type ApiResult = Result<(Status, Json<Value>), AppError>;

#[derive(FromForm)]
pub struct UploadForm<'r> {
    pub file: TempFile<'r>,
}

#[derive(FromForm)]
pub struct DeleteBody {
    pub key: String,
}

#[get("/media/list")]
pub async fn list(_user: CurrentUser) -> ApiResult {
    let items = service::list().await?;
    Ok((
        Status::Ok,
        Json(json!({ "status": true, "message": "OK", "data": items })),
    ))
}

#[post("/media/upload", data = "<form>")]
pub async fn upload(
    _user: CurrentUser,
    _csrf: CsrfProtected,
    form: Form<UploadForm<'_>>,
) -> ApiResult {
    const MAX_SIZE: usize = 2 * 1024 * 1024; // 2 MB
    let mut reader = form
        .file
        .open()
        .await
        .map_err(|e| AppError::internal(format!("read upload: {e}")))?;
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .await
        .map_err(|e| AppError::internal(format!("read upload: {e}")))?;
    if bytes.len() > MAX_SIZE {
        return Err(AppError::bad_request("File size exceeds 2MB limit."));
    }
    let data = service::upload(&bytes).await?;
    Ok((
        Status::Ok,
        Json(json!({ "status": true, "message": "OK", "data": data })),
    ))
}

// Body is form-encoded (`key=...`) — matches the jQuery file-manager plugin's
// `$.ajax({ data: { key } })` default content-type; CSRF still arrives via header.
#[post("/media/delete", data = "<body>")]
pub async fn delete(_user: CurrentUser, _csrf: CsrfProtected, body: Form<DeleteBody>) -> ApiResult {
    service::delete(&body.key).await?;
    Ok((
        Status::Ok,
        Json(json!({ "status": true, "message": "Deleted", "data": null })),
    ))
}

pub fn routes() -> Vec<rocket::Route> {
    routes![list, upload, delete]
}
