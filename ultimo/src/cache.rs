//! Server-side response caching.
//!
//! [`Cache`] is a middleware that stores successful `GET` responses and serves
//! repeat requests from the store without running the handler. Entries live
//! in process memory by default ([`MemoryCacheStore`]); pass a shared store
//! such as `ultimo::redis::RedisCacheStore` (`redis` feature) so every
//! instance of the app serves from, and invalidates, one cache.
//!
//! ```rust
//! use std::time::Duration;
//! use ultimo::cache::Cache;
//! use ultimo::prelude::*;
//!
//! let cache = Cache::new(Duration::from_secs(60));
//!
//! let mut app = Ultimo::new();
//! app.use_middleware(cache.middleware());
//!
//! app.get("/products", |ctx: Context| async move {
//!     ctx.json(json!([{ "id": 1, "name": "Widget" }])).await // runs once a minute at most
//! });
//!
//! let invalidate = cache.clone();
//! app.post("/products", move |ctx: Context| {
//!     let cache = invalidate.clone();
//!     async move {
//!         // ... create the product ...
//!         cache.invalidate("/products").await?;
//!         ctx.json(json!({ "ok": true })).await
//!     }
//! });
//! ```
//!
//! # What is cached
//!
//! A response is stored only when **all** of these hold:
//!
//! - the request is a `GET`;
//! - the request has no `Cookie` or `Authorization` header — unless that
//!   header is listed in [`vary`](Cache::vary) (one entry per value) or the
//!   cache is [`public`](Cache::public);
//! - the response is `200 OK` with a buffered body of at most
//!   [`max_body_size`](Cache::max_body_size) bytes (streams and SSE pass
//!   through);
//! - the response sets no cookie, has no `Cache-Control: no-store`, `private`
//!   or `no-cache`, and doesn't `Vary` on a header the cache doesn't key on.
//!
//! The key is the path, the raw query string and the values of the
//! [`vary`](Cache::vary) headers (add `host` if one instance serves several
//! domains). Served responses carry `X-Cache: HIT` or
//! `MISS` and, on a hit, an `Age` header.
//!
//! # Middleware order
//!
//! Register the cache **after** (inside) `compression`, `etag`, `request_id`
//! and `session`: it then stores the handler's plain output, while those
//! still run on every response, cached or not (so request ids aren't replayed
//! and compression follows each client's `Accept-Encoding`).

use crate::error::Result;
use crate::middleware::BoxedMiddleware;
use crate::response::UltimoBody;
use async_trait::async_trait;
use bytes::Bytes;
use hyper::header::{HeaderName, HeaderValue};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// A stored response: status, headers and body, plus when it was stored.
///
/// Stores keep it opaque: [`encode`](Self::encode) it to bytes and
/// [`decode`](Self::decode) it back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedResponse {
    status: u16,
    headers: Vec<(String, Vec<u8>)>,
    body: Bytes,
    stored_at: u64,
}

