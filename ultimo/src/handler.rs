//! Handler traits and types for async request handling

use crate::{context::Context, error::Result, response::Response};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Type alias for boxed async handler functions
pub type BoxedHandler =
    Arc<dyn Fn(Context) -> Pin<Box<dyn Future<Output = Result<Response>> + Send>> + Send + Sync>;

/// Trait for types that can be converted into handlers
pub trait IntoHandler {
    fn into_handler(self) -> BoxedHandler;
}

/// Implement IntoHandler for async functions with Context parameter
impl<F, Fut> IntoHandler for F
where
    F: Fn(Context) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<Response>> + Send + 'static,
{
    fn into_handler(self) -> BoxedHandler {
        Arc::new(move |ctx| Box::pin(self(ctx)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::response;

    #[tokio::test]
    async fn test_handler_trait() {
        let _handler =
            (|_ctx: Context| async move { response::helpers::text("Hello") }).into_handler();

        // Handler can be called (we'd need a real context to fully test)
        // This just verifies the trait implementation compiles
    }
}
