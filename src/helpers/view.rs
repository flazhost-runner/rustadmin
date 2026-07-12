//! Tera rendering helper (mirrors NodeAdmin `renderView()` + `route()`/`getFile()` view
//! globals). Every admin render injects theme + setting/auth/nav locals so the chrome can map
//! them to CSS variables and gate the sidebar — switching theme restyles the whole UI.

use std::collections::HashMap;

use rocket::fairing::Fairing;
use rocket_dyn_templates::tera::{self, Tera, Value as TeraValue};
use rocket_dyn_templates::Template;
use serde_json::{json, Map, Value};

use crate::config::themes::{get_theme, theme_names, DEFAULT_THEME, THEMES};
use crate::rbac::{has_access, registry};

/// All palettes as JSON (for the theme switcher swatches).
pub fn themes_json() -> Value {
    json!(THEMES)
}

/// Sidebar gating object (`nav.*`) computed from the current user's RBAC perms.
pub fn nav_for(is_admin: bool, perms: &[(String, String)]) -> Value {
    let can = |name: &str| has_access(is_admin, perms, name, "GET");
    json!({
        "components": can("admin.v1.components.index"),
        "permission": can("admin.v1.access.permission.index"),
        "role": can("admin.v1.access.role.index"),
        "user": can("admin.v1.access.user.index"),
        "setting": can("admin.v1.setting.index"),
    })
}

/// Tera function `route(name=..., <param>=...)` → the registry path with `<param>` filled.
fn route_fn(args: &HashMap<String, TeraValue>) -> tera::Result<TeraValue> {
    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| tera::Error::msg("route(): missing `name`"))?;
    let entry = registry().into_iter().find(|r| r.name == name);
    let mut path = match entry {
        Some(e) => e.path.to_string(),
        None => return Err(tera::Error::msg(format!("route(): unknown route `{name}`"))),
    };
    for (k, v) in args {
        if k == "name" {
            continue;
        }
        let val = v
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| v.to_string());
        path = path.replace(&format!("<{k}>"), &val);
    }
    Ok(TeraValue::String(path))
}

/// Tera function `get_file(path=...)` → driver-aware public URL for a stored object key.
/// Routes through [`crate::config::storage::object_url`]: local → `/storage/<key>`, oss/s3 →
/// absolute presigned URL. Returns empty string for null/missing/empty so `<img src>` stays blank.
fn get_file_fn(args: &HashMap<String, TeraValue>) -> tera::Result<TeraValue> {
    let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
    if path.is_empty() {
        return Ok(TeraValue::String(String::new()));
    }
    Ok(TeraValue::String(crate::config::storage::object_url(path)))
}

/// Register RustAdmin's Tera globals. Shared by the live engine and tests.
pub fn register_tera(tera: &mut Tera) {
    // Autoescape `.tera` output by default (EJS `<%= %>` parity); use `| safe` for raw HTML
    // (EJS `<%- %>`), e.g. sanitized rich-text descriptions.
    tera.autoescape_on(vec![".tera", ".html", ".htm"]);
    tera.register_function("route", route_fn);
    tera.register_function("get_file", get_file_fn);
}

/// The Template fairing with RustAdmin's Tera customizations attached.
pub fn template_fairing() -> impl Fairing {
    Template::custom(|engines| {
        register_tera(&mut engines.tera);
    })
}

/// Render a backend template, merging standard theme + chrome locals into `locals`.
///
/// Ensures the keys the layout chrome depends on always exist (so Tera never hits an
/// undefined access): `themes`, `themeName`, `theme`, `app_name`, `setting`, `auth`, `nav`.
pub fn render_view(name: &str, locals: Value, theme_name: Option<&str>) -> Template {
    // Theme precedence: explicit arg → cached site theme → default.
    let active: String = match theme_name {
        Some(t) => t.to_string(),
        None => crate::site::theme_name().unwrap_or_else(|| DEFAULT_THEME.to_string()),
    };
    let palette = get_theme(&active);

    let mut ctx: Map<String, Value> = match locals {
        Value::Object(m) => m,
        Value::Null => Map::new(),
        other => {
            let mut m = Map::new();
            m.insert("data".into(), other);
            m
        }
    };

    ctx.entry("themes").or_insert_with(themes_json);
    ctx.entry("themeNames")
        .or_insert_with(|| json!(theme_names()));
    ctx.entry("themeName").or_insert_with(|| json!(active));
    ctx.entry("theme").or_insert_with(|| json!(palette));
    ctx.entry("app_name").or_insert_with(|| json!("RustAdmin"));
    // active site setting (cached) unless the caller already supplied one
    if !ctx.contains_key("setting") {
        ctx.insert(
            "setting".to_string(),
            crate::site::setting().unwrap_or_else(|| json!({})),
        );
    }
    ctx.entry("auth")
        .or_insert_with(|| json!({ "name": "", "picture": null }));
    ctx.entry("nav").or_insert_with(|| {
        json!({
            "components": false, "permission": false, "role": false,
            "user": false, "setting": false, "maintenance": false
        })
    });
    ctx.entry("active_path").or_insert_with(|| json!(""));
    // active sidebar key: "dashboard"|"components"|"permission"|"role"|"user"|"setting"
    ctx.entry("active").or_insert_with(|| json!(""));
    // flash: { key, message } — default empty so templates can check `flash.message`.
    ctx.entry("flash").or_insert_with(|| json!({}));
    // inline form state (PRG): errors (field→msg) + old (field→value).
    ctx.entry("errors").or_insert_with(|| json!({}));
    ctx.entry("old").or_insert_with(|| json!({}));
    ctx.entry("filter").or_insert_with(|| json!({}));

    Template::render(name.to_string(), Value::Object(ctx))
}
