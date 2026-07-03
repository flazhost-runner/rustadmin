//! Phase 7 — public landing, web session login flow, and the FE-template preview proxy.

use rocket::http::{ContentType, Cookie, Status};
use rocket::local::asynchronous::Client;

use rust_admin::config::Config;
use rust_admin::db;
use rust_admin::migrations::Migrator;

use sea_orm_migration::MigratorTrait;

async fn client() -> Client {
    let conn = db::connect_in_memory().await.unwrap();
    Migrator::up(&conn, None).await.unwrap();
    let mut cfg = Config::from_env();
    cfg.jwt.secret = "t".into();
    cfg.session.secret = "s".into();
    Client::tracked(rust_admin::build_rocket_with_db(cfg, conn))
        .await
        .unwrap()
}

#[tokio::test]
async fn landing_renders_bound_to_setting() {
    let client = client().await;
    let res = client.get("/").dispatch().await;
    assert_eq!(res.status(), Status::Ok);
    // template responses must be served as HTML (not text/plain) so browsers render them
    assert_eq!(res.content_type(), Some(ContentType::HTML));
    let body = res.into_string().await.unwrap();
    assert!(body.contains("RustAdmin"));
    assert!(body.contains("Sign in"));
    assert!(
        body.contains("data-reveal"),
        "rich landing sections present"
    );
}

#[tokio::test]
async fn home_alias_works() {
    let client = client().await;
    assert_eq!(client.get("/home").dispatch().await.status(), Status::Ok);
}

#[tokio::test]
async fn web_login_establishes_session() {
    let client = client().await;

    // login page renders
    let res = client.get("/auth/login").dispatch().await;
    assert_eq!(res.status(), Status::Ok);
    assert!(res.into_string().await.unwrap().contains("Login"));

    // valid login → redirect; Client::tracked keeps the session cookie
    let res = client
        .post("/auth/login?_csrf=tok")
        .private_cookie(Cookie::new("csrf_token", "tok"))
        .header(ContentType::Form)
        .body("email=admin@admin.com&password=12345678")
        .dispatch()
        .await;
    assert!((300..400).contains(&res.status().code));

    // now the session works on a protected page
    let res = client.get("/admin/v1/dashboard").dispatch().await;
    assert_eq!(res.status(), Status::Ok, "session login grants access");
}

#[tokio::test]
async fn bad_login_redirects_back() {
    let client = client().await;
    let res = client
        .post("/auth/login?_csrf=tok")
        .private_cookie(Cookie::new("csrf_token", "tok"))
        .header(ContentType::Form)
        .body("email=admin@admin.com&password=wrong")
        .dispatch()
        .await;
    assert!((300..400).contains(&res.status().code));
    // and the dashboard is NOT accessible — a logged-out web request redirects to login
    let res = client.get("/admin/v1/dashboard").dispatch().await;
    assert_eq!(res.status(), Status::SeeOther);
    assert_eq!(res.headers().get_one("Location"), Some("/auth/login"));
}

#[tokio::test]
async fn unauthenticated_web_route_redirects_to_login() {
    let client = client().await;
    // GET an authenticated admin page while logged out → redirect to /auth/login (NOT 401/404)
    for path in [
        "/admin/v1/dashboard",
        "/admin/v1/access/user",
        "/admin/v1/setting",
    ] {
        let res = client.get(path).dispatch().await;
        assert_eq!(res.status(), Status::SeeOther, "{path} should redirect");
        assert_eq!(
            res.headers().get_one("Location"),
            Some("/auth/login"),
            "{path} should redirect to login"
        );
    }
    // API stays JSON 401 (no redirect)
    let res = client.get("/api/v1/access/user").dispatch().await;
    assert_eq!(res.status(), Status::Unauthorized);
}

#[tokio::test]
async fn fe_preview_validates_slug_anti_ssrf() {
    let client = client().await;
    // need an admin session; log in via web first
    client
        .post("/auth/login?_csrf=tok")
        .private_cookie(Cookie::new("csrf_token", "tok"))
        .header(ContentType::Form)
        .body("email=admin@admin.com&password=12345678")
        .dispatch()
        .await;

    // valid slug → HTML preview
    let res = client
        .get("/admin/v1/setting/fe-preview/agency-consulting-002-creative-agency")
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Ok);
    assert!(res.into_string().await.unwrap().contains("<html"));

    // invalid slug → rejected (anti-SSRF): never proxies (no 200 HTML; web error → redirect)
    let res = client
        .get("/admin/v1/setting/fe-preview/not-a-valid-slug")
        .dispatch()
        .await;
    assert_ne!(res.status(), Status::Ok, "invalid slug must not be proxied");
}
