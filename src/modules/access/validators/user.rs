//! User form DTO + validation (mirrors NodeAdmin UserCreate/UpdateValidator with Joi).

use std::collections::BTreeMap;

use rocket::form::FromForm;
use rocket::fs::TempFile;
use serde_json::{json, Value};

use crate::modules::access::services::user_service::{StoreUserInput, UpdateUserInput};
use crate::modules::access::validators::FormError;

/// Whitelisted user form fields. `picture` is a multipart file the controller uploads
/// (magic-byte validated) and is NOT part of the text validation.
#[derive(FromForm, Debug, Default)]
pub struct UserForm<'r> {
    pub code: Option<String>,
    pub name: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub timezone: Option<String>,
    pub password: Option<String>,
    pub password_confirmation: Option<String>,
    pub status: Option<String>,
    pub blocked: Option<String>,
    pub blocked_reason: Option<String>,
    pub roles: Vec<String>,
    pub picture: Option<TempFile<'r>>,
}

impl UserForm<'_> {
    fn trimmed(opt: &Option<String>) -> String {
        opt.as_deref().unwrap_or("").trim().to_string()
    }

    /// The `old` input object (everything except passwords) to repopulate the form.
    fn old(&self) -> Value {
        json!({
            "code": Self::trimmed(&self.code),
            "name": Self::trimmed(&self.name),
            "phone": Self::trimmed(&self.phone),
            "email": Self::trimmed(&self.email),
            "timezone": Self::trimmed(&self.timezone),
            "status": Self::trimmed(&self.status),
            "blocked_reason": Self::trimmed(&self.blocked_reason),
        })
    }

    /// Shared field checks. `require_password` differs between create (true) and update (false).
    fn check(&self, require_password: bool) -> BTreeMap<String, String> {
        let mut e = BTreeMap::new();
        if Self::trimmed(&self.code).is_empty() {
            e.insert("code".into(), "Code is required".into());
        }
        if Self::trimmed(&self.name).is_empty() {
            e.insert("name".into(), "Name is required".into());
        }
        let email = Self::trimmed(&self.email);
        if email.is_empty() {
            e.insert("email".into(), "Email is required".into());
        } else if !email.contains('@') || !email.contains('.') {
            e.insert("email".into(), "Email is invalid".into());
        }
        let pw = Self::trimmed(&self.password);
        if require_password || !pw.is_empty() {
            if pw.len() < 8 {
                e.insert(
                    "password".into(),
                    "Password must be at least 8 characters".into(),
                );
            } else if pw != Self::trimmed(&self.password_confirmation) {
                e.insert(
                    "password_confirmation".into(),
                    "Password confirmation does not match".into(),
                );
            }
        }
        if self.roles.iter().all(|r| r.trim().is_empty()) {
            e.insert("roles".into(), "Select at least one role".into());
        }
        e
    }

    fn errors_value(e: BTreeMap<String, String>) -> Value {
        let mut m = serde_json::Map::new();
        for (k, v) in e {
            m.insert(k, json!(v));
        }
        Value::Object(m)
    }

    fn roles_clean(&self) -> Vec<String> {
        self.roles
            .iter()
            .filter(|r| !r.trim().is_empty())
            .cloned()
            .collect()
    }

    /// Validate for create.
    pub fn validate_store(self) -> Result<StoreUserInput, FormError> {
        let errors = self.check(true);
        if !errors.is_empty() {
            return Err(FormError {
                errors: Self::errors_value(errors),
                old: self.old(),
            });
        }
        Ok(StoreUserInput {
            code: Self::trimmed(&self.code),
            name: Self::trimmed(&self.name),
            phone: opt(&self.phone),
            email: Self::trimmed(&self.email),
            timezone: opt(&self.timezone),
            password: Self::trimmed(&self.password),
            status: opt(&self.status),
            blocked: self.blocked.is_some(),
            blocked_reason: opt(&self.blocked_reason),
            picture: None,
            roles: self.roles_clean(),
        })
    }

    /// Validate for update (password optional).
    pub fn validate_update(self) -> Result<UpdateUserInput, FormError> {
        let errors = self.check(false);
        if !errors.is_empty() {
            return Err(FormError {
                errors: Self::errors_value(errors),
                old: self.old(),
            });
        }
        let pw = Self::trimmed(&self.password);
        Ok(UpdateUserInput {
            code: Self::trimmed(&self.code),
            name: Self::trimmed(&self.name),
            phone: opt(&self.phone),
            email: Self::trimmed(&self.email),
            timezone: opt(&self.timezone),
            password: if pw.is_empty() { None } else { Some(pw) },
            status: opt(&self.status),
            blocked: self.blocked.is_some(),
            blocked_reason: opt(&self.blocked_reason),
            picture: None,
            roles: self.roles_clean(),
        })
    }
}

fn opt(o: &Option<String>) -> Option<String> {
    let v = o.as_deref().unwrap_or("").trim().to_string();
    if v.is_empty() {
        None
    } else {
        Some(v)
    }
}
