//! Role service — CRUD + per-role permission assignment (mirrors NodeAdmin RoleService).

use std::collections::HashSet;

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
use crate::modules::access::models::{permission, role, roles_permissions};

#[derive(Debug, Default, Clone)]
pub struct RoleFilter {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub name: Option<String>,
    pub status: Option<String>,
    pub desc: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct RoleInput {
    pub name: String,
    pub status: Option<String>,
    pub desc: Option<String>,
}

pub struct RoleIndex {
    pub rows: Vec<role::Model>,
    pub meta: PaginationMeta,
}

/// Permission-assignment listing for one role.
pub struct RolePermIndex {
    pub role: role::Model,
    pub rows: Vec<Value>,
    pub meta: PaginationMeta,
}

#[async_trait]
pub trait IRoleService: Send + Sync {
    async fn index(&self, db: &DatabaseConnection, filter: &RoleFilter) -> AppResult<RoleIndex>;
    async fn store(&self, db: &DatabaseConnection, input: RoleInput) -> AppResult<String>;
    async fn find(&self, db: &DatabaseConnection, id: &str) -> AppResult<role::Model>;
    async fn update(&self, db: &DatabaseConnection, id: &str, input: RoleInput) -> AppResult<()>;
    async fn delete(&self, db: &DatabaseConnection, id: &str) -> AppResult<()>;
    async fn delete_selected(&self, db: &DatabaseConnection, ids: Vec<String>) -> AppResult<()>;

    // per-role permission management
    async fn list_permissions(
        &self,
        db: &DatabaseConnection,
        role_id: &str,
        filter: &PermAssignFilter,
    ) -> AppResult<RolePermIndex>;
    async fn assign(&self, db: &DatabaseConnection, role_id: &str, perm_id: &str) -> AppResult<()>;
    async fn unassign(
        &self,
        db: &DatabaseConnection,
        role_id: &str,
        perm_id: &str,
    ) -> AppResult<()>;
    async fn assign_selected(
        &self,
        db: &DatabaseConnection,
        role_id: &str,
        ids: Vec<String>,
    ) -> AppResult<()>;
    async fn unassign_selected(
        &self,
        db: &DatabaseConnection,
        role_id: &str,
        ids: Vec<String>,
    ) -> AppResult<()>;
}

#[derive(Debug, Default, Clone)]
pub struct PermAssignFilter {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub name: Option<String>,
    /// Active = assigned, Inactive = not assigned.
    pub status: Option<String>,
    pub desc: Option<String>,
}

pub struct RoleService;

#[async_trait]
impl IRoleService for RoleService {
    async fn index(&self, db: &DatabaseConnection, filter: &RoleFilter) -> AppResult<RoleIndex> {
        let params = PageParams::new(filter.page, filter.page_size, 10);
        let mut q = role::Entity::find();
        if let Some(v) = ne(&filter.name) {
            q = q.filter(ci_like(role::Column::Name, v));
        }
        if let Some(v) = ne(&filter.status) {
            q = q.filter(role::Column::Status.eq(v));
        }
        if let Some(v) = ne(&filter.desc) {
            q = q.filter(ci_like(role::Column::Desc, v));
        }
        q = q.order_by_asc(role::Column::Name);
        let paginator = q.paginate(db, params.page_size);
        let total = paginator.num_items().await?;
        let meta = PaginationMeta::new(total, params);
        let rows = paginator.fetch_page(meta.page - 1).await?;
        Ok(RoleIndex { rows, meta })
    }

    async fn store(&self, db: &DatabaseConnection, input: RoleInput) -> AppResult<String> {
        let id = Uuid::new_v4().to_string();
        role::ActiveModel {
            id: Set(id.clone()),
            name: Set(input.name),
            status: Set(input.status.unwrap_or_else(|| "Active".into())),
            desc: Set(input.desc),
            ..Default::default()
        }
        .insert(db)
        .await?;
        Ok(id)
    }

    async fn find(&self, db: &DatabaseConnection, id: &str) -> AppResult<role::Model> {
        role::Entity::find_by_id(id.to_string())
            .one(db)
            .await?
            .ok_or_else(|| AppError::not_found("Role not found"))
    }

    async fn update(&self, db: &DatabaseConnection, id: &str, input: RoleInput) -> AppResult<()> {
        let existing = self.find(db, id).await?;
        let mut am = existing.into_active_model();
        am.name = Set(input.name);
        am.status = Set(input.status.unwrap_or_else(|| "Active".into()));
        am.desc = Set(input.desc);
        am.update(db).await?;
        Ok(())
    }

