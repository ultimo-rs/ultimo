//! Integration tests for typed handler extractors.
//! Run with: cargo test -p ultimo --test extractors

use bytes::Bytes;
use http_body_util::Full;
use hyper::Request as HyperRequest;
use serde::Deserialize;
use ultimo::extract::{Path, Query, Valid};
use ultimo::prelude::*;

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
    app.get(
        "/items/:id",
        |Path(id): Path<u32>, Query(p): Query<Page>| async move {
            ultimo::response::helpers::text(format!("id={id} page={}", p.page))
        },
    );
    app.post("/users", |Valid(u): Valid<NewUser>| async move {
        ultimo::response::helpers::text(format!("created {}", u.name))
    });
    // Backward-compat: a Context-only handler still works.
    app.get("/ping", |_ctx: Context| async move {
        ultimo::response::helpers::text("pong")
    });
    app
}

fn get(uri: &str) -> HyperRequest<Full<Bytes>> {
    HyperRequest::builder()
        .uri(uri)
        .body(Full::new(Bytes::new()))
        .unwrap()
}

#[tokio::test]
async fn extracts_path_and_query() {
    let res = app().oneshot(get("/items/7?page=3")).await;
    assert_eq!(res.status(), 200);
    let body = http_body_util::BodyExt::collect(res.into_body())
        .await
        .unwrap()
        .to_bytes();
    assert_eq!(&body[..], b"id=7 page=3");
}

#[tokio::test]
async fn malformed_path_is_400() {
    let res = app().oneshot(get("/items/notanumber?page=3")).await;
    assert_eq!(res.status(), 400);
}

#[tokio::test]
async fn valid_body_ok_and_422() {
    let ok = HyperRequest::builder()
        .method("POST")
        .uri("/users")
        .body(Full::new(Bytes::from_static(br#"{"name":"ada"}"#)))
        .unwrap();
    assert_eq!(app().oneshot(ok).await.status(), 200);

    let bad = HyperRequest::builder()
        .method("POST")
        .uri("/users")
        .body(Full::new(Bytes::from_static(br#"{"name":"a"}"#)))
        .unwrap();
    assert_eq!(app().oneshot(bad).await.status(), 422);
}

#[tokio::test]
async fn context_handler_still_works() {
    assert_eq!(app().oneshot(get("/ping")).await.status(), 200);
}
