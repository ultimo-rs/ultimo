//! OAuth2 / OIDC login flow against a fake provider served on a local port
//! (the client makes real HTTP calls for discovery, token exchange, JWKS and
//! userinfo). Run with:
//! `cargo test -p ultimo --features "oauth2,testing" --test oauth2`

#![cfg(all(feature = "oauth2", feature = "testing"))]

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use rsa::pkcs1::EncodeRsaPrivateKey;
use rsa::traits::PublicKeyParts;
use rsa::RsaPrivateKey;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use ultimo::auth::oauth2::{OAuth2, Provider};
use ultimo::prelude::*;
use ultimo::session::{session, MemoryStore, SessionConfig};
use ultimo::testing::{TestClient, TestResponse};

const CLIENT_ID: &str = "my-client";
const CLIENT_SECRET: &str = "my-secret";
const REDIRECT_URI: &str = "http://app.test/auth/callback";

/// RSA key generation is slow in debug builds: do it once for all tests.
///
/// [`fake_provider`] forces this before it starts serving. Never let the first
/// call happen inside a request handler: in a debug build a 2048-bit keygen
/// takes anywhere from ~1 s to well over 10 s (prime search is random), and
/// while it runs every other test's `/token` request waits on the `OnceLock`
/// — long enough to trip the OAuth2 client's 10 s HTTP timeout, which fails
/// all of them at once with "error sending request".
fn signing_key() -> &'static (String, serde_json::Value) {
    static KEY: OnceLock<(String, serde_json::Value)> = OnceLock::new();
    KEY.get_or_init(|| {
        let sk = RsaPrivateKey::new(&mut rand::thread_rng(), 2048).unwrap();
        let pem = sk
            .to_pkcs1_pem(rsa::pkcs1::LineEnding::LF)
            .unwrap()
            .to_string();
        let jwks = serde_json::json!({ "keys": [{
            "kty": "RSA", "use": "sig", "alg": "RS256", "kid": "k1",
            "n": URL_SAFE_NO_PAD.encode(sk.n().to_bytes_be()),
            "e": URL_SAFE_NO_PAD.encode(sk.e().to_bytes_be()),
        }]});
        (pem, jwks)
    })
}

/// What the fake provider expects at its token endpoint. The test fills it in
/// from the authorize URL, playing the part of the user approving the login.
#[derive(Clone, Default)]
struct Grant {
    code: String,
    challenge: String,
    nonce: Option<String>,
    /// Put a different nonce in the ID token (replay simulation).
    wrong_nonce: bool,
}

#[derive(Clone)]
struct Fake {
    issuer: String,
    grant: Arc<Mutex<Grant>>,
}

