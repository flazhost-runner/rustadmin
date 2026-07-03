//! Media storage service — image upload (magic-byte validated), list, delete.
//! Files live under `storage/editor/`; served at `/storage/editor/<name>`.

use std::fs;
use std::path::PathBuf;

use serde_json::{json, Value};
use uuid::Uuid;

use crate::errors::{AppError, AppResult};

fn url_prefix() -> String {
    let base = crate::config::storage_base_path();
    format!("/{base}/editor")
}

/// Absolute editor storage dir (resolved from the app root so it works from any CWD).
fn editor_dir() -> PathBuf {
    let base = crate::config::storage_base_path();
    crate::config::asset(&format!("{base}/editor"))
}

/// Allowed image extensions (whitelist) keyed by detected magic-byte extension.
fn allowed_ext(ext: &str) -> bool {
    matches!(ext, "jpg" | "jpeg" | "png" | "gif" | "webp")
}

fn ensure_dir() -> AppResult<()> {
    fs::create_dir_all(editor_dir()).map_err(|e| AppError::internal(format!("storage init: {e}")))
}

/// Validate magic bytes + store the image; returns `{ name, url, key }`.
pub fn upload(bytes: &[u8]) -> AppResult<Value> {
    let kind = infer::get(bytes)
        .filter(|k| k.matcher_type() == infer::MatcherType::Image && allowed_ext(k.extension()))
        .ok_or_else(|| AppError::bad_request("Unsupported or invalid image file"))?;

    ensure_dir()?;
    let name = format!("{}.{}", Uuid::new_v4(), kind.extension());
    let key = format!("editor/{name}");
    let dest = editor_dir().as_path().join(&name);
    fs::write(&dest, bytes).map_err(|e| AppError::internal(format!("write file: {e}")))?;

    Ok(json!({ "name": name, "url": format!("{}/{name}", url_prefix()), "key": key }))
}

/// List stored images (most-recent-agnostic; name + url + key).
pub fn list() -> AppResult<Vec<Value>> {
    ensure_dir()?;
    let mut out = Vec::new();
    let entries =
        fs::read_dir(editor_dir()).map_err(|e| AppError::internal(format!("read dir: {e}")))?;
    for entry in entries.flatten() {
        if let Some(name) = entry.file_name().to_str() {
            if name.starts_with('.') {
                continue;
            }
            out.push(json!({
                "name": name,
                "url": format!("{}/{name}", url_prefix()),
                "key": format!("editor/{name}"),
            }));
        }
    }
    Ok(out)
}

/// Delete by key; key MUST match `editor/<safe-name>` (anti path-traversal).
pub fn delete(key: &str) -> AppResult<()> {
    let name = key
        .strip_prefix("editor/")
        .filter(|n| is_safe_name(n))
        .ok_or_else(|| AppError::bad_request("Invalid key"))?;
    let path: PathBuf = editor_dir().as_path().join(name);
    if path.exists() {
        fs::remove_file(&path).map_err(|e| AppError::internal(format!("delete file: {e}")))?;
    }
    Ok(())
}

/// A safe file name: no path separators, no `..`, only `[A-Za-z0-9._-]`.
fn is_safe_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains("..")
        && !name.contains('/')
        && !name.contains('\\')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_traversal_keys() {
        assert!(delete("editor/../../etc/passwd").is_err());
        assert!(delete("../secret").is_err());
        assert!(delete("noteditor/x.png").is_err());
    }

    #[test]
    fn rejects_non_image_bytes() {
        assert!(upload(b"this is not an image").is_err());
    }
}
