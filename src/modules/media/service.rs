//! Media storage service — image upload (magic-byte validated), list, delete.
//!
//! Storage is driver-agnostic: objects are addressed by **key** (`editor/<uuid>.<ext>`) and
//! all I/O goes through [`crate::config::storage`]. With `STORAGE_DRIVER=local` files live
//! under `STORAGE_BASE_PATH/editor/` and render at `/storage/editor/<name>`; with `oss`/`s3`
//! they live in the bucket and render as absolute presigned URLs. Switching is `.env`-only.

use serde_json::{json, Value};
use uuid::Uuid;

use crate::config::storage;
use crate::errors::{AppError, AppResult};

/// Key namespace for rich-text editor uploads.
const EDITOR_PREFIX: &str = "editor";

/// Allowed image extensions (whitelist) keyed by detected magic-byte extension.
fn allowed_ext(ext: &str) -> bool {
    matches!(ext, "jpg" | "jpeg" | "png" | "gif" | "webp")
}

/// Validate magic bytes + store the image; returns `{ name, url, key }`.
pub async fn upload(bytes: &[u8]) -> AppResult<Value> {
    let kind = infer::get(bytes)
        .filter(|k| k.matcher_type() == infer::MatcherType::Image && allowed_ext(k.extension()))
        .ok_or_else(|| AppError::bad_request("Unsupported or invalid image file"))?;

    let name = format!("{}.{}", Uuid::new_v4(), kind.extension());
    let key = format!("{EDITOR_PREFIX}/{name}");
    storage::put(&key, bytes).await?;

    Ok(json!({ "name": name, "url": storage::object_url(&key), "key": key }))
}

/// List stored images (name + url + key).
pub async fn list() -> AppResult<Vec<Value>> {
    let keys = storage::list(EDITOR_PREFIX).await?;
    Ok(keys
        .into_iter()
        .map(|key| {
            let name = key.rsplit('/').next().unwrap_or(&key).to_string();
            json!({ "name": name, "url": storage::object_url(&key), "key": key })
        })
        .collect())
}

/// Delete by key; key MUST match `editor/<safe-name>` (anti path-traversal).
pub async fn delete(key: &str) -> AppResult<()> {
    let name = key
        .strip_prefix("editor/")
        .filter(|n| is_safe_name(n))
        .ok_or_else(|| AppError::bad_request("Invalid key"))?;
    // Re-normalise to the canonical key so no traversal leaks past validation.
    storage::delete(&format!("{EDITOR_PREFIX}/{name}")).await
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

    #[tokio::test]
    async fn rejects_traversal_keys() {
        std::env::set_var("STORAGE_DRIVER", "local");
        assert!(delete("editor/../../etc/passwd").await.is_err());
        assert!(delete("../secret").await.is_err());
        assert!(delete("noteditor/x.png").await.is_err());
        std::env::remove_var("STORAGE_DRIVER");
    }

    #[tokio::test]
    async fn rejects_non_image_bytes() {
        std::env::set_var("STORAGE_DRIVER", "local");
        assert!(upload(b"this is not an image").await.is_err());
        std::env::remove_var("STORAGE_DRIVER");
    }
}
