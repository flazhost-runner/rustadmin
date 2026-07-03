//! `media` module — rich-text file manager backend (upload/list/delete images).
//! Auth via session + CSRF header; magic-byte validation (never trust client MIME).

pub mod controllers;
pub mod service;
