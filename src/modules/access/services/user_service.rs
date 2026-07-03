//! User service — business logic for the User resource (mirrors NodeAdmin `UserService`).
//! `paginate` + `ci_like` filters; roles batch-loaded (no N+1); `throw` via `AppError`.

use std::collections::HashMap;

use async_trait::async_trait;
use sea_orm::ActiveValue::Set;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel,
    PaginatorTrait, QueryFilter, QueryOrder,
};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::errors::{AppError, AppResult};
use crate::helpers::ci_like;
use crate::helpers::pagination::{PageParams, PaginationMeta};
use crate::modules::access::models::{role, user, users_roles};

/// Per-column filter (q_* without prefix). Built by the controller from query params.
#[derive(Debug, Default, Clone)]
pub struct UserFilter {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub code: Option<String>,
    pub name: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub status: Option<String>,
    pub role: Option<String>,
}

/// Validated input for store/update (produced by the validator).
#[derive(Debug, Default, Clone)]
pub struct StoreUserInput {
    pub code: String,
    pub name: String,
    pub phone: Option<String>,
    pub email: String,
    pub timezone: Option<String>,
    pub password: String,
    pub status: Option<String>,
    pub blocked: bool,
    pub blocked_reason: Option<String>,
    pub picture: Option<String>,
    pub roles: Vec<String>,
}

#[derive(Debug, Default, Clone)]
pub struct UpdateUserInput {
    pub code: String,
    pub name: String,
    pub phone: Option<String>,
    pub email: String,
    pub timezone: Option<String>,
    /// Empty/None = keep existing password.
    pub password: Option<String>,
    pub status: Option<String>,
    pub blocked: bool,
    pub blocked_reason: Option<String>,
    pub picture: Option<String>,
    pub roles: Vec<String>,
}

/// Index result: serialized user rows (with `roles`), pagination meta, and all roles (filter).
pub struct UserIndex {
    pub rows: Vec<Value>,
    pub meta: PaginationMeta,
    pub roles: Vec<role::Model>,
}

#[async_trait]
pub trait IUserService: Send + Sync {
    async fn index(&self, db: &DatabaseConnection, filter: &UserFilter) -> AppResult<UserIndex>;
    async fn all_roles(&self, db: &DatabaseConnection) -> AppResult<Vec<role::Model>>;
    async fn store(&self, db: &DatabaseConnection, input: StoreUserInput) -> AppResult<String>;
    async fn edit(
        &self,
        db: &DatabaseConnection,
        id: &str,
    ) -> AppResult<(user::Model, Vec<String>, Vec<role::Model>)>;
    async fn update(
        &self,
        db: &DatabaseConnection,
        id: &str,
        input: UpdateUserInput,
    ) -> AppResult<()>;
    async fn delete(&self, db: &DatabaseConnection, id: &str) -> AppResult<()>;
    async fn delete_selected(&self, db: &DatabaseConnection, ids: Vec<String>) -> AppResult<()>;
}

pub struct UserService;

#[async_trait]
impl IUserService for UserService {
    async fn index(&self, db: &DatabaseConnection, filter: &UserFilter) -> AppResult<UserIndex> {
        let params = PageParams::new(filter.page, filter.page_size, 10);

        let mut q = user::Entity::find();
        if let Some(v) = nonempty(&filter.code) {
            q = q.filter(ci_like(user::Column::Code, v));
        }
        if let Some(v) = nonempty(&filter.name) {
            q = q.filter(ci_like(user::Column::Name, v));
        }
        if let Some(v) = nonempty(&filter.phone) {
            q = q.filter(ci_like(user::Column::Phone, v));
        }
        if let Some(v) = nonempty(&filter.email) {
            q = q.filter(ci_like(user::Column::Email, v));
        }
        if let Some(v) = nonempty(&filter.status) {
            q = q.filter(user::Column::Status.eq(v));
        }
        if let Some(rid) = nonempty(&filter.role) {
            let uids: Vec<String> = users_roles::Entity::find()
                .filter(users_roles::Column::RoleId.eq(rid))
                .all(db)
                .await?
                .into_iter()
                .map(|x| x.user_id)
                .collect();
            // empty → match nothing
            q = q.filter(user::Column::Id.is_in(if uids.is_empty() {
                vec!["__none__".to_string()]
            } else {
                uids
            }));
        }
        q = q.order_by_desc(user::Column::CreatedAt);

        let paginator = q.paginate(db, params.page_size);
        let total = paginator.num_items().await?;
        let meta = PaginationMeta::new(total, params);
        let users = paginator.fetch_page(meta.page - 1).await?;

        let rows = attach_roles(db, &users).await?;
        let roles = self.all_roles(db).await?;
        Ok(UserIndex { rows, meta, roles })
    }

    async fn all_roles(&self, db: &DatabaseConnection) -> AppResult<Vec<role::Model>> {
        Ok(role::Entity::find()
            .order_by_asc(role::Column::Name)
            .all(db)
            .await?)
    }

    async fn store(&self, db: &DatabaseConnection, input: StoreUserInput) -> AppResult<String> {
        let roles = resolve_roles(db, &input.roles).await?;
        let id = Uuid::new_v4().to_string();
        let hashed = bcrypt::hash(&input.password, 10)?;

        user::ActiveModel {
            id: Set(id.clone()),
            code: Set(input.code),
            name: Set(input.name),
            phone: Set(input.phone),
            email: Set(input.email),
            password: Set(hashed),
            status: Set(input.status.unwrap_or_else(|| "Active".into())),
            timezone: Set(input.timezone.or_else(|| Some("UTC".into()))),
            blocked: Set(input.blocked),
            blocked_reason: Set(input.blocked_reason),
            picture: Set(input.picture),
            ..Default::default()
        }
        .insert(db)
        .await?;

        link_roles(db, &id, &roles).await?;
        Ok(id)
    }

