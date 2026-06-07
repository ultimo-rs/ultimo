//! JWT auth middleware (HS256) — verifies signed bearer/cookie tokens, attaches
//! validated claims to the request `Context`, and can issue tokens via `sign`.
//!
//! Verification is delegated to the audited `jsonwebtoken` crate, which pins the
//! expected algorithm and rejects `alg: none` and HS/RS confusion attacks.
//!
//! ```no_run
//! # use ultimo::Ultimo;
//! # use ultimo::auth::jwt::Jwt;
//! let mut app = Ultimo::new_without_defaults();
//! let jwt = Jwt::hs256(b"super-secret-key".to_vec());
//! app.use_middleware(jwt.clone().build());
//! ```
