//! Module generator — `cargo run --bin make-module <name>` (equivalent of `/make-module`).
//!
//! Scaffolds a convention-following feature module (entity, service+interface, controller,
//! routes, view, test stub) under `src/modules/<name>/`. Prints the manual wiring steps
//! (mount routes + manage the service in `src/lib.rs`, add the migration), then suggests
//! running the checker + tests.

use std::fs;
use std::path::Path;
use std::process::exit;

fn main() {
    let name = match std::env::args().nth(1) {
        Some(n) if n.chars().all(|c| c.is_ascii_lowercase() || c == '_') && !n.is_empty() => n,
        _ => {
            eprintln!("usage: cargo run --bin make-module <name>   (lowercase snake_case)");
            exit(2);
        }
    };
    let pascal = to_pascal(&name);
    let base = format!("src/modules/{name}");
    if Path::new(&base).exists() {
        eprintln!("module `{name}` already exists at {base}");
        exit(1);
    }

    write(&format!("{base}/mod.rs"), &mod_rs());
    write(&format!("{base}/models/mod.rs"), "pub mod model;\n");
    write(
        &format!("{base}/models/model.rs"),
        &entity_rs(&name, &pascal),
    );
    write(&format!("{base}/services/mod.rs"), &services_mod(&pascal));
    write(
        &format!("{base}/services/{name}_service.rs"),
        &service_rs(&name, &pascal),
    );
    write(
        &format!("{base}/controllers.rs"),
        &controller_rs(&name, &pascal),
    );
    write(
        &format!("templates/be/default/{name}/index.tera"),
        &view_tera(&pascal),
    );

    println!("✓ generated module `{name}` ({pascal})\n");
    println!("Next steps (wire it up):");
    println!("  1. src/modules/mod.rs       → add `pub mod {name};`");
    println!(
        "  2. src/migrations/          → add a Create{pascal}Table migration + register in mod.rs"
    );
    println!("  3. src/lib.rs               → manage `Arc<dyn I{pascal}Service>` + mount `{name}::controllers::routes()`");
    println!("  4. src/rbac/registry.rs     → add named routes for the module");
    println!("  5. tests/                   → add an integration test");
    println!("  6. verify                   → cargo run --bin checker && cargo test");
}

fn write(path: &str, content: &str) {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent).expect("create dir");
    }
    fs::write(path, content).expect("write file");
    println!("  + {path}");
}

fn to_pascal(s: &str) -> String {
    s.split('_')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

fn mod_rs() -> String {
    "//! Generated module. Wire into `src/modules/mod.rs` + `src/lib.rs`.\n\npub mod controllers;\npub mod models;\npub mod services;\n".into()
}

fn entity_rs(table: &str, pascal: &str) -> String {
    format!(
        "//! `{table}s` entity (portable column types only).\n\nuse sea_orm::entity::prelude::*;\nuse serde::{{Deserialize, Serialize}};\n\n#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]\n#[sea_orm(table_name = \"{table}s\")]\npub struct Model {{\n    #[sea_orm(primary_key, auto_increment = false)]\n    pub id: String,\n    pub name: String,\n    pub status: String,\n    pub created_at: DateTime,\n    pub updated_at: DateTime,\n}}\n\n#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]\npub enum Relation {{}}\n\nimpl ActiveModelBehavior for ActiveModel {{}}\n\n// {pascal}\n"
    )
}

fn services_mod(pascal: &str) -> String {
    let lower = pascal.to_lowercase();
    format!("pub mod {lower}_service;\npub use {lower}_service::{{I{pascal}Service, {pascal}Service}};\n")
}

fn service_rs(table: &str, pascal: &str) -> String {
    let lower = pascal.to_lowercase();
    format!(
        "//! {pascal} service (trait + impl). Shared via managed `State<Arc<dyn I{pascal}Service>>`.\n\nuse async_trait::async_trait;\nuse sea_orm::{{DatabaseConnection, EntityTrait}};\n\nuse crate::errors::AppResult;\nuse crate::modules::{table}::models::model;\n\n#[async_trait]\npub trait I{pascal}Service: Send + Sync {{\n    async fn all(&self, db: &DatabaseConnection) -> AppResult<Vec<model::Model>>;\n}}\n\npub struct {pascal}Service;\n\n#[async_trait]\nimpl I{pascal}Service for {pascal}Service {{\n    async fn all(&self, db: &DatabaseConnection) -> AppResult<Vec<model::Model>> {{\n        Ok(model::Entity::find().all(db).await?)\n    }}\n}}\n\n// {lower}\n"
    )
}

fn controller_rs(table: &str, pascal: &str) -> String {
    let lower = pascal.to_lowercase();
    format!(
        "//! {pascal} web controller (thin: parse -> service -> render).\n\nuse std::sync::Arc;\n\nuse rocket::http::CookieJar;\nuse rocket::State;\nuse rocket_dyn_templates::Template;\nuse sea_orm::DatabaseConnection;\nuse serde_json::json;\n\nuse crate::errors::AppError;\nuse crate::guards::Authorized;\nuse crate::helpers::view::render_view;\nuse crate::modules::access::controllers::web::{{chrome, merge}};\nuse crate::modules::{table}::services::I{pascal}Service;\nuse crate::security::csrf::ensure_token;\n\n#[get(\"/{table}\")]\npub async fn index(\n    auth: Authorized,\n    db: &State<DatabaseConnection>,\n    svc: &State<Arc<dyn I{pascal}Service>>,\n    cookies: &CookieJar<'_>,\n) -> Result<Template, AppError> {{\n    let datas = svc.all(db.inner()).await?;\n    let csrf = ensure_token(cookies);\n    let mut page = json!({{ \"datas\": datas }});\n    merge(&mut page, chrome(&auth.0, &csrf, \"{lower}\"));\n    Ok(render_view(\"be/default/{table}/index\", page, None))\n}}\n\npub fn routes() -> Vec<rocket::Route> {{\n    routes![index]\n}}\n"
    )
}

fn view_tera(pascal: &str) -> String {
    format!(
        "{{% extends \"layouts/be/default/main\" %}}\n{{% block content %}}\n<div class=\"flex items-center justify-between mb-6\">\n  <h1 class=\"text-2xl font-bold text-gray-800\">{pascal} Management</h1>\n</div>\n<div class=\"tw-card p-6\">\n  <p class=\"text-gray-600\">Generated module. Build the canonical index table here.</p>\n</div>\n{{% endblock content %}}\n"
    )
}