    async fn edit(
        &self,
        db: &DatabaseConnection,
        id: &str,
    ) -> AppResult<(user::Model, Vec<String>, Vec<role::Model>)> {
        let u = user::Entity::find_by_id(id.to_string())
            .one(db)
            .await?
            .ok_or_else(|| AppError::not_found("User not found"))?;
        let role_ids = users_roles::Entity::find()
            .filter(users_roles::Column::UserId.eq(id))
            .all(db)
            .await?
            .into_iter()
            .map(|x| x.role_id)
            .collect();
        let all = self.all_roles(db).await?;
        Ok((u, role_ids, all))
    }

    async fn update(
        &self,
        db: &DatabaseConnection,
        id: &str,
        input: UpdateUserInput,
    ) -> AppResult<()> {
        let existing = user::Entity::find_by_id(id.to_string())
            .one(db)
            .await?
            .ok_or_else(|| AppError::not_found("User not found"))?;
        let roles = resolve_roles(db, &input.roles).await?;

        let mut am = existing.into_active_model();
        am.code = Set(input.code);
        am.name = Set(input.name);
        am.phone = Set(input.phone);
        am.email = Set(input.email);
        am.status = Set(input.status.unwrap_or_else(|| "Active".into()));
        am.timezone = Set(input.timezone.or_else(|| Some("UTC".into())));
        am.blocked = Set(input.blocked);
        am.blocked_reason = Set(input.blocked_reason);
        if let Some(pw) = input.password.filter(|p| !p.is_empty()) {
            am.password = Set(bcrypt::hash(&pw, 10)?);
        }
        if let Some(pic) = input.picture {
            am.picture = Set(Some(pic));
        }
        am.update(db).await?;

        users_roles::Entity::delete_many()
            .filter(users_roles::Column::UserId.eq(id))
            .exec(db)
            .await?;
        link_roles(db, id, &roles).await?;
        Ok(())
    }

    async fn delete(&self, db: &DatabaseConnection, id: &str) -> AppResult<()> {
        let exists = user::Entity::find_by_id(id.to_string()).one(db).await?;
        if exists.is_none() {
            return Err(AppError::not_found("User not found"));
        }
        users_roles::Entity::delete_many()
            .filter(users_roles::Column::UserId.eq(id))
            .exec(db)
            .await?;
        user::Entity::delete_by_id(id.to_string()).exec(db).await?;
        Ok(())
    }

    async fn delete_selected(&self, db: &DatabaseConnection, ids: Vec<String>) -> AppResult<()> {
        if ids.is_empty() {
            return Ok(());
        }
        users_roles::Entity::delete_many()
            .filter(users_roles::Column::UserId.is_in(ids.clone()))
            .exec(db)
            .await?;
        user::Entity::delete_many()
            .filter(user::Column::Id.is_in(ids))
            .exec(db)
            .await?;
        Ok(())
    }
}

// --- helpers ---

fn nonempty(opt: &Option<String>) -> Option<&str> {
    opt.as_deref().filter(|s| !s.trim().is_empty())
}

async fn resolve_roles(db: &DatabaseConnection, ids: &[String]) -> AppResult<Vec<role::Model>> {
    let roles = role::Entity::find()
        .filter(role::Column::Id.is_in(ids.to_vec()))
        .all(db)
        .await?;
    if roles.is_empty() {
        return Err(AppError::not_found("Roles Not Found"));
    }
    Ok(roles)
}

async fn link_roles(
    db: &DatabaseConnection,
    user_id: &str,
    roles: &[role::Model],
) -> AppResult<()> {
    for r in roles {
        users_roles::ActiveModel {
            user_id: Set(user_id.to_string()),
            role_id: Set(r.id.clone()),
        }
        .insert(db)
        .await?;
    }
    Ok(())
}

/// Serialize users with an embedded `roles` array (batch-loaded — no N+1).
async fn attach_roles(db: &DatabaseConnection, users: &[user::Model]) -> AppResult<Vec<Value>> {
    let uids: Vec<String> = users.iter().map(|u| u.id.clone()).collect();
    let links = if uids.is_empty() {
        vec![]
    } else {
        users_roles::Entity::find()
            .filter(users_roles::Column::UserId.is_in(uids))
            .all(db)
            .await?
    };
    let rids: Vec<String> = links.iter().map(|l| l.role_id.clone()).collect();
    let roles_by_id: HashMap<String, role::Model> = if rids.is_empty() {
        HashMap::new()
    } else {
        role::Entity::find()
            .filter(role::Column::Id.is_in(rids))
            .all(db)
            .await?
            .into_iter()
            .map(|r| (r.id.clone(), r))
            .collect()
    };

    let mut rows = Vec::with_capacity(users.len());
    for u in users {
        let role_arr: Vec<Value> = links
            .iter()
            .filter(|l| l.user_id == u.id)
            .filter_map(|l| roles_by_id.get(&l.role_id))
            .map(|r| json!({ "id": r.id, "name": r.name }))
            .collect();
        let mut uv = serde_json::to_value(u).unwrap_or_else(|_| json!({}));
        uv["roles"] = Value::Array(role_arr);
        rows.push(uv);
    }
    Ok(rows)
}
