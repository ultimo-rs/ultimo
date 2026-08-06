# Typed Handler Extractors — Design

**Date:** 2026-08-06
**Status:** Approved (brainstorm), pending spec review
**Roadmap:** 0.7 — "Typed Handler Extractors + Validation"

## Goal

Let route handlers declare **typed** path / query / body parameters that Ultimo
parses (and optionally validates) automatically, returning the right HTTP error
on bad input. This completes the type-safe story on the **input** side —
mirroring the type-safe **output** already delivered by the generated TypeScript
client.

```rust
// before
app.get("/users/:id", |ctx: Context| async move {
    let id: u32 = ctx.req.param("id")?.parse().map_err(|_| UltimoError::BadRequest("bad id".into()))?;
    // ...
});

// after
app.get("/users/:id", |Path(id): Path<u32>, Query(f): Query<Filter>| async move {
    // id: u32 and f: Filter already parsed; malformed input → 400 automatically
});

app.post("/users", |Valid(body): Valid<CreateUser>| async move {
    // body deserialized AND validated; invalid → 422 with field details
});
```

## Enabling insight

Ultimo's `Context` **already buffers and caches the request body** (`ctx.req.json()`
/ `bytes()` are documented as callable multiple times). So every extractor can
borrow `&Context` and read what it needs — path params, the query string, or the
cached body — with **no body-ownership transfer**. This removes the reason axum
splits `FromRequestParts` (non-body) from `FromRequest` (body-consuming): we need
**one** trait.

## Architecture

### The `FromRequest` trait (new module `ultimo/src/extract.rs`)

```rust
#[async_trait::async_trait]
pub trait FromRequest: Sized {
    async fn from_request(ctx: &Context) -> Result<Self>;
}
```

(`async-trait` is already a dependency.)

### Extractors shipped in v1

| Extractor | Reads from `&Context` | Failure → status |
|---|---|---|
| `Path<T: DeserializeOwned>` | captured route params (`:name`) | parse error → **400** |
| `Query<T: DeserializeOwned>` | the URI query string | parse error → **400** |
| `Json<T: DeserializeOwned>` | the cached request body | deserialize error → **400** |
| `Valid<T: DeserializeOwned + Validate>` | the cached body (deserialize **then** `validate`) | parse → **400**, validation → **422** |
| `Context` | itself | never |

- `Query<T>` deserializes the query string with `serde_urlencoded` (coerces
  strings into `T`'s field types).
- `Path<T>` deserializes the captured params (a `name → string` map) into `T`
  via a small string-coercing deserializer — supports the single-param case
  (`Path<u32>` on a one-`:param` route) and the struct case (`Path<Struct>` whose
  fields match the `:param` names). Exact deserializer mechanics are pinned in
  the implementation plan (spike first, as with ts-rs).
- **`Context` implements `FromRequest`** (returns the context), so every existing
  `|ctx: Context| async move { … }` handler keeps compiling **unchanged** — it is
  now simply a one-extractor handler.

### `IntoHandler` for multi-parameter handlers (`ultimo/src/handler.rs`)

Replace the single `impl IntoHandler for Fn(Context) -> Fut` with a macro that
generates impls for `Fn(T1, …, Tn) -> Fut` where every `Ti: FromRequest`, for
arities `n = 0..=8`:

```rust
// generated for each arity; each Ti extracted from &ctx in order, then the
// handler is called. Return type stays Result<Response>.
impl<F, Fut, T1, /* … */> IntoHandler for F
where
    F: Fn(T1, /* … */) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<Response>> + Send + 'static,
    T1: FromRequest + Send + 'static, /* … */
{ /* build BoxedHandler: extract each Ti, short-circuit on Err, call self */ }
```

Different arities are distinct `Fn` traits → no coherence overlap. Because
`Context: FromRequest`, the arity-1 impl subsumes the old `Fn(Context)` case, so
removing the special impl is **non-breaking for users**.

### Error semantics

- **Syntactically malformed / undeserializable** input (bad path/query/JSON) →
  `UltimoError::BadRequest` → **400**.
- **Well-formed but invalid** (`Valid<T>` validation failure) →
  `UltimoError::Validation` → **422**.
- **Change required:** `UltimoError::Validation` currently maps to **400**
  (`error.rs`). Flip it to **422** (Unprocessable Entity — the correct code for
  validation failures, and what FastAPI uses). `Valid<T>` reuses the existing
  `validate()` helper, so this makes all validation errors consistent. This is an
  intentional, minor behavior change (existing `validate()` callers move
  400→422); noted in the changelog. It is not an API-signature change, so
  `semver-checks` is unaffected. The existing `error.rs` unit test asserting
  `Validation.status_code() == 400` must be updated to `422` as part of this
  change.

## Data flow

`dispatch` builds `Context` (already does), then the `BoxedHandler` (produced by
`into_handler`) calls `Ti::from_request(&ctx).await?` for each parameter in order
and invokes the user's async fn with the extracted values. Any extractor `Err`
short-circuits into the standard error→response path (which now yields 400/422).

## Testing

- **Unit tests per extractor** (`extract.rs`): success + failure-status for
  `Path`, `Query`, `Json`, `Valid` (including 422 on validation failure), and
  `Context` round-trip. Build a `Context` via the existing test helpers /
  `oneshot` seam.
- **Integration test**: register a handler taking multiple extractors
  (`Path` + `Query` + `Valid`) and dispatch via `Ultimo::oneshot`, asserting a
  200 on good input, 400 on malformed, 422 on invalid.
- **Backward-compat test**: an existing-style `|ctx: Context|` handler still
  compiles and responds (guards the `IntoHandler` change).

## Scope

- **v1:** `Path`, `Query`, `Json`, `Valid` + `Context`; arities 0..=8; 400/422
  semantics; input side only.
- **Out of scope:** `Header<T>` typed extractor (reachable today via
  `ctx.req.header`; add on demand); `State`/extension extractors; returning a
  serializable value directly from handlers (`IntoResponse` sugar — a separate
  later item); optional/`Option<T>` extractor combinators.

## Dependencies

- `serde_urlencoded` for `Query`/`Path` string→type coercion (small, pure Rust;
  add as a direct dependency if not already resolvable). No feature gate — typed
  extractors are core, not opt-in.
