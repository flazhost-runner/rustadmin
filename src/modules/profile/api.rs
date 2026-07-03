//! Profile API controller — GET /api/v1/profile (JWT-authorized).

use std::sync::Arc;

use rocket::http::Status;
use rocket::serde::json::{json, Json};
use rocket::State;
use sea_orm::DatabaseConnection;
use serde_json::Value;

use crate::errors::AppError;
use crate::guards::CurrentUser;
use crate::modules::profile::service::IProfileService;

type ApiResult = Result<(Status, Json<Value>), AppError>;

#[get("/profile")]
pub async fn index(
    auth: CurrentUser,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IProfileService>>,
) -> ApiResult {
    let user = svc.get(db.inner(), &auth.id).await?;
    Ok((
        Status::Ok,
        Json(json!({
            "status": true,
            "message": "OK",
            "data": {
                "id":       user.id,
                "name":     user.name,
                "email":    user.email,
                "timezone": user.timezone.unwrap_or_default(),
                "picture":  user.picture.unwrap_or_default(),
                "status":   user.status,
            }
        })),
    ))
}
