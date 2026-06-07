//! JWT auth middleware (HS256) — verifies signed bearer/cookie tokens, attaches
//! validated claims to the request `Context`, and can issue tokens via `sign`.
//!
//! Verification is delegated to the audited `jsonwebtoken` crate, which pins the
//! expected algorithm and rejects `alg: none` and HS/RS confusion attacks.
//!
//! ```
//! use ultimo::auth::jwt::Jwt;
//!
//! let jwt = Jwt::hs256(b"super-secret-key");
//! // Issue a token (claims must include `exp`).
//! let token = jwt
//!     .sign(&serde_json::json!({ "sub": "ada", "exp": 4_102_444_800u64 }))
//!     .unwrap();
//! // Verify it and read the claims back.
//! let claims: serde_json::Value = jwt.decode(&token).unwrap();
//! assert_eq!(claims["sub"], "ada");
//! ```

use crate::error::{Result, UltimoError};
use jsonwebtoken::{
    decode as jwt_decode, encode as jwt_encode, Algorithm, DecodingKey, EncodingKey, Header,
    Validation,
};
use serde::{de::DeserializeOwned, Serialize};

/// Where the middleware looks for the token on an incoming request.
#[derive(Debug, Clone)]
enum TokenSource {
    /// `Authorization: Bearer <token>` (default).
    Bearer,
    /// A named cookie carrying the raw token.
    Cookie(String),
}

/// JWT auth configuration. Verifies (`build`) and issues (`sign`) tokens using a
/// shared HS256 secret. Secure-by-default: `exp` is validated, the algorithm is
/// pinned to HS256, and `alg: none` / algorithm-confusion tokens are rejected.
#[derive(Clone)]
pub struct Jwt {
    encoding: EncodingKey,
    decoding: DecodingKey,
    validation: Validation,
    source: TokenSource,
    /// When false (default), a missing/invalid token yields 401. When true, the
    /// request passes through unauthenticated (no claims attached).
    optional: bool,
}

impl Jwt {
    /// Configure HS256 with a symmetric secret. The same secret signs and verifies.
    pub fn hs256(secret: impl AsRef<[u8]>) -> Self {
        let secret = secret.as_ref();
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
            validation: Validation::new(Algorithm::HS256),
            source: TokenSource::Bearer,
            optional: false,
        }
    }

    /// Require the `iss` claim to equal `issuer`.
    pub fn issuer(mut self, issuer: impl Into<String>) -> Self {
        self.validation.set_issuer(&[issuer.into()]);
        self
    }

    /// Require the `aud` claim to equal `audience`.
    pub fn audience(mut self, audience: impl Into<String>) -> Self {
        self.validation.set_audience(&[audience.into()]);
        self
    }

    /// Clock-skew tolerance (seconds) applied to `exp`/`nbf` checks.
    pub fn leeway(mut self, seconds: u64) -> Self {
        self.validation.leeway = seconds;
        self
    }

    /// Read the token from `Authorization: Bearer <token>` (the default).
    pub fn from_bearer(mut self) -> Self {
        self.source = TokenSource::Bearer;
        self
    }

    /// Read the token from a named cookie instead of the Authorization header.
    pub fn from_cookie(mut self, name: impl Into<String>) -> Self {
        self.source = TokenSource::Cookie(name.into());
        self
    }

    /// Make authentication optional: unauthenticated requests pass through with
    /// no claims attached, instead of receiving a 401. Handlers decide what to do
    /// when `ctx.jwt_claims()` is `None`.
    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }

    /// Issue a signed HS256 token for the given claims (which must include `exp`).
    pub fn sign<T: Serialize>(&self, claims: &T) -> Result<String> {
        jwt_encode(&Header::new(Algorithm::HS256), claims, &self.encoding)
            .map_err(|e| UltimoError::Internal(format!("JWT signing failed: {e}")))
    }

    /// Verify a token and deserialize its claims. Errors on bad signature,
    /// expired/`nbf` violations, wrong `iss`/`aud`, or `alg: none`.
    pub fn decode<T: DeserializeOwned>(&self, token: &str) -> Result<T> {
        jwt_decode::<T>(token, &self.decoding, &self.validation)
            .map(|data| data.claims)
            .map_err(|e| UltimoError::Unauthorized(format!("invalid JWT: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Claims {
        sub: String,
        exp: usize,
    }

    fn far_future() -> usize {
        // Fixed timestamp well beyond any reasonable test clock (year 2100).
        4_102_444_800
    }

    #[test]
    fn sign_then_decode_roundtrip() {
        let jwt = Jwt::hs256(b"test-secret".to_vec());
        let token = jwt
            .sign(&Claims {
                sub: "ada".into(),
                exp: far_future(),
            })
            .unwrap();
        // The signed token has three dot-separated segments.
        assert_eq!(token.split('.').count(), 3);

        let claims: Claims = jwt.decode(&token).unwrap();
        assert_eq!(
            claims,
            Claims {
                sub: "ada".into(),
                exp: far_future()
            }
        );
    }

    #[test]
    fn decode_rejects_bad_signature() {
        let signer = Jwt::hs256(b"secret-a".to_vec());
        let verifier = Jwt::hs256(b"secret-b".to_vec());
        let token = signer
            .sign(&Claims {
                sub: "ada".into(),
                exp: far_future(),
            })
            .unwrap();
        assert!(verifier.decode::<Claims>(&token).is_err());
    }

    #[test]
    fn decode_rejects_expired() {
        let jwt = Jwt::hs256(b"secret".to_vec());
        // exp in the past (epoch second 1) with zero leeway → expired.
        let token = jwt
            .sign(&Claims {
                sub: "ada".into(),
                exp: 1,
            })
            .unwrap();
        assert!(jwt.decode::<Claims>(&token).is_err());
    }
}
