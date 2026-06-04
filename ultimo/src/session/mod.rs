//! Cookie-based session management.
//!
//! Enable with the `session` feature. Register the middleware with a store, then
//! read/write the session via [`Context::session`](crate::context::Context::session).
//!
//! ```
//! use ultimo::session::{session, MemoryStore, SessionConfig};
//! use ultimo::{Context, Ultimo};
//!
//! # async fn build() {
//! let mut app = Ultimo::new_without_defaults();
//! app.use_middleware(session(MemoryStore::new(), SessionConfig::default()));
//!
//! app.get("/login", |ctx: Context| async move {
//!     ctx.session().await.set("user_id", &42u64).await?;
//!     ctx.text("logged in").await
//! });
//! # }
//! ```

mod config;
mod middleware;
mod store;

pub use config::SessionConfig;
pub use middleware::session;
pub use store::{MemoryStore, SessionStore};

use crate::error::Result;
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Session payload: arbitrary JSON values keyed by string.
pub type SessionData = HashMap<String, serde_json::Value>;

/// A handle to the current session. Cheap to clone (shares inner state), so the
/// middleware and the handler observe the same session.
#[derive(Clone)]
pub struct Session {
    inner: Arc<SessionInner>,
}

struct SessionInner {
    id: RwLock<String>,
    data: RwLock<SessionData>,
    dirty: AtomicBool,
    destroyed: AtomicBool,
    regenerate: AtomicBool,
}

impl Session {
    pub(crate) fn new(id: String, data: SessionData) -> Self {
        Self {
            inner: Arc::new(SessionInner {
                id: RwLock::new(id),
                data: RwLock::new(data),
                dirty: AtomicBool::new(false),
                destroyed: AtomicBool::new(false),
                regenerate: AtomicBool::new(false),
            }),
        }
    }

    /// Current session id.
    pub async fn id(&self) -> String {
        self.inner.id.read().await.clone()
    }

    /// Get a typed value by key.
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let data = self.inner.data.read().await;
        match data.get(key) {
            Some(v) => Ok(Some(serde_json::from_value(v.clone())?)),
            None => Ok(None),
        }
    }

    /// Set a typed value (marks the session dirty).
    pub async fn set<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let v = serde_json::to_value(value)?;
        self.inner.data.write().await.insert(key.to_string(), v);
        self.inner.dirty.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Remove a key (marks dirty).
    pub async fn remove(&self, key: &str) {
        self.inner.data.write().await.remove(key);
        self.inner.dirty.store(true, Ordering::SeqCst);
    }

    /// Clear all data (marks dirty).
    pub async fn clear(&self) {
        self.inner.data.write().await.clear();
        self.inner.dirty.store(true, Ordering::SeqCst);
    }

    /// Request a fresh session id on the next persist (session-fixation
    /// defense — call this on login / privilege change). The middleware
    /// generates the new id and destroys the old store entry.
    pub fn regenerate(&self) {
        self.inner.regenerate.store(true, Ordering::SeqCst);
        self.inner.dirty.store(true, Ordering::SeqCst);
    }

    /// Destroy the session (server-side entry + cookie are cleared).
    pub fn destroy(&self) {
        self.inner.destroyed.store(true, Ordering::SeqCst);
    }

    // --- internal accessors used by the middleware ---
    pub(crate) fn is_dirty(&self) -> bool {
        self.inner.dirty.load(Ordering::SeqCst)
    }
    pub(crate) fn is_destroyed(&self) -> bool {
        self.inner.destroyed.load(Ordering::SeqCst)
    }
    pub(crate) async fn snapshot(&self) -> SessionData {
        self.inner.data.read().await.clone()
    }
    pub(crate) fn wants_regenerate(&self) -> bool {
        self.inner.regenerate.load(Ordering::SeqCst)
    }
    pub(crate) async fn is_empty(&self) -> bool {
        self.inner.data.read().await.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn set_get_and_dirty() {
        let s = Session::new("id".into(), SessionData::new());
        assert!(!s.is_dirty());
        s.set("n", &7u32).await.unwrap();
        assert!(s.is_dirty());
        assert_eq!(s.get::<u32>("n").await.unwrap(), Some(7));
        assert!(!s.is_empty().await);
    }

    #[tokio::test]
    async fn regenerate_sets_flag() {
        let s = Session::new("old".into(), SessionData::new());
        assert!(!s.wants_regenerate());
        s.regenerate();
        assert!(s.wants_regenerate());
        assert!(s.is_dirty());
    }
}
