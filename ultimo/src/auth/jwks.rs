//! Remote JWKS client: fetches a provider's JSON Web Key Set, caches it with a
//! TTL, and refetches once on a cache-miss (key rotation). The fetch step is a
//! closure so the caching logic is testable without network.

use crate::error::{Result, UltimoError};
use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::DecodingKey;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

type FetchFut = Pin<Box<dyn Future<Output = Result<(JwkSet, Duration)>> + Send>>;
type FetchFn = Arc<dyn Fn() -> FetchFut + Send + Sync>;

struct Cached {
    set: JwkSet,
    expires_at: Instant,
}

/// Resolves JWK decoding keys for a provider, with a TTL cache and
/// rotation-aware refetch.
#[derive(Clone)]
pub struct JwksClient {
    fetch: FetchFn,
    cache: Arc<RwLock<Option<Cached>>>,
}

/// Parse `max-age` (seconds) from a `Cache-Control` header value.
fn max_age(cache_control: Option<&str>) -> Option<Duration> {
    let cc = cache_control?;
    cc.split(',')
        .filter_map(|d| d.trim().strip_prefix("max-age="))
        .find_map(|v| v.parse::<u64>().ok())
        .map(Duration::from_secs)
}

impl JwksClient {
    /// Construct from a raw fetch closure returning `(JwkSet, ttl)`.
    pub fn from_fetch<F>(f: F) -> Self
    where
        F: Fn() -> FetchFut + Send + Sync + 'static,
    {
        Self {
            fetch: Arc::new(f),
            cache: Arc::new(RwLock::new(None)),
        }
    }

    /// Verify against a fixed in-memory key set (offline / air-gapped / tests).
    pub fn from_set(set: JwkSet) -> Self {
        Self::from_fetch(move || {
            let set = set.clone();
            Box::pin(async move { Ok((set, Duration::from_secs(31_536_000))) }) as FetchFut
        })
    }

    /// Fetch from a known JWKS URL.
    #[cfg(feature = "oidc")]
    pub fn from_url(url: String) -> Self {
        let http = reqwest::Client::new();
        Self::from_fetch(move || {
            let (http, url) = (http.clone(), url.clone());
            Box::pin(async move { fetch_jwks(&http, &url).await }) as FetchFut
        })
    }

    /// Discover the JWKS URL from an OIDC issuer, then fetch it.
    #[cfg(feature = "oidc")]
    pub fn from_issuer(issuer: String) -> Self {
        let http = reqwest::Client::new();
        Self::from_fetch(move || {
            let (http, issuer) = (http.clone(), issuer.clone());
            Box::pin(async move {
                let disco = format!(
                    "{}/.well-known/openid-configuration",
                    issuer.trim_end_matches('/')
                );
                #[derive(serde::Deserialize)]
                struct Disco {
                    jwks_uri: String,
                }
                let cfg: Disco = http
                    .get(&disco)
                    .send()
                    .await
                    .and_then(|r| r.error_for_status())
                    .map_err(|e| UltimoError::Unauthorized(format!("OIDC discovery failed: {e}")))?
                    .json()
                    .await
                    .map_err(|e| UltimoError::Unauthorized(format!("OIDC discovery parse: {e}")))?;
                fetch_jwks(&http, &cfg.jwks_uri).await
            }) as FetchFut
        })
    }

    /// Resolve a decoding key for `kid`, fetching/caching as needed. On a
    /// cache-miss for `kid`, refetch exactly once (handles key rotation).
    pub async fn decoding_key(&self, kid: &str) -> Result<DecodingKey> {
        if let Some(key) = self.lookup_fresh(kid).await {
            return Ok(key);
        }
        self.refetch().await?;
        self.lookup_any(kid)
            .await
            .ok_or_else(|| UltimoError::Unauthorized(format!("no JWKS key matches kid '{kid}'")))
    }

    /// Cache hit only if fresh (not expired) and it contains `kid`.
    async fn lookup_fresh(&self, kid: &str) -> Option<DecodingKey> {
        let guard = self.cache.read().await;
        match guard.as_ref() {
            Some(c) if c.expires_at > Instant::now() => c
                .set
                .find(kid)
                .and_then(|jwk| DecodingKey::from_jwk(jwk).ok()),
            _ => None,
        }
    }

    /// Look up `kid` in whatever is currently cached (post-refetch), regardless
    /// of freshness.
    async fn lookup_any(&self, kid: &str) -> Option<DecodingKey> {
        let guard = self.cache.read().await;
        guard
            .as_ref()
            .and_then(|c| c.set.find(kid))
            .and_then(|jwk| DecodingKey::from_jwk(jwk).ok())
    }

    async fn refetch(&self) -> Result<()> {
        let (set, ttl) = (self.fetch)().await?;
        *self.cache.write().await = Some(Cached {
            set,
            expires_at: Instant::now() + ttl,
        });
        Ok(())
    }
}

#[cfg(feature = "oidc")]
async fn fetch_jwks(http: &reqwest::Client, url: &str) -> Result<(JwkSet, Duration)> {
    let resp = http
        .get(url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| UltimoError::Unauthorized(format!("JWKS fetch failed: {e}")))?;
    let ttl = max_age(
        resp.headers()
            .get(reqwest::header::CACHE_CONTROL)
            .and_then(|v| v.to_str().ok()),
    )
    .unwrap_or(Duration::from_secs(3600));
    let set: JwkSet = resp
        .json()
        .await
        .map_err(|e| UltimoError::Unauthorized(format!("JWKS parse failed: {e}")))?;
    Ok((set, ttl))
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    use rsa::pkcs1::EncodeRsaPrivateKey;
    use rsa::traits::PublicKeyParts;
    use rsa::RsaPrivateKey;

    // Build a (private PEM, JwkSet) pair for a given kid — offline test keys.
    fn keypair(kid: &str) -> (String, JwkSet) {
        let mut rng = rand::thread_rng();
        let sk = RsaPrivateKey::new(&mut rng, 2048).unwrap();
        let pem = sk
            .to_pkcs1_pem(rsa::pkcs1::LineEnding::LF)
            .unwrap()
            .to_string();
        let n = URL_SAFE_NO_PAD.encode(sk.n().to_bytes_be());
        let e = URL_SAFE_NO_PAD.encode(sk.e().to_bytes_be());
        let set = serde_json::from_value(serde_json::json!({
            "keys": [{ "kty": "RSA", "use": "sig", "alg": "RS256", "kid": kid, "n": n, "e": e }]
        }))
        .unwrap();
        (pem, set)
    }

    #[tokio::test]
    async fn from_set_serves_key_without_fetching() {
        let (_pem, set) = keypair("k1");
        let client = JwksClient::from_set(set);
        assert!(client.decoding_key("k1").await.is_ok());
    }

    #[tokio::test]
    async fn unknown_kid_triggers_one_refetch_then_fails() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let (_pem, set) = keypair("k1");
        let calls = Arc::new(AtomicUsize::new(0));
        let calls2 = calls.clone();
        let client = JwksClient::from_fetch(move || {
            let set = set.clone();
            calls2.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move { Ok((set, Duration::from_secs(3600))) }) as FetchFut
        });
        assert!(client.decoding_key("k1").await.is_ok());
        assert!(client.decoding_key("missing").await.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn max_age_parsing() {
        assert_eq!(
            max_age(Some("public, max-age=600")),
            Some(Duration::from_secs(600))
        );
        assert_eq!(max_age(Some("no-store")), None);
        assert_eq!(max_age(None), None);
    }
}
