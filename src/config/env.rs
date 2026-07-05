//! The single source of environment configuration.
//!
//! Equivalent of NodeAdmin `src/config/env.ts`. Env var **names are kept identical** for
//! parity across ports. Type conversion + validation happen here; required secrets
//! (`SESSION_SECRET`, `JWT_SECRET`) fail-fast in production.

use std::env;
use std::path::PathBuf;

/// Application root for assets (templates/static/storage), so the app runs from ANY working
/// directory (not only the project root). Resolution order:
/// `APP_ROOT` env → the compiled crate dir (dev, via `CARGO_MANIFEST_DIR`) → current dir.
pub fn app_root() -> PathBuf {
    if let Some(r) = opt("APP_ROOT") {
        return PathBuf::from(r);
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if manifest.join("templates").is_dir() {
        return manifest;
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Resolve an asset path relative to [`app_root`].
pub fn asset(rel: &str) -> PathBuf {
    app_root().join(rel)
}

/// Storage base path (`STORAGE_BASE_PATH`, default `storage`). Modules must use this
/// accessor instead of reading the environment directly (checker-enforced).
pub fn storage_base_path() -> String {
    get("STORAGE_BASE_PATH", "storage")
}

/// The port Rocket actually binds (merged into the figment by `build_rocket`).
///
/// `APP_PORT` is authoritative — parity with NodeAdmin/GoAdmin, which listen directly on
/// `APP_PORT`. `ROCKET_PORT` stays a framework-native escape hatch that wins when set
/// (container entrypoints exported it before `APP_PORT` was honored; keeps old deploys
/// working). Without it, Rocket ignored `APP_PORT` and fell back to its own default 8000.
pub fn bind_port(app_port: u16) -> u16 {
    num("ROCKET_PORT", app_port)
}

/// Application run mode. `Full` = web UI + REST API; `Api` = REST API only (stateless).
/// Selected at runtime via `APP_MODE` so a single codebase serves both variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Full,
    Api,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub name: String,
    pub mode: AppMode,
}

#[derive(Debug, Clone)]
pub struct DbConfig {
    /// `sqlite` (dev default) | `mysql` | `postgres`.
    pub kind: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database: String,
    /// Optional full URL override (`DATABASE_URL`); wins over the parts above.
    pub url: Option<String>,
    pub logging: bool,
    pub connection_limit: u32,
}

impl DbConfig {
    /// Build a dialect-agnostic SeaORM connection URL from the configured parts.
    pub fn connection_url(&self) -> String {
        if let Some(url) = &self.url {
            return url.clone();
        }
        match self.kind.as_str() {
            "sqlite" => {
                let path = if self.database.is_empty() {
                    "rust_admin.sqlite".to_string()
                } else {
                    self.database.clone()
                };
                // `mode=rwc` creates the file if missing.
                format!("sqlite://{path}?mode=rwc")
            }
            "postgres" | "postgresql" => format!(
                "postgres://{}:{}@{}:{}/{}",
                self.username, self.password, self.host, self.port, self.database
            ),
            // default to mysql-style DSN
            _ => format!(
                "mysql://{}:{}@{}:{}/{}",
                self.username, self.password, self.host, self.port, self.database
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub secret: String,
    pub ttl_ms: i64,
}

#[derive(Debug, Clone)]
pub struct JwtConfig {
    pub secret: String,
    /// e.g. `1h`, `30m`, `7d` (mirrors NodeAdmin `JWT_EXPIRES_IN`).
    pub expires_in: String,
    /// Pinned to HS256.
    pub algorithm: &'static str,
}

impl JwtConfig {
    /// Resolve `expires_in` (`1h`/`30m`/`7d`/seconds) to seconds. Defaults to 1h.
    pub fn expires_secs(&self) -> i64 {
        parse_duration_secs(&self.expires_in).unwrap_or(3600)
    }
}

#[derive(Debug, Clone)]
pub struct SecurityConfig {
    pub bcrypt_rounds: u32,
    pub otp_expiry_ms: i64,
}

#[derive(Debug, Clone)]
pub struct MailConfig {
    pub host: String,
    pub port: u16,
    pub secure: bool,
    pub username: String,
    pub password: String,
    pub from_name: String,
    pub from_address: String,
}

#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub driver: String,
    pub base_path: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub endpoint: String,
    pub bucket: String,
    pub region: String,
    pub ssl: bool,
}

/// Root configuration. Built once via [`Config::from_env`] and shared as Rocket managed
/// state (`State<Config>`).
#[derive(Debug, Clone)]
pub struct Config {
    pub node_env: String,
    pub is_prod: bool,
    pub app: AppConfig,
    pub db: DbConfig,
    pub redis_url: String,
    pub session: SessionConfig,
    pub jwt: JwtConfig,
    pub security: SecurityConfig,
    pub mail: MailConfig,
    pub storage: StorageConfig,
    pub default_page_size: u64,
    /// Name of the role that bypasses all RBAC checks.
    pub administrator_role: String,
}

impl Config {
    /// Read + validate configuration from the process environment.
    ///
    /// # Panics
    /// In production (`NODE_ENV=production`) when a required secret is empty (fail-fast).
    pub fn from_env() -> Self {
        let node_env = get("NODE_ENV", "development");
        let is_prod = node_env == "production";

        Config {
            app: AppConfig {
                host: get("APP_HOST", "http://localhost"),
                port: num("APP_PORT", 3000),
                name: get("APP_NAME", "RustAdmin"),
                mode: if get("APP_MODE", "full") == "api" {
                    AppMode::Api
                } else {
                    AppMode::Full
                },
            },
            db: DbConfig {
                kind: get("DB_TYPE", "sqlite"),
                host: get("DB_HOST", "127.0.0.1"),
                port: num("DB_PORT", 3306),
                username: get("DB_USERNAME", ""),
                password: get("DB_PASSWORD", ""),
                database: get("DB_DATABASE", ""),
                url: opt("DATABASE_URL"),
                logging: boolean("DB_LOGGING", false),
                connection_limit: num("DB_CONNECTION_LIMIT", 10),
            },
            redis_url: get("REDIS_URL", "redis://127.0.0.1:6379"),
            session: SessionConfig {
                secret: required("SESSION_SECRET", is_prod),
                ttl_ms: num::<i64>("SESSION_TTL_HOURS", 6) * 60 * 60 * 1000,
            },
            jwt: JwtConfig {
                secret: required("JWT_SECRET", is_prod),
                expires_in: get("JWT_EXPIRES_IN", "1h"),
                algorithm: "HS256",
            },
            security: SecurityConfig {
                bcrypt_rounds: num("BCRYPT_ROUNDS", 10),
                otp_expiry_ms: num::<i64>("OTP_EXPIRY_MINUTES", 10) * 60 * 1000,
            },
            mail: MailConfig {
                host: get("MAIL_HOST", ""),
                port: num("MAIL_PORT", 587),
                secure: boolean("MAIL_SECURE", false),
                username: get("MAIL_USERNAME", ""),
                password: get("MAIL_PASSWORD", ""),
                from_name: get("MAIL_FROM_NAME", "RustAdmin"),
                from_address: get("MAIL_FROM_ADDRESS", "no-reply@example.com"),
            },
            storage: StorageConfig {
                driver: get("STORAGE_DRIVER", "local"),
                base_path: get("STORAGE_BASE_PATH", "storage"),
                access_key_id: get("STORAGE_ACCESS_KEY_ID", ""),
                secret_access_key: get("STORAGE_SECRET_ACCESS_KEY", ""),
                endpoint: get("STORAGE_ENDPOINT", ""),
                bucket: get("STORAGE_BUCKET", ""),
                region: get("STORAGE_REGION", ""),
                ssl: boolean("STORAGE_SSL", true),
            },
            default_page_size: num("DEFAULT_PAGE_SIZE", 10),
            administrator_role: get("ADMINISTRATOR_ROLE", "Administrator"),
            node_env,
            is_prod,
        }
    }
}

// --- small env helpers (mirror NodeAdmin makeEnvHelpers) ---

fn opt(key: &str) -> Option<String> {
    env::var(key).ok().filter(|v| !v.is_empty())
}

fn get(key: &str, default: &str) -> String {
    opt(key).unwrap_or_else(|| default.to_string())
}

fn num<T: std::str::FromStr>(key: &str, default: T) -> T {
    opt(key).and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn boolean(key: &str, default: bool) -> bool {
    match opt(key) {
        Some(v) => matches!(v.to_lowercase().as_str(), "1" | "true" | "yes" | "on"),
        None => default,
    }
}

/// Required secret: empty in production → fail-fast; in dev returns the empty string.
fn required(key: &str, is_prod: bool) -> String {
    match opt(key) {
        Some(v) => v,
        None if is_prod => panic!(
            "Missing required env var `{key}` in production — refusing to start with a default secret"
        ),
        None => String::new(),
    }
}

/// Parse `1h` / `30m` / `45s` / `7d` / bare-seconds → seconds.
fn parse_duration_secs(s: &str) -> Option<i64> {
    let s = s.trim();
    if let Ok(n) = s.parse::<i64>() {
        return Some(n);
    }
    let (num, unit) = s.split_at(s.len().checked_sub(1)?);
    let n: i64 = num.parse().ok()?;
    match unit {
        "s" => Some(n),
        "m" => Some(n * 60),
        "h" => Some(n * 3600),
        "d" => Some(n * 86400),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bind_port_prefers_rocket_port_env() {
        // No ROCKET_PORT → APP_PORT value is authoritative.
        env::remove_var("ROCKET_PORT");
        assert_eq!(bind_port(9000), 9000);
        // ROCKET_PORT set → framework escape hatch wins (old container entrypoints).
        env::set_var("ROCKET_PORT", "8081");
        assert_eq!(bind_port(9000), 8081);
        env::remove_var("ROCKET_PORT");
    }

    #[test]
    fn parses_durations() {
        assert_eq!(parse_duration_secs("1h"), Some(3600));
        assert_eq!(parse_duration_secs("30m"), Some(1800));
        assert_eq!(parse_duration_secs("7d"), Some(604800));
        assert_eq!(parse_duration_secs("90"), Some(90));
        assert_eq!(parse_duration_secs("nope"), None);
    }

    #[test]
    fn sqlite_url_built() {
        let db = DbConfig {
            kind: "sqlite".into(),
            host: String::new(),
            port: 0,
            username: String::new(),
            password: String::new(),
            database: "test.sqlite".into(),
            url: None,
            logging: false,
            connection_limit: 5,
        };
        assert_eq!(db.connection_url(), "sqlite://test.sqlite?mode=rwc");
    }

    #[test]
    fn url_override_wins() {
        let db = DbConfig {
            kind: "mysql".into(),
            host: "h".into(),
            port: 3306,
            username: "u".into(),
            password: "p".into(),
            database: "d".into(),
            url: Some("sqlite::memory:".into()),
            logging: false,
            connection_limit: 5,
        };
        assert_eq!(db.connection_url(), "sqlite::memory:");
    }
}
