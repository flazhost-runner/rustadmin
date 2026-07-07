//! Phase 5 — Role + Permission resources: CRUD, registry auto-sync, and per-role
//! permission assignment (web + api).

use rocket::http::{ContentType, Cookie, Header, Status};
use rocket::local::asynchronous::Client;

use rust_admin::config::Config;
use rust_admin::db;
use rust_admin::migrations::Migrator;
use rust_admin::modules::access::models::{permission, role, roles_permissions, user};

use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter};
use sea_orm_migration::MigratorTrait;

async fn setup() -> (Client, DatabaseConnection, String, String) {
    let conn = db::connect_in_memory().await.unwrap();
    Migrator::up(&conn, None).await.unwrap();
    let mut cfg = Config::from_env();
    cfg.jwt.secret = "test-jwt".into();
    cfg.session.secret = "test-session".into();
    let client = Client::tracked(rust_admin::build_rocket_with_db(cfg, conn.clone()))
        .await
        .unwrap();
    let admin = user::Entity::find()
        .filter(user::Column::Email.eq("admin@admin.com"))
        .one(&conn)
        .await
        .unwrap()
        .unwrap();
    let admin_role = role::Entity::find()
        .filter(role::Column::Name.eq("Administrator"))
        .one(&conn)
        .await
        .unwrap()
        .unwrap();
    (client, conn, admin.id, admin_role.id)
}

fn admin_cookie(id: &str) -> Cookie<'static> {
    Cookie::new("uid", id.to_string())
}

#[tokio::test]
async fn web_role_index_and_create() {
    let (client, db, admin_id, _role) = setup().await;

    let res = client
        .get("/admin/v1/access/role")
        .private_cookie(admin_cookie(&admin_id))
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Ok);
    let body = res.into_string().await.unwrap();
    assert!(body.contains("Role List"));
    assert!(body.contains("Administrator"));

    // create a role
    let res = client
        .post("/admin/v1/access/role/store?_csrf=tok")
        .private_cookie(admin_cookie(&admin_id))
        .private_cookie(Cookie::new("csrf_token", "tok"))
        .header(ContentType::Form)
        .body("name=Editor&status=Active&desc=Edits%20content")
        .dispatch()
        .await;
    assert!((300..400).contains(&res.status().code));
    let made = role::Entity::find()
        .filter(role::Column::Name.eq("Editor"))
        .one(&db)
        .await
        .unwrap();
    assert!(made.is_some());
}

#[tokio::test]
async fn permission_index_syncs_from_registry() {
    let (client, db, admin_id, _role) = setup().await;
    assert_eq!(
        permission::Entity::find().count(&db).await.unwrap(),
        0,
        "no permissions before first visit"
    );

    let res = client
        .get("/admin/v1/access/permission")
        .private_cookie(admin_cookie(&admin_id))
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Ok);
    assert!(res.into_string().await.unwrap().contains("Permission List"));

    // lazy auto-sync populated permissions from the route registry
    let count = permission::Entity::find().count(&db).await.unwrap();
    assert!(count > 20, "registry permissions synced, got {count}");

    // idempotent: visiting again does not duplicate
    client
        .get("/admin/v1/access/permission")
        .private_cookie(admin_cookie(&admin_id))
        .dispatch()
        .await;
    assert_eq!(permission::Entity::find().count(&db).await.unwrap(), count);
}

#[tokio::test]
async fn role_permission_assign_and_unassign() {
    let (client, db, admin_id, role_id) = setup().await;

    // sync permissions first (open permission page)
    client
        .get("/admin/v1/access/permission")
        .private_cookie(admin_cookie(&admin_id))
        .dispatch()
        .await;
    let perm = permission::Entity::find().one(&db).await.unwrap().unwrap();

    // permission management page renders
    let res = client
        .get(format!("/admin/v1/access/role/{role_id}/permission"))
        .private_cookie(admin_cookie(&admin_id))
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Ok);
    assert!(res.into_string().await.unwrap().contains("Permission List"));

    // assign (GET, per NodeAdmin spec)
    let res = client
        .get(format!(
            "/admin/v1/access/role/{role_id}/permission/{}/assign",
            perm.id
        ))
        .private_cookie(admin_cookie(&admin_id))
        .dispatch()
        .await;
    assert!((300..400).contains(&res.status().code));
    let linked = roles_permissions::Entity::find()
        .filter(roles_permissions::Column::RoleId.eq(role_id.clone()))
        .filter(roles_permissions::Column::PermissionId.eq(perm.id.clone()))
        .count(&db)
        .await
        .unwrap();
    assert_eq!(linked, 1, "permission assigned to role");

    // unassign
    client
        .get(format!(
            "/admin/v1/access/role/{role_id}/permission/{}/unassign",
            perm.id
        ))
        .private_cookie(admin_cookie(&admin_id))
        .dispatch()
        .await;
    let linked = roles_permissions::Entity::find()
        .filter(roles_permissions::Column::RoleId.eq(role_id))
        .filter(roles_permissions::Column::PermissionId.eq(perm.id))
        .count(&db)
        .await
        .unwrap();
    assert_eq!(linked, 0, "permission unassigned");
}

#[tokio::test]
async fn api_role_and_permission_list() {
    let (client, _db, _admin, _role) = setup().await;
    // login
    let res = client
        .post("/api/v1/auth/login")
        .header(ContentType::JSON)
        .body(r#"{"email":"admin@admin.com","password":"12345678"}"#)
        .dispatch()
        .await;
    let token = res.into_json::<serde_json::Value>().await.unwrap()["data"]["token"]
        .as_str()
        .unwrap()
        .to_string();
    let bearer = Header::new("Authorization", format!("Bearer {token}"));

    let res = client
        .get("/api/v1/access/role")
        .header(bearer.clone())
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Ok);

    let res = client
        .get("/api/v1/access/permission")
        .header(bearer)
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Ok);
    let v: serde_json::Value = res.into_json().await.unwrap();
    // Canonical NodeAdmin list envelope: { status, message, data: { datas, paginate_data } }.
    assert_eq!(v["status"], serde_json::json!(true));
    assert!(
        !v["data"]["datas"].as_array().unwrap().is_empty(),
        "api synced permissions"
    );
    assert!(
        v["data"]["paginate_data"]["total_data"].as_u64().unwrap() > 0,
        "paginate_data present"
    );
}

// Parity with the NodeAdmin fix: `edit` on a missing id must 404 (canonical envelope) for both
// role and permission, never 200 with an empty body.
#[tokio::test]
async fn api_edit_missing_id_returns_404() {
    let (client, _db, _admin, _role) = setup().await;
    let res = client
        .post("/api/v1/auth/login")
        .header(ContentType::JSON)
        .body(r#"{"email":"admin@admin.com","password":"12345678"}"#)
        .dispatch()
        .await;
    let token = res.into_json::<serde_json::Value>().await.unwrap()["data"]["token"]
        .as_str()
        .unwrap()
        .to_string();
    let bearer = Header::new("Authorization", format!("Bearer {token}"));

    let missing = "00000000-0000-0000-0000-000000000000";
    for resource in ["role", "permission"] {
        let res = client
            .get(format!("/api/v1/access/{resource}/{missing}/edit"))
            .header(bearer.clone())
            .dispatch()
            .await;
        assert_eq!(res.status(), Status::NotFound, "{resource} edit missing id");
        let v: serde_json::Value = res.into_json().await.unwrap();
        assert_eq!(v["status"], serde_json::json!(false), "{resource} envelope");
    }
}
