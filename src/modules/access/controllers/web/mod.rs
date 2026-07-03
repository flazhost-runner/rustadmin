//! Web (admin UI) controllers for the access module.

pub mod permission;
pub mod role;
pub mod user;

use serde_json::{json, Map, Value};

use crate::guards::CurrentUser;
use crate::helpers::view::nav_for;

/// Build the shared chrome locals (auth + sidebar gating + csrf + active menu key).
pub fn chrome(user: &CurrentUser, csrf: &str, active: &str) -> Value {
    json!({
        "auth": { "name": user.name, "picture": null },
        "nav": nav_for(user.is_admin, &user.perms),
        "csrf_token": csrf,
        "active": active,
    })
}

/// Merge object `extra` into object `base` (shallow); used to layer chrome onto page data.
pub fn merge(base: &mut Value, extra: Value) {
    if let (Value::Object(b), Value::Object(e)) = (base, extra) {
        for (k, v) in e {
            b.insert(k, v);
        }
    }
}

/// Build the `?q_*` query suffix (for pagination links) from current filters, excluding q_page.
pub fn filter_query(pairs: &[(&str, Option<&str>)]) -> String {
    let mut map: Map<String, Value> = Map::new();
    for (k, v) in pairs {
        if let Some(val) = v {
            if !val.trim().is_empty() {
                map.insert((*k).to_string(), json!(val));
            }
        }
    }
    let mut out = String::new();
    for (k, v) in &map {
        out.push('&');
        out.push_str(k);
        out.push('=');
        out.push_str(&urlencode(v.as_str().unwrap_or("")));
    }
    out
}

fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
