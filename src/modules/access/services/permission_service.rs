//! Permission service — CRUD + **auto-sync from the route registry** (no hardcoded list;
//! mirrors NodeAdmin `getAllRegisteredRoute`). Synced lazily when the Permission page opens.

use std::collections::HashSet;

use async_trait::async_trait;
use sea_orm::ActiveValue::Set;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel,
    PaginatorTrait, QueryFilter, QueryOrder,
};
use uuid::Uuid;

use crate::errors::{AppError, AppResult};
use crate::helpers::ci_like;
use crate::helpers::pagination::{PageParams, PaginationMeta};
use crate::modules::access::models::permission;
use crate::rbac::derived_permissions;

#[derive(Debug, Default, Clone)]
pub struct PermissionFilter {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub name: Option<String>,
    pub guard: Option<String>,
    pub method: Option<String>,
    pub status: Option<String>,
    pub desc: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct PermissionInput {
    pub name: String,
    pub guard_name: Option<String>,
    pub method: Option<String>,
    pub status: Option<String>,
    pub desc: Option<String>,
}

pub struct PermissionIndex {
    pub rows: Vec<permission::Model>,
    pub meta: PaginationMeta,
}

#[async_trait]
pub trait IPermissionService: Send + Sync {
    /// Upsert permissions derived from the named-route registry (idempotent).
    async fn sync_from_registry(&self, db: &DatabaseConnection) -> AppResult<usize>;
    async fn index(
        &self,
        db: &DatabaseConnection,
        filter: &PermissionFilter,
    ) -> AppResult<PermissionIndex>;
    async fn store(&self, db: &DatabaseConnection, input: PermissionInput) -> AppResult<String>;
    async fn find(&self, db: &DatabaseConnection, id: &str) -> AppResult<permission::Model>;
    async fn update(
        &self,
        db: &DatabaseConnection,
        id: &str,
        input: PermissionInput,
    ) -> AppResult<()>;
    async fn delete(&self, db: &DatabaseConnection, id: &str) -> AppResult<()>;
    async fn delete_selected(&self, db: &DatabaseConnection, ids: Vec<String>) -> AppResult<()>;
}

pub struct PermissionService;

#[async_trait]
impl IPermissionService for PermissionService {
    async fn sync_from_registry(&self, db: &DatabaseConnection) -> AppResult<usize> {
        // existing (name, method) pairs
        let existing: HashSet<(String, String)> = permission::Entity::find()
            .all(db)
            .await?
            .into_iter()
            .map(|p| (p.name, p.method.unwrap_or_default()))
            .collect();

        let mut added = 0usize;
        for perm in derived_permissions() {
            let key = (perm.name.clone(), perm.method.clone());
            if existing.contains(&key) {
                continue;
            }
            permission::ActiveModel {
                id: Set(Uuid::new_v4().to_string()),
                name: Set(perm.name),
                guard_name: Set(perm.guard),
                method: Set(Some(perm.method)),
                status: Set("Active".into()),
                desc: Set(None),
                ..Default::default()
            }
            .insert(db)
            .await?;
            added += 1;
        }
        Ok(added)
    }

    async fn index(
        &self,
        db: &DatabaseConnection,
        filter: &PermissionFilter,
    ) -> AppResult<PermissionIndex> {
        let params = PageParams::new(filter.page, filter.page_size, 10);
        let mut q = permission::Entity::find();
        if let Some(v) = ne(&filter.name) {
            q = q.filter(ci_like(permission::Column::Name, v));
        }
        if let Some(v) = ne(&filter.guard) {
            q = q.filter(permission::Column::GuardName.eq(v));
        }
        if let Some(v) = ne(&filter.method) {
            q = q.filter(permission::Column::Method.eq(v));
        }
        if let Some(v) = ne(&filter.status) {
            q = q.filter(permission::Column::Status.eq(v));
        }
        if let Some(v) = ne(&filter.desc) {
            q = q.filter(ci_like(permission::Column::Desc, v));
        }
        q = q.order_by_asc(permission::Column::Name);
        let paginator = q.paginate(db, params.page_size);
        let total = paginator.num_items().await?;
        let meta = PaginationMeta::new(total, params);
        let rows = paginator.fetch_page(meta.page - 1).await?;
        Ok(PermissionIndex { rows, meta })
    }

    async fn store(&self, db: &DatabaseConnection, input: PermissionInput) -> AppResult<String> {
        let id = Uuid::new_v4().to_string();
        permission::ActiveModel {
            id: Set(id.clone()),
            name: Set(input.name),
            guard_name: Set(input.guard_name.unwrap_or_else(|| "web".into())),
            method: Set(input.method),
            status: Set(input.status.unwrap_or_else(|| "Active".into())),
            desc: Set(input.desc),
            ..Default::default()
        }
        .insert(db)
        .await?;
        Ok(id)
    }

    async fn find(&self, db: &DatabaseConnection, id: &str) -> AppResult<permission::Model> {
        permission::Entity::find_by_id(id.to_string())
            .one(db)
            .await?
            .ok_or_else(|| AppError::not_found("Permission not found"))
    }

    async fn update(
        &self,
        db: &DatabaseConnection,
        id: &str,
        input: PermissionInput,
    ) -> AppResult<()> {
        let existing = self.find(db, id).await?;
        let mut am = existing.into_active_model();
        am.name = Set(input.name);
        if let Some(g) = input.guard_name {
            am.guard_name = Set(g);
        }
        am.method = Set(input.method);
        am.status = Set(input.status.unwrap_or_else(|| "Active".into()));
        am.desc = Set(input.desc);
        am.update(db).await?;
        Ok(())
    }

    async fn delete(&self, db: &DatabaseConnection, id: &str) -> AppResult<()> {
        self.find(db, id).await?;
        permission::Entity::delete_by_id(id.to_string())
            .exec(db)
            .await?;
        Ok(())
    }

    async fn delete_selected(&self, db: &DatabaseConnection, ids: Vec<String>) -> AppResult<()> {
        if ids.is_empty() {
            return Ok(());
        }
        permission::Entity::delete_many()
            .filter(permission::Column::Id.is_in(ids))
            .exec(db)
            .await?;
        Ok(())
    }
}

fn ne(opt: &Option<String>) -> Option<&str> {
    opt.as_deref().filter(|s| !s.trim().is_empty())
}
