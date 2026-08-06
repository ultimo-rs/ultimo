//! Handler traits and types for async request handling

use crate::{context::Context, error::Result, response::Response};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Type alias for boxed async handler functions
pub type BoxedHandler =
    Arc<dyn Fn(Context) -> Pin<Box<dyn Future<Output = Result<Response>> + Send>> + Send + Sync>;

/// Trait for types that can be converted into handlers.
///
/// `Args` is the tuple of extractor parameter types the handler takes; it lets
/// the compiler distinguish handlers of different arities (a plain
/// `trait IntoHandler` would leave those type parameters unconstrained and the
/// per-arity impls overlapping). It is always inferred at the call site.
pub trait IntoHandler<Args> {
    fn into_handler(self) -> BoxedHandler;
}

use crate::extract::FromRequest;

/// Implement `IntoHandler` for async functions whose parameters are all
/// extractors (`FromRequest`). Generated for arities 0..=8. Each parameter is
/// extracted from the request in order; any extractor error short-circuits into
/// the standard error → response path (e.g. 400 / 422).
///
/// `Context` itself implements `FromRequest`, so the arity-1 impl subsumes the
/// previous `Fn(Context)` handler — existing `|ctx: Context|` handlers keep
/// working. The `Clone` bound lets the handler be moved into the per-request
/// async block; ordinary closures capturing `Arc`/`Clone` state satisfy it.
macro_rules! impl_into_handler {
    ( $( $ty:ident ),* ) => {
        impl<F, Fut, $( $ty ),*> IntoHandler<( $( $ty, )* )> for F
        where
            F: Fn( $( $ty ),* ) -> Fut + Clone + Send + Sync + 'static,
            Fut: Future<Output = Result<Response>> + Send + 'static,
            $( $ty: FromRequest + Send + 'static, )*
        {
            #[allow(non_snake_case, unused_variables, unused_mut)]
            fn into_handler(self) -> BoxedHandler {
                Arc::new(move |ctx: Context| {
                    let handler = self.clone();
                    Box::pin(async move {
                        $( let $ty = <$ty as FromRequest>::from_request(&ctx).await?; )*
                        handler( $( $ty ),* ).await
                    })
                })
            }
        }
    };
}

impl_into_handler!();
impl_into_handler!(T1);
impl_into_handler!(T1, T2);
impl_into_handler!(T1, T2, T3);
impl_into_handler!(T1, T2, T3, T4);
impl_into_handler!(T1, T2, T3, T4, T5);
impl_into_handler!(T1, T2, T3, T4, T5, T6);
impl_into_handler!(T1, T2, T3, T4, T5, T6, T7);
impl_into_handler!(T1, T2, T3, T4, T5, T6, T7, T8);

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
