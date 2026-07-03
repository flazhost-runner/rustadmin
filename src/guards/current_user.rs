//! `CurrentUser` (authentication) + `JwtClaims` guards, and web-session cookie helpers.
//!
//! Auth lanes:
//! - **Web**: a private (encrypted) cookie `uid` holds the user id (set on web login).
//! - **API**: an `Authorization: Bearer <jwt>` header; the token is verified (HS256) and its
//!   `jti` checked against the [`TokenStore`] blacklist (so logout truly invalidates it).

use std::sync::Arc;

use rocket::http::{Cookie, CookieJar, SameSite, Status};
use rocket::request::{FromRequest, Outcome, Request};

use crate::config::Config;
use crate::modules::access::context::{load_user_context, UserContext};
use crate::security::blacklist::TokenStore;
use crate::security::jwt;

use sea_orm::DatabaseConnection;

const SESSION_COOKIE: &str = "uid";

/// The authenticated user + resolved RBAC context.
#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub id: String,
    pub name: String,
    pub is_admin: bool,
    pub perms: Vec<(String, String)>,
}

impl From<UserContext> for CurrentUser {
    fn from(c: UserContext) -> Self {
        CurrentUser {
            id: c.id,
            name: c.name,
            is_admin: c.is_admin,
            perms: c.perms,
        }
    }
}

/// Set the web session cookie (called on successful web login).
pub fn set_web_session(cookies: &CookieJar<'_>, user_id: &str) {
    let cookie = Cookie::build((SESSION_COOKIE, user_id.to_string()))
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/");
    cookies.add_private(cookie);
}

/// Clear the web session cookie (logout).
pub fn clear_web_session(cookies: &CookieJar<'_>) {
    cookies.remove_private(Cookie::from(SESSION_COOKIE));
}

/// Resolve the user id from the web cookie or the API bearer token.
/// Returns `Err(Status)` to short-circuit the guard.
async fn resolve_user_id(req: &Request<'_>, cfg: &Config) -> Result<String, Status> {
    // Web session cookie first.
    if let Some(c) = req.cookies().get_private(SESSION_COOKIE) {
        return Ok(c.value().to_string());
    }
    // API bearer token.
    if let Some(auth) = req.headers().get_one("Authorization") {
        if let Some(token) = auth.strip_prefix("Bearer ") {
            let claims =
                jwt::verify(&cfg.jwt.secret, token.trim()).map_err(|_| Status::Unauthorized)?;
            // blacklist check (logout invalidation)
            if let Some(store) = req.rocket().state::<Arc<dyn TokenStore>>() {
                if store.is_blacklisted(&claims.jti) {
                    return Err(Status::Unauthorized);
                }
            }
            return Ok(claims.sub);
        }
    }
    Err(Status::Unauthorized)
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for CurrentUser {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let cfg = match req.rocket().state::<Config>() {
            Some(c) => c,
            None => return Outcome::Error((Status::InternalServerError, ())),
        };
        let db = match req.rocket().state::<DatabaseConnection>() {
            Some(d) => d,
            None => return Outcome::Error((Status::InternalServerError, ())),
        };

        let user_id = match resolve_user_id(req, cfg).await {
            Ok(id) => id,
            Err(status) => return Outcome::Error((status, ())),
        };

        match load_user_context(db, &user_id, &cfg.administrator_role).await {
            Ok(ctx) => Outcome::Success(CurrentUser::from(ctx)),
            Err(_) => Outcome::Error((Status::Unauthorized, ())),
        }
    }
}

/// Guard that only verifies a bearer JWT and yields its claims — **without** the blacklist
/// check or DB load. Used by logout to obtain the `jti` to blacklist.
pub struct JwtClaims(pub jwt::Claims);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for JwtClaims {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let cfg = match req.rocket().state::<Config>() {
            Some(c) => c,
            None => return Outcome::Error((Status::InternalServerError, ())),
        };
        let Some(auth) = req.headers().get_one("Authorization") else {
            return Outcome::Error((Status::Unauthorized, ()));
        };
        let Some(token) = auth.strip_prefix("Bearer ") else {
            return Outcome::Error((Status::Unauthorized, ()));
        };
        match jwt::verify(&cfg.jwt.secret, token.trim()) {
            Ok(claims) => Outcome::Success(JwtClaims(claims)),
            Err(_) => Outcome::Error((Status::Unauthorized, ())),
        }
    }
}
