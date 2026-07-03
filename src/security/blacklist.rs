//! JWT blacklist store (logout invalidation).
//!
//! On logout the token's `jti` is blacklisted for its remaining TTL; the API guard rejects
//! any token whose `jti` is listed. **NodeAdmin lesson**: this must be tested against a store
//! that *behaves like the runtime one* (entries actually expire / actually persist within the
//! process), not a mock that always returns "not blacklisted". The in-memory store below does
//! exactly that, and the integration test exercises login→access→logout→401 through it.
//!
//! A Redis-backed impl can be dropped in behind the same trait for multi-instance deploys.

use std::collections::HashMap;
use std::sync::Mutex;

use chrono::Utc;

/// Abstraction over the blacklist backend (DIP — guards depend on this trait).
pub trait TokenStore: Send + Sync {
    /// Blacklist `jti` for `ttl_secs` seconds.
    fn blacklist(&self, jti: &str, ttl_secs: i64);
    /// Is `jti` currently blacklisted (and not yet expired)?
    fn is_blacklisted(&self, jti: &str) -> bool;
}

/// Process-local blacklist with real expiry semantics.
#[derive(Default)]
pub struct InMemoryTokenStore {
    /// jti → expiry (epoch seconds).
    inner: Mutex<HashMap<String, i64>>,
}

impl InMemoryTokenStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl TokenStore for InMemoryTokenStore {
    fn blacklist(&self, jti: &str, ttl_secs: i64) {
        let expiry = Utc::now().timestamp() + ttl_secs.max(0);
        let mut map = self.inner.lock().unwrap();
        map.insert(jti.to_string(), expiry);
    }

    fn is_blacklisted(&self, jti: &str) -> bool {
        let now = Utc::now().timestamp();
        let mut map = self.inner.lock().unwrap();
        match map.get(jti).copied() {
            Some(expiry) if expiry > now => true,
            Some(_) => {
                // expired → clean up and treat as not blacklisted (mirrors Redis TTL eviction)
                map.remove(jti);
                false
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blacklists_until_ttl() {
        let store = InMemoryTokenStore::new();
        assert!(!store.is_blacklisted("jti-1"));
        store.blacklist("jti-1", 3600);
        assert!(store.is_blacklisted("jti-1"));
        assert!(!store.is_blacklisted("jti-2"));
    }

    #[test]
    fn expired_entry_is_evicted() {
        let store = InMemoryTokenStore::new();
        store.blacklist("jti-x", -1); // already expired
        assert!(!store.is_blacklisted("jti-x"));
    }
}
