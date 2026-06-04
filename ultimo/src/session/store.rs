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
#[derive(Clone, Default)]
pub struct MemoryStore {
    inner: Arc<RwLock<HashMap<String, (SessionData, Instant)>>>,
}

impl MemoryStore {
    /// Create an empty in-memory store.
    pub fn new() -> Self {
        Self::default()
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
}
