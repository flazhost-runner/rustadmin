//! Auth service (trait + impl) — credentials, registration, password-reset (OTP).
//! `@injectable`-equivalent: shared as `State<Arc<dyn IAuthService>>` (managed state = DI).

use async_trait::async_trait;
use chrono::Utc;
use sea_orm::ActiveValue::Set;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
};
use uuid::Uuid;

use crate::errors::{AppError, AppResult};
use crate::helpers::otp;
use crate::modules::access::models::user;

#[async_trait]
pub trait IAuthService: Send + Sync {
    /// Verify credentials; returns the user on success, else `AppError` (401/403).
    async fn authenticate(
        &self,
        db: &DatabaseConnection,
        email: &str,
        password: &str,
    ) -> AppResult<user::Model>;

    /// Self-register a new user (no roles assigned).
    async fn register(
        &self,
        db: &DatabaseConnection,
        name: &str,
        email: &str,
        password: &str,
    ) -> AppResult<user::Model>;

    /// Begin password reset: store a hashed OTP + expiry; returns the plaintext OTP
    /// (the caller delivers it via email — here it is logged).
    async fn request_password_reset(
        &self,
        db: &DatabaseConnection,
        email: &str,
    ) -> AppResult<String>;

    /// Complete password reset: verify OTP (unexpired) then set the new password.
    async fn reset_password(
        &self,
        db: &DatabaseConnection,
        email: &str,
        otp_code: &str,
        new_password: &str,
    ) -> AppResult<()>;
}

pub struct AuthService {
    pub bcrypt_rounds: u32,
    pub otp_expiry_ms: i64,
}

impl AuthService {
    pub fn new(bcrypt_rounds: u32, otp_expiry_ms: i64) -> Self {
        Self {
            bcrypt_rounds,
            otp_expiry_ms,
        }
    }

    async fn by_email(db: &DatabaseConnection, email: &str) -> AppResult<Option<user::Model>> {
        Ok(user::Entity::find()
            .filter(user::Column::Email.eq(email))
            .one(db)
            .await?)
    }
}

#[async_trait]
impl IAuthService for AuthService {
    async fn authenticate(
        &self,
        db: &DatabaseConnection,
        email: &str,
        password: &str,
    ) -> AppResult<user::Model> {
        let u = Self::by_email(db, email)
            .await?
            .ok_or_else(|| AppError::unauthorized("Wrong email or password."))?;
        if u.blocked {
            return Err(AppError::forbidden("Account is blocked"));
        }
        if !bcrypt::verify(password, &u.password).unwrap_or(false) {
            return Err(AppError::unauthorized("Wrong email or password."));
        }
        Ok(u)
    }

    async fn register(
        &self,
        db: &DatabaseConnection,
        name: &str,
        email: &str,
        password: &str,
    ) -> AppResult<user::Model> {
        if Self::by_email(db, email).await?.is_some() {
            return Err(AppError::conflict("Email already exists."));
        }
        let id = Uuid::new_v4().to_string();
        let code: String = format!(
            "{:010}",
            (Utc::now().timestamp_millis() % 10_000_000_000) as u64
        );
        let hashed = bcrypt::hash(password, self.bcrypt_rounds)?;
        let model = user::ActiveModel {
            id: Set(id),
            code: Set(code),
            name: Set(name.to_string()),
            email: Set(email.to_string()),
            password: Set(hashed),
            status: Set("Active".into()),
            timezone: Set(Some("UTC".into())),
            blocked: Set(false),
            ..Default::default()
        }
        .insert(db)
        .await?;
        Ok(model)
    }

    async fn request_password_reset(
        &self,
        db: &DatabaseConnection,
        email: &str,
    ) -> AppResult<String> {
        let u = Self::by_email(db, email)
            .await?
            .ok_or_else(|| AppError::not_found("Email not found."))?;
        let code = otp::generate_otp(6);
        let hashed = otp::hash_otp(&code, self.bcrypt_rounds)?;
        let expires = otp::expiry_from(Utc::now().timestamp_millis(), self.otp_expiry_ms);
        let mut am = u.into_active_model();
        am.password_otp = Set(Some(hashed));
        am.password_otp_expires = Set(Some(expires));
        am.update(db).await?;
        info!("Password reset OTP for {email}: {code}"); // delivered via email in production
        Ok(code)
    }

    async fn reset_password(
        &self,
        db: &DatabaseConnection,
        email: &str,
        otp_code: &str,
        new_password: &str,
    ) -> AppResult<()> {
        let u = Self::by_email(db, email)
            .await?
            .ok_or_else(|| AppError::not_found("Email not found"))?;
        let hashed = u
            .password_otp
            .clone()
            .ok_or_else(|| AppError::bad_request("No reset request found"))?;
        let expires = u.password_otp_expires.unwrap_or(0);
        if Utc::now().timestamp_millis() > expires {
            return Err(AppError::bad_request("OTP has expired."));
        }
        if !otp::verify_otp(otp_code, &hashed) {
            return Err(AppError::bad_request("OTP is invalid."));
        }
        let mut am = u.into_active_model();
        am.password = Set(bcrypt::hash(new_password, self.bcrypt_rounds)?);
        am.password_otp = Set(None);
        am.password_otp_expires = Set(None);
        am.update(db).await?;
        Ok(())
    }
}
