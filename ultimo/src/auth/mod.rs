//! Authentication & authorization.
//!
//! - `jwt` — stateless JWT (JSON Web Token) verification + signing (feature `jwt`).
//! - `api_key` — API-key validation against a pluggable store (feature `api-key`).
//! - `oauth2` — "Sign in with …" login flow: OAuth2 authorization code + PKCE,
//!   OIDC ID-token verification, session-backed sign-in (feature `oauth2`).
//!
//! Both middlewares normalize their result into a [`Principal`] on the request
//! `Context`, which the authorization guards (`Context::require_scope`, etc.)
//! read — so authorization is decoupled from which method authenticated the call.

#[cfg(feature = "jwt")]
pub mod jwt;

#[cfg(feature = "oidc")]
pub mod jwks;
#[cfg(feature = "oidc")]
pub use jwks::JwksClient;

#[cfg(feature = "api-key")]
pub mod api_key;

#[cfg(feature = "oauth2")]
pub mod oauth2;

/// The normalized authenticated caller, populated by an auth middleware and read
/// by the authorization guards on [`crate::Context`].
///
/// Roles can be modeled as scopes (e.g. `"role:admin"`) — scopes are the single
/// authorization concept.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Principal {
    /// Subject / key id of the caller (`sub` claim for JWT, key id for API keys).
    pub id: Option<String>,
    /// Scopes granted to the caller, used by the guards.
    pub scopes: Vec<String>,
}

impl Principal {
    /// Whether the caller has the given scope.
    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.iter().any(|s| s == scope)
    }
}
