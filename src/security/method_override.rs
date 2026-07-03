//! HTTP method-override fairing (mirrors NodeAdmin `method-override`).
//!
//! HTML forms can only `GET`/`POST`, so update/delete forms POST to `...?_method=PUT|DELETE`.
//! This is a **request fairing**, which runs *before* routing — so Rocket matches the route
//! by the overridden method (the Go port needed a server-level wrapper for the same reason).
//! Only `POST` is upgraded, and only to PUT/PATCH/DELETE.

use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Method;
use rocket::{Data, Request};

pub struct MethodOverride;

#[rocket::async_trait]
impl Fairing for MethodOverride {
    fn info(&self) -> Info {
        Info {
            name: "Method Override",
            kind: Kind::Request,
        }
    }

    async fn on_request(&self, req: &mut Request<'_>, _data: &mut Data<'_>) {
        if req.method() != Method::Post {
            return;
        }
        let Some(Ok(m)) = req.query_value::<String>("_method") else {
            return;
        };
        match m.to_ascii_uppercase().as_str() {
            "PUT" => req.set_method(Method::Put),
            "PATCH" => req.set_method(Method::Patch),
            "DELETE" => req.set_method(Method::Delete),
            _ => {}
        }
    }
}
