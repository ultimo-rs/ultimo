//! Integration tests for OIDC/JWKS verification (offline — keys are generated
//! in-test and injected via `Jwt::jwks_from_set`).
//! Run with: cargo test -p ultimo --features oidc --test oidc

#![cfg(feature = "oidc")]

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use bytes::Bytes;
use http_body_util::Full;
use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use rsa::pkcs1::EncodeRsaPrivateKey;
use rsa::traits::PublicKeyParts;
use rsa::RsaPrivateKey;
use serde::Serialize;
use ultimo::auth::jwt::Jwt;
use ultimo::prelude::*;

#[derive(Serialize)]
struct Claims {
    sub: String,
    exp: usize,
    iss: String,
    aud: String,
}

struct Signer {
    pem: String,
    set: JwkSet,
    kid: String,
}

fn signer(kid: &str) -> Signer {
    let mut rng = rand::thread_rng();
    let sk = RsaPrivateKey::new(&mut rng, 2048).unwrap();
    let pem = sk
        .to_pkcs1_pem(rsa::pkcs1::LineEnding::LF)
        .unwrap()
        .to_string();
    let n = URL_SAFE_NO_PAD.encode(sk.n().to_bytes_be());
    let e = URL_SAFE_NO_PAD.encode(sk.e().to_bytes_be());
    let set = serde_json::from_value(serde_json::json!({
        "keys": [{ "kty": "RSA", "use": "sig", "alg": "RS256", "kid": kid, "n": n, "e": e }]
    }))
    .unwrap();
    Signer {
        pem,
        set,
        kid: kid.to_string(),
    }
}

impl Signer {
    fn token(&self, claims: &Claims) -> String {
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(self.kid.clone());
        let enc = EncodingKey::from_rsa_pem(self.pem.as_bytes()).unwrap();
        encode(&header, claims, &enc).unwrap()
    }
}

fn app(set: JwkSet) -> Ultimo {
    let mut app = Ultimo::new_without_defaults();
    app.use_middleware(
        Jwt::jwks_from_set(set)
            .issuer("https://issuer.example")
            .audience("my-api")
            .build(),
    );
    app.get("/me", |ctx: Context| async move {
        match ctx.jwt_claims().await {
            Some(_) => ultimo::response::helpers::text("ok"),
            None => ultimo::response::helpers::text("anon"),
        }
    });
    app
}

fn bearer(token: &str) -> hyper::Request<Full<Bytes>> {
    hyper::Request::builder()
        .uri("/me")
        .header("authorization", format!("Bearer {token}"))
        .body(Full::new(Bytes::new()))
        .unwrap()
}

fn good_claims() -> Claims {
    Claims {
        sub: "u1".into(),
        exp: 4102444800,
        iss: "https://issuer.example".into(),
        aud: "my-api".into(),
    }
}

#[tokio::test]
async fn valid_rs256_token_is_accepted() {
    let s = signer("k1");
    let token = s.token(&good_claims());
    let res = app(s.set).oneshot(bearer(&token)).await;
    assert_eq!(res.status(), 200);
}

#[tokio::test]
async fn token_signed_by_unknown_key_is_401() {
    let real = signer("k1");
    let attacker = signer("k2"); // different key + kid
    let token = attacker.token(&good_claims());
    let res = app(real.set).oneshot(bearer(&token)).await;
    assert_eq!(res.status(), 401);
}

#[tokio::test]
async fn wrong_audience_is_401() {
    let s = signer("k1");
    let mut c = good_claims();
    c.aud = "other-api".into();
    let token = s.token(&c);
    let res = app(s.set).oneshot(bearer(&token)).await;
    assert_eq!(res.status(), 401);
}

#[tokio::test]
async fn missing_token_is_401() {
    let s = signer("k1");
    let res = app(s.set)
        .oneshot(
            hyper::Request::builder()
                .uri("/me")
                .body(Full::new(Bytes::new()))
                .unwrap(),
        )
        .await;
    assert_eq!(res.status(), 401);
}
