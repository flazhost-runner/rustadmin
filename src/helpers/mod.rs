//! DRY helpers shared across modules (mirrors NodeAdmin `helpers/` + `utils/`).
//!
//! - [`pagination`] — page math + meta + windowed page numbers.
//! - [`ci_like`] — case-insensitive LIKE for any dialect (`LOWER(col) LIKE LOWER(?)`).
//! - [`otp`] — secure OTP generate/hash/verify.
//! - [`view`] — Tera rendering with the standard theme/setting locals injected.
//! - [`response`] — JSON response envelope for the API.
//! - [`forms`] — query/body cleanup (strip empty filter fields).

pub mod ci_like;
pub mod flash;
pub mod forms;
pub mod otp;
pub mod pagination;
pub mod response;
pub mod view;

pub use ci_like::ci_like;
pub use pagination::{page_window, PageParams, PaginationMeta};
