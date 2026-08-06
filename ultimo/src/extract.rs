//! Typed request extractors: implement `FromRequest` to pull typed data from a
//! request. Handlers may take any number of these as parameters, e.g.
//! `|Path(id): Path<u32>, Query(q): Query<Filter>| async move { … }`.

use crate::context::Context;
use crate::error::{Result, UltimoError};
use serde::de::{self, DeserializeOwned, Deserializer, Visitor};
use std::fmt;
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

/// Extracts typed path parameters captured by the route (`:name` segments).
/// A single-parameter route deserializes into a scalar (`Path<u32>`); a
/// multi-parameter route deserializes into a struct whose fields match the
/// parameter names (`Path<Struct>`).
pub struct Path<T>(pub T);

#[async_trait::async_trait]
impl<T: DeserializeOwned + Send> FromRequest for Path<T> {
    async fn from_request(ctx: &Context) -> Result<Self> {
        let params = ctx.req.params();
        let value: T = if params.len() == 1 {
            let raw = params.values().next().expect("len == 1");
            T::deserialize(ScalarStr(raw))
                .map_err(|e| UltimoError::BadRequest(format!("Invalid path parameter: {e}")))?
        } else {
            let qs = serde_urlencoded::to_string(params)
                .map_err(|e| UltimoError::BadRequest(format!("Invalid path parameters: {e}")))?;
            serde_urlencoded::from_str::<T>(&qs)
                .map_err(|e| UltimoError::BadRequest(format!("Invalid path parameters: {e}")))?
        };
        Ok(Path(value))
    }
}

/// A serde deserializer over a single string that parses numeric/bool targets
/// but passes strings through unchanged (so `Path<String>` of "42" stays "42").
struct ScalarStr<'a>(&'a str);

#[derive(Debug)]
struct ScalarErr(String);
impl fmt::Display for ScalarErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ScalarErr {}
impl de::Error for ScalarErr {
    fn custom<M: fmt::Display>(msg: M) -> Self {
        ScalarErr(msg.to_string())
    }
}

macro_rules! scalar_parse {
    ($method:ident, $visit:ident, $ty:ty) => {
        fn $method<V: Visitor<'de>>(self, v: V) -> std::result::Result<V::Value, Self::Error> {
            let parsed: $ty = self.0.parse().map_err(|_| {
                ScalarErr(format!("cannot parse '{}' as {}", self.0, stringify!($ty)))
            })?;
            v.$visit(parsed)
        }
    };
}

impl<'de> Deserializer<'de> for ScalarStr<'de> {
    type Error = ScalarErr;

    scalar_parse!(deserialize_i8, visit_i8, i8);
    scalar_parse!(deserialize_i16, visit_i16, i16);
    scalar_parse!(deserialize_i32, visit_i32, i32);
    scalar_parse!(deserialize_i64, visit_i64, i64);
    scalar_parse!(deserialize_u8, visit_u8, u8);
    scalar_parse!(deserialize_u16, visit_u16, u16);
    scalar_parse!(deserialize_u32, visit_u32, u32);
    scalar_parse!(deserialize_u64, visit_u64, u64);
    scalar_parse!(deserialize_f32, visit_f32, f32);
    scalar_parse!(deserialize_f64, visit_f64, f64);
    scalar_parse!(deserialize_bool, visit_bool, bool);

    fn deserialize_str<V: Visitor<'de>>(self, v: V) -> std::result::Result<V::Value, Self::Error> {
        v.visit_borrowed_str(self.0)
    }
    fn deserialize_string<V: Visitor<'de>>(
        self,
        v: V,
    ) -> std::result::Result<V::Value, Self::Error> {
        v.visit_str(self.0)
    }
    fn deserialize_any<V: Visitor<'de>>(self, v: V) -> std::result::Result<V::Value, Self::Error> {
        v.visit_borrowed_str(self.0)
    }
    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        v: V,
    ) -> std::result::Result<V::Value, Self::Error> {
        v.visit_newtype_struct(self)
    }

    serde::forward_to_deserialize_any! {
        char bytes byte_buf option unit unit_struct seq tuple tuple_struct
        map struct enum identifier ignored_any
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

    fn ctx_with_params(pairs: &[(&str, &str)]) -> Context {
        let parts = hyper::Request::builder()
            .uri("http://x/")
            .body(())
            .unwrap()
            .into_parts()
            .0;
        let params: crate::router::Params = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Context::from_parts(parts, Bytes::new(), params)
    }

    #[tokio::test]
    async fn path_single_u32() {
        let ctx = ctx_with_params(&[("id", "42")]);
        let Path(id) = Path::<u32>::from_request(&ctx).await.unwrap();
        assert_eq!(id, 42);
    }

    #[tokio::test]
    async fn path_single_string_numeric_value_stays_string() {
        let ctx = ctx_with_params(&[("id", "42")]);
        let Path(id) = Path::<String>::from_request(&ctx).await.unwrap();
        assert_eq!(id, "42"); // NOT coerced to a number
    }

    #[derive(Deserialize)]
    struct Coord {
        x: u32,
        y: u32,
    }

    #[tokio::test]
    async fn path_struct_multi_param() {
        let ctx = ctx_with_params(&[("x", "3"), ("y", "5")]);
        let Path(c) = Path::<Coord>::from_request(&ctx).await.unwrap();
        assert_eq!((c.x, c.y), (3, 5));
    }

    #[tokio::test]
    async fn path_single_bad_is_400() {
        let ctx = ctx_with_params(&[("id", "notanumber")]);
        let err = Path::<u32>::from_request(&ctx).await.err().unwrap();
        assert_eq!(err.status_code(), 400);
    }

    #[tokio::test]
    async fn context_identity_extractor() {
        let ctx = ctx_with("", b"");
        let extracted = Context::from_request(&ctx).await.unwrap();
        assert_eq!(extracted.req.method(), &hyper::Method::GET);
    }
}
