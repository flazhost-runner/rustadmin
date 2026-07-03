//! Migration CLI: `cargo run --bin migrate [up|down|fresh|refresh|status]` (default `up`).
//! Mirrors NodeAdmin `npm run migration:*`.

use rust_admin::config::Config;
use rust_admin::db;
use rust_admin::migrations::Migrator;
use sea_orm_migration::MigratorTrait;

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();
    let cfg = Config::from_env();
    let conn = db::connect(&cfg).await.expect("database connection failed");

    let cmd = std::env::args().nth(1).unwrap_or_else(|| "up".to_string());
    let result = match cmd.as_str() {
        "up" => Migrator::up(&conn, None).await,
        "down" => Migrator::down(&conn, Some(1)).await,
        "fresh" => Migrator::fresh(&conn).await,
        "refresh" => Migrator::refresh(&conn).await,
        "status" => Migrator::status(&conn).await,
        other => {
            eprintln!("unknown command `{other}` (expected up|down|fresh|refresh|status)");
            std::process::exit(2);
        }
    };

    match result {
        Ok(()) => println!("migrate {cmd}: ok"),
        Err(e) => {
            eprintln!("migrate {cmd}: failed — {e}");
            std::process::exit(1);
        }
    }
}
