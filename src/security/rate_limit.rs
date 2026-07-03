//! In-memory sliding-window rate limiter.
//!
//! NodeAdmin standard:
//! - `authLimiter`: 10 requests / 15 minutes / IP — login, register, OTP request
//! - `otpLimiter`:  5 requests / 15 minutes / IP — OTP process (reset password)

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};

/// Sliding-window counter keyed by client IP.
pub struct RateLimiter {
    store: Mutex<HashMap<String, Vec<Instant>>>,
    max_requests: usize,
    window: Duration,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window_secs: u64) -> Self {
        Self {
            store: Mutex::new(HashMap::new()),
            max_requests,
            window: Duration::from_secs(window_secs),
        }
    }

    /// Returns `true` if the request is allowed; `false` if the limit is exceeded.
    pub fn check(&self, ip: &str) -> bool {
        let now = Instant::now();
        let mut store = self.store.lock().unwrap_or_else(|e| e.into_inner());
        let entries = store.entry(ip.to_string()).or_default();
        entries.retain(|&t| now.duration_since(t) < self.window);
        if entries.len() >= self.max_requests {
            return false;
        }
        entries.push(now);
        true
    }
}

/// Managed state wiring: one limiter for auth routes (10/15min) and one for OTP (5/15min).
pub struct AuthLimiter(pub RateLimiter);
pub struct OtpLimiter(pub RateLimiter);

impl AuthLimiter {
    pub fn new() -> Self {
        Self(RateLimiter::new(10, 15 * 60))
    }
}

impl Default for AuthLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl OtpLimiter {
    pub fn new() -> Self {
        Self(RateLimiter::new(5, 15 * 60))
    }
}

impl Default for OtpLimiter {
    fn default() -> Self {
        Self::new()
    }
}

fn client_ip(req: &Request<'_>) -> String {
    req.headers()
        .get_one("X-Forwarded-For")
        .or_else(|| req.headers().get_one("X-Real-IP"))
        .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
        .unwrap_or_else(|| {
            req.client_ip()
                .map(|ip| ip.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        })
}

fn is_loopback(ip: &str) -> bool {
    ip == "127.0.0.1" || ip == "::1" || ip == "0:0:0:0:0:0:0:1"
}

/// Request guard that enforces `authLimiter` (10/15min).
pub struct AuthRateLimit;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthRateLimit {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let ip = client_ip(req);
        if is_loopback(&ip) {
            return Outcome::Success(AuthRateLimit);
        }
        match req.rocket().state::<AuthLimiter>() {
            Some(limiter) if limiter.0.check(&ip) => Outcome::Success(AuthRateLimit),
            Some(_) => Outcome::Error((Status::TooManyRequests, ())),
            None => Outcome::Success(AuthRateLimit), // limiter not configured — pass through
        }
    }
}

/// Request guard that enforces `otpLimiter` (5/15min).
pub struct OtpRateLimit;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for OtpRateLimit {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let ip = client_ip(req);
        if is_loopback(&ip) {
            return Outcome::Success(OtpRateLimit);
        }
        match req.rocket().state::<OtpLimiter>() {
            Some(limiter) if limiter.0.check(&ip) => Outcome::Success(OtpRateLimit),
            Some(_) => Outcome::Error((Status::TooManyRequests, ())),
            None => Outcome::Success(OtpRateLimit),
        }
    }
}
