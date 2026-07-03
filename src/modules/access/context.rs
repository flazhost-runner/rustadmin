//! Load a user's authorization context (roles → permissions) for RBAC.
//!
//! Mirrors how NodeAdmin's `AccessMiddleware` resolves the current user's permissions. Used
//! by the [`crate::guards`] to decide access. Admin = the user holds the `Administrator` role.

use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::errors::{AppError, AppResult};
use crate::modules::access::models::{permission, role, roles_permissions, user, users_roles};

/// Resolved authorization context for one user.
#[derive(Debug, Clone)]
pub struct UserContext {
    pub id: String,
    pub name: String,
    pub is_admin: bool,
    /// (route name, HTTP method-uppercased) pairs the user is granted.
    pub perms: Vec<(String, String)>,
}

/// Load the context for `user_id`. `admin_role` is the bypass role name (from config).
pub async fn load_user_context(
    db: &DatabaseConnection,
    user_id: &str,
    admin_role: &str,
) -> AppResult<UserContext> {
    let u = user::Entity::find_by_id(user_id.to_string())
        .one(db)
        .await?
        .ok_or_else(|| AppError::unauthorized("User not found"))?;

    let role_ids: Vec<String> = users_roles::Entity::find()
        .filter(users_roles::Column::UserId.eq(user_id))
        .all(db)
        .await?
        .into_iter()
        .map(|r| r.role_id)
        .collect();

    if role_ids.is_empty() {
        return Ok(UserContext {
            id: u.id,
            name: u.name,
            is_admin: false,
            perms: vec![],
        });
    }

    let roles = role::Entity::find()
        .filter(role::Column::Id.is_in(role_ids.clone()))
        .all(db)
        .await?;
    let is_admin = roles.iter().any(|r| r.name == admin_role);

    let perm_ids: Vec<String> = roles_permissions::Entity::find()
        .filter(roles_permissions::Column::RoleId.is_in(role_ids))
        .all(db)
        .await?
        .into_iter()
        .map(|rp| rp.permission_id)
        .collect();

    let perms = if perm_ids.is_empty() {
        vec![]
    } else {
        permission::Entity::find()
            .filter(permission::Column::Id.is_in(perm_ids))
            .all(db)
            .await?
            .into_iter()
            .map(|p| (p.name, p.method.unwrap_or_default().to_uppercase()))
            .collect()
    };

    Ok(UserContext {
        id: u.id,
        name: u.name,
        is_admin,
        perms,
    })
}
