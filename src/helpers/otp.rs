//! Secure OTP helpers (mirrors NodeAdmin `helpers/otp.ts`).
//!
//! OTP = crypto-random digits, stored **hashed** (bcrypt) with an expiry — never plaintext,
//! never `Math.random`. Used by the password-reset flow.

use bcrypt::{hash, verify};
use rand::rngs::OsRng;
use rand::Rng;

use crate::errors::AppResult;

/// Generate a numeric OTP of `len` digits using a cryptographically secure RNG.
pub fn generate_otp(len: usize) -> String {
    let mut rng = OsRng;
    (0..len)
        .map(|_| std::char::from_digit(rng.gen_range(0..10), 10).unwrap())
        .collect()
}

/// Hash an OTP with bcrypt at the configured cost.
pub fn hash_otp(otp: &str, rounds: u32) -> AppResult<String> {
    Ok(hash(otp, rounds)?)
}

/// Verify a plaintext OTP against its bcrypt hash. Returns `false` on any error.
pub fn verify_otp(otp: &str, hashed: &str) -> bool {
    verify(otp, hashed).unwrap_or(false)
}

/// Absolute expiry timestamp (epoch ms) given "now" and a validity window in ms.
/// `password_otp_expires` is a `bigint` (epoch ms) in the canonical schema.
pub fn expiry_from(now_ms: i64, window_ms: i64) -> i64 {
    now_ms + window_ms
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn otp_is_digits_of_len() {
        let otp = generate_otp(6);
        assert_eq!(otp.len(), 6);
        assert!(otp.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn hash_roundtrip() {
        let otp = generate_otp(6);
        let h = hash_otp(&otp, 4).unwrap(); // low cost for test speed
        assert!(verify_otp(&otp, &h));
        assert!(!verify_otp("000000", &h) || otp == "000000");
        assert!(!verify_otp(&otp, "not-a-hash"));
    }

    #[test]
    fn expiry_math() {
        assert_eq!(expiry_from(1_000, 600_000), 601_000);
    }
}
