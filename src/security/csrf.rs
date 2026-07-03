//! CSRF protection (synchronizer token).
//!
//! The token lives in a **private (encrypted) cookie**; mutating requests must echo it via
//! the `X-CSRF-Token` header *or* a `_csrf` query param. Per the PORTING_GUIDE per-language
//! caveat, RustAdmin reads token from **header → query** (NOT the form body) so the check
//! never has to consume/parse the request body — uniform across POST/PUT/DELETE. Mutating
//! forms therefore put the token in the action query string:
//! `action="...?_csrf={{ csrf_token }}&_method=PUT"`.

use rocket::http::{Cookie, CookieJar, SameSite, Status};
use rocket::request::{FromRequest, Outcome, Request};
use subtle::ConstantTimeEq;
use uuid::Uuid;

pub const CSRF_COOKIE: &str = "csrf_token";

/// Get the current CSRF token, creating + storing one if absent. Controllers call this to
/// embed the token into rendered forms.
pub fn ensure_token(cookies: &CookieJar<'_>) -> String {
    if let Some(c) = cookies.get_private(CSRF_COOKIE) {
        return c.value().to_string();
    }
    let token = Uuid::new_v4().to_string();
    let cookie = Cookie::build((CSRF_COOKIE, token.clone()))
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/");
    cookies.add_private(cookie);
    token
}

/// The submitted token from header or query (no body parsing).
/// Accepts both `x-csrf-token` (lowercase, standard) and `X-CSRF-Token` (legacy).
fn submitted_token(req: &Request<'_>) -> Option<String> {
    // Try lowercase first (standard NodeAdmin client sends lowercase)
    for name in &["x-csrf-token", "X-CSRF-Token", "X-Csrf-Token"] {
        if let Some(h) = req.headers().get_one(name) {
            return Some(h.to_string());
        }
    }
    match req.query_value::<String>("_csrf") {
        Some(Ok(v)) => Some(v),
        _ => None,
    }
}

/// Request guard enforcing a valid CSRF token on mutating web routes. Yields `403` on failure.
pub struct CsrfProtected;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for CsrfProtected {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let expected = req
            .cookies()
            .get_private(CSRF_COOKIE)
            .map(|c| c.value().to_string());
        match (expected, submitted_token(req)) {
            (Some(e), Some(s)) if !e.is_empty() && e.as_bytes().ct_eq(s.as_bytes()).into() => {
                Outcome::Success(CsrfProtected)
            }
            _ => Outcome::Error((Status::Forbidden, ())),
        }
    }
}
