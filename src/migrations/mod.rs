//! Database migrations (SeaORM, reversible, dialect-portable).
//!
//! Equivalent of NodeAdmin `modules/*/migrations/*`. Uses the SeaORM schema builder (NOT
//! vendor SQL), so the same migrations run on SQLite/MySQL/Postgres. Produces the
//! **canonical schema** that must stay byte-identical across all ports (see PORTING_GUIDE).

pub use sea_orm_migration::prelude::*;

mod m0001_create_users;
mod m0002_create_roles;
mod m0003_create_permissions;
mod m0004_create_users_roles;
mod m0005_create_roles_permissions;
mod m0006_create_settings;
mod m0007_seed_admin;
mod m0008_add_missing_columns;

/// The migrator: ordered list of all migrations. Tests call `Migrator::up(&db, None)`.
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m0001_create_users::Migration),
            Box::new(m0002_create_roles::Migration),
            Box::new(m0003_create_permissions::Migration),
            Box::new(m0004_create_users_roles::Migration),
            Box::new(m0005_create_roles_permissions::Migration),
            Box::new(m0006_create_settings::Migration),
            Box::new(m0007_seed_admin::Migration),
            Box::new(m0008_add_missing_columns::Migration),
        ]
    }
}
