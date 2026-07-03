//! Web auth routes — session login/logout, registration, password reset (OTP).

use std::sync::Arc;

use rocket::form::Form;
use rocket::http::CookieJar;
use rocket::request::FlashMessage;
use rocket::response::{Flash, Redirect};
use rocket::State;
use rocket_dyn_templates::Template;
use sea_orm::DatabaseConnection;
use serde_json::json;

use crate::guards::{clear_web_session, set_web_session};
use crate::helpers::view::render_view;
use crate::modules::auth::service::IAuthService;
use crate::security::csrf::{ensure_token, CsrfProtected};
use crate::security::rate_limit::{AuthRateLimit, OtpRateLimit};

const DASHBOARD: &str = "/admin/v1/dashboard";
const LOGIN: &str = "/auth/login";

fn flash_v(f: &Option<FlashMessage<'_>>) -> serde_json::Value {
    match f {
        Some(m) => json!({ "key": m.kind(), "message": m.message() }),
        None => json!({}),
    }
}

#[derive(rocket::FromForm)]
pub struct LoginForm {
    pub email: String,
    pub password: String,
    pub remember: Option<String>,
}

#[derive(rocket::FromForm)]
pub struct RegisterForm {
    pub name: String,
    pub email: String,
    pub password: String,
}

#[derive(rocket::FromForm)]
pub struct ResetReqForm {
    pub email: String,
}

#[derive(rocket::FromForm)]
pub struct ResetProcForm {
    pub email: String,
    pub otp: String,
    pub password: String,
}

#[get("/auth/login")]
pub fn login_page(cookies: &CookieJar<'_>, flash: Option<FlashMessage<'_>>) -> Template {
    let csrf = ensure_token(cookies);
    render_view(
        "be/default/auth/login",
        json!({ "csrf_token": csrf, "flash": flash_v(&flash) }),
        None,
    )
}

#[post("/auth/login", data = "<form>")]
pub async fn login_post(
    _csrf: CsrfProtected,
    _rl: AuthRateLimit,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IAuthService>>,
    cookies: &CookieJar<'_>,
    form: Form<LoginForm>,
) -> Flash<Redirect> {
    match svc
        .authenticate(db.inner(), &form.email, &form.password)
        .await
    {
        Ok(user) => {
            set_web_session(cookies, &user.id);
            Flash::success(Redirect::to(DASHBOARD), "Login Success.")
        }
        Err(e) => Flash::error(Redirect::to(LOGIN), e.message().to_string()),
    }
}

#[post("/auth/logout")]
pub fn logout(_csrf: CsrfProtected, cookies: &CookieJar<'_>) -> Flash<Redirect> {
    clear_web_session(cookies);
    Flash::success(Redirect::to(LOGIN), "Logged out")
}

#[get("/auth/register")]
pub fn register_page(cookies: &CookieJar<'_>, flash: Option<FlashMessage<'_>>) -> Template {
    let csrf = ensure_token(cookies);
    render_view(
        "be/default/auth/register",
        json!({ "csrf_token": csrf, "flash": flash_v(&flash) }),
        None,
    )
}

#[post("/auth/register", data = "<form>")]
pub async fn register_post(
    _csrf: CsrfProtected,
    _rl: AuthRateLimit,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IAuthService>>,
    cookies: &CookieJar<'_>,
    form: Form<RegisterForm>,
) -> Flash<Redirect> {
    if form.password.len() < 8 {
        return Flash::error(
            Redirect::to("/auth/register"),
            "Password must be at least 8 characters",
        );
    }
    match svc
        .register(db.inner(), &form.name, &form.email, &form.password)
        .await
    {
        Ok(user) => {
            set_web_session(cookies, &user.id);
            Flash::success(Redirect::to(DASHBOARD), "Register Success.")
        }
        Err(e) => Flash::error(Redirect::to("/auth/register"), e.message().to_string()),
    }
}

#[get("/admin/v1/auth/reset/req")]
pub fn reset_req(cookies: &CookieJar<'_>, flash: Option<FlashMessage<'_>>) -> Template {
    let csrf = ensure_token(cookies);
    render_view(
        "be/default/auth/reset_req",
        json!({ "csrf_token": csrf, "flash": flash_v(&flash) }),
        None,
    )
}

#[post("/admin/v1/auth/reset/request", data = "<form>")]
pub async fn reset_request(
    _csrf: CsrfProtected,
    _rl: AuthRateLimit,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IAuthService>>,
    form: Form<ResetReqForm>,
) -> Flash<Redirect> {
    match svc.request_password_reset(db.inner(), &form.email).await {
        Ok(_) => Flash::success(
            Redirect::to("/admin/v1/auth/reset/proc"),
            "OTP Send Success.",
        ),
        Err(e) => Flash::error(
            Redirect::to("/admin/v1/auth/reset/req"),
            e.message().to_string(),
        ),
    }
}

#[get("/admin/v1/auth/reset/proc")]
pub fn reset_proc(cookies: &CookieJar<'_>, flash: Option<FlashMessage<'_>>) -> Template {
    let csrf = ensure_token(cookies);
    render_view(
        "be/default/auth/reset_proc",
        json!({ "csrf_token": csrf, "flash": flash_v(&flash) }),
        None,
    )
}

#[post("/admin/v1/auth/reset/process", data = "<form>")]
pub async fn reset_process(
    _csrf: CsrfProtected,
    _rl: OtpRateLimit,
    db: &State<DatabaseConnection>,
    svc: &State<Arc<dyn IAuthService>>,
    form: Form<ResetProcForm>,
) -> Flash<Redirect> {
    if form.password.len() < 8 {
        return Flash::error(
            Redirect::to("/admin/v1/auth/reset/proc"),
            "Password must be at least 8 characters",
        );
    }
    match svc
        .reset_password(db.inner(), &form.email, &form.otp, &form.password)
        .await
    {
        Ok(_) => Flash::success(Redirect::to(LOGIN), "Reset Password Success."),
        Err(e) => Flash::error(
            Redirect::to("/admin/v1/auth/reset/proc"),
            e.message().to_string(),
        ),
    }
}

pub fn routes() -> Vec<rocket::Route> {
    routes![
        login_page,
        login_post,
        logout,
        register_page,
        register_post,
        reset_req,
        reset_request,
        reset_proc,
        reset_process,
    ]
}