/// Start the fake provider on 127.0.0.1:<random>.
async fn fake_provider() -> Fake {
    // Generate the key now, before any HTTP request (and its timeout) is in
    // flight — see `signing_key`.
    signing_key();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let issuer = format!("http://{}", listener.local_addr().unwrap());
    let fake = Fake {
        issuer: issuer.clone(),
        grant: Arc::new(Mutex::new(Grant::default())),
    };

    let mut app = Ultimo::new_without_defaults();
    let iss = issuer.clone();
    app.get("/.well-known/openid-configuration", move |ctx: Context| {
        let iss = iss.clone();
        async move {
            ctx.json(json!({
                "issuer": iss,
                "authorization_endpoint": format!("{iss}/authorize"),
                "token_endpoint": format!("{iss}/token"),
                "userinfo_endpoint": format!("{iss}/userinfo"),
                "jwks_uri": format!("{iss}/jwks"),
            }))
            .await
        }
    });
    app.get("/jwks", |ctx: Context| async move {
        ctx.json(signing_key().1.clone()).await
    });
    app.get("/userinfo", |ctx: Context| async move {
        if ctx.req.header("authorization").as_deref() != Some("Bearer access-123") {
            ctx.status(401).await;
            return ctx.json(json!({ "message": "bad token" })).await;
        }
        ctx.json(json!({ "id": 583231, "login": "octocat", "email": "octo@github.test" }))
            .await
    });
    let (grant, iss) = (fake.grant.clone(), issuer.clone());
    app.post("/token", move |ctx: Context| {
        let (grant, iss) = (grant.clone(), iss.clone());
        async move {
            let form: HashMap<String, String> =
                serde_urlencoded::from_str(&ctx.req.text().await?).unwrap();
            let g = grant.lock().unwrap().clone();
            let verifier = form.get("code_verifier").cloned().unwrap_or_default();
            let pkce_ok =
                URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())) == g.challenge;
            let ok = form.get("grant_type").map(String::as_str) == Some("authorization_code")
                && form.get("code") == Some(&g.code)
                && form.get("redirect_uri").map(String::as_str) == Some(REDIRECT_URI)
                && form.get("client_id").map(String::as_str) == Some(CLIENT_ID)
                && form.get("client_secret").map(String::as_str) == Some(CLIENT_SECRET)
                && pkce_ok;
            if !ok {
                ctx.status(400).await;
                return ctx.json(json!({ "error": "invalid_grant" })).await;
            }
            let nonce = if g.wrong_nonce {
                Some("someone-elses-nonce".to_string())
            } else {
                g.nonce
            };
            let mut header = Header::new(Algorithm::RS256);
            header.kid = Some("k1".into());
            let id_token = encode(
                &header,
                &json!({
                    "iss": iss, "aud": CLIENT_ID, "sub": "user-42",
                    "email": "ada@example.com", "name": "Ada",
                    "exp": 4_102_444_800u64, "nonce": nonce,
                }),
                &EncodingKey::from_rsa_pem(signing_key().0.as_bytes()).unwrap(),
            )
            .unwrap();
            ctx.json(json!({
                "access_token": "access-123", "token_type": "Bearer",
                "id_token": id_token, "expires_in": 3600,
            }))
            .await
        }
    });
    tokio::spawn(app.serve_with_shutdown(listener, std::future::pending()));
    fake
}

fn app(oauth: OAuth2) -> TestClient {
    let mut app = Ultimo::new_without_defaults();
    app.use_middleware(session(
        MemoryStore::new(),
        SessionConfig::default().secure(false),
    ));
    app.get("/auth/login", oauth.login_handler());
    app.get("/auth/callback", oauth.callback_handler());
    app.get("/me", |ctx: Context| async move {
        let user = OAuth2::current_user(&ctx).await?;
        ctx.json(json!({
            "sub": user.as_ref().map(|u| u.subject.clone()),
            "email": user.as_ref().and_then(|u| u.email.clone()),
            "name": user.as_ref().and_then(|u| u.name.clone()),
        }))
        .await
    });
    TestClient::new(app)
}

fn cookie(res: &TestResponse) -> String {
    res.header("set-cookie")
        .expect("Set-Cookie")
        .split(';')
        .next()
        .unwrap()
        .to_string()
}

/// Start a login; returns the session cookie and the authorize URL's params.
async fn start_login(client: &TestClient, path: &str) -> (String, HashMap<String, String>) {
    let res = client.get(path).send().await;
    assert_eq!(res.status(), 302);
    let location = res.header("location").unwrap().to_string();
    let query = location.split_once('?').unwrap().1;
    (cookie(&res), serde_urlencoded::from_str(query).unwrap())
}

/// The user approves: the provider will accept `code` for this login.
fn approve(fake: &Fake, params: &HashMap<String, String>, code: &str) {
    *fake.grant.lock().unwrap() = Grant {
        code: code.into(),
        challenge: params["code_challenge"].clone(),
        nonce: params.get("nonce").cloned(),
        wrong_nonce: false,
    };
}

async fn oidc(fake: &Fake) -> OAuth2 {
    OAuth2::discover(&fake.issuer, CLIENT_ID, CLIENT_SECRET, REDIRECT_URI)
        .await
        .unwrap()
}

