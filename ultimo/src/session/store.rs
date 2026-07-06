//! Session store trait and in-memory implementation.

use super::SessionData;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Backing store for session data. Implement this for Redis/SQL/etc. backends.
#[async_trait]
pub trait SessionStore: Send + Sync {
    /// Load session data by id, or `None` if absent/expired.
    async fn load(&self, id: &str) -> Option<SessionData>;
    /// Persist session data under id with a time-to-live.
    async fn store(&self, id: &str, data: &SessionData, ttl: Duration);
    /// Remove a session.
    async fn destroy(&self, id: &str);
}

/// In-memory store. Suitable for single-process apps and tests.
///
/// Unbounded by default (matches pre-1.0 behavior). Under sustained load —
/// especially if any pre-auth handler ever writes to the session — an
/// unbounded store can be grown without limit by an attacker minting fresh
/// sessions. For production, prefer [`with_max_sessions`](Self::with_max_sessions)
/// to cap live sessions (oldest-expiring evicted first), or use a shared
/// backend (Redis/SQL) sized by its own eviction policy.
#[derive(Clone, Default)]
pub struct MemoryStore {
    inner: Arc<RwLock<HashMap<String, (SessionData, Instant)>>>,
    max_sessions: Option<usize>,
}

impl MemoryStore {
    /// Create an empty in-memory store with no cap on live sessions.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an in-memory store capped at `max` live sessions. Once at
    /// capacity, the soonest-to-expire session is evicted to make room for a
    /// new one — a bound on worst-case memory use under sustained load.
    pub fn with_max_sessions(max: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
            max_sessions: Some(max),
        }
    }
}

#[async_trait]
impl SessionStore for MemoryStore {
    async fn load(&self, id: &str) -> Option<SessionData> {
        let map = self.inner.read().await;
        match map.get(id) {
            Some((data, expiry)) if *expiry > Instant::now() => Some(data.clone()),
            _ => None,
        }
    }

    async fn store(&self, id: &str, data: &SessionData, ttl: Duration) {
        let mut map = self.inner.write().await;
        // Opportunistic eviction of expired entries.
        let now = Instant::now();
        map.retain(|_, (_, exp)| *exp > now);

        if let Some(max) = self.max_sessions {
            // Make room by evicting the soonest-to-expire entries first.
            while map.len() >= max && !map.contains_key(id) {
                let Some(oldest) = map
                    .iter()
                    .min_by_key(|(_, (_, exp))| *exp)
                    .map(|(k, _)| k.clone())
                else {
                    break;
                };
                map.remove(&oldest);
            }
        }

        map.insert(id.to_string(), (data.clone(), now + ttl));
    }

    async fn destroy(&self, id: &str) {
        self.inner.write().await.remove(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn store_load_destroy_roundtrip() {
        let s = MemoryStore::new();
        let mut data = SessionData::new();
        data.insert("k".into(), serde_json::json!("v"));
        s.store("id1", &data, Duration::from_secs(60)).await;
        assert_eq!(
            s.load("id1").await.unwrap().get("k").unwrap(),
            &serde_json::json!("v")
        );
        s.destroy("id1").await;
        assert!(s.load("id1").await.is_none());
    }

    #[tokio::test]
    async fn expired_entries_are_not_loaded() {
        let s = MemoryStore::new();
        s.store("id2", &SessionData::new(), Duration::from_millis(1))
            .await;
        tokio::time::sleep(Duration::from_millis(5)).await;
        assert!(s.load("id2").await.is_none());
    }

    #[tokio::test]
    async fn max_sessions_evicts_soonest_expiring() {
        // Bound memory: at capacity, the entry closest to expiry is evicted
        // to make room for a new session (anti unbounded-growth DoS).
        let s = MemoryStore::with_max_sessions(2);
        s.store("a", &SessionData::new(), Duration::from_secs(10))
            .await;
        s.store("b", &SessionData::new(), Duration::from_secs(20))
            .await;
        assert!(s.load("a").await.is_some());
        assert!(s.load("b").await.is_some());

        // "c" pushes us over capacity; "a" (soonest expiry) is evicted.
        s.store("c", &SessionData::new(), Duration::from_secs(30))
            .await;
        assert!(s.load("a").await.is_none(), "oldest-expiring entry evicted");
        assert!(s.load("b").await.is_some());
        assert!(s.load("c").await.is_some());
    }

    #[tokio::test]
    async fn max_sessions_allows_re_storing_existing_id() {
        // Updating an existing id must not evict itself to make room.
        let s = MemoryStore::with_max_sessions(1);
        s.store("a", &SessionData::new(), Duration::from_secs(10))
            .await;
        let mut data = SessionData::new();
        data.insert("k".into(), serde_json::json!("v"));
        s.store("a", &data, Duration::from_secs(10)).await;
        assert_eq!(
            s.load("a").await.unwrap().get("k").unwrap(),
            &serde_json::json!("v")
        );
    }
}
