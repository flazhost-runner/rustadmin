//! Access validators — Rocket `FromForm` DTOs + validation that yields inline field errors
//! and `old` input (anti mass-assignment: only whitelisted fields are read).

pub mod permission;
pub mod role;
pub mod user;

use serde_json::Value;

/// Validation failure carrying per-field messages + the `old` input to repopulate the form.
pub struct FormError {
    pub errors: Value,
    pub old: Value,
}
