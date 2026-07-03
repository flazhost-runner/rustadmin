//! Media file-manager endpoints (the Trumbowyg "filemanager" plugin's backend):
//! list → upload → list → delete → list, plus the CSRF negative control.
//! Mirrors the real plugin flow: session cookie (`uid`), CSRF via `X-CSRF-Token`
//! header, multipart upload (`file`), and **form-encoded** delete (`key=...`).

use rocket::http::{ContentType, Cookie, Header, Status};
use rocket::local::asynchronous::Client;

use rust_admin::config::Config;
use rust_admin::db;
use rust_admin::migrations::Migrator;
use rust_admin::modules::access::models::user;

use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use sea_orm_migration::MigratorTrait;

// A valid 1x1 PNG (magic bytes pass `infer`'s image check).
const PNG: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 4, 0,
    0, 0, 181, 28, 12, 2, 0, 0, 0, 11, 73, 68, 65, 84, 120, 218, 99, 100, 96, 248, 95, 15, 0, 2,
    135, 1, 128, 235, 71, 186, 146, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];

async fn setup() -> (Client, String) {
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
    (client, admin.id)
}

fn multipart_png() -> (ContentType, Vec<u8>) {
    let boundary = "X-MEDIA-BOUNDARY";
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        b"Content-Disposition: form-data; name=\"file\"; filename=\"a.png\"\r\n\
          Content-Type: image/png\r\n\r\n",
    );
    body.extend_from_slice(PNG);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    let ct = ContentType::new("multipart", "form-data").with_params(("boundary", boundary));
    (ct, body)
}

fn uid(id: &str) -> Cookie<'static> {
    Cookie::new("uid", id.to_string())
}
fn csrf_cookie() -> Cookie<'static> {
    Cookie::new("csrf_token", "tok")
}
fn csrf_header() -> Header<'static> {
    Header::new("X-CSRF-Token", "tok")
}

#[tokio::test]
async fn filemanager_list_upload_delete_roundtrip() {
    let (client, admin_id) = setup().await;

    // 1) list starts empty (or at least returns the success envelope)
    let res = client
        .get("/admin/v1/media/list")
        .private_cookie(uid(&admin_id))
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Ok);
    let v: serde_json::Value = res.into_json().await.unwrap();
    // Canonical NodeAdmin envelope: { status, message, data }.
    assert_eq!(v["status"], true);
    let before = v["data"].as_array().unwrap().len();

    // 2) upload a PNG (multipart, CSRF header) → returns { data: { name, url, key } }
    let (ct, body) = multipart_png();
    let res = client
        .post("/admin/v1/media/upload")
        .private_cookie(uid(&admin_id))
        .private_cookie(csrf_cookie())
        .header(csrf_header())
        .header(ct)
        .body(body)
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Ok, "upload should succeed");
    let v: serde_json::Value = res.into_json().await.unwrap();
    let key = v["data"]["key"].as_str().unwrap().to_string();
    assert!(key.starts_with("editor/"), "key namespaced under editor/");
    assert!(v["data"]["url"]
        .as_str()
        .unwrap()
        .contains("/storage/editor/"));

    // 3) list now has one more
    let res = client
        .get("/admin/v1/media/list")
        .private_cookie(uid(&admin_id))
        .dispatch()
        .await;
    let v: serde_json::Value = res.into_json().await.unwrap();
    assert_eq!(v["data"].as_array().unwrap().len(), before + 1);

    // 4) delete via FORM-ENCODED body (`key=...`) — what the jQuery plugin sends
    let res = client
        .post("/admin/v1/media/delete")
        .private_cookie(uid(&admin_id))
        .private_cookie(csrf_cookie())
        .header(csrf_header())
        .header(ContentType::Form)
        .body(format!("key={key}"))
        .dispatch()
        .await;
    assert_eq!(
        res.status(),
        Status::Ok,
        "form-encoded delete should succeed"
    );

    // 5) back to the original count
    let res = client
        .get("/admin/v1/media/list")
        .private_cookie(uid(&admin_id))
        .dispatch()
        .await;
    let v: serde_json::Value = res.into_json().await.unwrap();
    assert_eq!(v["data"].as_array().unwrap().len(), before);
}

#[tokio::test]
async fn upload_requires_csrf() {
    let (client, admin_id) = setup().await;
    let (ct, body) = multipart_png();
    // No CSRF header/cookie → guard rejects (403), which the web 403 catcher turns into a
    // redirect. Either way the upload never runs.
    let res = client
        .post("/admin/v1/media/upload")
        .private_cookie(uid(&admin_id))
        .header(ct)
        .body(body)
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::SeeOther);
    assert_eq!(
        res.headers().get_one("Location"),
        Some("/admin/v1/dashboard")
    );
}

#[tokio::test]
async fn list_requires_auth() {
    let (client, _admin) = setup().await;
    // Media lives under the web mount (`/admin/v1`, not `/api`), so an unauthenticated
    // request is caught and redirected to login — never served, never 200.
    let res = client.get("/admin/v1/media/list").dispatch().await;
    assert_eq!(res.status(), Status::SeeOther);
    assert_eq!(res.headers().get_one("Location"), Some("/auth/login"));
}
