# Typed Handler Extractors Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let route handlers declare typed `Path`/`Query`/`Json`/`Valid` parameters that Ultimo parses and validates automatically, with 400 on malformed input and 422 on validation failure.

**Architecture:** One async `FromRequest` trait; every extractor borrows `&Context` (the body is already buffered/cached, so no ownership transfer). A declarative macro generates `IntoHandler` for handler fns of arity 0..=8 where every parameter is `FromRequest`; `Context: FromRequest` keeps existing `|ctx: Context|` handlers working. `Query` uses `serde_urlencoded`; `Path` uses `serde_urlencoded` for multi-param structs and a small scalar deserializer for a single param.

**Tech Stack:** Rust, `async-trait` (already a dep), `serde`, `serde_json` (already deps), `serde_urlencoded` (add as direct dep; already in the lockfile transitively).

**Spec:** `docs/superpowers/specs/2026-08-06-typed-handler-extractors-design.md`

## Global Constraints

- 100% safe Rust (`#![forbid(unsafe_code)]`); no `unsafe`.
- Typed extractors are **core** (no Cargo feature gate).
- Handler return type stays `Result<Response>` (input side only; no `IntoResponse` sugar).
- Backward compatible: existing `|ctx: Context| async move { … }` handlers must compile and behave unchanged.
- Error mapping: malformed/undeserializable → `UltimoError::BadRequest` (400); validation failure → `UltimoError::Validation` (422).
- `Params` is `HashMap<String, String>` (unordered); `Context` has `pub req: Request`; body accessors (`ctx.req.json()/bytes()`) are async and cached.

## Facts pinned by spike (do not re-derive)

- `Request` (in `ultimo/src/context.rs`) stores a private `uri: hyper::Uri`; there is **no** public raw-query accessor yet → Task 1 adds `query_string()`.
- `serde_urlencoded::from_str::<T>` deserializes a `k=v&…` string type-directed (correct for both `String` and numeric fields), but **errors on a bare single value** (`"42"` → "expected map").
- The `serde_json` coercion bridge mis-handles `Path<String>` when the value looks numeric → **not used** for Path.
- `serde_urlencoded::to_string(&HashMap<String,String>)` reconstructs a query string from the param map (order nondeterministic — irrelevant for struct fields).

---

### Task 1: Foundations — dep, `query_string()` accessor, and 400→422 flip

**Files:**
- Modify: `ultimo/Cargo.toml` (deps)
- Modify: `ultimo/src/context.rs` (add `Request::query_string`)
- Modify: `ultimo/src/error.rs` (Validation status 400→422 + its unit test)

**Interfaces:**
- Produces: `Request::query_string(&self) -> Option<&str>`; `UltimoError::Validation.status_code() == 422`.

- [ ] **Step 1: Add `serde_urlencoded` as a direct dependency**

In `ultimo/Cargo.toml` under `[dependencies]`, add:

```toml
serde_urlencoded = "0.7"
```

- [ ] **Step 2: Write the failing test for the 422 flip**

In `ultimo/src/error.rs`, find the existing test asserting the Validation status is 400 and change it to expect 422:

```rust
    // (in the error tests module) validation now maps to 422 Unprocessable Entity
    let err = UltimoError::Validation {
        message: "Validation failed".to_string(),
        details: vec![],
    };
    assert_eq!(err.status_code(), 422);
```

- [ ] **Step 3: Run it to verify it fails**

Run: `cargo test -p ultimo --lib error::`
Expected: FAIL — Validation currently returns 400.

- [ ] **Step 4: Flip the status mapping**

In `ultimo/src/error.rs` `status_code()`, change the Validation arm:

```rust
            UltimoError::Validation { .. } => 422,
```

- [ ] **Step 5: Add the `query_string` accessor**

In `ultimo/src/context.rs`, in `impl Request`, next to `query()`:

```rust
    /// The raw query string (everything after `?`), if present.
    pub fn query_string(&self) -> Option<&str> {
        self.uri.query()
    }
```

- [ ] **Step 6: Run tests + build**

Run: `cargo test -p ultimo --lib error:: && cargo build -p ultimo`
Expected: PASS / clean.

- [ ] **Step 7: Commit**

