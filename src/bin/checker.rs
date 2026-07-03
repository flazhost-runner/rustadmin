//! Convention checker (CI gate) — equivalent of NodeAdmin `nodeadmin check`.
//!
//! Scans `src/modules/**` for deviations from AGENTS.md and reports them; exits non-zero on
//! any violation. Text-based scanning keeps it dependency-free and fast; the checks are
//! high-signal (the same rules the human review enforces).
//!
//! Run: `cargo run --bin checker`

use std::fs;
use std::path::{Path, PathBuf};
use std::process::exit;

fn main() {
    let root = PathBuf::from("src/modules");
    let mut violations: Vec<String> = Vec::new();

    let files = rust_files(&root);
    for file in &files {
        let content = match fs::read_to_string(file) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let path = file.display().to_string();
        check_env(&path, &content, &mut violations);
        check_entity_types(file, &content, &mut violations);
        check_service_interface(file, &content, &mut violations);
        check_controller_di(file, &content, &mut violations);
    }

    // Contextual completeness: integration tests must exist.
    let tests = rust_files(&PathBuf::from("tests"));
    if tests.is_empty() {
        violations.push("tests/: no integration tests found (TEST is mandatory)".into());
    }

    if violations.is_empty() {
        println!(
            "✓ convention checker: {} module files scanned, no violations",
            files.len()
        );
    } else {
        eprintln!(
            "✗ convention checker found {} violation(s):",
            violations.len()
        );
        for v in &violations {
            eprintln!("  - {v}");
        }
        exit(1);
    }
}

fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            out.extend(rust_files(&p));
        } else if p.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(p);
        }
    }
    out
}

/// Env may only be read in `src/config` — never inside modules.
fn check_env(path: &str, content: &str, v: &mut Vec<String>) {
    for needle in ["std::env::var", "env::var(", "std::env::vars"] {
        if content.contains(needle) {
            v.push(format!(
                "{path}: reads env ({needle}) — use `crate::config` only"
            ));
        }
    }
}

/// Entity column types must be portable (no vendor types / collations).
fn check_entity_types(file: &Path, content: &str, v: &mut Vec<String>) {
    if !file.to_string_lossy().contains("/models/") {
        return;
    }
    // Vendor column types are non-portable (entities must use abstract SeaORM types).
    // Note: SeaORM's `auto_increment = false` is the portable way to opt out, so we do NOT
    // flag it — only true vendor type names.
    for bad in [
        "longtext",
        "mediumtext",
        "collation",
        "\"datetime\"",
        "tinyint",
    ] {
        if content.to_lowercase().contains(&bad.to_lowercase()) {
            v.push(format!(
                "{}: non-portable column type `{bad}`",
                file.display()
            ));
        }
    }
}

/// A `*Service` struct must implement its `I*Service` trait in the same file.
fn check_service_interface(file: &Path, content: &str, v: &mut Vec<String>) {
    if !file.to_string_lossy().contains("service") {
        return;
    }
    for line in content.lines() {
        let line = line.trim();
        if let Some(name) = struct_name(line) {
            if name.ends_with("Service") {
                let impl_needle = format!("Service for {name}");
                let trait_needle = format!("I{name}");
                if !(content.contains(&impl_needle) && content.contains(&trait_needle)) {
                    v.push(format!(
                        "{}: `{name}` must implement its `I{name}` interface (Dependency Inversion)",
                        file.display()
                    ));
                }
            }
        }
    }
}

/// Controllers must receive services from managed `State`, not construct them.
fn check_controller_di(file: &Path, content: &str, v: &mut Vec<String>) {
    if !file.to_string_lossy().contains("controller") {
        return;
    }
    // crude: flag direct constructor calls like `XService::new(` in a controller.
    for line in content.lines() {
        let t = line.trim();
        if t.contains("Service::new(") && !t.starts_with("//") {
            v.push(format!(
                "{}: controller constructs a service directly — inject via `&State<Arc<dyn I*Service>>`",
                file.display()
            ));
        }
    }
}

/// Extract a struct name from a `pub struct Name` / `struct Name` line.
fn struct_name(line: &str) -> Option<&str> {
    let rest = line
        .strip_prefix("pub struct ")
        .or_else(|| line.strip_prefix("struct "))?;
    let name: &str = rest.split([' ', '{', '(', ';', '<']).next()?;
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}
