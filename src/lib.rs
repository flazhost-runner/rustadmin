//! RustAdmin — library crate.
//!
//! Holds all application logic (config, errors, helpers, db, rbac, security, guards,
//! modules) so that integration tests under `tests/` and the tool binaries under
//! `src/bin/` can import it. The web/api server (`src/main.rs`) is a thin wrapper around
//! [`build_rocket`].
//!
//! Port of NodeAdmin keeping the same *concepts* via native Rust/Rocket idioms — see
//! `AGENTS.md` and `docs/PORTING_GUIDE.md` in the NodeAdmin reference.

#[macro_use]
extern crate rocket;

use std::sync::Arc;

use rocket::fairing::AdHoc;
use rocket::fs::FileServer;
use rocket::http::Status;
use rocket::request::Request;
use rocket::response::status::Custom;
use rocket::response::Redirect;
use rocket::serde::json::Json;
use rocket::{Build, Rocket};
use sea_orm::DatabaseConnection;
use serde_json::{json, Value};

pub mod config;
pub mod db;
pub mod errors;
pub mod guards;
pub mod helpers;
pub mod migrations;
pub mod modules;
pub mod rbac;
pub mod security;
pub mod site;

use config::{AppMode, Config};
use modules::access::services::{
    IPermissionService, IRoleService, IUserService, PermissionService, RoleService, UserService,
};
use modules::auth::service::{AuthService, IAuthService};
use modules::home::{FeCatalogService, IFeCatalogService};
use modules::profile::service::{IProfileService, ProfileService};
use modules::setting::services::{ISettingService, SettingService};
use security::blacklist::{InMemoryTokenStore, TokenStore};
use security::headers::{HtmlContentType, SecurityHeaders};
use security::method_override::MethodOverride;
use security::rate_limit::{AuthLimiter, OtpLimiter};

/// Health endpoint — always mounted in both `full` and `api` modes.
#[get("/healthz")]
fn healthz() -> &'static str {
    "ok"
}

/// Catcher response: a web redirect or an API JSON error (a guard can't redirect directly,
/// so unauthenticated/forbidden statuses are mapped here — mirrors NodeAdmin
/// `ensureAuthenticated` which `res.redirect('/auth/login')` for web, JSON 401 for api).
#[derive(Responder)]
#[allow(clippy::large_enum_variant)]
enum CatchResponse {
    Web(Redirect),
    Api(Custom<Json<Value>>),
}

fn is_api(req: &Request<'_>) -> bool {
    req.uri().path().as_str().starts_with("/api")
}

/// 401 → web redirects to the login page; api returns JSON.
#[catch(401)]
fn unauthorized(req: &Request<'_>) -> CatchResponse {
    if is_api(req) {
        CatchResponse::Api(Custom(
            Status::Unauthorized,
            Json(json!({ "status": false, "message": "Unauthenticated", "data": null })),
        ))
    } else {
        CatchResponse::Web(Redirect::to("/auth/login"))
    }
}

/// 403 → web sends the (authenticated) user back to the dashboard; api returns JSON.
#[catch(403)]
fn forbidden(req: &Request<'_>) -> CatchResponse {
    if is_api(req) {
        CatchResponse::Api(Custom(
            Status::Forbidden,
            Json(json!({ "status": false, "message": "Forbidden", "data": null })),
        ))
    } else {
        CatchResponse::Web(Redirect::to("/admin/v1/dashboard"))
    }
}

/// Build the Rocket instance from the environment, connecting the DB on ignite.
pub fn build_rocket() -> Rocket<Build> {
    let _ = dotenvy::dotenv();
    let cfg = Config::from_env();
    assemble(cfg, None)
}

/// Build the Rocket instance with a pre-made DB connection (used by integration tests with
/// an in-memory, already-migrated database).
pub fn build_rocket_with_db(cfg: Config, db: DatabaseConnection) -> Rocket<Build> {
    assemble(cfg, Some(db))
}

