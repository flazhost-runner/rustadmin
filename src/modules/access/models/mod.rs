//! SeaORM entities for the access module. Column names/types match the canonical schema
//! created by the migrations (entity column types are not authoritative — migrations are).

pub mod permission;
pub mod role;
pub mod roles_permissions;
pub mod user;
pub mod users_roles;