```bash
git add ultimo/Cargo.toml Cargo.lock ultimo/src/context.rs ultimo/src/error.rs
git commit -m "feat(core): validation -> 422; add Request::query_string; serde_urlencoded dep"
```

---

### Task 2: `FromRequest` trait + `Context`/`Json`/`Query`/`Valid` extractors

**Files:**
- Create: `ultimo/src/extract.rs`
- Modify: `ultimo/src/lib.rs` (add `pub mod extract;` + re-exports)
- Test: unit tests inside `ultimo/src/extract.rs`

**Interfaces:**
- Produces:
  - `pub trait FromRequest: Sized { async fn from_request(ctx: &Context) -> Result<Self>; }` (via `#[async_trait::async_trait]`).
  - `pub struct Json<T>(pub T);`, `pub struct Query<T>(pub T);`, `pub struct Valid<T>(pub T);`
  - `impl FromRequest for Context` (clones the context).
- Consumes: `Context`, `ctx.req.json()`, `ctx.req.query_string()` (Task 1), `crate::validate`.

- [ ] **Step 1: Write the failing tests**

Create `ultimo/src/extract.rs` with the tests first (a helper builds a `Context` from parts):

```rust
//! Typed request extractors: implement `FromRequest` to pull typed data from a
//! request. Handlers may take any number of these as parameters.

use crate::context::Context;
use crate::error::{Result, UltimoError};
use serde::de::DeserializeOwned;
use validator::Validate;

/// A type that can be extracted from the request context.
#[async_trait::async_trait]
pub trait FromRequest: Sized {
    async fn from_request(ctx: &Context) -> Result<Self>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{Context, Request};
    use bytes::Bytes;
    use serde::Deserialize;

    fn ctx_with(query: &str, body: &[u8]) -> Context {
        let parts = hyper::Request::builder()
            .uri(format!("http://x/?{query}"))
            .body(())
            .unwrap()
            .into_parts()
            .0;
        let req = Request::from_parts(parts, Bytes::copy_from_slice(body), Default::default());
        Context::from_request_for_test(req)
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
        let err = Query::<Filter>::from_request(&ctx).await.unwrap_err();
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
        let err = Valid::<NewUser>::from_request(&bad).await.unwrap_err();
        assert_eq!(err.status_code(), 422);
    }
}
```

- [ ] **Step 2: Add a test-only Context constructor if one does not exist**

`Context::from_request_for_test` is used by the tests. If `ultimo/src/context.rs` has no equivalent public/`pub(crate)` test constructor, add one next to the existing constructors:

```rust
    /// Build a Context wrapping `req` with no DB, for tests.
    #[cfg(test)]
    pub(crate) fn from_request_for_test(req: Request) -> Self { /* mirror the fields set by the real constructor, DB = None */ }
```

Match the real constructor's field initialization (see `context.rs:252` area); set any store/state fields to their empty defaults and the database to `None`.

- [ ] **Step 3: Run to verify it fails**

Run: `cargo test -p ultimo --lib extract::`
Expected: FAIL — `Json`/`Query`/`Valid` not defined.

- [ ] **Step 4: Implement the extractors**

Add to `ultimo/src/extract.rs` (above the tests):

```rust
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

/// The full request context (identity extractor — keeps `|ctx: Context|` handlers working).
#[async_trait::async_trait]
impl FromRequest for Context {
    async fn from_request(ctx: &Context) -> Result<Self> {
        Ok(ctx.clone())
    }
}
```

> If `Context` is not `Clone`, derive/implement `Clone` for it in `context.rs` (its fields — `Request` with `Arc<RwLock<..>>` body, maps, optional DB handle — are all cheaply clonable; add `#[derive(Clone)]` or a manual impl). This is required for the identity extractor.

- [ ] **Step 5: Wire the module + re-exports**

In `ultimo/src/lib.rs`, add `pub mod extract;` and re-export the common items (mirror how other modules are surfaced, e.g. in the prelude):

```rust
pub mod extract;
pub use extract::{FromRequest, Json, Query, Valid};
```

Add `FromRequest, Json, Query, Valid, Path` (Path lands in Task 3) to the `prelude` module as well.

- [ ] **Step 6: Run to verify it passes**

