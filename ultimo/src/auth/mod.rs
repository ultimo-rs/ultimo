//! Authentication middleware.
//!
//! Currently provides JWT (JSON Web Token) verification + signing behind the
//! `jwt` feature. API-key auth and authorization guards are planned follow-ups.

pub mod jwt;
