//! Phase 3 — auth + RBAC over HTTP (real JWT, real in-memory blacklist store).
//!
//! The headline test is the NodeAdmin lesson: login → access (200) → logout → access (401),
//! exercised through a store that *behaves like the runtime one* (not an always-smooth mock).

use rocket::http::{ContentType, Header, Status};
use rocket::local::asynchronous::Client;

use rust_admin::config::Config;
use rust_admin::db;
use rust_admin::migrations::Migrator;
use rust_admin::modules::access::context::load_user_context;
use rust_admin::modules::access::models::{permission, role, roles_permissions, user, users_roles};

use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use sea_orm_migration::MigratorTrait;
use uuid::Uuid;

async fn migrated_db() -> DatabaseConnection {
    let conn = db::connect_in_memory().await.unwrap();
    Migrator::up(&conn, None).await.unwrap();
    conn
}

fn test_cfg() -> Config {
    let mut cfg = Config::from_env();
    cfg.jwt.secret = "test-jwt-secret".to_string();
    cfg.session.secret = "test-session-secret".to_string();
    cfg
}

async fn client_with(conn: DatabaseConnection) -> Client {
    let rocket = rust_admin::build_rocket_with_db(test_cfg(), conn);
    Client::tracked(rocket).await.unwrap()
}

fn bearer(token: &str) -> Header<'static> {
    Header::new("Authorization", format!("Bearer {token}"))
}

#[tokio::test]
async fn login_access_logout_blacklist_flow() {
    let client = client_with(migrated_db().await).await;

    // login
    let res = client
        .post("/api/v1/auth/login")
        .header(ContentType::JSON)
        .body(r#"{"email":"admin@admin.com","password":"12345678"}"#)
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().await.unwrap();
    let token = body["data"]["token"].as_str().unwrap().to_string();

    // access protected endpoint → 200
    let res = client
        .get("/api/v1/auth/me")
        .header(bearer(&token))
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Ok, "token should work before logout");

    // logout (blacklists jti)
    let res = client
        .post("/api/v1/auth/logout")
        .header(bearer(&token))
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Ok);

    // same token now rejected → 401 (proves blacklist actually invalidates)
    let res = client
        .get("/api/v1/auth/me")
        .header(bearer(&token))
        .dispatch()
        .await;
    assert_eq!(
        res.status(),
        Status::Unauthorized,
        "token must be rejected after logout"
    );
}

#[tokio::test]
async fn wrong_password_is_unauthorized() {
    let client = client_with(migrated_db().await).await;
    let res = client
        .post("/api/v1/auth/login")
        .header(ContentType::JSON)
        .body(r#"{"email":"admin@admin.com","password":"nope"}"#)
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Unauthorized);
}

#[tokio::test]
async fn me_without_token_is_unauthorized() {
    let client = client_with(migrated_db().await).await;
    let res = client.get("/api/v1/auth/me").dispatch().await;
    assert_eq!(res.status(), Status::Unauthorized);
}

#[tokio::test]
async fn rbac_context_loads_perms_and_admin_bypasses() {
    let conn = migrated_db().await;

    // Editor role + one permission + a user holding that role.
    let role_id = Uuid::new_v4().to_string();
    role::ActiveModel {
        id: Set(role_id.clone()),
        name: Set("Editor".into()),
        status: Set("Active".into()),
        desc: Set(None),
        ..Default::default()
    }
    .insert(&conn)
    .await
    .unwrap();

    let perm_id = Uuid::new_v4().to_string();
    permission::ActiveModel {
        id: Set(perm_id.clone()),
        name: Set("admin.v1.access.user.index".into()),
        guard_name: Set("web".into()),
        method: Set(Some("GET".into())),
        status: Set("Active".into()),
        desc: Set(None),
        ..Default::default()
    }
    .insert(&conn)
    .await
    .unwrap();

    roles_permissions::ActiveModel {
        role_id: Set(role_id.clone()),
        permission_id: Set(perm_id),
    }
    .insert(&conn)
    .await
    .unwrap();

    let user_id = Uuid::new_v4().to_string();
    user::ActiveModel {
        id: Set(user_id.clone()),
        code: Set("0000000002".into()),
        name: Set("Ed".into()),
        email: Set("ed@example.com".into()),
        password: Set("x".into()),
        status: Set("Active".into()),
        blocked: Set(false),
        ..Default::default()
    }
    .insert(&conn)
    .await
    .unwrap();

    users_roles::ActiveModel {
        user_id: Set(user_id.clone()),
        role_id: Set(role_id),
    }
    .insert(&conn)
    .await
    .unwrap();

    let ctx = load_user_context(&conn, &user_id, "Administrator")
        .await
        .unwrap();
    assert!(!ctx.is_admin, "Editor is not admin");
    assert!(
        ctx.perms
            .iter()
            .any(|(n, m)| n == "admin.v1.access.user.index" && m == "GET"),
        "editor should have the granted permission"
    );

    // seeded admin bypasses
    let admin = user::Entity::find()
        .filter(user::Column::Email.eq("admin@admin.com"))
        .one(&conn)
        .await
        .unwrap()
        .unwrap();
    let actx = load_user_context(&conn, &admin.id, "Administrator")
        .await
        .unwrap();
    assert!(actx.is_admin, "Administrator role bypasses RBAC");
}
