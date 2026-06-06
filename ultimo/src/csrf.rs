//! CSRF protection via the **double-submit cookie** pattern.
//!
//! The middleware issues a random token in a (non-HttpOnly) cookie so the
//! frontend can read it and echo it back in a request header on unsafe methods
//! (POST/PUT/PATCH/DELETE). The token in the cookie and the header must match
//! (compared in constant time) or the request is rejected with **403**. Safe
//! methods (GET/HEAD/OPTIONS) are exempt and just (re)issue the token.
//!
//! Security rests on the same-origin policy: a cross-site attacker can neither
//! read the victim's cookie nor set the matching header.
//!
//! ```
//! # use ultimo::Ultimo;
//! let mut app = Ultimo::new_without_defaults();
//! app.use_middleware(ultimo::csrf::csrf());
//! ```

use crate::cookie::{Cookie, SameSite};
use crate::error::Result;
use crate::middleware::{BoxedMiddleware, Next};
use crate::response::{Response, ResponseBuilder};
use crate::Context;
use base64::Engine;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// CSRF middleware configuration. Secure-by-default.
#[derive(Debug, Clone)]
pub struct Csrf {
    cookie_name: String,
    header_name: String,
    secure: bool,
    same_site: SameSite,
}

impl Default for Csrf {
    fn default() -> Self {
        Self {
            cookie_name: "csrf_token".to_string(),
            header_name: "x-csrf-token".to_string(),
            secure: true,
            same_site: SameSite::Lax,
        }
    }
}

impl Csrf {
    /// Secure defaults.
    pub fn new() -> Self {
        Self::default()
    }
    /// Name of the CSRF token cookie (default `csrf_token`).
    pub fn cookie_name(mut self, name: impl Into<String>) -> Self {
        self.cookie_name = name.into();
        self
    }
    /// Request header carrying the echoed token (default `x-csrf-token`).
    pub fn header_name(mut self, name: impl Into<String>) -> Self {
        self.header_name = name.into();
        self
    }
    /// Set the cookie `Secure` attribute (disable only for local HTTP dev).
    pub fn secure(mut self, v: bool) -> Self {
        self.secure = v;
        self
    }
    /// Set the cookie `SameSite` attribute.
    pub fn same_site(mut self, v: SameSite) -> Self {
        self.same_site = v;
        self
    }

    /// Build the CSRF middleware.
    pub fn build(self) -> BoxedMiddleware {
        let cfg = Arc::new(self);
        Arc::new(move |ctx: Context, next: Next| {
            let cfg = cfg.clone();
            Box::pin(async move {
                let existing = ctx.cookie(&cfg.cookie_name);

                // Validate unsafe methods: cookie token must equal the header.
                if is_unsafe(ctx.req.method()) {
                    let valid = match (&existing, ctx.req.header(&cfg.header_name)) {
                        (Some(c), Some(h)) => ct_eq(c.as_bytes(), h.as_bytes()),
                        _ => false,
                    };
                    if !valid {
                        return Ok(forbidden());
                    }
                }

                // Bootstrap a token cookie if the client doesn't have one yet.
                if existing.is_none() {
                    let token = generate_token();
                    let cookie = Cookie::new(cfg.cookie_name.clone(), token)
                        .secure(cfg.secure)
                        .same_site(cfg.same_site)
                        .path("/");
                    // Note: intentionally NOT HttpOnly — the frontend must read it.
                    let _ = ctx.set_cookie(cookie).await;
                }

                next(ctx).await
            }) as Pin<Box<dyn Future<Output = Result<Response>> + Send>>
        })
    }
}

/// CSRF middleware with secure defaults.
pub fn csrf() -> BoxedMiddleware {
    Csrf::new().build()
}

fn is_unsafe(method: &hyper::Method) -> bool {
    !matches!(
        *method,
        hyper::Method::GET | hyper::Method::HEAD | hyper::Method::OPTIONS | hyper::Method::TRACE
    )
}

/// Constant-time byte comparison (avoids leaking match position via timing).
fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// 256-bit URL-safe token. Panics if the OS RNG fails (never weak fallback).
fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("OS RNG failure generating CSRF token");
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn forbidden() -> Response {
    ResponseBuilder::new()
        .status(403)
        .text("CSRF token missing or invalid")
        .build()
        .unwrap_or_else(|_| crate::response::helpers::text("Forbidden").unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ct_eq_basic() {
        assert!(ct_eq(b"abc", b"abc"));
        assert!(!ct_eq(b"abc", b"abd"));
        assert!(!ct_eq(b"abc", b"abcd"));
    }

    #[test]
    fn unsafe_methods() {
        assert!(is_unsafe(&hyper::Method::POST));
        assert!(is_unsafe(&hyper::Method::DELETE));
        assert!(!is_unsafe(&hyper::Method::GET));
        assert!(!is_unsafe(&hyper::Method::OPTIONS));
    }
}
