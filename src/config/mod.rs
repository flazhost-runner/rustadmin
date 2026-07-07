//! Centralised, validated configuration (Twelve-Factor).
//!
//! Mirrors NodeAdmin's `src/config/*`: env is read **only here** (modules never touch
//! the environment directly — enforced by the convention checker). Secrets fail-fast in
//! production. Sub-modules: [`env`] (the [`Config`] struct), [`themes`] (9 admin palettes),
//! [`app`] (active view/layout), [`fe_templates`] (frontend-template catalog metadata).

pub mod app;
pub mod env;
pub mod fe_templates;
pub mod storage;
pub mod themes;

pub use env::{
    app_root, asset, bind_port, storage, storage_base_path, AppMode, Config, StorageConfig,
};
pub use themes::{Theme, DEFAULT_THEME, THEMES, THEME_NAMES};