#[tokio::test]
async fn oidc_login_signs_the_user_in() {
    let fake = fake_provider().await;
    let client = app(oidc(&fake).await);

    let (pre_login, params) = start_login(&client, "/auth/login?return_to=/dashboard").await;
    assert_eq!(params["response_type"], "code");
    assert_eq!(params["client_id"], CLIENT_ID);
    assert_eq!(params["redirect_uri"], REDIRECT_URI);
    assert_eq!(params["scope"], "openid email profile");
    assert_eq!(params["code_challenge_method"], "S256");
    assert!(params.contains_key("nonce"));

    approve(&fake, &params, "code-1");
    let res = client
        .get(&format!(
            "/auth/callback?code=code-1&state={}",
            params["state"]
        ))
        .header("cookie", &pre_login)
        .send()
        .await;
    assert_eq!(res.status(), 302, "{}", res.text());
    assert_eq!(res.header("location"), Some("/dashboard"));
    let signed_in = cookie(&res);
    assert_ne!(signed_in, pre_login, "session id must rotate on login");

    let me = client.get("/me").header("cookie", &signed_in).send().await;
    assert_eq!(
        me.json::<serde_json::Value>(),
        json!({ "sub": "user-42", "email": "ada@example.com", "name": "Ada" })
    );
    // The pre-login session id is dead.
    let old = client.get("/me").header("cookie", &pre_login).send().await;
    assert_eq!(
        old.json::<serde_json::Value>()["sub"],
        serde_json::Value::Null
    );
}

#[tokio::test]
async fn state_mismatch_is_rejected() {
    let fake = fake_provider().await;
    let client = app(oidc(&fake).await);

    let (cookie, params) = start_login(&client, "/auth/login").await;
    approve(&fake, &params, "code-1");
    let res = client
        .get("/auth/callback?code=code-1&state=forged")
        .header("cookie", &cookie)
        .send()
        .await;
    assert_eq!(res.status(), 400);
}

#[tokio::test]
async fn callback_without_a_login_in_progress_is_rejected() {
    let fake = fake_provider().await;
    let client = app(oidc(&fake).await);
    let res = client.get("/auth/callback?code=x&state=y").send().await;
    assert_eq!(res.status(), 400);
}

#[tokio::test]
async fn a_login_attempt_can_only_be_used_once() {
    let fake = fake_provider().await;
    let client = app(oidc(&fake).await);

    let (cookie, params) = start_login(&client, "/auth/login").await;
    approve(&fake, &params, "code-1");
    let callback = format!("/auth/callback?code=code-1&state={}", params["state"]);
    let first = client.get(&callback).header("cookie", &cookie).send().await;
    assert_eq!(first.status(), 302);
    // Replaying the same callback with the pre-login session fails.
    let replay = client.get(&callback).header("cookie", &cookie).send().await;
    assert_eq!(replay.status(), 400);
}

#[tokio::test]
async fn wrong_id_token_nonce_is_rejected() {
    let fake = fake_provider().await;
    let client = app(oidc(&fake).await);

    let (cookie, params) = start_login(&client, "/auth/login").await;
    approve(&fake, &params, "code-1");
    fake.grant.lock().unwrap().wrong_nonce = true;
    let res = client
        .get(&format!(
            "/auth/callback?code=code-1&state={}",
            params["state"]
        ))
        .header("cookie", &cookie)
        .send()
        .await;
    assert_eq!(res.status(), 401);
}

#[tokio::test]
async fn wrong_pkce_verifier_fails_the_token_exchange() {
    let fake = fake_provider().await;
    let client = app(oidc(&fake).await);

    let (cookie, params) = start_login(&client, "/auth/login").await;
    approve(&fake, &params, "code-1");
    // The provider saw a different challenge than this login's verifier.
    fake.grant.lock().unwrap().challenge = "not-the-challenge".into();
    let res = client
        .get(&format!(
            "/auth/callback?code=code-1&state={}",
            params["state"]
        ))
        .header("cookie", &cookie)
        .send()
        .await;
    assert_eq!(res.status(), 401);
}

