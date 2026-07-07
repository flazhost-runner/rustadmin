//! Phase 5 — User resource end-to-end: web canonical table + create/store, method-override
//! DELETE (and the negative control), and the API verbose paths (with REST-style paths 404).

use rocket::http::{ContentType, Cookie, Header, Status};
use rocket::local::asynchronous::Client;

use rust_admin::config::Config;
use rust_admin::db;
use rust_admin::migrations::Migrator;
use rust_admin::modules::access::models::{role, user};

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

fn enc(s: &str) -> String {
    s.replace('@', "%40").replace(' ', "%20")
}

// ---------- WEB ----------

#[tokio::test]
async fn web_index_lists_users() {
    let (client, _db, admin_id, _role) = setup().await;
    let res = client
        .get("/admin/v1/access/user")
        .private_cookie(Cookie::new("uid", admin_id))
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Ok);
    let body = res.into_string().await.unwrap();
    assert!(body.contains("User List"));
    assert!(body.contains("admin@admin.com"));
    assert!(body.contains("checkall")); // canonical select-all
}

#[tokio::test]
async fn web_create_form_renders() {
    let (client, _db, admin_id, _role) = setup().await;
    let res = client
        .get("/admin/v1/access/user/create")
        .private_cookie(Cookie::new("uid", admin_id))
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Ok);
    let body = res.into_string().await.unwrap();
    assert!(body.contains("User Form"));
    // Picture field 1:1 with NodeAdmin: multipart form, plain .form-control file input,
    // an always-rendered preview, and the previewImage handler.
    assert!(body.contains("enctype=\"multipart/form-data\""));
    assert!(body.contains(r#"type="file" class="form-control"#));
    assert!(body.contains(r#"name="picture""#));
    assert!(body.contains("previewImage(event)"));
    assert!(body.contains(r#"id="preview""#));
}

#[tokio::test]
async fn web_store_creates_user() {
    let (client, db, admin_id, role_id) = setup().await;
    let body = format!(
        "code=U2&name=NewUser&email={}&password=password123&password_confirmation=password123&status=Active&roles={}",
        enc("new@example.com"),
        role_id
    );
    let res = client
        .post("/admin/v1/access/user/store?_csrf=tok")
        .private_cookie(Cookie::new("uid", admin_id))
        .private_cookie(Cookie::new("csrf_token", "tok"))
        .header(ContentType::Form)
        .body(body)
        .dispatch()
        .await;
    assert!(
        (300..400).contains(&res.status().code),
        "store should redirect (PRG), got {}",
        res.status()
    );
    let created = user::Entity::find()
        .filter(user::Column::Email.eq("new@example.com"))
        .one(&db)
        .await
        .unwrap();
    assert!(created.is_some(), "user persisted");
}

#[tokio::test]
async fn web_delete_requires_method_override() {
    let (client, db, admin_id, _role) = setup().await;
    let admin_id2 = admin_id.clone();

    // Negative control: POST without ?_method=DELETE has no matching route → 404.
    let res = client
        .post(format!("/admin/v1/access/user/{admin_id}/delete?_csrf=tok"))
        .private_cookie(Cookie::new("uid", admin_id.clone()))
        .private_cookie(Cookie::new("csrf_token", "tok"))
        .dispatch()
        .await;
    assert_eq!(
        res.status(),
        Status::NotFound,
        "no override → no DELETE route"
    );

    // With ?_method=DELETE the override fairing routes to the DELETE handler.
    let res = client
        .post(format!(
            "/admin/v1/access/user/{admin_id2}/delete?_method=DELETE&_csrf=tok"
        ))
        .private_cookie(Cookie::new("uid", admin_id2.clone()))
        .private_cookie(Cookie::new("csrf_token", "tok"))
        .dispatch()
        .await;
    assert!((300..400).contains(&res.status().code), "delete redirects");
    let gone = user::Entity::find_by_id(admin_id2).one(&db).await.unwrap();
    assert!(gone.is_none(), "user actually deleted via override");
}

// ---------- API ----------

async fn login(client: &Client) -> String {
    let res = client
        .post("/api/v1/auth/login")
        .header(ContentType::JSON)
        .body(r#"{"email":"admin@admin.com","password":"12345678"}"#)
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Ok);
    let v: serde_json::Value = res.into_json().await.unwrap();
    v["data"]["token"].as_str().unwrap().to_string()
}

fn bearer(t: &str) -> Header<'static> {
    Header::new("Authorization", format!("Bearer {t}"))
}

#[tokio::test]
async fn api_verbose_crud_and_rest_paths_404() {
    let (client, db, _admin, role_id) = setup().await;
    let token = login(&client).await;

    // verbose index
    let res = client
        .get("/api/v1/access/user")
        .header(bearer(&token))
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Ok);

    // verbose store
    let res = client
        .post("/api/v1/access/user/store")
        .header(ContentType::JSON)
        .header(bearer(&token))
        .body(format!(
            r#"{{"code":"A1","name":"ApiUser","email":"api@example.com","password":"password123","roles":["{role_id}"]}}"#
        ))
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Created);
    let exists = user::Entity::find()
        .filter(user::Column::Email.eq("api@example.com"))
        .count(&db)
        .await
        .unwrap();
    assert_eq!(exists, 1);

    // REST-style paths must NOT exist (verbose-only, per PORTING_GUIDE)
    let res = client
        .get("/api/v1/access/user/some-id")
        .header(bearer(&token))
        .dispatch()
        .await;
    assert_eq!(
        res.status(),
        Status::NotFound,
        "REST GET /:id is not a route"
    );
}

#[tokio::test]
async fn api_requires_auth() {
    let (client, _db, _admin, _role) = setup().await;
    let res = client.get("/api/v1/access/user").dispatch().await;
    assert_eq!(res.status(), Status::Unauthorized);
}

// Parity with the NodeAdmin fix: `edit` on a missing id must 404 (canonical envelope),
// never 200 with an empty body.
#[tokio::test]
async fn api_edit_missing_id_returns_404() {
    let (client, _db, _admin, _role) = setup().await;
    let token = login(&client).await;
    let res = client
        .get("/api/v1/access/user/00000000-0000-0000-0000-000000000000/edit")
        .header(bearer(&token))
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::NotFound);
    let v: serde_json::Value = res.into_json().await.unwrap();
    assert_eq!(v["status"], false);
}
