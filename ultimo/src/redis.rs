//! Redis-backed stores, so several app instances behind a load balancer share
//! sessions and rate limits.
//!
//! Requires the `redis` Cargo feature. Built on the [`redis`](https://docs.rs/redis)
//! crate (re-exported as [`client`]), whose 1.x line needs **Rust 1.88** — the
//! rest of Ultimo keeps its 1.86 MSRV.
//!
//! - [`RedisSessionStore`] — a [`SessionStore`](crate::session::SessionStore)
//!   for the `session` middleware (also needs the `session` feature).
//! - [`RedisRateLimitStore`] — a [`RateLimitStore`] for
//!   [`RateLimiter::store`](crate::middleware::builtin::RateLimiter::store).
//!
//! Both take a [`ConnectionManager`]: one multiplexed connection that
//! reconnects on its own and is cheap to clone, so a single
//! [`connect`] at startup serves the whole app.
//!
//! ```rust,no_run
//! use ultimo::prelude::*;
//! use ultimo::middleware::builtin::RateLimiter;
//! use ultimo::redis::{connect, RedisRateLimitStore, RedisSessionStore};
//! use ultimo::session::{session, SessionConfig};
//!
//! # async fn run() -> ultimo::Result<()> {
//! let conn = connect("redis://127.0.0.1:6379").await?;
//!
//! let mut app = Ultimo::new();
//! app.use_middleware(session(
//!     RedisSessionStore::new(conn.clone()),
//!     SessionConfig::default(),
//! ));
//! app.use_middleware(
//!     RateLimiter::new(100, 60)
//!         .store(RedisRateLimitStore::new(conn))
//!         .build(),
//! );
//! # Ok(())
//! # }
//! ```

use crate::error::{Result, UltimoError};
use crate::middleware::builtin::RateLimitStore;
use ::redis::aio::ConnectionManager;
use ::redis::Script;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

/// The [`redis`](https://docs.rs/redis) crate this module is built on, so you
/// can use its types (e.g. `client::aio::ConnectionManager`) without adding a
/// matching dependency yourself.
pub use ::redis as client;

/// Open a [`ConnectionManager`] to `url` (`redis://host:port/db`, or
/// `rediss://` with one of the redis crate's TLS features enabled).
///
/// Fails if Redis can't be reached now; after that the manager reconnects on
/// its own.
pub async fn connect(url: &str) -> Result<ConnectionManager> {
    let client = ::redis::Client::open(url)
        .map_err(|e| UltimoError::Internal(format!("invalid Redis URL: {e}")))?;
    client
        .get_connection_manager()
        .await
        .map_err(|e| UltimoError::Internal(format!("failed to connect to Redis: {e}")))
}

/// Session store keeping each session as a JSON string under
/// `{prefix}{session id}` (default prefix `ultimo:session:`), expiring with the
/// session's TTL.
///
/// Redis errors are logged with `tracing` and treated as "no session" on load
/// (the user is logged out rather than the request failing).
#[cfg(feature = "session")]
#[derive(Clone)]
pub struct RedisSessionStore {
    conn: ConnectionManager,
    prefix: Arc<str>,
}

#[cfg(feature = "session")]
impl RedisSessionStore {
    /// A store using `conn`, with the key prefix `ultimo:session:`.
    pub fn new(conn: ConnectionManager) -> Self {
        Self {
            conn,
            prefix: Arc::from("ultimo:session:"),
        }
    }

    /// Use a custom key prefix, e.g. to share one Redis between apps.
    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Arc::from(prefix.into());
        self
    }

    fn key(&self, id: &str) -> String {
        format!("{}{}", self.prefix, id)
    }
}

#[cfg(feature = "session")]
#[async_trait::async_trait]
impl crate::session::SessionStore for RedisSessionStore {
    async fn load(&self, id: &str) -> Option<crate::session::SessionData> {
        let mut conn = self.conn.clone();
        let raw: Option<String> = match ::redis::cmd("GET")
            .arg(self.key(id))
            .query_async(&mut conn)
            .await
        {
            Ok(raw) => raw,
            Err(e) => {
                tracing::warn!(error = %e, "redis session load failed");
                return None;
            }
        };
        let raw = raw?;
        match serde_json::from_str(&raw) {
            Ok(data) => Some(data),
            Err(e) => {
                tracing::warn!(error = %e, "redis session data is not valid JSON");
                None
            }
        }
    }