Run: `cargo test -p ultimo --lib extract::`
Expected: PASS (query typed + 400, json, valid ok + 422).

- [ ] **Step 7: Commit**

```bash
git add ultimo/src/extract.rs ultimo/src/lib.rs ultimo/src/context.rs
git commit -m "feat(extract): FromRequest trait + Json/Query/Valid/Context extractors"
```

---

### Task 3: `Path` extractor (single-scalar + struct)

**Files:**
- Modify: `ultimo/src/extract.rs` (add `Path` + a scalar deserializer)
- Test: unit tests inside `ultimo/src/extract.rs`

**Interfaces:**
- Produces: `pub struct Path<T>(pub T);` with `impl FromRequest for Path<T> where T: DeserializeOwned`.

- [ ] **Step 1: Write the failing tests**

Add to the `tests` module in `ultimo/src/extract.rs`:

```rust
    fn ctx_with_params(pairs: &[(&str, &str)]) -> Context {
        let parts = hyper::Request::builder().uri("http://x/").body(()).unwrap().into_parts().0;
        let params: crate::router::Params =
            pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        let req = Request::from_parts(parts, bytes::Bytes::new(), params);
        Context::from_request_for_test(req)
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

    #[derive(serde::Deserialize)]
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
        let err = Path::<u32>::from_request(&ctx).await.unwrap_err();
        assert_eq!(err.status_code(), 400);
    }
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p ultimo --lib extract::tests::path`
Expected: FAIL — `Path` not defined.

- [ ] **Step 3: Implement `Path` + the scalar deserializer**

Add to `ultimo/src/extract.rs`. The single-param case uses a tiny type-directed scalar deserializer that parses numbers/bools and passes strings through; the multi-param case round-trips the param map through `serde_urlencoded`.

```rust
use serde::de::{self, Deserializer, Visitor};
use std::fmt;

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
            let raw = params.values().next().expect("len==1");
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(&self.0) }
}
impl std::error::Error for ScalarErr {}
impl de::Error for ScalarErr {
    fn custom<M: fmt::Display>(msg: M) -> Self { ScalarErr(msg.to_string()) }
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
    fn deserialize_string<V: Visitor<'de>>(self, v: V) -> std::result::Result<V::Value, Self::Error> {
        v.visit_str(self.0)
    }
    fn deserialize_any<V: Visitor<'de>>(self, v: V) -> std::result::Result<V::Value, Self::Error> {
        v.visit_borrowed_str(self.0)
    }
    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self, _name: &'static str, v: V,
    ) -> std::result::Result<V::Value, Self::Error> {
        v.visit_newtype_struct(self)
    }

    serde::forward_to_deserialize_any! {
        char bytes byte_buf option unit unit_struct seq tuple tuple_struct
        map struct enum identifier ignored_any
    }
}
```

> Verify the exact `serde` `Deserializer` associated items compile (this is the spike step): `cargo build -p ultimo` and fix any signature drift against the installed `serde` before running tests.

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test -p ultimo --lib extract::tests::path`
Expected: PASS (single u32, single String stays string, struct multi-param, bad → 400).

- [ ] **Step 5: Add `Path` to the re-exports**

Ensure `ultimo/src/lib.rs` re-exports and prelude include `Path` (added in Task 2 Step 5).

- [ ] **Step 6: Commit**

```bash
git add ultimo/src/extract.rs ultimo/src/lib.rs
git commit -m "feat(extract): Path extractor (single-scalar + struct params)"
```

---

### Task 4: Macro'd `IntoHandler` for arities 0..=8 + backward compat

**Files:**
- Modify: `ultimo/src/handler.rs` (replace the single `Fn(Context)` impl with a macro)
- Test: integration test `ultimo/tests/extractors.rs`

**Interfaces:**
- Consumes: `FromRequest` (Task 2), all extractors.
- Produces: `IntoHandler` for `Fn(T1..Tn) -> Fut` (`n = 0..=8`, each `Ti: FromRequest`, `F: Clone`), returning `Result<Response>`.

- [ ] **Step 1: Write the failing integration test**

Create `ultimo/tests/extractors.rs`:

```rust
//! Integration tests for typed handler extractors.
//! Run with: cargo test -p ultimo --test extractors

