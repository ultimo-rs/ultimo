//! Session configuration.

use crate::cookie::SameSite;
use std::time::Duration;

/// Session middleware configuration. Secure-by-default.
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Name of the session-id cookie.
    pub cookie_name: String,
    /// Server-side + cookie lifetime.
    pub ttl: Duration,
    /// `HttpOnly` cookie attribute (default true).
    pub http_only: bool,
    /// `Secure` cookie attribute (default true).
    pub secure: bool,
    /// `SameSite` cookie attribute (default Lax).
    pub same_site: SameSite,
    /// Cookie `Path` (default "/").
    pub path: String,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            cookie_name: "ultimo_sid".to_string(),
            ttl: Duration::from_secs(60 * 60 * 24),
            http_only: true,
            secure: true,
            same_site: SameSite::Lax,
            path: "/".to_string(),
        }
    }
}

impl SessionConfig {
    /// Set the session cookie name.
    pub fn cookie_name(mut self, n: impl Into<String>) -> Self {
        self.cookie_name = n.into();
        self
    }
    /// Set the session lifetime.
    pub fn ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }
    /// Set the `Secure` attribute (disable only for local HTTP dev).
    pub fn secure(mut self, v: bool) -> Self {
        self.secure = v;
        self
    }
    /// Set the `SameSite` attribute.
    pub fn same_site(mut self, v: SameSite) -> Self {
        self.same_site = v;
        self
    }

    /// Validate invariants. `SameSite=None` requires `Secure` (browser rule).
    /// Panics on violation — a misconfigured session is a programming error.
    pub fn validated(self) -> Self {
        if self.same_site == SameSite::None && !self.secure {
            panic!("SessionConfig: SameSite=None requires secure=true");
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secure_defaults() {
        let c = SessionConfig::default();
        assert!(c.http_only && c.secure);
        assert_eq!(c.same_site, SameSite::Lax);
    }

    #[test]
    #[should_panic(expected = "SameSite=None requires secure")]
    fn samesite_none_requires_secure() {
        SessionConfig::default()
            .same_site(SameSite::None)
            .secure(false)
            .validated();
    }
}