fn assemble(cfg: Config, db: Option<DatabaseConnection>) -> Rocket<Build> {
    let mode = cfg.app.mode;

    // Resolve the template dir absolutely so the app runs from ANY working directory
    // (rocket_dyn_templates defaults to a CWD-relative "templates"). Static/storage dirs are
    // resolved the same way below via `config::asset`.
    let figment = rocket::Config::figment().merge((
        "template_dir",
        config::asset("templates").to_string_lossy().to_string(),
    ));

    // DI container ≈ Rocket managed state. Services are shared as trait objects.
    let token_store: Arc<dyn TokenStore> = Arc::new(InMemoryTokenStore::new());
    let auth_limiter = AuthLimiter::new();
    let otp_limiter = OtpLimiter::new();
    let auth_service: Arc<dyn IAuthService> = Arc::new(AuthService::new(
        cfg.security.bcrypt_rounds,
        cfg.security.otp_expiry_ms,
    ));
    let user_service: Arc<dyn IUserService> = Arc::new(UserService);
    let role_service: Arc<dyn IRoleService> = Arc::new(RoleService);
    let permission_service: Arc<dyn IPermissionService> = Arc::new(PermissionService);
    let setting_service: Arc<dyn ISettingService> = Arc::new(SettingService);
    let profile_service: Arc<dyn IProfileService> = Arc::new(ProfileService);
    let fe_catalog: Arc<dyn IFeCatalogService> = Arc::new(FeCatalogService::new());

    let mut rocket = rocket::custom(figment)
        .manage(cfg)
        .manage(token_store)
        .manage(auth_limiter)
        .manage(otp_limiter)
        .manage(auth_service)
        .manage(user_service)
        .manage(role_service)
        .manage(permission_service)
        .manage(setting_service)
        .manage(profile_service)
        .manage(fe_catalog)
        .attach(SecurityHeaders)
        .attach(HtmlContentType)
        .attach(MethodOverride)
        .register("/", catchers![unauthorized, forbidden])
        .mount("/", routes![healthz])
        .mount("/api/v1/auth", modules::auth::routes::api::routes())
        .mount("/api/v1", modules::access::routes::api::routes())
        .mount(
            "/api/v1",
            routes![modules::profile::api::index, modules::setting::api::index,],
        );

    // DB: inject (tests) or connect on ignite (server).
    match db {
        Some(conn) => {
            rocket = rocket.manage(conn);
        }
        None => {
            rocket = rocket.attach(AdHoc::try_on_ignite("Database", |rocket| async move {
                let cfg = rocket.state::<Config>().expect("config managed").clone();
                let conn = match db::connect(&cfg).await {
                    Ok(conn) => conn,
                    Err(e) => {
                        error!("database connection failed: {e}");
                        return Err(rocket);
                    }
                };
                // Auto-migrate in dev so `cargo run` is self-bootstrapping (idempotent).
                // In production run `cargo run --bin migrate up` explicitly.
                if !cfg.is_prod {
                    use sea_orm_migration::MigratorTrait;
                    if let Err(e) = migrations::Migrator::up(&conn, None).await {
                        error!("auto-migration failed: {e}");
                        return Err(rocket);
                    }
                    info!("database migrated (dev auto-migrate)");
                }
                Ok(rocket.manage(conn))
            }));
        }
    }

    // Web-only layer (skipped purely-additively in API-only mode).
    if mode == AppMode::Full {
        rocket = rocket
            .mount("/", modules::home::controllers::routes())
            .mount("/", modules::auth::routes::web::routes())
            .mount("/admin/v1", modules::access::routes::web::routes())
            .mount("/admin/v1", modules::setting::controllers::routes())
            .mount("/admin/v1", modules::dashboard::controllers::routes())
            .mount("/admin/v1", modules::components::controllers::routes())
            .mount("/admin/v1", modules::profile::controllers::routes())
            .mount("/admin/v1", modules::media::controllers::routes())
            .mount(
                "/be/default",
                FileServer::from(config::asset("static/be/default")).rank(10),
            )
            .mount(
                "/static",
                FileServer::from(config::asset("static")).rank(11),
            )
            .mount(
                "/storage",
                FileServer::from(config::asset("storage")).rank(12),
            )
            .attach(helpers::view::template_fairing());
    }

    rocket
        .attach(AdHoc::on_liftoff("PrimeSetting", |rocket| {
            Box::pin(async move {
                if let (Some(db), Some(svc)) = (
                    rocket.state::<DatabaseConnection>(),
                    rocket.state::<Arc<dyn ISettingService>>(),
                ) {
                    if let Err(e) = svc.get(db).await {
                        warn!("could not prime site setting cache: {e}");
                    }
                }
            })
        }))
        .attach(AdHoc::on_liftoff("Banner", |rocket| {
            Box::pin(async move {
                let rc = rocket.config();
                info!("RustAdmin listening on {}:{}", rc.address, rc.port);
            })
        }))
}
