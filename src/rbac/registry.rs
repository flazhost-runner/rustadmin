//! The canonical named-route registry.
//!
//! Names/paths/methods are pinned by PORTING_GUIDE ("Named routes admin + METHOD"). The
//! `access` resources use the namespace `access` + **singular** resource (`user`/`role`/
//! `permission`). API is **symmetric & verbose** (NOT REST): same paths/names as web minus
//! the `create` form page.
//!
//! As modules are implemented they mount routes matching these entries; this list is the
//! single source of truth for RBAC + permission auto-sync.

/// One named route: stable `name`, HTTP `method`, and a path pattern (`<param>` segments).
#[derive(Debug, Clone, Copy)]
pub struct RouteEntry {
    pub name: &'static str,
    pub method: &'static str,
    pub path: &'static str,
}

const fn e(name: &'static str, method: &'static str, path: &'static str) -> RouteEntry {
    RouteEntry { name, method, path }
}

/// Full canonical registry (web + api).
pub fn registry() -> Vec<RouteEntry> {
    let mut v = vec![
        // ---- public / home ----
        e("web.home.root", "GET", "/"),
        e("web.home.index", "GET", "/home"),
        // ---- web auth ----
        e("web.auth.login", "GET", "/auth/login"),
        e("web.auth.login.post", "POST", "/auth/login"),
        e("web.auth.register", "GET", "/auth/register"),
        e("web.auth.register.post", "POST", "/auth/register"),
        e("web.auth.logout", "POST", "/auth/logout"),
        // ---- password reset (public, OTP) ----
        e("admin.v1.auth.reset.req", "GET", "/admin/v1/auth/reset/req"),
        e(
            "admin.v1.auth.reset.request",
            "POST",
            "/admin/v1/auth/reset/request",
        ),
        e(
            "admin.v1.auth.reset.proc",
            "GET",
            "/admin/v1/auth/reset/proc",
        ),
        e(
            "admin.v1.auth.reset.process",
            "POST",
            "/admin/v1/auth/reset/process",
        ),
        // ---- admin chrome pages ----
        e("admin.v1.dashboard.index", "GET", "/admin/v1/dashboard"),
        e("admin.v1.components.index", "GET", "/admin/v1/components"),
        e("admin.v1.profile.index", "GET", "/admin/v1/profile"),
        e("admin.v1.profile.update", "PUT", "/admin/v1/profile/update"),
        e("admin.v1.setting.index", "GET", "/admin/v1/setting"),
        e("admin.v1.setting.update", "PUT", "/admin/v1/setting/update"),
        e(
            "admin.v1.setting.fe_preview",
            "GET",
            "/admin/v1/setting/fe-preview/<slug>",
        ),
        // ---- media (rich-text file manager) ----
        e("admin.v1.media.list", "GET", "/admin/v1/media/list"),
        e("admin.v1.media.upload", "POST", "/admin/v1/media/upload"),
        e("admin.v1.media.delete", "POST", "/admin/v1/media/delete"),
        // ---- role → permission management (web) ----
        e(
            "admin.v1.access.role.permission",
            "GET",
            "/admin/v1/access/role/<id>/permission",
        ),
        e(
            "admin.v1.access.role.permission.assign",
            "GET",
            "/admin/v1/access/role/<id>/permission/<permission_id>/assign",
        ),
        e(
            "admin.v1.access.role.permission.assign_selected",
            "POST",
            "/admin/v1/access/role/<id>/permission/assign_selected",
        ),
        e(
            "admin.v1.access.role.permission.unassign",
            "GET",
            "/admin/v1/access/role/<id>/permission/<permission_id>/unassign",
        ),
        e(
            "admin.v1.access.role.permission.unassign_selected",
            "POST",
            "/admin/v1/access/role/<id>/permission/unassign_selected",
        ),
        // ---- role → permission management (api) ----
        e(
            "api.v1.access.role.permission",
            "GET",
            "/api/v1/access/role/<id>/permission",
        ),
        e(
            "api.v1.access.role.permission.assign",
            "GET",
            "/api/v1/access/role/<id>/permission/<permission_id>/assign",
        ),
        e(
            "api.v1.access.role.permission.assign_selected",
            "POST",
            "/api/v1/access/role/<id>/permission/assign_selected",
        ),
        e(
            "api.v1.access.role.permission.unassign",
            "GET",
            "/api/v1/access/role/<id>/permission/<permission_id>/unassign",
        ),
        e(
            "api.v1.access.role.permission.unassign_selected",
            "POST",
            "/api/v1/access/role/<id>/permission/unassign_selected",
        ),
        // ---- api auth ----
        e("api.v1.auth.login", "POST", "/api/v1/auth/login"),
        // logout = POST (mutation: blacklists the token — GET must not have side effects)
        e("api.v1.auth.logout", "POST", "/api/v1/auth/logout"),
    ];

    // access CRUD for each resource (web has the create form page; api omits it).
    for r in ["user", "role", "permission"] {
        v.extend(access_web_crud(r));
        v.extend(access_api_crud(r));
    }
    v
}