    async fn delete(&self, db: &DatabaseConnection, id: &str) -> AppResult<()> {
        self.find(db, id).await?;
        roles_permissions::Entity::delete_many()
            .filter(roles_permissions::Column::RoleId.eq(id))
            .exec(db)
            .await?;
        role::Entity::delete_by_id(id.to_string()).exec(db).await?;
        Ok(())
    }

    async fn delete_selected(&self, db: &DatabaseConnection, ids: Vec<String>) -> AppResult<()> {
        if ids.is_empty() {
            return Ok(());
        }
        roles_permissions::Entity::delete_many()
            .filter(roles_permissions::Column::RoleId.is_in(ids.clone()))
            .exec(db)
            .await?;
        role::Entity::delete_many()
            .filter(role::Column::Id.is_in(ids))
            .exec(db)
            .await?;
        Ok(())
    }

    async fn list_permissions(
        &self,
        db: &DatabaseConnection,
        role_id: &str,
        filter: &PermAssignFilter,
    ) -> AppResult<RolePermIndex> {
        let role = self.find(db, role_id).await?;
        let params = PageParams::new(filter.page, filter.page_size, 10);

        let assigned: HashSet<String> = roles_permissions::Entity::find()
            .filter(roles_permissions::Column::RoleId.eq(role_id))
            .all(db)
            .await?
            .into_iter()
            .map(|x| x.permission_id)
            .collect();

        let mut q = permission::Entity::find();
        if let Some(v) = ne(&filter.name) {
            q = q.filter(ci_like(permission::Column::Name, v));
        }
        if let Some(v) = ne(&filter.desc) {
            q = q.filter(ci_like(permission::Column::Desc, v));
        }
        // assigned filter via id set
        if let Some(st) = ne(&filter.status) {
            let ids: Vec<String> = assigned.iter().cloned().collect();
            if st == "Active" {
                q = q.filter(permission::Column::Id.is_in(if ids.is_empty() {
                    vec!["__none__".to_string()]
                } else {
                    ids
                }));
            } else if st == "Inactive" && !ids.is_empty() {
                q = q.filter(permission::Column::Id.is_not_in(ids));
            }
        }
        q = q.order_by_asc(permission::Column::Name);

        let paginator = q.paginate(db, params.page_size);
        let total = paginator.num_items().await?;
        let meta = PaginationMeta::new(total, params);
        let perms = paginator.fetch_page(meta.page - 1).await?;

        let rows = perms
            .into_iter()
            .map(|p| {
                let is_assigned = assigned.contains(&p.id);
                let mut v = serde_json::to_value(&p).unwrap_or_else(|_| json!({}));
                v["assigned"] = json!(is_assigned);
                v
            })
            .collect();

        Ok(RolePermIndex { role, rows, meta })
    }

    async fn assign(&self, db: &DatabaseConnection, role_id: &str, perm_id: &str) -> AppResult<()> {
        self.find(db, role_id).await?;
        let exists =
            roles_permissions::Entity::find_by_id((role_id.to_string(), perm_id.to_string()))
                .one(db)
                .await?;
        if exists.is_none() {
            roles_permissions::ActiveModel {
                role_id: Set(role_id.to_string()),
                permission_id: Set(perm_id.to_string()),
            }
            .insert(db)
            .await?;
        }
        Ok(())
    }

    async fn unassign(
        &self,
        db: &DatabaseConnection,
        role_id: &str,
        perm_id: &str,
    ) -> AppResult<()> {
        roles_permissions::Entity::delete_many()
            .filter(roles_permissions::Column::RoleId.eq(role_id))
            .filter(roles_permissions::Column::PermissionId.eq(perm_id))
            .exec(db)
            .await?;
        Ok(())
    }

    async fn assign_selected(
        &self,
        db: &DatabaseConnection,
        role_id: &str,
        ids: Vec<String>,
    ) -> AppResult<()> {
        for pid in ids {
            self.assign(db, role_id, &pid).await?;
        }
        Ok(())
    }

    async fn unassign_selected(
        &self,
        db: &DatabaseConnection,
        role_id: &str,
        ids: Vec<String>,
    ) -> AppResult<()> {
        if ids.is_empty() {
            return Ok(());
        }
        roles_permissions::Entity::delete_many()
            .filter(roles_permissions::Column::RoleId.eq(role_id))
            .filter(roles_permissions::Column::PermissionId.is_in(ids))
            .exec(db)
            .await?;
        Ok(())
    }
}

fn ne(opt: &Option<String>) -> Option<&str> {
    opt.as_deref().filter(|s| !s.trim().is_empty())
}
