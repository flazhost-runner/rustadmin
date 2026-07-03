//! `Authorized` guard — route-driven RBAC (authenticate → authorize).
//!
//! Derives `(name, method)` from the live request via the route registry reverse-lookup,
//! then checks [`crate::rbac::has_access`]. **No argument** is passed to the guard — the
//! permission to check IS the current route. Administrator bypasses (handled in `has_access`).

use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};

use crate::guards::current_user::CurrentUser;
use crate::rbac::{get_name_by_path_and_method, has_access};

/// Wraps the authenticated user once the current route's permission is satisfied.
pub struct Authorized(pub CurrentUser);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Authorized {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // 1) authenticate
        let user = match CurrentUser::from_request(req).await {
            Outcome::Success(u) => u,
            Outcome::Error(e) => return Outcome::Error(e),
            Outcome::Forward(f) => return Outcome::Forward(f),
        };

        // 2) authorize: resolve this route's name + method, then check access
        let path = req.uri().path().as_str();
        let method = req.method().as_str();
        match get_name_by_path_and_method(path, method) {
            Some(name) if has_access(user.is_admin, &user.perms, name, method) => {
                Outcome::Success(Authorized(user))
            }
            // Known route but not permitted, or route not in registry → forbid.
            _ => Outcome::Error((Status::Forbidden, ())),
        }
    }
}
