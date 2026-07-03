//! Security infrastructure (cross-cutting): JWT (HS256), token blacklist, CSRF, security
//! headers, and HTTP method-override. Mirrors NodeAdmin's helmet/csrf/jwt/blacklist layer.
//!
//! Web auth uses Rocket **private (encrypted) cookies** — stateless, so the app scales
//! horizontally without sticky sessions or a server-side session store. The pieces that
//! genuinely need shared server state (JWT logout blacklist, rate-limit) use a
//! [`blacklist::TokenStore`] (in-memory default; Redis-backed impl pluggable).

pub mod blacklist;
pub mod csrf;
pub mod headers;
pub mod jwt;
pub mod method_override;
pub mod rate_limit;

pub use blacklist::{InMemoryTokenStore, TokenStore};
pub use jwt::Claims;
pub use rate_limit::{AuthLimiter, AuthRateLimit, OtpLimiter, OtpRateLimit};
