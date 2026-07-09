//! MySQL-only: convert the audit `TIMESTAMP` columns to `DATETIME`.
//!
//! The create-migrations declared `created_at`/`updated_at`/`email_verified_at`
//! with `.timestamp()`, which on MySQL yields `TIMESTAMP`. The entities type
//! these as `DateTime` (`chrono::NaiveDateTime`), and sqlx-mysql maps
//! `NaiveDateTime` to `DATETIME` only — a `TIMESTAMP` column requires
//! `DateTime<Utc>`. So every read fails with:
//!   "Rust type Option<NaiveDateTime> (as SQL type DATETIME) is not compatible
//!    with SQL type TIMESTAMP"
//! e.g. priming the settings cache → the login page renders 500.
//!
//! SQLite (dev) and Postgres are unaffected: their `timestamp`/naive column
//! type already decodes into `NaiveDateTime`. Their `ALTER` semantics differ
//! (and SQLite can't `MODIFY COLUMN`), so this migration is a no-op there.
//!
//! Existing MySQL databases already ran the create-migrations as `TIMESTAMP`;
//! this ALTER converts them in place (values preserved). Fresh MySQL databases
//! create `TIMESTAMP` then get converted here — either way they converge on
//! `DATETIME`.

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::DatabaseBackend;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// (table, column, nullable) — every column the entities decode as `DateTime`.
const COLS: &[(&str, &str, bool)] = &[
    ("users", "email_verified_at", true),
    ("users", "created_at", false),
    ("users", "updated_at", false),
    ("roles", "created_at", false),
    ("roles", "updated_at", false),
    ("permissions", "created_at", false),
    ("permissions", "updated_at", false),
    ("settings", "created_at", false),
    ("settings", "updated_at", false),
];

fn col_def(name: &str, nullable: bool, use_timestamp: bool) -> ColumnDef {
    let mut def = ColumnDef::new(Alias::new(name));
    if use_timestamp {
        def.timestamp();
    } else {
        def.date_time();
    }
    if nullable {
        def.null();
    } else {
        def.not_null().default(Expr::current_timestamp());
    }
    def.to_owned()
}

async fn convert(manager: &SchemaManager<'_>, use_timestamp: bool) -> Result<(), DbErr> {
    // MySQL only — see module docs.
    if manager.get_database_backend() != DatabaseBackend::MySql {
        return Ok(());
    }
    for (table, col, nullable) in COLS {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new(*table))
                    .modify_column(col_def(col, *nullable, use_timestamp))
                    .to_owned(),
            )
            .await?;
    }
    Ok(())
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        convert(manager, false).await // → DATETIME
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        convert(manager, true).await // → TIMESTAMP
    }
}
