//! Database connection (SeaORM, dialect-agnostic).
//!
//! Equivalent of NodeAdmin `src/config/ormconfig.ts`. The dialect (sqlite/mysql/postgres)
//! is chosen at runtime from [`Config`] — no code change to switch DB. Tests use SQLite
//! in-memory for speed (see `tests/`).

use std::time::Duration;

use sea_orm::{ConnectOptions, Database, DatabaseConnection, DbErr};

use crate::config::Config;

/// Connect using the configured DSN + pool settings.
pub async fn connect(cfg: &Config) -> Result<DatabaseConnection, DbErr> {
    connect_url(
        &cfg.db.connection_url(),
        cfg.db.connection_limit,
        cfg.db.logging,
    )
    .await
}

/// Connect to an explicit URL (used by tests, e.g. `sqlite::memory:`).
pub async fn connect_url(
    url: &str,
    max_conn: u32,
    logging: bool,
) -> Result<DatabaseConnection, DbErr> {
    let mut opt = ConnectOptions::new(url.to_owned());
    opt.max_connections(max_conn.max(1))
        .min_connections(1)
        .connect_timeout(Duration::from_secs(8))
        .sqlx_logging(logging);
    Database::connect(opt).await
}

/// Open a fresh in-memory SQLite connection (test convenience).
pub async fn connect_in_memory() -> Result<DatabaseConnection, DbErr> {
    connect_url("sqlite::memory:", 1, false).await
}
