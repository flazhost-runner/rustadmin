//! Route-driven RBAC (mirrors NodeAdmin `AccessMiddleware` + `getAllRegisteredRoute`).
//!
//! Authorization is **not** subject-based. A permission is the tuple
//! `(name, method, guard)` where `name` is the named-route (e.g.
//! `admin.v1.access.user.delete`), `method` the HTTP method, and `guard` derived from the
//! name prefix (`api.*` → api, else web). Permissions are **auto-scanned from this
//! registry** — there is no hardcoded permission list.
//!
//! The registry mirrors exactly what the modules mount (names/paths/methods are pinned by
//! the PORTING_GUIDE). The `Authorized` request guard (Phase 3) derives `(name, method)`
//! from the live request via [`get_name_by_path_and_method`] then calls [`has_access`].

mod registry;

pub use registry::{registry, RouteEntry};

/// Which auth lane a route belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Guard {
    Web,
    Api,
}

impl Guard {
    pub fn as_str(self) -> &'static str {
        match self {
            Guard::Web => "web",
            Guard::Api => "api",
        }
    }
}

/// Guard for a named route: `api.*` → api, everything else → web.
pub fn guard_for(name: &str) -> Guard {
    if name.starts_with("api.") {
        Guard::Api
    } else {
        Guard::Web
    }
}

/// A permission as stored/derived: route name + HTTP method + guard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Permission {
    pub name: String,
    pub method: String,
    pub guard: String,
}

/// All permissions derivable from the route registry (used by the auto-sync in Phase 3).
pub fn derived_permissions() -> Vec<Permission> {
    registry()
        .iter()
        .map(|r| Permission {
            name: r.name.to_string(),
            method: r.method.to_uppercase(),
            guard: guard_for(r.name).as_str().to_string(),
        })
        .collect()
}

/// Reverse-lookup the named route for a concrete request `(path, method)`.
/// Path patterns use `<param>` segments (e.g. `/admin/v1/access/user/<id>/edit`).
pub fn get_name_by_path_and_method(path: &str, method: &str) -> Option<&'static str> {
    let method = method.to_uppercase();
    registry()
        .iter()
        .find(|r| r.method.eq_ignore_ascii_case(&method) && path_matches(r.path, path))
        .map(|r| r.name)
}

/// True if a concrete `path` matches a registry `pattern` (with `<param>` wildcards).
fn path_matches(pattern: &str, path: &str) -> bool {
    let p: Vec<&str> = split_path(pattern);
    let a: Vec<&str> = split_path(path);
    if p.len() != a.len() {
        return false;
    }
    p.iter()
        .zip(a.iter())
        .all(|(pp, aa)| pp.starts_with('<') || pp == aa)
}

fn split_path(path: &str) -> Vec<&str> {
    let trimmed = path.trim_matches('/');
    if trimmed.is_empty() {
        vec![] // root "/"
    } else {
        trimmed.split('/').collect()
    }
}

/// Core authorization check (matches **name AND method**; Administrator bypasses).
///
/// `perms` is the set the current user has, as `(name, method)` pairs (method uppercased).
pub fn has_access(is_admin: bool, perms: &[(String, String)], name: &str, method: &str) -> bool {
    if is_admin {
        return true;
    }
    let method = method.to_uppercase();
    perms
        .iter()
        .any(|(n, m)| n == name && m.eq_ignore_ascii_case(&method))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guard_derivation() {
        assert_eq!(guard_for("api.v1.access.user.index"), Guard::Api);
        assert_eq!(guard_for("admin.v1.access.user.index"), Guard::Web);
        assert_eq!(guard_for("web.auth.login"), Guard::Web);
    }

    #[test]
    fn reverse_lookup_with_params() {
        assert_eq!(
            get_name_by_path_and_method("/admin/v1/access/user/123/edit", "GET"),
            Some("admin.v1.access.user.edit")
        );
        assert_eq!(
            get_name_by_path_and_method("/admin/v1/access/user/123/delete", "DELETE"),
            Some("admin.v1.access.user.delete")
        );
        // same path, different method → different (or no) route
        assert_eq!(
            get_name_by_path_and_method("/admin/v1/access/user", "GET"),
            Some("admin.v1.access.user.index")
        );
        assert_eq!(
            get_name_by_path_and_method("/", "GET"),
            Some("web.home.root")
        );
        assert_eq!(get_name_by_path_and_method("/nope/nope", "GET"), None);
    }

    #[test]
    fn name_and_method_are_distinct_permissions() {
        let perms = vec![("admin.v1.access.user.index".to_string(), "GET".to_string())];
        assert!(has_access(
            false,
            &perms,
            "admin.v1.access.user.index",
            "GET"
        ));
        // has GET but not DELETE on the same name
        assert!(!has_access(
            false,
            &perms,
            "admin.v1.access.user.index",
            "DELETE"
        ));
        // administrator bypasses everything
        assert!(has_access(true, &[], "anything", "DELETE"));
    }

    #[test]
    fn registry_is_nonempty_and_unique() {
        let reg = registry();
        assert!(!reg.is_empty());
        // (name, method) pairs must be unique
        let mut seen = std::collections::HashSet::new();
        for r in reg.iter() {
            assert!(
                seen.insert((r.name, r.method)),
                "duplicate route entry: {} {}",
                r.method,
                r.name
            );
        }
    }
}
