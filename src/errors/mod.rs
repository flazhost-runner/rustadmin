//! Central error handling (Clean Code / SOLID-S).
//!
//! Equivalent of NodeAdmin `src/errors/AppError.ts` + the `errorHandler` middleware.
//! Services **return** [`AppError`] (Rust's idiom for `throw`); the [`rocket::response::Responder`]
//! impl is the single place that turns it into an HTTP response:
//! - `/api/*` → JSON via the shared response envelope.
//! - web → flash message + redirect (PRG), back to the referrer.
//!
//! Non-`AppError`/internal details never leak to users in production.

use std::collections::HashMap;
use std::fmt;

use rocket::http::Status;
use rocket::request::Request;
use rocket::response::{self, Flash, Redirect, Responder};
use rocket::serde::json::{json, Json};

use crate::config::Config;

/// Application error. Constructed by services; rendered by the `Responder` impl.
#[derive(Debug, Clone)]
pub enum AppError {
    /// 404
    NotFound(String),
    /// 409
    Conflict(String),
    /// 422 — with optional per-field messages for inline form validation.
    Validation {
        message: String,
        fields: HashMap<String, String>,
    },
    /// 401
    Unauthorized(String),
    /// 403
    Forbidden(String),
    /// 400
    BadRequest(String),
    /// 500 — message is for logs only; users see a generic message in production.
    Internal(String),
}

/// Convenience result alias used across services/controllers.
pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn not_found(msg: impl Into<String>) -> Self {
        AppError::NotFound(msg.into())
    }
    pub fn conflict(msg: impl Into<String>) -> Self {
        AppError::Conflict(msg.into())
    }
    pub fn validation(msg: impl Into<String>) -> Self {
        AppError::Validation {
            message: msg.into(),
            fields: HashMap::new(),
        }
    }
    pub fn validation_fields(msg: impl Into<String>, fields: HashMap<String, String>) -> Self {
        AppError::Validation {
            message: msg.into(),
            fields,
        }
    }
    pub fn unauthorized(msg: impl Into<String>) -> Self {
        AppError::Unauthorized(msg.into())
    }
    pub fn forbidden(msg: impl Into<String>) -> Self {
        AppError::Forbidden(msg.into())
    }
    pub fn bad_request(msg: impl Into<String>) -> Self {
        AppError::BadRequest(msg.into())
    }
    pub fn internal(msg: impl Into<String>) -> Self {
        AppError::Internal(msg.into())
    }

    pub fn status(&self) -> Status {
        match self {
            AppError::NotFound(_) => Status::NotFound,
            AppError::Conflict(_) => Status::Conflict,
            AppError::Validation { .. } => Status::UnprocessableEntity,
            AppError::Unauthorized(_) => Status::Unauthorized,
            AppError::Forbidden(_) => Status::Forbidden,
            AppError::BadRequest(_) => Status::BadRequest,
            AppError::Internal(_) => Status::InternalServerError,
        }
    }

    /// The developer-facing message (used in logs and in non-prod responses).
    pub fn message(&self) -> &str {
        match self {
            AppError::NotFound(m)
            | AppError::Conflict(m)
            | AppError::Unauthorized(m)
            | AppError::Forbidden(m)
            | AppError::BadRequest(m)
            | AppError::Internal(m) => m,
            AppError::Validation { message, .. } => message,
        }
    }

    pub fn fields(&self) -> Option<&HashMap<String, String>> {
        match self {
            AppError::Validation { fields, .. } => Some(fields),
            _ => None,
        }
    }

    /// Message safe to show the user. Internal errors are masked in production.
    fn client_message(&self, is_prod: bool) -> String {
        match self {
            AppError::Internal(_) if is_prod => {
                "An unexpected error occurred. Please try again later.".to_string()
            }
            _ => self.message().to_string(),
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.status().code, self.message())
    }
}

impl std::error::Error for AppError {}

// Common conversions so services can use `?` on infra errors.
impl From<sea_orm::DbErr> for AppError {
    fn from(e: sea_orm::DbErr) -> Self {
        AppError::Internal(format!("db error: {e}"))
    }
}

impl From<bcrypt::BcryptError> for AppError {
    fn from(e: bcrypt::BcryptError) -> Self {
        AppError::Internal(format!("hash error: {e}"))
    }
}

impl<'r> Responder<'r, 'static> for AppError {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'static> {
        let is_prod = req
            .rocket()
            .state::<Config>()
            .map(|c| c.is_prod)
            .unwrap_or(false);

        // Always log the developer-facing detail.
        if self.status().code >= 500 {
            error!("{} — {}", self.status(), self.message());
        } else {
            warn!("{} — {}", self.status(), self.message());
        }

        let path = req.uri().path().as_str().to_string();
        let msg = self.client_message(is_prod);

        if path.starts_with("/api") {
            let body = json!({
                "status": false,
                "message": msg,
                "errors": self.fields(),
            });
            (self.status(), Json(body)).respond_to(req)
        } else {
            let referer = req
                .headers()
                .get_one("Referer")
                .filter(|r| !r.is_empty())
                .unwrap_or("/")
                .to_string();
            Flash::error(Redirect::to(referer), msg).respond_to(req)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_codes_map() {
        assert_eq!(AppError::not_found("x").status(), Status::NotFound);
        assert_eq!(AppError::conflict("x").status(), Status::Conflict);
        assert_eq!(
            AppError::validation("x").status(),
            Status::UnprocessableEntity
        );
        assert_eq!(AppError::unauthorized("x").status(), Status::Unauthorized);
        assert_eq!(
            AppError::internal("x").status(),
            Status::InternalServerError
        );
    }

    #[test]
    fn internal_masked_in_prod() {
        let e = AppError::internal("secret db dsn leaked");
        assert!(e.client_message(true).contains("unexpected"));
        assert_eq!(e.client_message(false), "secret db dsn leaked");
    }
}
