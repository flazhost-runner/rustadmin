//! API auth endpoints (JWT). Mounted at `/api/v1/auth`.
//! - `POST /login`  → issue HS256 token.
//! - `POST /logout` → blacklist the token's `jti` for its remaining TTL.
//! - `GET  /me`     → current user (blacklist-checked via `CurrentUser`).

use std::sync::Arc;

use rocket::http::Status;
use rocket::serde::json::{json, Json};
use rocket::{Route, State};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use serde_json::Value;

use crate::config::Config;
use crate::errors::AppError;
use crate::guards::{CurrentUser, JwtClaims};
use crate::modules::auth::service::IAuthService;
use crate::security::blacklist::TokenStore;
use crate::security::jwt;
use crate::security::rate_limit::AuthRateLimit;

#[derive(Debug, Deserialize)]
pub struct LoginDto {
    pub email: String,
    pub password: String,
}

#[post("/login", data = "<body>")]
pub async fn login(
    body: Json<LoginDto>,
    _rl: AuthRateLimit,
    db: &State<DatabaseConnection>,
    cfg: &State<Config>,
    svc: &State<Arc<dyn IAuthService>>,
) -> Result<(Status, Json<Value>), AppError> {
    let user = svc
        .authenticate(db.inner(), &body.email, &body.password)
        .await?;
    let (token, claims) = jwt::issue(&cfg.jwt.secret, &user.id, cfg.jwt.expires_secs())?;
    Ok((
        Status::Ok,
        Json(json!({
            "status": true,
            "message": "OK",
            "data": {
                "token": token,
                "token_type": "Bearer",
                "expires_at": claims.exp,
                "user": { "id": user.id, "name": user.name, "email": user.email }
            }
        })),
    ))
}

// POST: logout is a mutation (blacklists the token); GET must not have side effects.
#[post("/logout")]
pub async fn logout(
    claims: JwtClaims,
    store: &State<Arc<dyn TokenStore>>,
) -> (Status, Json<Value>) {
    store.blacklist(&claims.0.jti, claims.0.ttl_secs());
    (
        Status::Ok,
        Json(json!({ "status": true, "message": "Logged out" })),
    )
}

#[get("/me")]
pub async fn me(user: CurrentUser) -> (Status, Json<Value>) {
    (
        Status::Ok,
        Json(json!({
            "status": true,
            "data": { "id": user.id, "name": user.name, "is_admin": user.is_admin }
        })),
    )
}

pub fn routes() -> Vec<Route> {
    routes![login, logout, me]
}