const MAGIC: &[u8; 4] = b"UCR1";

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl CachedResponse {
    /// A response stored now.
    pub fn new(status: u16, headers: Vec<(String, Vec<u8>)>, body: Bytes) -> Self {
        Self {
            status,
            headers,
            body,
            stored_at: unix_now(),
        }
    }

    /// HTTP status code.
    pub fn status(&self) -> u16 {
        self.status
    }

    /// Response headers as `(lowercase name, raw value)` pairs.
    pub fn headers(&self) -> &[(String, Vec<u8>)] {
        &self.headers
    }

    /// Response body.
    pub fn body(&self) -> &Bytes {
        &self.body
    }

    /// Seconds since the response was stored (the `Age` header).
    pub fn age(&self) -> u64 {
        unix_now().saturating_sub(self.stored_at)
    }

    /// Serialize to a compact binary form, for stores that keep bytes.
    pub fn encode(&self) -> Vec<u8> {
        let header_len: usize = self
            .headers
            .iter()
            .map(|(n, v)| 8 + n.len() + v.len())
            .sum();
        let mut out = Vec::with_capacity(4 + 2 + 8 + 4 + header_len + 4 + self.body.len());
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&self.status.to_be_bytes());
        out.extend_from_slice(&self.stored_at.to_be_bytes());
        put_len(&mut out, self.headers.len());
        for (name, value) in &self.headers {
            put_len(&mut out, name.len());
            out.extend_from_slice(name.as_bytes());
            put_len(&mut out, value.len());
            out.extend_from_slice(value);
        }
        put_len(&mut out, self.body.len());
        out.extend_from_slice(&self.body);
        out
    }

    /// Parse bytes produced by [`encode`](Self::encode). `None` if they're
    /// malformed or truncated.
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let mut r = Reader(bytes);
        if r.take(4)? != MAGIC {
            return None;
        }
        let status = u16::from_be_bytes(r.take(2)?.try_into().ok()?);
        let stored_at = u64::from_be_bytes(r.take(8)?.try_into().ok()?);
        let count = r.len()?;
        let mut headers = Vec::with_capacity(count.min(256));
        for _ in 0..count {
            let n = r.len()?;
            let name = String::from_utf8(r.take(n)?.to_vec()).ok()?;
            let n = r.len()?;
            headers.push((name, r.take(n)?.to_vec()));
        }
        let n = r.len()?;
        let body = Bytes::copy_from_slice(r.take(n)?);
        if !r.0.is_empty() {
            return None;
        }
        Some(Self {
            status,
            headers,
            body,
            stored_at,
        })
    }
}

fn put_len(out: &mut Vec<u8>, len: usize) {
    out.extend_from_slice(&u32::try_from(len).unwrap_or(u32::MAX).to_be_bytes());
}

struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        if self.0.len() < n {
            return None;
        }
        let (head, rest) = self.0.split_at(n);
        self.0 = rest;
        Some(head)
    }

    fn len(&mut self) -> Option<usize> {
        let raw = self.take(4)?;
        Some(u32::from_be_bytes(raw.try_into().ok()?) as usize)
    }
}

/// Backing store for [`Cache`].
///
/// [`MemoryCacheStore`] is the default; `ultimo::redis::RedisCacheStore`
/// (`redis` feature) shares one cache between app instances. Lookups and
/// writes can't fail the request: a store that errors should log and behave
/// as a miss.
// async_trait's boxed futures are already #[must_use]; newer clippy flags the expansion.
#[allow(clippy::double_must_use)]
#[async_trait]
pub trait CacheStore: Send + Sync {
    /// The live entry under `key`, if any.
    async fn get(&self, key: &str) -> Option<CachedResponse>;

    /// Store `value` under `key`, expiring after `ttl`.
    async fn set(&self, key: &str, value: CachedResponse, ttl: Duration);

    /// Delete every entry whose key starts with `prefix`.
    async fn remove_prefix(&self, prefix: &str) -> Result<()>;
}

/// In-process [`CacheStore`]: the default for [`Cache`].
///
/// Holds at most [`max_entries`](Self::max_entries) responses (default
/// 10 000). When full, expired entries are dropped first, then the one
/// closest to expiry. Clones share the same entries.
#[derive(Clone)]
pub struct MemoryCacheStore {
    entries: Arc<Mutex<HashMap<String, (CachedResponse, Instant)>>>,
    max_entries: usize,
}

impl Default for MemoryCacheStore {
    fn default() -> Self {
        Self {
            entries: Arc::default(),
            max_entries: 10_000,
        }
    }
}

impl MemoryCacheStore {
    /// An empty store holding up to 10 000 entries.
    pub fn new() -> Self {
        Self::default()
    }

    /// Cap the number of stored responses (at least 1).
    pub fn max_entries(mut self, max: usize) -> Self {
        self.max_entries = max.max(1);
        self
    }

    /// Number of stored entries (expired ones included until evicted).
    pub fn len(&self) -> usize {
        self.lock().len()
    }

