//! Integration tests for the JWT auth middleware (feature `jwt`).
#![cfg(feature = "jwt")]

use http_body_util::Full;
use hyper::Request as HyperRequest;
use serde::{Deserialize, Serialize};
use ultimo::auth::jwt::Jwt;
use ultimo::{Context, Ultimo};

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

fn far_future() -> usize {
    4_102_444_800 // ~year 2100
}

fn empty() -> Full<bytes::Bytes> {
    Full::new(bytes::Bytes::new())
}

fn app(jwt: Jwt) -> Ultimo {
    let mut app = Ultimo::new_without_defaults();
    app.use_middleware(jwt.build());
    app.get("/me", |ctx: Context| async move {
        let sub: String = ctx
            .jwt_claims()
            .await
            .and_then(|c| c.get("sub").and_then(|v| v.as_str().map(String::from)))
            .unwrap_or_else(|| "anonymous".to_string());
        ctx.json(serde_json::json!({ "sub": sub })).await
    });
    app
}

#[tokio::test]
async fn valid_token_is_accepted_and_claims_attached() {
    let jwt = Jwt::hs256(b"secret");
    let token = jwt
        .sign(&Claims {
            sub: "ada".into(),
            exp: far_future(),
        })
        .unwrap();

    let req = HyperRequest::builder()
        .uri("/me")
        .header("authorization", format!("Bearer {token}"))
        .body(empty())
        .unwrap();
    let res = app(jwt).oneshot(req).await;
    assert_eq!(res.status(), 200);
}

#[tokio::test]
async fn missing_token_is_rejected_with_401() {
    let jwt = Jwt::hs256(b"secret");
    let req = HyperRequest::builder().uri("/me").body(empty()).unwrap();
    let res = app(jwt).oneshot(req).await;
    assert_eq!(res.status(), 401);
    assert_eq!(
        res.headers()
            .get("www-authenticate")
            .map(|v| v.to_str().unwrap()),
        Some("Bearer")
    );
}

#[tokio::test]
async fn bad_signature_is_rejected_with_401() {
    let signer = Jwt::hs256(b"secret-a");
    let verifier = Jwt::hs256(b"secret-b");
    let token = signer
        .sign(&Claims {
            sub: "ada".into(),
            exp: far_future(),
        })
        .unwrap();

    let req = HyperRequest::builder()
        .uri("/me")
        .header("authorization", format!("Bearer {token}"))
        .body(empty())
        .unwrap();
    let res = app(verifier).oneshot(req).await;
    assert_eq!(res.status(), 401);
}

#[tokio::test]
async fn alg_none_token_is_rejected() {
    // Hand-crafted unsigned token with "alg":"none". jsonwebtoken must reject it
    // because the validation pins HS256.
    // header {"alg":"none","typ":"JWT"} . payload {"sub":"ada","exp":4102444800} . (empty sig)
    let token = "eyJhbGciOiJub25lIiwidHlwIjoiSldUIn0.\
eyJzdWIiOiJhZGEiLCJleHAiOjQxMDI0NDQ4MDB9.";
    let jwt = Jwt::hs256(b"secret");
    let req = HyperRequest::builder()
        .uri("/me")
        .header("authorization", format!("Bearer {token}"))
        .body(empty())
        .unwrap();
    let res = app(jwt).oneshot(req).await;
    assert_eq!(res.status(), 401);
}

#[tokio::test]
async fn optional_mode_passes_through_without_token() {
    let jwt = Jwt::hs256(b"secret").optional();
    let req = HyperRequest::builder().uri("/me").body(empty()).unwrap();
    let res = app(jwt).oneshot(req).await;
    assert_eq!(res.status(), 200); // handler returns "anonymous"
}

#[tokio::test]
async fn cookie_source_reads_token_from_cookie() {
    let jwt = Jwt::hs256(b"secret").from_cookie("token");
    let signed = jwt
        .sign(&Claims {
            sub: "ada".into(),
            exp: far_future(),
        })
        .unwrap();

    let req = HyperRequest::builder()
        .uri("/me")
        .header("cookie", format!("token={signed}"))
        .body(empty())
        .unwrap();
    let res = app(jwt).oneshot(req).await;
    assert_eq!(res.status(), 200);
}
