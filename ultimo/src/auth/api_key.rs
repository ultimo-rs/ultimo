//! API-key authentication — validates a presented key against a pluggable store,
//! resolves it to an [`ApiKeyIdentity`], and attaches that identity to the
//! request `Context`. Missing/invalid keys are rejected with **401**.
//!
//! Secure-by-default: the built-in [`StaticKeys`] store keeps only **SHA-256
//! digests** of keys (never the raw secret) and compares in constant time. API
//! keys are high-entropy random secrets — not passwords — so SHA-256 is the
//! correct hash here (bcrypt/argon2 are for low-entropy passwords).
//!
//! For database-backed keys, implement [`ApiKeyStore`] for your backend; look up
//! by the key's hash, not the raw value.
//!
//! ```
//! use ultimo::auth::api_key::{ApiKey, StaticKeys};
//!
//! let store = StaticKeys::new()
//!     .insert("key-abc", "service-a")
//!     .with_scopes("key-def", "service-b", ["read", "write"]);
//! let _mw = ApiKey::new(store).header_name("x-api-key").build();
//! ```

use crate::error::Result;
use crate::middleware::{BoxedMiddleware, Next};
use crate::response::{Response, ResponseBuilder};
use crate::Context;
use async_trait::async_trait;
use sha2::{Digest, Sha256};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// The identity a valid API key resolves to. Never contains the raw secret.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApiKeyIdentity {
    /// A stable label / key id for the caller (e.g. a service or tenant name).
    pub id: String,
    /// Optional scopes for authorization decisions.
    pub scopes: Vec<String>,
}

/// Validates presented API keys. Implement this for a database/Redis-backed key
/// store; the built-in [`StaticKeys`] covers in-memory configuration.
#[async_trait]
pub trait ApiKeyStore: Send + Sync {
    /// Resolve a presented key to an identity, or `None` to reject it.
    ///
    /// Custom implementations should look the key up by its **hash** (not the
    /// raw value) and compare in constant time, mirroring [`StaticKeys`].
    async fn validate(&self, presented_key: &str) -> Option<ApiKeyIdentity>;
}

/// In-memory key store. Keys are SHA-256 hashed at construction — only digests
/// are retained — and compared in constant time on each request.
#[derive(Default)]
pub struct StaticKeys {
    entries: Vec<([u8; 32], ApiKeyIdentity)>,
}

impl StaticKeys {
    /// An empty store. Add keys with [`insert`](Self::insert) /
    /// [`with_scopes`](Self::with_scopes).
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a key mapped to an identity `id` with no scopes.
    pub fn insert(mut self, key: impl AsRef<[u8]>, id: impl Into<String>) -> Self {
        self.entries.push((
            hash_key(key.as_ref()),
            ApiKeyIdentity {
                id: id.into(),
                scopes: Vec::new(),
            },
        ));
        self
    }

    /// Add a key mapped to an identity `id` with the given scopes.
    pub fn with_scopes(
        mut self,
        key: impl AsRef<[u8]>,
        id: impl Into<String>,
        scopes: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.entries.push((
            hash_key(key.as_ref()),
            ApiKeyIdentity {
                id: id.into(),
                scopes: scopes.into_iter().map(Into::into).collect(),
            },
        ));
        self
    }
}

#[async_trait]
impl ApiKeyStore for StaticKeys {
    async fn validate(&self, presented_key: &str) -> Option<ApiKeyIdentity> {
        let presented = hash_key(presented_key.as_bytes());
        // Compare against every entry without early-return so neither the match
        // position nor whether any key matched leaks via timing.
        let mut found: Option<&ApiKeyIdentity> = None;
        for (digest, identity) in &self.entries {
            if ct_eq(&presented, digest) {
                found = Some(identity);
            }
        }
        found.cloned()
    }
}

/// Where the middleware reads the API key from.
#[derive(Debug, Clone)]
enum KeySource {
    /// A request header (default `x-api-key`).
    Header(String),
    /// A query-string parameter.
    Query(String),
}

