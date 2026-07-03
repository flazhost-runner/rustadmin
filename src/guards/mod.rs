//! Rocket request guards — the idiomatic place for auth + RBAC (mirrors NodeAdmin
//! `ensureAuthenticated` → `AccessMiddleware`). Order is enforced by composition:
//! [`Authorized`] requires a [`CurrentUser`] first (authenticate → authorize).

pub mod authorized;
pub mod current_user;

pub use authorized::Authorized;
pub use current_user::{clear_web_session, set_web_session, CurrentUser, JwtClaims};
