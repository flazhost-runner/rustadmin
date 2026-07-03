//! `add-ui` — upgrade an API-only install to Full (UI + API). Idempotent.
//!
//! RustAdmin uses a single codebase with a runtime `APP_MODE` switch, and the web layer is
//! mounted under a presence guard (the diff between variants is purely additive). So the
//! upgrade is: ensure the web templates/static are present (they are, in this repo) and set
//! `APP_MODE=full` in `.env`, then verify. Re-running is safe.
//!
//! Run: `cargo run --bin add-ui`

use std::fs;
use std::path::Path;

fn main() {
    println!("add-ui: upgrading install to Full (UI + API)\n");

    // 1. Confirm the UI assets exist (in a real API-only checkout these would be copied in).
    let required = [
        "templates/layouts/be/default/main.tera",
        "templates/layouts/fe/default/main.tera",
        "static/be/default/vendor/trumbowyg/filemanager.js",
    ];
    let mut missing = Vec::new();
    for p in required {
        if !Path::new(p).exists() {
            missing.push(p);
        }
    }
    if missing.is_empty() {
        println!("  ✓ web templates + static assets present");
    } else {
        for m in &missing {
            println!("  ! missing UI file: {m} (copy from the Full template)");
        }
    }

    // 2. Set APP_MODE=full in .env (idempotent).
    let env_path = ".env";
    let mut env = fs::read_to_string(env_path).unwrap_or_default();
    if env.lines().any(|l| l.trim_start().starts_with("APP_MODE=")) {
        if env.contains("APP_MODE=api") {
            env = env.replace("APP_MODE=api", "APP_MODE=full");
            fs::write(env_path, &env).expect("write .env");
            println!("  ✓ switched APP_MODE=api → APP_MODE=full in .env");
        } else {
            println!("  ✓ APP_MODE already set (full)");
        }
    } else {
        if !env.is_empty() && !env.ends_with('\n') {
            env.push('\n');
        }
        env.push_str("APP_MODE=full\n");
        fs::write(env_path, &env).expect("write .env");
        println!("  ✓ added APP_MODE=full to .env");
    }

    println!("\nNext: verify with");
    println!("  cargo run --bin checker && cargo test && APP_MODE=full cargo run");
}
