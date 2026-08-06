//! Typed request extractors: implement `FromRequest` to pull typed data from a
//! request. Handlers may take any number of these as parameters, e.g.
//! `|Path(id): Path<u32>, Query(q): Query<Filter>| async move { … }`.

use crate::context::Context;
use crate::error::{Result, UltimoError};
use serde::de::DeserializeOwned;
use validator::Validate;

/// A type that can be extracted from the request context.
///
/// Every extractor borrows `&Context`; the request body is buffered and cached,
/// so multiple body-reading extractors on the same handler are fine.
#[async_trait::async_trait]
pub trait FromRequest: Sized {
    async fn from_request(ctx: &Context) -> Result<Self>;
}

/// Extracts and deserializes the JSON request body.
pub struct Json<T>(pub T);

#[async_trait::async_trait]
impl<T: DeserializeOwned + Send> FromRequest for Json<T> {
    async fn from_request(ctx: &Context) -> Result<Self> {
        ctx.req.json::<T>().await.map(Json)
    }
}

/// Extracts and deserializes typed query-string parameters.
pub struct Query<T>(pub T);

#[async_trait::async_trait]
impl<T: DeserializeOwned + Send> FromRequest for Query<T> {
    async fn from_request(ctx: &Context) -> Result<Self> {
        let qs = ctx.req.query_string().unwrap_or("");
        serde_urlencoded::from_str::<T>(qs)
            .map(Query)
            .map_err(|e| UltimoError::BadRequest(format!("Invalid query parameters: {e}")))
    }
}

/// Extracts the JSON body and runs `validator` validation (422 on failure).
pub struct Valid<T>(pub T);

#[async_trait::async_trait]
impl<T: DeserializeOwned + Validate + Send> FromRequest for Valid<T> {
    async fn from_request(ctx: &Context) -> Result<Self> {
        let value = ctx.req.json::<T>().await?; // 400 on malformed JSON
        crate::validate(&value)?; // UltimoError::Validation -> 422
        Ok(Valid(value))
    }
}

/// The full request context — the identity extractor, so existing
/// `|ctx: Context| async move { … }` handlers keep working unchanged.
#[async_trait::async_trait]
impl FromRequest for Context {
    async fn from_request(ctx: &Context) -> Result<Self> {
        Ok(ctx.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use serde::Deserialize;

    fn ctx_with(query: &str, body: &[u8]) -> Context {
        let parts = hyper::Request::builder()
            .uri(format!("http://x/?{query}"))
            .body(())
            .unwrap()
            .into_parts()
            .0;
        Context::from_parts(parts, Bytes::copy_from_slice(body), Default::default())
    }

    #[derive(Deserialize)]
    struct Filter {
        page: u32,
        q: String,
    }

    #[tokio::test]
    async fn query_parses_typed_fields() {
        let ctx = ctx_with("page=2&q=rust", b"");
        let Query(f) = Query::<Filter>::from_request(&ctx).await.unwrap();
        assert_eq!(f.page, 2);
        assert_eq!(f.q, "rust");
    }

    #[tokio::test]
    async fn query_malformed_is_400() {
        let ctx = ctx_with("page=notanumber&q=x", b"");
        let err = Query::<Filter>::from_request(&ctx).await.err().unwrap();
        assert_eq!(err.status_code(), 400);
    }

    #[derive(Deserialize)]
    struct Body {
        name: String,
    }

    #[tokio::test]
    async fn json_parses_body() {
        let ctx = ctx_with("", br#"{"name":"ada"}"#);
        let Json(b) = Json::<Body>::from_request(&ctx).await.unwrap();
        assert_eq!(b.name, "ada");
    }

    #[derive(Deserialize, Validate)]
    struct NewUser {
        #[validate(length(min = 3))]
        name: String,
    }

    #[tokio::test]
    async fn valid_ok_and_422() {
        let ok = ctx_with("", br#"{"name":"ada"}"#);
        assert!(Valid::<NewUser>::from_request(&ok).await.is_ok());

        let bad = ctx_with("", br#"{"name":"a"}"#);
        let err = Valid::<NewUser>::from_request(&bad).await.err().unwrap();
        assert_eq!(err.status_code(), 422);
    }

    #[tokio::test]
    async fn context_identity_extractor() {
        let ctx = ctx_with("", b"");
        let extracted = Context::from_request(&ctx).await.unwrap();
        assert_eq!(extracted.req.method(), &hyper::Method::GET);
    }
}
