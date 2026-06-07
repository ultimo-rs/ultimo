//! Authentication middleware.
//!
//! - `jwt` — stateless JWT (JSON Web Token) verification + signing (feature `jwt`).
//! - `api_key` — API-key validation against a pluggable store (feature `api-key`).
//!
//! Authorization guards that consume these identities are a planned follow-up.

#[cfg(feature = "jwt")]
pub mod jwt;

#[cfg(feature = "api-key")]
pub mod api_key;
