//! Security response headers (helmet equivalent) + HTML content-type for template responses.

use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::{ContentType, Header};
use rocket::{Request, Response};

/// Serve template (Tera) output as `text/html`.
///
/// `rocket_dyn_templates` derives a response's Content-Type from the template name's
/// extension. RustAdmin templates use a single `.tera` extension (so template names match the
/// `be_view()` paths — no inner `.html`), which leaves the type as `text/plain`. With
/// `X-Content-Type-Options: nosniff` set, browsers then render the HTML as plain text. This
/// fairing rewrites `text/plain` responses to `text/html`. The only other `text/plain`
/// endpoint is `/healthz` (a harmless "ok"); JSON, static files, and redirects are untouched.
pub struct HtmlContentType;

#[rocket::async_trait]
impl Fairing for HtmlContentType {
    fn info(&self) -> Info {
        Info {
            name: "HTML content-type for templates",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _req: &'r Request<'_>, res: &mut Response<'r>) {
        if res.content_type() == Some(ContentType::Plain) {
            res.set_header(ContentType::HTML);
        }
    }
}

pub struct SecurityHeaders;

#[rocket::async_trait]
impl Fairing for SecurityHeaders {
    fn info(&self) -> Info {
        Info {
            name: "Security Headers",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _req: &'r Request<'_>, res: &mut Response<'r>) {
        res.set_header(Header::new("X-Content-Type-Options", "nosniff"));
        res.set_header(Header::new("X-Frame-Options", "DENY"));
        res.set_header(Header::new("Referrer-Policy", "no-referrer"));
        res.set_header(Header::new(
            "Strict-Transport-Security",
            "max-age=31536000; includeSubDomains",
        ));
    }
}