#[tokio::test]
async fn provider_error_is_reported() {
    let fake = fake_provider().await;
    let client = app(oidc(&fake).await);

    let (cookie, params) = start_login(&client, "/auth/login").await;
    let res = client
        .get(&format!(
            "/auth/callback?error=access_denied&state={}",
            params["state"]
        ))
        .header("cookie", &cookie)
        .send()
        .await;
    assert_eq!(res.status(), 401);
    assert!(res.text().contains("access_denied"), "{}", res.text());
}

#[tokio::test]
async fn return_to_cannot_leave_the_site() {
    let fake = fake_provider().await;
    let client = app(oidc(&fake).await);

    for evil in [
        "https://evil.example",
        "//evil.example",
        "%2F%2Fevil.example",
    ] {
        let (cookie, params) = start_login(&client, &format!("/auth/login?return_to={evil}")).await;
        approve(&fake, &params, "code-1");
        let res = client
            .get(&format!(
                "/auth/callback?code=code-1&state={}",
                params["state"]
            ))
            .header("cookie", &cookie)
            .send()
            .await;
        assert_eq!(res.header("location"), Some("/"), "{evil}");
    }
}

#[tokio::test]
async fn plain_oauth2_identifies_the_user_via_userinfo() {
    let fake = fake_provider().await;
    let github_like = OAuth2::new(
        Provider {
            authorization_url: format!("{}/authorize?prompt=consent", fake.issuer),
            token_url: format!("{}/token", fake.issuer),
            userinfo_url: Some(format!("{}/userinfo", fake.issuer)),
        },
        CLIENT_ID,
        CLIENT_SECRET,
        REDIRECT_URI,
    )
    .scopes(["read:user"]);
    let client = app(github_like);

    let (cookie_pre, params) = start_login(&client, "/auth/login").await;
    // Existing query on the authorize URL is kept; no OIDC nonce.
    assert_eq!(params["prompt"], "consent");
    assert_eq!(params["scope"], "read:user");
    assert!(!params.contains_key("nonce"));

    approve(&fake, &params, "code-9");
    let res = client
        .get(&format!(
            "/auth/callback?code=code-9&state={}",
            params["state"]
        ))
        .header("cookie", &cookie_pre)
        .send()
        .await;
    assert_eq!(res.status(), 302, "{}", res.text());
    let me = client
        .get("/me")
        .header("cookie", &cookie(&res))
        .send()
        .await;
    assert_eq!(
        me.json::<serde_json::Value>(),
        json!({ "sub": "583231", "email": "octo@github.test", "name": "octocat" })
    );
}

#[tokio::test]
async fn on_login_hook_can_refuse_a_user() {
    let fake = fake_provider().await;
    let oauth = oidc(&fake).await.on_login(|_ctx, login| async move {
        Err(UltimoError::Forbidden(format!(
            "{} is not allowed",
            login.user.email.unwrap_or_default()
        )))
    });
    let client = app(oauth);

    let (cookie, params) = start_login(&client, "/auth/login").await;
    approve(&fake, &params, "code-1");
    let res = client
        .get(&format!(
            "/auth/callback?code=code-1&state={}",
            params["state"]
        ))
        .header("cookie", &cookie)
        .send()
        .await;
    assert_eq!(res.status(), 403);
    assert!(res.text().contains("ada@example.com"));
}

#[tokio::test]
async fn discovery_rejects_a_mismatched_issuer() {
    let fake = fake_provider().await;
    // Same server, but we claim a different issuer than the document reports.
    let wrong = format!("{}/", fake.issuer.replace("127.0.0.1", "localhost"));
    let result = OAuth2::discover(&wrong, CLIENT_ID, CLIENT_SECRET, REDIRECT_URI).await;
    assert!(result.is_err());
}