    /// `true` if nothing is stored.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, (CachedResponse, Instant)>> {
        self.entries.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[async_trait]
impl CacheStore for MemoryCacheStore {
    async fn get(&self, key: &str) -> Option<CachedResponse> {
        let mut map = self.lock();
        match map.get(key) {
            Some((value, expires)) if *expires > Instant::now() => Some(value.clone()),
            Some(_) => {
                map.remove(key);
                None
            }
            None => None,
        }
    }

    async fn set(&self, key: &str, value: CachedResponse, ttl: Duration) {
        let now = Instant::now();
        let mut map = self.lock();
        if !map.contains_key(key) && map.len() >= self.max_entries {
            map.retain(|_, (_, expires)| *expires > now);
            if map.len() >= self.max_entries {
                let soonest = map
                    .iter()
                    .min_by_key(|(_, (_, expires))| *expires)
                    .map(|(k, _)| k.clone());
                if let Some(k) = soonest {
                    map.remove(&k);
                }
            }
        }
        map.insert(key.to_string(), (value, now + ttl));
    }

    async fn remove_prefix(&self, prefix: &str) -> Result<()> {
        self.lock().retain(|k, _| !k.starts_with(prefix));
        Ok(())
    }
}

/// Response-caching middleware. See the [module docs](self) for what gets
/// cached and where to register it.
///
/// Clones share the same store, so keep one around to
/// [`invalidate`](Self::invalidate) entries when data changes.
#[derive(Clone)]
pub struct Cache {
    ttl: Duration,
    store: Arc<dyn CacheStore>,
    scope: Arc<str>,
    vary: Arc<[String]>,
    public: bool,
    max_body_size: usize,
}

impl Cache {
    /// A cache keeping each response for `ttl`, in a new [`MemoryCacheStore`].
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            store: Arc::new(MemoryCacheStore::new()),
            scope: Arc::from("default"),
            vary: Arc::from(Vec::new()),
            public: false,
            max_body_size: 1024 * 1024,
        }
    }

    /// Keep entries in `store` instead of a private in-memory map.
    pub fn store(mut self, store: impl CacheStore + 'static) -> Self {
        self.store = Arc::new(store);
        self
    }

    /// Namespace this cache's keys within a shared [`store`](Self::store)
    /// (default `default`). [`clear`](Self::clear) only touches its own scope.
    pub fn scope(mut self, scope: impl Into<String>) -> Self {
        self.scope = Arc::from(scope.into());
        self
    }

    /// Request headers whose values are part of the cache key, e.g.
    /// `["accept-language"]` to keep one entry per language. Listing `cookie`
    /// or `authorization` caches per user instead of bypassing those requests.
    pub fn vary<I, S>(mut self, headers: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.vary = headers
            .into_iter()
            .map(|h| h.into().to_ascii_lowercase())
            .collect();
        self
    }

    /// Cache requests that carry `Cookie` or `Authorization` too, sharing one
    /// entry between all users. Only use this when responses never depend on
    /// who is asking.
    pub fn public(mut self) -> Self {
        self.public = true;
        self
    }

    /// Largest body to store, in bytes (default 1 MiB). Larger responses are
    /// served normally but not cached.
    pub fn max_body_size(mut self, bytes: usize) -> Self {
        self.max_body_size = bytes;
        self
    }

    /// Drop every cached variant (query strings, `vary` values) of `path`,
    /// e.g. after the data behind it changed.
    pub async fn invalidate(&self, path: &str) -> Result<()> {
        self.store
            .remove_prefix(&format!("{}:{}?", self.scope, path))
            .await
    }

    /// Drop every entry in this cache's [`scope`](Self::scope).
    pub async fn clear(&self) -> Result<()> {
        self.store.remove_prefix(&format!("{}:", self.scope)).await
    }

    /// The middleware. Calling it again returns another handle on the same
    /// cache.
    pub fn middleware(&self) -> BoxedMiddleware {
        let cache = self.clone();
        Arc::new(move |ctx, next| {
            let cache = cache.clone();
            Box::pin(async move {
                if ctx.req.method() != hyper::Method::GET || cache.bypass(&ctx) {
                    return next(ctx).await;
                }

                let key = cache.key(&ctx);
                if let Some(hit) = cache.store.get(&key).await {
                    return Ok(serve(hit));
                }

                let res = next(ctx).await?;
                if !cache.storable(&res) {
                    return Ok(with_x_cache(res, "MISS"));
                }
                let (parts, body) = res.into_parts();
                let UltimoBody::Full(full) = body else {
                    let res = hyper::Response::from_parts(parts, body);
                    return Ok(with_x_cache(res, "MISS"));
                };
                use http_body_util::BodyExt;
                let Ok(collected) = full.collect().await;
                let bytes = collected.to_bytes();
                if bytes.len() <= cache.max_body_size {
                    let headers = parts
                        .headers
                        .iter()
                        .filter(|(name, _)| !SKIP_HEADERS.contains(&name.as_str()))
                        .map(|(name, value)| (name.to_string(), value.as_bytes().to_vec()))
                        .collect();
                    let entry = CachedResponse::new(parts.status.as_u16(), headers, bytes.clone());
                    cache.store.set(&key, entry, cache.ttl).await;
                }
                let res = hyper::Response::from_parts(parts, UltimoBody::full(bytes));
                Ok(with_x_cache(res, "MISS"))
            })
        })
    }

    /// Requests carrying credentials skip the cache unless it's public or
    /// keys on that header.
    fn bypass(&self, ctx: &crate::Context) -> bool {
        !self.public
            && ["cookie", "authorization"]
                .iter()
                .any(|h| ctx.req.header(h).is_some() && !self.vary.iter().any(|v| v == h))
    }

    /// `{scope}:{path}?{query}` then `\n{header}={value}` per vary header.
    fn key(&self, ctx: &crate::Context) -> String {
        let mut key = format!(
            "{}:{}?{}",
            self.scope,
            ctx.req.path(),
            ctx.req.query_string().unwrap_or_default()
        );
        for name in self.vary.iter() {
            key.push('\n');
            key.push_str(name);
            key.push('=');
            key.push_str(&ctx.req.header(name).unwrap_or_default());
        }
        key
    }

    fn storable(&self, res: &crate::response::Response) -> bool {
        use hyper::header::{CACHE_CONTROL, SET_COOKIE, VARY};
        if res.status() != hyper::StatusCode::OK || res.headers().contains_key(SET_COOKIE) {
            return false;
        }
        let no_store = res.headers().get_all(CACHE_CONTROL).iter().any(|v| {
            v.to_str().unwrap_or("").split(',').any(|d| {
                let d = d.trim().to_ascii_lowercase();
                d == "no-store" || d == "no-cache" || d.starts_with("private")
            })
        });
        let unkeyed_vary = res.headers().get_all(VARY).iter().any(|v| {
            v.to_str().unwrap_or("*").split(',').any(|h| {
                let h = h.trim().to_ascii_lowercase();
                !h.is_empty() && !self.vary.contains(&h)
            })
        });
        !no_store && !unkeyed_vary
    }
}

fn serve(hit: CachedResponse) -> crate::response::Response {
    let mut builder = hyper::Response::builder().status(hit.status);
    if let Some(headers) = builder.headers_mut() {
        for (name, value) in &hit.headers {
            if let (Ok(name), Ok(value)) = (
                HeaderName::from_bytes(name.as_bytes()),
                HeaderValue::from_bytes(value),
            ) {
                headers.append(name, value);
            }
        }
        headers.insert(hyper::header::AGE, HeaderValue::from(hit.age()));
        headers.insert("x-cache", HeaderValue::from_static("HIT"));
    }
    builder
        .body(UltimoBody::full(hit.body))
        .unwrap_or_else(|_| hyper::Response::new(UltimoBody::empty()))
}

/// Never stored: recomputed per response.
const SKIP_HEADERS: &[&str] = &["content-length", "age", "x-cache", "set-cookie", "date"];

fn with_x_cache(
    mut res: crate::response::Response,
    value: &'static str,
) -> crate::response::Response {
    res.headers_mut()
        .insert("x-cache", HeaderValue::from_static(value));
    res
}
