//! Phase 4 — the admin chrome renders through the real Tera pipeline (template fairing +
//! `route()`/`get_file()` globals + theme injection). Verifies the 1:1 chrome markers are
//! present: themed CSS vars, sidebar gradient + exact menu labels, user dropdown, the global
//! image fallback, and the Toast/Modal vanilla JS.

#[macro_use]
extern crate rocket;

use rocket::local::blocking::Client;
use rocket_dyn_templates::Template;
use serde_json::json;

#[get("/__smoke")]
fn smoke() -> Template {
    rust_admin::helpers::view::render_view(
        "be/default/_smoke",
        json!({
            "app_name": "RustAdmin",
            "setting": { "name": "RustAdmin", "copyright": "© RustAdmin" },
            "auth": { "name": "Administrator", "picture": null },
            "nav": { "components": true, "permission": true, "role": true, "user": true, "setting": true },
            "active": "dashboard",
            "csrf_token": "tok-123"
        }),
        Some("Blue"),
    )
}

fn client() -> Client {
    let rocket = rocket::build()
        .attach(rust_admin::helpers::view::template_fairing())
        .mount("/", routes![smoke]);
    Client::tracked(rocket).unwrap()
}

#[test]
fn chrome_renders_with_theme_and_menu() {
    let client = client();
    let res = client.get("/__smoke").dispatch();
    assert_eq!(res.status(), rocket::http::Status::Ok);
    let body = res.into_string().unwrap();

    // active theme (Blue) wired into CSS variables + tailwind config
    assert!(body.contains("--primary: #3B82F6"), "theme primary CSS var");
    assert!(body.contains("primary: '#3B82F6'"), "tailwind config color");

    // sidebar chrome + EXACT menu labels (singular English, untranslated)
    assert!(body.contains("sidebar-gradient"));
    assert!(body.contains(">Dashboard<"));
    assert!(body.contains(">UI Components<"));
    assert!(body.contains(">Permission<"));
    assert!(body.contains(">Role<"));
    assert!(body.contains(">User<"));
    assert!(body.contains(">Setting<"));
    assert!(body.contains("Maintenance"));

    // named-route hrefs resolved via route()
    assert!(body.contains("/admin/v1/access/user"));
    assert!(body.contains("/admin/v1/setting"));

    // topbar user dropdown + logout POST form with CSRF in query
    assert!(body.contains("Welcome, Administrator"));
    assert!(body.contains("logout-form"));
    assert!(body.contains("/auth/logout?_csrf=tok-123"));

    // foot: global image fallback + vanilla Toast/Modal/Confirm
    assert!(
        body.contains("imgFallback"),
        "global image fallback present"
    );
    assert!(body.contains("window.Toast"));
    assert!(body.contains("confirmDialog"));

    // component classes present (Tailwind @apply re-implementation)
    assert!(body.contains(".btn-primary"));
    assert!(body.contains(".tw-card"));
    assert!(body.contains("text-bg-primary"));
}

#[test]
fn active_menu_item_is_marked() {
    let client = client();
    let body = client.get("/__smoke").dispatch().into_string().unwrap();
    // Dashboard link should carry the `active` class
    assert!(
        body.contains("font-medium active"),
        "active nav class applied"
    );
}
