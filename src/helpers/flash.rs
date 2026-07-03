//! One-redirect form state (inline per-field errors + `old` input), PRG pattern.
//!
//! Mirrors NodeAdmin's `errors`/`old` stashed in the session for a single redirect. We store
//! a small JSON blob in a private (encrypted) cookie that is read-and-cleared on the next GET,
//! then exposed to the view as `errors` (field→message) and `old` (field→value).

use rocket::http::{Cookie, CookieJar};
use serde_json::{json, Value};

const FORM_COOKIE: &str = "form_state";

/// Stash validation `errors` + `old` input for the next request (the redirect target).
pub fn stash(cookies: &CookieJar<'_>, errors: &Value, old: &Value) {
    let payload = json!({ "errors": errors, "old": old }).to_string();
    cookies.add_private(Cookie::build((FORM_COOKIE, payload)).path("/"));
}

/// Read-and-clear the stashed `(errors, old)`. Returns empty objects when absent.
pub fn take(cookies: &CookieJar<'_>) -> (Value, Value) {
    if let Some(c) = cookies.get_private(FORM_COOKIE) {
        let parsed: Value = serde_json::from_str(c.value()).unwrap_or_else(|_| json!({}));
        cookies.remove_private(Cookie::from(FORM_COOKIE));
        let errors = parsed.get("errors").cloned().unwrap_or_else(|| json!({}));
        let old = parsed.get("old").cloned().unwrap_or_else(|| json!({}));
        return (errors, old);
    }
    (json!({}), json!({}))
}
