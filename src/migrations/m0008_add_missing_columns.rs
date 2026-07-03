//! RETIRED — intentionally a no-op, kept only for migration-history compatibility.
//!
//! This migration previously added `roles.guard_name` and `settings.favicon`, but neither
//! column exists in the canonical NodeAdmin schema (`guard_name` belongs to `permissions`
//! only; the favicon is served from `settings.icon`). It also broke fresh databases twice
//! over: the seed (m0007) ran *before* this migration yet referenced `roles.guard_name`,
//! and the column-existence probe used `pragma_table_info` (SQLite-only), failing on
//! MySQL/Postgres.
//!
//! The migration must stay registered under its original name: SeaORM refuses to run when
//! an applied migration is missing from the migration files. Databases that already applied
//! the old version keep the extra columns (nullable/defaulted — harmless and ignored by the
//! entities); fresh databases get the canonical schema.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
