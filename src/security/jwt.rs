//! JWT issuance/verification — **HS256 pinned** (mirrors NodeAdmin `env.jwt.algorithm`).
//!
//! Claims carry `sub` (user id) + `jti` (unique id, used for blacklist on logout) + `exp`/`iat`.

use chrono::Utc;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject — the user id.
    pub sub: String,
    /// JWT id — unique per token; the blacklist key on logout.
    pub jti: String,
    /// Expiry (epoch seconds).
    pub exp: i64,
    /// Issued-at (epoch seconds).
    pub iat: i64,
}

impl Claims {
    /// Seconds remaining until expiry (>= 0). Used as the blacklist TTL on logout.
    pub fn ttl_secs(&self) -> i64 {
        (self.exp - Utc::now().timestamp()).max(0)
    }
}

/// Issue a signed token for `user_id`, valid for `expires_secs`.
pub fn issue(secret: &str, user_id: &str, expires_secs: i64) -> Result<(String, Claims), AppError> {
    let now = Utc::now().timestamp();
    let claims = Claims {
        sub: user_id.to_string(),
        jti: Uuid::new_v4().to_string(),
        iat: now,
        exp: now + expires_secs.max(1),
    };
    let token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::internal(format!("jwt encode: {e}")))?;
    Ok((token, claims))
}

/// Verify a token (HS256 only) and return its claims. Rejects wrong-alg / expired tokens.
pub fn verify(secret: &str, token: &str) -> Result<Claims, AppError> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map(|data| data.claims)
    .map_err(|_| AppError::unauthorized("Invalid or expired token"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let (token, claims) = issue("secret", "user-1", 3600).unwrap();
        let decoded = verify("secret", &token).unwrap();
        assert_eq!(decoded.sub, "user-1");
        assert_eq!(decoded.jti, claims.jti);
    }

    #[test]
    fn rejects_wrong_secret() {
        let (token, _) = issue("secret", "user-1", 3600).unwrap();
        assert!(verify("other-secret", &token).is_err());
    }

    #[test]
    fn rejects_expired() {
        // negative-ish: expires immediately, so it's already expired by validation leeway? use 1s then can't wait.
        let (token, _) = issue("secret", "u", 1).unwrap();
        // tamper exp into the past by re-issuing with a manual claim
        let now = Utc::now().timestamp();
        // exp well beyond the default 60s validation leeway so it's unambiguously expired
        let past = Claims {
            sub: "u".into(),
            jti: "j".into(),
            iat: now - 7300,
            exp: now - 7200,
        };
        let expired = encode(
            &Header::new(Algorithm::HS256),
            &past,
            &EncodingKey::from_secret(b"secret"),
        )
        .unwrap();
        assert!(verify("secret", &expired).is_err());
        // sanity: the fresh token still verifies
        assert!(verify("secret", &token).is_ok());
    }
}
