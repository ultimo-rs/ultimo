//! # Ultimo - Modern Rust Web Framework
//!
//! Lightweight, type-safe Rust web framework inspired by Hono.js with automatic TypeScript type generation.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use ultimo::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> ultimo::Result<()> {
//!     let mut app = Ultimo::new();
//!
//!     app.use_middleware(ultimo::middleware::builtin::logger());
//!
//!     app.get("/", |ctx: Context| async move {
//!         ctx.json(serde_json::json!({"message": "Hello Ultimo!"})).await
//!     });
//!
//!     app.get("/users/:id", |ctx: Context| async move {
//!         let id = ctx.req.param("id")?;
//!         ctx.json(serde_json::json!({"id": id})).await
//!     });
//!
//!     app.listen("127.0.0.1:3000").await
//! }
//! ```

pub mod app;
pub mod context;
pub mod error;
pub mod handler;
pub mod middleware;
pub mod openapi;
pub mod response;
pub mod router;
pub mod rpc;
pub mod validation;

#[cfg(feature = "database")]
pub mod database;

// Re-exports for convenience
pub use app::Ultimo;
pub use context::Context;
pub use error::{Result, UltimoError};
pub use rpc::{RpcRegistry, RpcRequest, RpcResponse};
pub use validation::validate;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::app::Ultimo;
    pub use crate::context::Context;
    pub use crate::error::{Result, UltimoError};
    pub use crate::middleware;
    pub use crate::rpc::{RpcRegistry, RpcRequest, RpcResponse};
    pub use crate::validation::validate;
    pub use serde::{Deserialize, Serialize};
    pub use serde_json::json;
    pub use validator::Validate;

    #[cfg(feature = "database")]
    pub use crate::database;
}
