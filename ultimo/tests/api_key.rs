//! Integration tests for the API-key auth middleware (feature `api-key`).
#![cfg(feature = "api-key")]

use http_body_util::Full;
use hyper::Request as HyperRequest;
use ultimo::auth::api_key::{ApiKey, StaticKeys};
use ultimo::{Context, Ultimo};

fn empty() -> Full<bytes::Bytes> {
    Full::new(bytes::Bytes::new())
}

fn store() -> StaticKeys {
    StaticKeys::new()
        .insert("key-abc", "service-a")
        .with_scopes("key-def", "service-b", ["read", "write"])
}

fn app(api: ApiKey<StaticKeys>) -> Ultimo {
    let mut app = Ultimo::new_without_defaults();
    app.use_middleware(api.build());
    app.get("/me", |ctx: Context| async move {
        match ctx.api_key().await {
            Some(id) => {
                ctx.json(serde_json::json!({ "id": id.id, "scopes": id.scopes }))
                    .await
            }
            None => ctx.json(serde_json::json!({ "id": null })).await,
        }
    });
    app
}

#[tokio::test]
async fn valid_key_is_accepted_and_identity_attached() {
    let req = HyperRequest::builder()
        .uri("/me")
        .header("x-api-key", "key-def")
        .body(empty())
        .unwrap();
    let res = app(ApiKey::new(store())).oneshot(req).await;
    assert_eq!(res.status(), 200);
    let body = collect(res).await;
    assert!(body.contains("service-b"));
    assert!(body.contains("read"));
    assert!(body.contains("write"));
}

#[tokio::test]
async fn invalid_key_is_rejected_with_401() {
    let req = HyperRequest::builder()
        .uri("/me")
        .header("x-api-key", "wrong")
        .body(empty())
        .unwrap();
    let res = app(ApiKey::new(store())).oneshot(req).await;
    assert_eq!(res.status(), 401);
}

#[tokio::test]
async fn missing_key_is_rejected_with_401() {
    let req = HyperRequest::builder().uri("/me").body(empty()).unwrap();
    let res = app(ApiKey::new(store())).oneshot(req).await;
    assert_eq!(res.status(), 401);
}

#[tokio::test]
async fn optional_mode_passes_through_without_key() {
    let req = HyperRequest::builder().uri("/me").body(empty()).unwrap();
    let res = app(ApiKey::new(store()).optional()).oneshot(req).await;
    assert_eq!(res.status(), 200); // handler sees no identity
}

#[tokio::test]
async fn query_source_reads_key_from_query_param() {
    let req = HyperRequest::builder()
        .uri("/me?api_key=key-abc")
        .body(empty())
        .unwrap();
    let res = app(ApiKey::new(store()).from_query("api_key"))
        .oneshot(req)
        .await;
    assert_eq!(res.status(), 200);
    assert!(collect(res).await.contains("service-a"));
}

#[tokio::test]
async fn custom_header_name_is_honored() {
    let req = HyperRequest::builder()
        .uri("/me")
        .header("authorization", "key-abc")
        .body(empty())
        .unwrap();
    let res = app(ApiKey::new(store()).header_name("authorization"))
        .oneshot(req)
        .await;
    assert_eq!(res.status(), 200);
}

#[tokio::test]
async fn guarded_route_uses_api_key_scopes() {
    // A route guarded by require_scope, behind the api-key middleware. The
    // identity's scopes flow into the Principal the guard reads.
    fn guarded() -> Ultimo {
        let mut app = Ultimo::new_without_defaults();
        app.use_middleware(ApiKey::new(store()).build());
        app.get("/admin", |ctx: Context| async move {
            ctx.require_scope("write").await?;
            ctx.json(serde_json::json!({ "ok": true })).await
        });
        app
    }

    // key-def has scopes [read, write] → allowed.
    let req = HyperRequest::builder()
        .uri("/admin")
        .header("x-api-key", "key-def")
        .body(empty())
        .unwrap();
    assert_eq!(guarded().oneshot(req).await.status(), 200);

    // key-abc has no scopes → 403.
    let req = HyperRequest::builder()
        .uri("/admin")
        .header("x-api-key", "key-abc")
        .body(empty())
        .unwrap();
    assert_eq!(guarded().oneshot(req).await.status(), 403);
}

async fn collect(res: ultimo::response::Response) -> String {
    use http_body_util::BodyExt;
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}
