//! API (REST/JWT) controllers for the access module. Paths are **verbose & symmetric** to
//! web (NOT REST): `/store`, `/<id>/edit`, `/<id>/update` (PUT), `/<id>/delete` (DELETE),
//! `/delete_selected`.

pub mod permission;
pub mod role;
pub mod user;