use bytes::Bytes;
use http_body_util::Full;
use hyper::Request as HyperRequest;
use serde::Deserialize;
use ultimo::prelude::*;
use ultimo::extract::{Path, Query, Valid};

#[derive(Deserialize)]
struct Page {
    page: u32,
}

#[derive(Deserialize, validator::Validate)]
struct NewUser {
    #[validate(length(min = 3))]
    name: String,
}

fn app() -> Ultimo {
    let mut app = Ultimo::new_without_defaults();
    app.get("/items/:id", |Path(id): Path<u32>, Query(p): Query<Page>| async move {
        response::helpers::text(format!("id={id} page={}", p.page))
    });
    app.post("/users", |Valid(u): Valid<NewUser>| async move {
        response::helpers::text(format!("created {}", u.name))
    });
    // Backward-compat: a Context-only handler still works.
    app.get("/ping", |_ctx: Context| async move { response::helpers::text("pong") });
    app
}

fn get(uri: &str) -> HyperRequest<Full<Bytes>> {
    HyperRequest::builder().uri(uri).body(Full::new(Bytes::new())).unwrap()
}

#[tokio::test]
async fn extracts_path_and_query() {
    let res = app().oneshot(get("/items/7?page=3")).await;
    assert_eq!(res.status(), 200);
    let body = http_body_util::BodyExt::collect(res.into_body()).await.unwrap().to_bytes();
    assert_eq!(&body[..], b"id=7 page=3");
}

#[tokio::test]
async fn malformed_path_is_400() {
    let res = app().oneshot(get("/items/notanumber?page=3")).await;
    assert_eq!(res.status(), 400);
}