    async fn store(&self, id: &str, data: &crate::session::SessionData, ttl: Duration) {
        let raw = match serde_json::to_string(data) {
            Ok(raw) => raw,
            Err(e) => {
                tracing::warn!(error = %e, "failed to serialize session");
                return;
            }
        };
        let mut conn = self.conn.clone();
        let millis = u64::try_from(ttl.as_millis()).unwrap_or(u64::MAX).max(1);
        let result: ::redis::RedisResult<()> = ::redis::cmd("SET")
            .arg(self.key(id))
            .arg(raw)
            .arg("PX")
            .arg(millis)
            .query_async(&mut conn)
            .await;
        if let Err(e) = result {
            tracing::warn!(error = %e, "redis session store failed");
        }
    }

    async fn destroy(&self, id: &str) {
        let mut conn = self.conn.clone();
        let result: ::redis::RedisResult<()> = ::redis::cmd("DEL")
            .arg(self.key(id))
            .query_async(&mut conn)
            .await;
        if let Err(e) = result {
            tracing::warn!(error = %e, "redis session destroy failed");
        }
    }
}

/// Token bucket in one atomic script, timed by the Redis server's clock so app
/// instances with skewed clocks still agree. Needs Redis 5+ (scripts that read
/// `TIME` and then write).
const TOKEN_BUCKET: &str = r#"
local max = tonumber(ARGV[1])
local window_ms = tonumber(ARGV[2])
local t = redis.call('TIME')
local now = tonumber(t[1]) * 1000 + math.floor(tonumber(t[2]) / 1000)
local b = redis.call('HMGET', KEYS[1], 'tokens', 'ts')
local tokens = tonumber(b[1])
local ts = tonumber(b[2])
if tokens == nil or ts == nil then
  tokens = max
  ts = now
end
tokens = math.min(max, tokens + math.max(0, now - ts) * max / window_ms)
local allowed = 0
if tokens >= 1 then
  tokens = tokens - 1
  allowed = 1
end
redis.call('HSET', KEYS[1], 'tokens', tostring(tokens), 'ts', now)
redis.call('PEXPIRE', KEYS[1], window_ms)
return allowed
"#;

fn token_bucket() -> &'static Script {
    static SCRIPT: OnceLock<Script> = OnceLock::new();
    SCRIPT.get_or_init(|| Script::new(TOKEN_BUCKET))
}

/// Rate-limit store keeping each token bucket as a Redis hash under
/// `{prefix}{key}` (default prefix `ultimo:ratelimit:`), so every instance of
/// the app draws from the same bucket. Same algorithm as the in-memory store.
///
/// If Redis errors (unreachable, timeout, …) the request is **allowed** and the
/// error logged: a Redis outage degrades to "no rate limiting" rather than "no
/// service". Call [`fail_closed`](Self::fail_closed) to reject instead.
#[derive(Clone)]
pub struct RedisRateLimitStore {
    conn: ConnectionManager,
    prefix: Arc<str>,
    fail_closed: bool,
}

impl RedisRateLimitStore {
    /// A store using `conn`, with the key prefix `ultimo:ratelimit:`.
    pub fn new(conn: ConnectionManager) -> Self {
        Self {
            conn,
            prefix: Arc::from("ultimo:ratelimit:"),
            fail_closed: false,
        }
    }

    /// Use a custom key prefix, e.g. to share one Redis between apps.
    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Arc::from(prefix.into());
        self
    }

    /// Reject requests (429) when Redis can't be asked, instead of allowing them.
    pub fn fail_closed(mut self) -> Self {
        self.fail_closed = true;
        self
    }
}

#[async_trait::async_trait]
impl RateLimitStore for RedisRateLimitStore {
    async fn try_acquire(&self, key: &str, max_requests: u64, window: Duration) -> bool {
        let mut conn = self.conn.clone();
        let window_ms = u64::try_from(window.as_millis()).unwrap_or(u64::MAX).max(1);
        let result: ::redis::RedisResult<i64> = token_bucket()
            .key(format!("{}{}", self.prefix, key))
            .arg(max_requests)
            .arg(window_ms)
            .invoke_async(&mut conn)
            .await;
        match result {
            Ok(allowed) => allowed == 1,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    fail_closed = self.fail_closed,
                    "redis rate limit check failed"
                );
                !self.fail_closed
            }
        }
    }
}
