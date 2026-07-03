//! Global site-setting snapshot (the cached singleton `Setting`).
//!
//! Equivalent of NodeAdmin's `settingCache` (TTL + invalidate-on-update). Loaded once at
//! boot and refreshed whenever Setting is saved, so every render (`render_view`) can inject
//! the active theme + site fields without each controller re-querying the DB.

use std::sync::RwLock;

use serde_json::Value;

pub struct SiteSnapshot {
    pub theme_name: String,
    pub setting: Value,
}

// `RwLock::new` is const since Rust 1.63 → usable in a `static`.
static SITE: RwLock<Option<SiteSnapshot>> = RwLock::new(None);

/// Replace the cached snapshot (called on boot + after every Setting save).
pub fn set(theme_name: impl Into<String>, setting: Value) {
    if let Ok(mut guard) = SITE.write() {
        *guard = Some(SiteSnapshot {
            theme_name: theme_name.into(),
            setting,
        });
    }
}

/// Active theme name, if loaded.
pub fn theme_name() -> Option<String> {
    SITE.read().ok()?.as_ref().map(|s| s.theme_name.clone())
}

/// Cached setting object, if loaded.
pub fn setting() -> Option<Value> {
    SITE.read().ok()?.as_ref().map(|s| s.setting.clone())
}