/// Web CRUD entries for an `access` resource (singular).
fn access_web_crud(resource: &'static str) -> Vec<RouteEntry> {
    // names/paths are static; map the resource to its pinned literals.
    match resource {
        "user" => vec![
            e("admin.v1.access.user.index", "GET", "/admin/v1/access/user"),
            e(
                "admin.v1.access.user.create",
                "GET",
                "/admin/v1/access/user/create",
            ),
            e(
                "admin.v1.access.user.store",
                "POST",
                "/admin/v1/access/user/store",
            ),
            e(
                "admin.v1.access.user.edit",
                "GET",
                "/admin/v1/access/user/<id>/edit",
            ),
            e(
                "admin.v1.access.user.update",
                "PUT",
                "/admin/v1/access/user/<id>/update",
            ),
            e(
                "admin.v1.access.user.delete",
                "DELETE",
                "/admin/v1/access/user/<id>/delete",
            ),
            e(
                "admin.v1.access.user.delete_selected",
                "POST",
                "/admin/v1/access/user/delete_selected",
            ),
        ],
        "role" => vec![
            e("admin.v1.access.role.index", "GET", "/admin/v1/access/role"),
            e(
                "admin.v1.access.role.create",
                "GET",
                "/admin/v1/access/role/create",
            ),
            e(
                "admin.v1.access.role.store",
                "POST",
                "/admin/v1/access/role/store",
            ),
            e(
                "admin.v1.access.role.edit",
                "GET",
                "/admin/v1/access/role/<id>/edit",
            ),
            e(
                "admin.v1.access.role.update",
                "PUT",
                "/admin/v1/access/role/<id>/update",
            ),
            e(
                "admin.v1.access.role.delete",
                "DELETE",
                "/admin/v1/access/role/<id>/delete",
            ),
            e(
                "admin.v1.access.role.delete_selected",
                "POST",
                "/admin/v1/access/role/delete_selected",
            ),
        ],
        "permission" => vec![
            e(
                "admin.v1.access.permission.index",
                "GET",
                "/admin/v1/access/permission",
            ),
            e(
                "admin.v1.access.permission.create",
                "GET",
                "/admin/v1/access/permission/create",
            ),
            e(
                "admin.v1.access.permission.store",
                "POST",
                "/admin/v1/access/permission/store",
            ),
            e(
                "admin.v1.access.permission.edit",
                "GET",
                "/admin/v1/access/permission/<id>/edit",
            ),
            e(
                "admin.v1.access.permission.update",
                "PUT",
                "/admin/v1/access/permission/<id>/update",
            ),
            e(
                "admin.v1.access.permission.delete",
                "DELETE",
                "/admin/v1/access/permission/<id>/delete",
            ),
            e(
                "admin.v1.access.permission.delete_selected",
                "POST",
                "/admin/v1/access/permission/delete_selected",
            ),
        ],
        _ => vec![],
    }
}

/// API CRUD entries (verbose paths, symmetric to web, minus the create form).
fn access_api_crud(resource: &'static str) -> Vec<RouteEntry> {
    match resource {
        "user" => vec![
            e("api.v1.access.user.index", "GET", "/api/v1/access/user"),
            e(
                "api.v1.access.user.store",
                "POST",
                "/api/v1/access/user/store",
            ),
            e(
                "api.v1.access.user.edit",
                "GET",
                "/api/v1/access/user/<id>/edit",
            ),
            e(
                "api.v1.access.user.update",
                "PUT",
                "/api/v1/access/user/<id>/update",
            ),
            e(
                "api.v1.access.user.delete",
                "DELETE",
                "/api/v1/access/user/<id>/delete",
            ),
            e(
                "api.v1.access.user.delete_selected",
                "POST",
                "/api/v1/access/user/delete_selected",
            ),
        ],
        "role" => vec![
            e("api.v1.access.role.index", "GET", "/api/v1/access/role"),
            e(
                "api.v1.access.role.store",
                "POST",
                "/api/v1/access/role/store",
            ),
            e(
                "api.v1.access.role.edit",
                "GET",
                "/api/v1/access/role/<id>/edit",
            ),
            e(
                "api.v1.access.role.update",
                "PUT",
                "/api/v1/access/role/<id>/update",
            ),
            e(
                "api.v1.access.role.delete",
                "DELETE",
                "/api/v1/access/role/<id>/delete",
            ),
            e(
                "api.v1.access.role.delete_selected",
                "POST",
                "/api/v1/access/role/delete_selected",
            ),
        ],
        "permission" => vec![
            e(
                "api.v1.access.permission.index",
                "GET",
                "/api/v1/access/permission",
            ),
            e(
                "api.v1.access.permission.store",
                "POST",
                "/api/v1/access/permission/store",
            ),
            e(
                "api.v1.access.permission.edit",
                "GET",
                "/api/v1/access/permission/<id>/edit",
            ),
            e(
                "api.v1.access.permission.update",
                "PUT",
                "/api/v1/access/permission/<id>/update",
            ),
            e(
                "api.v1.access.permission.delete",
                "DELETE",
                "/api/v1/access/permission/<id>/delete",
            ),
            e(
                "api.v1.access.permission.delete_selected",
                "POST",
                "/api/v1/access/permission/delete_selected",
            ),
        ],
        _ => vec![],
    }
}
