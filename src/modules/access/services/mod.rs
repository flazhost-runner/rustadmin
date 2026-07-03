//! Access services (trait + impl). Shared as `State<Arc<dyn I*Service>>` (DI container).

pub mod permission_service;
pub mod role_service;
pub mod user_service;

pub use permission_service::{IPermissionService, PermissionService};
pub use role_service::{IRoleService, RoleService};
pub use user_service::{IUserService, UserService};