/// API-key auth middleware, generic over the [`ApiKeyStore`]. Secure-by-default.
pub struct ApiKey<S: ApiKeyStore> {
    store: Arc<S>,
    source: KeySource,
    /// When false (default), a missing/invalid key yields 401. When true, the
    /// request passes through unauthenticated (no identity attached).
    optional: bool,
}

impl<S: ApiKeyStore + 'static> ApiKey<S> {
    /// Validate keys against `store`, read from the `x-api-key` header.
    pub fn new(store: S) -> Self {
        Self {
            store: Arc::new(store),
            source: KeySource::Header("x-api-key".to_string()),
            optional: false,
        }
    }

    /// Read the key from a different request header.
    pub fn header_name(mut self, name: impl Into<String>) -> Self {
        self.source = KeySource::Header(name.into());
        self
    }

    /// Read the key from a query-string parameter instead of a header.
    pub fn from_query(mut self, name: impl Into<String>) -> Self {
        self.source = KeySource::Query(name.into());
        self
    }

    /// Make authentication optional: unauthenticated requests pass through with
    /// no identity attached, instead of receiving a 401.
    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }

    /// Build the verification middleware.
    pub fn build(self) -> BoxedMiddleware {
        let cfg = Arc::new(self);
        Arc::new(move |ctx: Context, next: Next| {
            let cfg = cfg.clone();
            Box::pin(async move {
                let presented = match &cfg.source {
                    KeySource::Header(name) => ctx.req.header(name),
                    KeySource::Query(name) => ctx.req.query(name),
                };
                match presented {
                    Some(key) => match cfg.store.validate(&key).await {
                        Some(identity) => {
                            let principal = crate::auth::Principal {
                                id: Some(identity.id.clone()),
                                scopes: identity.scopes.clone(),
                            };
                            ctx.set_api_key(identity).await;
                            ctx.set_principal(principal).await;
                            next(ctx).await
                        }
                        None if cfg.optional => next(ctx).await,
                        None => Ok(unauthorized()),
                    },
                    None if cfg.optional => next(ctx).await,
                    None => Ok(unauthorized()),
                }
            }) as Pin<Box<dyn Future<Output = Result<Response>> + Send>>
        })
    }
}

/// SHA-256 digest of a key. API keys are high-entropy secrets, so a fast hash is
/// the correct choice (not a password KDF).
fn hash_key(key: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(key);
    hasher.finalize().into()
}

/// Constant-time equality over equal-length digests.
fn ct_eq(a: &[u8; 32], b: &[u8; 32]) -> bool {
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn unauthorized() -> Response {
    ResponseBuilder::new()
        .status(401)
        .text("Unauthorized")
        .build()
        .unwrap_or_else(|_| crate::response::helpers::text("Unauthorized").unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn valid_key_resolves_to_identity_with_scopes() {
        let store = StaticKeys::new()
            .insert("key-abc", "service-a")
            .with_scopes("key-def", "service-b", ["read", "write"]);

        let a = store.validate("key-abc").await.unwrap();
        assert_eq!(a.id, "service-a");
        assert!(a.scopes.is_empty());

        let b = store.validate("key-def").await.unwrap();
        assert_eq!(b.id, "service-b");
        assert_eq!(b.scopes, vec!["read".to_string(), "write".to_string()]);
    }

    #[tokio::test]
    async fn unknown_key_is_rejected() {
        let store = StaticKeys::new().insert("key-abc", "service-a");
        assert!(store.validate("nope").await.is_none());
        // Empty store rejects everything.
        assert!(StaticKeys::new().validate("anything").await.is_none());
    }

    #[test]
    fn ct_eq_matches_only_identical_digests() {
        let a = hash_key(b"key-abc");
        let same = hash_key(b"key-abc");
        let other = hash_key(b"key-xyz");
        assert!(ct_eq(&a, &same));
        assert!(!ct_eq(&a, &other));
    }
}
