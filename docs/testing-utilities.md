# Testing Ultimo apps

Ultimo ships first-class testing utilities behind the `testing` feature. The
`TestClient` drives your app **in-process** — no socket, no ports, fully
deterministic and fast.

> This guide is about testing **your application** built with Ultimo. For the
> framework's own internal testing strategy and coverage standards, see
> [TESTING.md](TESTING.md).

## Enable the feature

Add `ultimo` with the `testing` feature as a **dev-dependency**:

```toml
[dev-dependencies]
ultimo = { version = "0.3", features = ["testing"] }
serde_json = "1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## TestClient: requests and assertions

```rust
use ultimo::testing::TestClient;
use ultimo::{Context, Ultimo};

fn app() -> Ultimo {
    let mut app = Ultimo::new_without_defaults();
    app.get("/hello", |ctx: Context| async move { ctx.text("hi").await });
    app.post("/echo", |ctx: Context| async move {
        let body: serde_json::Value = ctx.req.json().await.unwrap_or_default();
        ctx.json(body).await
    });
    app
}

#[tokio::test]
async fn hello_works() {
    let client = TestClient::new(app());

    let res = client.get("/hello").send().await;

    res.assert_ok();            // 200
    assert_eq!(res.text(), "hi");
}
```

### Building requests

The builder returned by `client.get/post/put/delete/patch/head/options(path)`
(or `client.request(method, path)`) is fluent:

```rust
let res = client
    .post("/users")
    .bearer("my-token")                          // Authorization: Bearer my-token
    .header("x-trace", "abc")
    .query(&[("page", "2"), ("q", "ada")])       // ?page=2&q=ada
    .json(&serde_json::json!({ "name": "Ada" })) // sets content-type: application/json
    .send()
    .await;
```

Other body setters: `.body(bytes)` and `.text("…")`.

### Inspecting responses

All accessors are synchronous (the body is already buffered):

- `res.status() -> StatusCode`
- `res.header(name) -> Option<&str>`, `res.headers() -> &HeaderMap`
- `res.text() -> String`, `res.bytes() -> &Bytes`
- `res.json::<T>() -> T` (panics with the body on parse failure)

Chainable assertions (panic with a clear message; return `&Self`):

```rust
res.assert_status(201)
   .assert_header("content-type", "application/json")
   .assert_json(&serde_json::json!({ "id": 1 }));
```

There are also `assert_ok()` (200) and `assert_status_is_success()` (2xx).

### Macros

```rust
ultimo::assert_status!(res, 200);
ultimo::assert_json_eq!(res.json::<serde_json::Value>(), serde_json::json!({ "ok": true }));
```

## Testing middleware in isolation

Build a `Context` with `test_context()` and run a single middleware against it
with `run_middleware`. Construct the middleware the same way the built-ins do —
`Arc::new(|ctx, next| Box::pin(async move { … }))`:

```rust
use std::sync::Arc;
use ultimo::middleware::{BoxedMiddleware, Next};
use ultimo::testing::{run_middleware, test_context};
use ultimo::Context;

fn auth() -> BoxedMiddleware {
    Arc::new(|ctx: Context, next: Next| {
        Box::pin(async move {
            if ctx.req.header("authorization").is_none() {
                return ultimo::response::ResponseBuilder::new()
                    .status(401)
                    .text("unauthorized")
                    .build();
            }
            next(ctx).await
        })
    })
}

#[tokio::test]
async fn blocks_unauthenticated() {
    let ctx = test_context().path("/private").build();
    let res = run_middleware(auth(), ctx, |ctx| async move { ctx.text("ok").await })
        .await
        .unwrap();
    assert_eq!(res.status(), 401);
}
```

`test_context()` supports `.method()`, `.path()`, `.header()`, and `.body()`.

## Database tests with automatic rollback

Enable `testing` plus a `sqlx-*` backend. `with_test_transaction` runs your
closure inside a transaction that is **always rolled back**, so tests never
mutate persistent state. The closure mirrors sqlx's own `Pool::transaction`
shape — return a boxed future via `Box::pin(async move { … })`:

```rust
use ultimo::testing::with_test_transaction;

let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
sqlx::query("CREATE TABLE t (id INTEGER PRIMARY KEY)").execute(&pool).await.unwrap();

with_test_transaction(&pool, |tx| {
    Box::pin(async move {
        sqlx::query("INSERT INTO t (id) VALUES (1)")
            .execute(&mut **tx)
            .await
            .unwrap();
        Ok(())
    })
})
.await
.unwrap();

// The insert was rolled back — the table is empty again.
let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM t")
    .fetch_one(&pool).await.unwrap();
assert_eq!(count, 0);
```

## Fixtures

Load typed fixtures from JSON files:

```rust
use ultimo::testing::load_fixture;

#[derive(serde::Deserialize)]
struct User { id: u32, name: String }

let user: User = load_fixture("tests/fixtures/user.json");
```

For seed/cleanup lifecycles, implement the `Fixture` trait (`setup`/`teardown`).
