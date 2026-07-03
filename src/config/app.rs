//! Active view/layout selection (mirrors NodeAdmin `src/config/app.ts`).
//!
//! The backend uses the `be/default` view set; the frontend (landing) uses `fe/default`.
//! Tera template names are referenced relative to the `templates/` root.

/// Backend (admin) view namespace.
pub const BE_VIEW: &str = "be/default";
/// Backend layout namespace.
pub const BE_LAYOUT: &str = "layouts/be/default";
/// Frontend (landing) view namespace.
pub const FE_VIEW: &str = "fe/default";
/// Frontend layout namespace.
pub const FE_LAYOUT: &str = "layouts/fe/default";

/// Build a backend template path, e.g. `be_view("access/users/index")`.
pub fn be_view(view: &str) -> String {
    format!("{BE_VIEW}/{view}")
}

/// Build a frontend template path.
pub fn fe_view(view: &str) -> String {
    format!("{FE_VIEW}/{view}")
}
