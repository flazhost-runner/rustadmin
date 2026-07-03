//! Phase 6 — Setting / Dashboard / Components / Profile pages render, and Setting + Profile
//! updates persist. (Media magic-byte validation is covered by unit tests in the service.)

use rocket::http::{ContentType, Cookie, Status};
use rocket::local::asynchronous::Client;

use rust_admin::config::Config;
use rust_admin::db;
use rust_admin::migrations::Migrator;
use rust_admin::modules::access::models::user;
use rust_admin::modules::setting::models::setting;

use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use sea_orm_migration::MigratorTrait;

async fn setup() -> (Client, DatabaseConnection, String) {
    let conn = db::connect_in_memory().await.unwrap();
    Migrator::up(&conn, None).await.unwrap();
    let mut cfg = Config::from_env();
    cfg.jwt.secret = "t".into();
    cfg.session.secret = "s".into();
    let client = Client::tracked(rust_admin::build_rocket_with_db(cfg, conn.clone()))
        .await
        .unwrap();
    let admin = user::Entity::find()
        .filter(user::Column::Email.eq("admin@admin.com"))
        .one(&conn)
        .await
        .unwrap()
        .unwrap();
    (client, conn, admin.id)
}

fn uid(id: &str) -> Cookie<'static> {
    Cookie::new("uid", id.to_string())
}

#[tokio::test]
async fn admin_pages_render() {
    let (client, _db, admin) = setup().await;
    // (path, markers that must be present — parity with NodeAdmin views)
    let cases = [
        (
            "/admin/v1/dashboard",
            vec![
                "Dashboard Overview",
                "Recent Activities",
                "Top Products",
                "Recent Orders",
                "Theme Aktif",
            ],
        ),
        (
            "/admin/v1/components",
            vec![
                "1. Stat Card",
                "Rich Text Editor",
                "9. Data Table",
                "Confirm Dialog",
            ],
        ),
        (
            "/admin/v1/setting",
            vec![
                "Admin Theme",
                "theme-swatch",
                "Frontend Template",
                "fe-card",
                "Setting Form",
                "[initial]",
            ],
        ),
        (
            "/admin/v1/profile",
            vec![
                "User Form",
                "name=\"code\"",
                "name=\"timezone\"",
                "name=\"status\"",
                "name=\"picture\"",
            ],
        ),
    ];
    for (path, markers) in cases {
        let res = client
            .get(path)
            .private_cookie(uid(&admin))
            .dispatch()
            .await;
        assert_eq!(res.status(), Status::Ok, "{path} should render");
        let body = res.into_string().await.unwrap();
        for m in markers {
            assert!(body.contains(m), "{path} should contain `{m}`");
        }
    }
}

#[tokio::test]
async fn setting_update_persists_theme() {
    let (client, db, admin) = setup().await;
    let res = client
        .post("/admin/v1/setting/update?_method=PUT&_csrf=tok")
        .private_cookie(uid(&admin))
        .private_cookie(Cookie::new("csrf_token", "tok"))
        .header(ContentType::Form)
        .body("name=MyAdmin&theme=Red&fe_template=agency-consulting-002-creative-agency")
        .dispatch()
        .await;
    assert!((300..400).contains(&res.status().code));

    let s = setting::Entity::find().one(&db).await.unwrap().unwrap();
    assert_eq!(s.theme.as_deref(), Some("Red"));
    assert_eq!(s.name.as_deref(), Some("MyAdmin"));
}

#[tokio::test]
async fn setting_sanitizes_description_html() {
    let (client, db, admin) = setup().await;
    let res = client
        .post("/admin/v1/setting/update?_method=PUT&_csrf=tok")
        .private_cookie(uid(&admin))
        .private_cookie(Cookie::new("csrf_token", "tok"))
        .header(ContentType::Form)
        .body("description=%3Cp%3EHi%3C%2Fp%3E%3Cscript%3Ealert(1)%3C%2Fscript%3E")
        .dispatch()
        .await;
    assert!((300..400).contains(&res.status().code));
    let s = setting::Entity::find().one(&db).await.unwrap().unwrap();
    let desc = s.description.unwrap_or_default();
    assert!(desc.contains("<p>Hi</p>"));
    assert!(!desc.contains("<script"), "script must be stripped: {desc}");
}

#[tokio::test]
async fn profile_update_persists() {
    let (client, db, admin) = setup().await;
    let res = client
        .post("/admin/v1/profile/update?_method=PUT&_csrf=tok")
        .private_cookie(uid(&admin))
        .private_cookie(Cookie::new("csrf_token", "tok"))
        .header(ContentType::Form)
        .body("name=Renamed&email=admin@admin.com")
        .dispatch()
        .await;
    assert!((300..400).contains(&res.status().code));
    let u = user::Entity::find_by_id(admin)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(u.name, "Renamed");
}
