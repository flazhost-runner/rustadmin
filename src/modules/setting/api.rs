//! Setting API controller — GET /api/v1/setting (JWT-authorized).

use std::sync::Arc;

use rocket::http::Status;
use rocket::serde::json::{json, Json};
use rocket::State;
use sea_orm::DatabaseConnection;
use serde_json::Value;

use crate::errors::AppError;
use crate::guards::CurrentUser;
use crate::modules::setting::services::setting_service::ISettingService;

type ApiResult = Result<(Status, Json<Value>), AppError>;

#[get("/setting")]
pub async fn index(
    _auth: CurrentUser,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn ISettingService>>,
) -> ApiResult {
    let s = svc.get(db.inner()).await?;
    Ok((
        Status::Ok,
        Json(json!({
            "status": true,
            "message": "OK",
            "data": {
                "id":          s.id,
                "name":        s.name.unwrap_or_default(),
                "theme":       s.theme.unwrap_or_default(),
                "fe_template": s.fe_template.unwrap_or_default(),
            }
        })),
    ))
}