#[tokio::test]
async fn valid_body_ok_and_422() {
    let ok = HyperRequest::builder().method("POST").uri("/users")
        .body(Full::new(Bytes::from_static(br#"{"name":"ada"}"#))).unwrap();
    assert_eq!(app().oneshot(ok).await.status(), 200);

    let bad = HyperRequest::builder().method("POST").uri("/users")
        .body(Full::new(Bytes::from_static(br#"{"name":"a"}"#))).unwrap();
    assert_eq!(app().oneshot(bad).await.status(), 422);
}

#[tokio::test]
async fn context_handler_still_works() {
    assert_eq!(app().oneshot(get("/ping")).await.status(), 200);
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p ultimo --test extractors`
Expected: FAIL to compile — multi-parameter handlers do not implement `IntoHandler` yet.

- [ ] **Step 3: Replace the handler impl with the arity macro**

In `ultimo/src/handler.rs`, delete the existing `impl<F, Fut> IntoHandler for F where F: Fn(Context) -> Fut` block and replace it with a macro that generates impls for tuples of extractors:

```rust
use crate::extract::FromRequest;

macro_rules! impl_into_handler {
    ( $( $ty:ident ),* ) => {
        impl<F, Fut, $( $ty ),*> IntoHandler for F
        where
            F: Fn( $( $ty ),* ) -> Fut + Clone + Send + Sync + 'static,
            Fut: Future<Output = Result<Response>> + Send + 'static,
            $( $ty: FromRequest + Send + 'static, )*
        {
            #[allow(non_snake_case, unused_variables)]
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
```

> The arity-1 impl (`impl_into_handler!(T1)`) plus `Context: FromRequest` (Task 2) subsumes the old `Fn(Context)` impl — that is what keeps `|ctx: Context|` handlers working. The `F: Clone` bound is new (needed to move the handler into the per-request async block); ordinary closures capturing `Arc`/`Clone` state satisfy it.

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test -p ultimo --test extractors`
Expected: PASS (path+query, 400, 422, context-handler).

- [ ] **Step 5: Confirm the whole default suite still compiles/passes**

Run: `cargo test -p ultimo --lib && cargo build -p ultimo --all-targets`
Expected: PASS — existing Context-only handlers across the codebase still compile under the new impls.

- [ ] **Step 6: Commit**

```bash
git add ultimo/src/handler.rs ultimo/tests/extractors.rs
git commit -m "feat(handler): typed extractor handlers via arity macro (0..=8); keep Context handlers"
```

---

### Task 5: Docs, roadmap, CI, verification gate

**Files:**
- Modify: `docs-site/docs/pages/api-reference.mdx` (document the extractors)
- Modify: `docs-site/docs/pages/roadmap.mdx` (mark shipped)
- Modify: `.github/workflows/ci.yml` (run the extractors integration test)
- Modify: `CHANGELOG.md` note is handled by release-plz commits — do NOT hand-edit

- [ ] **Step 1: Document extractors in the API reference**

In `docs-site/docs/pages/api-reference.mdx`, add a "Typed Extractors" subsection near the handler/Context docs:

````mdx
### Typed extractors

Handlers may take typed parameters instead of a raw `Context`. Each implements
`FromRequest` and is parsed from the request automatically:

```rust
use ultimo::extract::{Path, Query, Valid};

// GET /users/:id?verbose=true
app.get("/users/:id", |Path(id): Path<u32>, Query(q): Query<Opts>| async move { /* … */ });

// POST /users  — deserializes the JSON body AND validates it
app.post("/users", |Valid(body): Valid<CreateUser>| async move { /* … */ });
```

| Extractor | Source | Failure |
|---|---|---|
| `Path<T>` | route `:params` (single scalar, or a struct of names) | 400 |
| `Query<T>` | query string | 400 |
| `Json<T>` | request body | 400 |
| `Valid<T>` | request body + `validator` | 400 (parse) / **422** (invalid) |
| `Context` | the whole request | — |

`|ctx: Context|` handlers keep working unchanged (`Context` is itself an
extractor). Validation failures now return **422 Unprocessable Entity**.
````

- [ ] **Step 2: Mark the roadmap item shipped**

In `docs-site/docs/pages/roadmap.mdx`, change the `Typed Handler Extractors + Validation` Feature Status row from `📋 Planned | 0.7.0` to `✅ Available | 0.7.0`, and turn the matching v0.7.0 timeline bullet into a `✅` shipped line.

- [ ] **Step 3: Add the integration test to CI**

In `.github/workflows/ci.yml`, in the `test` job's default-feature test area, add:

```yaml
      - name: Typed extractor tests
        run: cargo test -p ultimo --test extractors
```

- [ ] **Step 4: Run the verification gate**

Run:
```bash
cargo fmt --all --check
cargo clippy -p ultimo --lib --all-targets -- -D warnings
cargo test -p ultimo --lib
cargo test -p ultimo --test extractors
bash scripts/check-versions.sh
```
Expected: all clean/pass. (`cargo fmt --all` first if needed.)

- [ ] **Step 5: Commit**

```bash
git add docs-site/docs/pages/api-reference.mdx docs-site/docs/pages/roadmap.mdx .github/workflows/ci.yml
git commit -m "docs+ci: document typed extractors, mark roadmap shipped, run extractor tests"
```

---

## Self-review notes

- **Spec coverage:** `FromRequest` trait + `extract.rs` (Task 2); `Path`/`Query`/`Json`/`Valid`/`Context` (Tasks 2–3); macro'd `IntoHandler` 0..=8 replacing `Fn(Context)` with `Context: FromRequest` backward-compat (Task 4); 400/422 semantics + `Validation`→422 + its test (Task 1 + Task 2 Valid); `serde_urlencoded` dep + `query_string()` (Task 1); tests unit + integration + backward-compat (Tasks 2–4); docs (Task 5). Header/IntoResponse explicitly out of scope. ✔
- **Type consistency:** `FromRequest::from_request(ctx: &Context) -> Result<Self>` is used identically across all extractors and the macro. Extractor names `Path`/`Query`/`Json`/`Valid` match spec and re-exports. `F: Clone` bound documented as new.
- **Risk flags:** Task 3's `ScalarStr` deserializer and Task 4's macro are the two type-heavy pieces — each has a compile-spike step and concrete TDD assertions (single `u32`, numeric-looking `String`, struct, 400/422, backward-compat) that lock behavior. `Context` must be `Clone` (Task 2 note).
- **Edge case (documented, out of scope for v1):** a single-parameter route deserialized into a *struct* target hits the scalar branch and will error; use the bare type for single params, a struct for multiple.
