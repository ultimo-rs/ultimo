//! Testing utilities for Ultimo applications.
//!
//! Enable with `ultimo = { version = "…", features = ["testing"] }` under
//! `[dev-dependencies]`. The [`TestClient`] drives your app in-process — no
//! socket, fully deterministic.
//!
//! ```
//! use ultimo::testing::TestClient;
//! use ultimo::{Context, Ultimo};
//!
//! # async fn run() {
//! let mut app = Ultimo::new_without_defaults();
//! app.get("/ping", |ctx: Context| async move { ctx.text("pong").await });
//!
//! let client = TestClient::new(app);
//! let res = client.get("/ping").send().await;
//! res.assert_ok();
//! assert_eq!(res.text(), "pong");
//! # }
//! ```

mod client;
mod fixtures;
mod middleware;
mod response;

#[cfg(feature = "sqlx")]
mod database;

pub use client::{TestClient, TestRequest};
pub use fixtures::{load_fixture, Fixture};
pub use middleware::{run_middleware, test_context, TestContextBuilder};
pub use response::TestResponse;

#[cfg(feature = "sqlx")]
pub use database::with_test_transaction;

/// Assert two values are equal as JSON, with a readable diff on failure.
///
/// ```
/// # use ultimo::assert_json_eq;
/// assert_json_eq!(serde_json::json!({ "a": 1 }), serde_json::json!({ "a": 1 }));
/// ```
#[macro_export]
macro_rules! assert_json_eq {
    ($actual:expr, $expected:expr $(,)?) => {{
        let actual = ::serde_json::to_value(&$actual).expect("actual is serializable");
        let expected = ::serde_json::to_value(&$expected).expect("expected is serializable");
        assert_eq!(
            actual, expected,
            "JSON mismatch\n  actual:   {}\n  expected: {}",
            actual, expected
        );
    }};
}

/// Assert a [`TestResponse`](crate::testing::TestResponse) has the given status.
#[macro_export]
macro_rules! assert_status {
    ($res:expr, $code:expr $(,)?) => {{
        $res.assert_status($code);
    }};
}
