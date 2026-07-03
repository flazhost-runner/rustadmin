//! Form/query helpers (mirrors NodeAdmin `removeEmptyFields()`).
//!
//! Used to clean per-column filter maps (`q_*`) before building queries, and to expose
//! `old` input back to forms after a validation redirect.

use std::collections::BTreeMap;

/// Drop entries whose value is empty/whitespace. Returns a stable (sorted) map so the
/// resulting query string is deterministic.
pub fn remove_empty_fields<I, K, V>(pairs: I) -> BTreeMap<String, String>
where
    I: IntoIterator<Item = (K, V)>,
    K: Into<String>,
    V: Into<String>,
{
    pairs
        .into_iter()
        .map(|(k, v)| (k.into(), v.into()))
        .filter(|(_, v)| !v.trim().is_empty())
        .collect()
}

/// Build a `?a=b&c=d` query string (values percent-safe enough for our ascii filter values).
pub fn to_query_string(pairs: &BTreeMap<String, String>) -> String {
    if pairs.is_empty() {
        return String::new();
    }
    let q = pairs
        .iter()
        .map(|(k, v)| format!("{}={}", k, urlencode(v)))
        .collect::<Vec<_>>()
        .join("&");
    format!("?{q}")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_empty() {
        let m = remove_empty_fields([("q_name", "  "), ("q_status", "Active"), ("q_code", "")]);
        assert_eq!(m.len(), 1);
        assert_eq!(m.get("q_status").map(String::as_str), Some("Active"));
    }

    #[test]
    fn query_string_sorted_and_encoded() {
        let m = remove_empty_fields([("q_name", "John Doe"), ("q_status", "Active")]);
        assert_eq!(to_query_string(&m), "?q_name=John%20Doe&q_status=Active");
        assert_eq!(to_query_string(&BTreeMap::new()), "");
    }
}
