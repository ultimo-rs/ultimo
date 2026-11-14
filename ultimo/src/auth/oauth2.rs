//! "Sign in with …": the OAuth2 authorization-code flow with PKCE, plus
//! OpenID Connect ID-token verification, ending in a session-backed sign-in.
//!
//! Requires the `oauth2` feature (which enables `oidc` and `session`). Two
//! routes do the whole flow:
//!
//! 1. **Login** ([`OAuth2::login_handler`]) — generates `state`, a PKCE
//!    verifier and (for OIDC) a `nonce`, keeps them in the session, and
//!    redirects the browser to the provider.
//! 2. **Callback** ([`OAuth2::callback_handler`]) — checks `state`, exchanges
//!    the code (with the PKCE verifier) for tokens, identifies the user (a
//!    verified ID token for OIDC providers, the userinfo endpoint otherwise),
//!    rotates the session id and signs the user in.
//!
//! ```rust,no_run
//! use ultimo::prelude::*;
//! use ultimo::auth::oauth2::OAuth2;
//! use ultimo::session::{session, MemoryStore, SessionConfig};
//!
//! # async fn run() -> ultimo::Result<()> {
//! let google = OAuth2::discover(
//!     "https://accounts.google.com",
//!     std::env::var("GOOGLE_CLIENT_ID").unwrap(),
//!     std::env::var("GOOGLE_CLIENT_SECRET").unwrap(),
//!     "https://myapp.example/auth/callback",
//! )
//! .await?;
//!
//! let mut app = Ultimo::new();
//! app.use_middleware(session(MemoryStore::new(), SessionConfig::default()));
//! app.get("/auth/login", google.login_handler());
//! app.get("/auth/callback", google.callback_handler());
//! app.get("/me", |ctx: Context| async move {
//!     match OAuth2::current_user(&ctx).await? {
//!         Some(user) => ctx.json(json!({ "sub": user.subject, "email": user.email })).await,
//!         None => Err(UltimoError::Unauthorized("not signed in".into())),
//!     }
//! });
//! # Ok(())
//! # }
//! ```
//!
//! The session cookie must reach the callback, which arrives as a top-level
//! cross-site redirect from the provider: keep `SameSite=Lax` (the default) —
//! `Strict` drops the cookie there and every callback fails the `state` check.

use crate::auth::jwks::JwksClient;
use crate::context::Context;
use crate::error::{Result, UltimoError};
use crate::response::Response;
use crate::session::Session;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use jsonwebtoken::{Algorithm, Validation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

/// Session key holding the in-flight login (state, PKCE verifier, nonce).
const PENDING_KEY: &str = "oauth2.pending";
/// Session key holding the signed-in [`OAuthUser`].
const USER_KEY: &str = "oauth2.user";

type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;
type LoginHook = Arc<dyn Fn(Context, OAuthLogin) -> BoxFuture<Result<Response>> + Send + Sync>;

/// A provider's OAuth2 endpoints, for providers without OIDC discovery.
/// OIDC providers are easier to set up with [`OAuth2::discover`].
#[derive(Clone, Debug)]
pub struct Provider {
    /// Where the browser is sent to sign in (`authorization_endpoint`).
    pub authorization_url: String,
    /// Where the code is exchanged for tokens (`token_endpoint`).
    pub token_url: String,
    /// Returns the signed-in user's profile for the access token. Required
    /// unless the provider issues OIDC ID tokens.
    pub userinfo_url: Option<String>,
}

/// The signed-in user, as stored in the session.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OAuthUser {
    /// Stable provider-specific user id: the ID token's `sub`, or the
    /// userinfo `sub` / `id`.
    pub subject: String,
    /// Email address, if the provider shared one.
    pub email: Option<String>,
    /// Display name (`name`, falling back to `login`/`preferred_username`).
    pub name: Option<String>,
    /// The ID-token claims (OIDC) or userinfo response it was built from.
    pub claims: serde_json::Value,
}

/// Tokens returned by the provider's token endpoint.
#[derive(Clone, Debug, Deserialize)]
pub struct OAuthTokens {
    /// Bearer token for the provider's APIs.
    pub access_token: String,
    /// Usually `Bearer`.
    #[serde(default)]
    pub token_type: Option<String>,
    /// Present if the provider issued one (often needs an `offline_access`
    /// scope or provider-specific parameter).
    #[serde(default)]
    pub refresh_token: Option<String>,
    /// The raw OIDC ID token, already verified when this is handed to you.
    #[serde(default)]
    pub id_token: Option<String>,
    /// Access-token lifetime in seconds.
    #[serde(default)]
    pub expires_in: Option<u64>,
    /// Scopes actually granted, if the provider reports them.
    #[serde(default)]
    pub scope: Option<String>,
}

/// A completed login, handed to an [`on_login`](OAuth2::on_login) hook.
#[derive(Clone, Debug)]
pub struct OAuthLogin {
    /// Who signed in.
    pub user: OAuthUser,
    /// The provider's tokens. Not stored anywhere by Ultimo.
    pub tokens: OAuthTokens,
    /// Where the login asked to return to (a same-site path, default `/`).
    pub return_to: String,
}

/// What the login route stashes in the session for the callback to check.
#[derive(Serialize, Deserialize)]
struct Pending {
    state: String,
    verifier: String,
    nonce: Option<String>,
    return_to: String,
}

/// OIDC-specific configuration: how to verify ID tokens.
struct Oidc {
    issuer: String,
    jwks: JwksClient,
}

struct Config {
    provider: Provider,
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    scopes: Vec<String>,
    basic_auth: bool,
    oidc: Option<Oidc>,
    on_login: Option<LoginHook>,
    http: reqwest::Client,
}

/// An OAuth2 / OIDC client for one provider. Cheap to clone.
///
/// See the [module docs](self) for the flow and an example.
#[derive(Clone)]
pub struct OAuth2 {
    config: Arc<Config>,
}

impl OAuth2 {
    /// Configure an **OpenID Connect** provider from its issuer URL (fetches
    /// `{issuer}/.well-known/openid-configuration` once). Requests the
    /// `openid email profile` scopes and verifies the returned ID token's
    /// signature (via the provider's JWKS), issuer, audience, expiry and nonce.
    ///
    /// Works with Google, Microsoft Entra ID, Auth0, Okta, Keycloak, Cognito,
    /// GitLab and other OIDC providers. Fails if discovery fails or the
    /// document's `issuer` doesn't match `issuer`.
    pub async fn discover(
        issuer: &str,
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        redirect_uri: impl Into<String>,
    ) -> Result<Self> {
        let http = http_client()?;
        let url = format!(
            "{}/.well-known/openid-configuration",
            issuer.trim_end_matches('/')
        );
        #[derive(Deserialize)]
        struct Discovery {
            issuer: String,
            authorization_endpoint: String,
            token_endpoint: String,
            #[serde(default)]
            userinfo_endpoint: Option<String>,
            jwks_uri: String,
        }
        let doc: Discovery = http
            .get(&url)
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| UltimoError::Internal(format!("OIDC discovery failed: {e}")))?
            .json()
            .await
            .map_err(|e| UltimoError::Internal(format!("OIDC discovery parse failed: {e}")))?;
        // OIDC Discovery §4.3: the document's issuer must be the one we asked.
        if doc.issuer.trim_end_matches('/') != issuer.trim_end_matches('/') {
            return Err(UltimoError::Internal(format!(
                "OIDC discovery issuer mismatch: expected {issuer}, got {}",
                doc.issuer
            )));
        }
        let mut client = Self::build(
            Provider {
                authorization_url: doc.authorization_endpoint,
                token_url: doc.token_endpoint,
                userinfo_url: doc.userinfo_endpoint,
            },
            client_id.into(),
            client_secret.into(),
            redirect_uri.into(),
            vec!["openid".into(), "email".into(), "profile".into()],
            http,
        );
        client.config_mut().oidc = Some(Oidc {
            issuer: doc.issuer,
            jwks: JwksClient::from_url(doc.jwks_uri),
        });
        Ok(client)
    }

    /// Configure a plain **OAuth2** provider (no OIDC). The user is identified
    /// by calling [`Provider::userinfo_url`] with the access token. No scopes
    /// are requested until you set [`scopes`](Self::scopes).
    pub fn new(
        provider: Provider,
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        redirect_uri: impl Into<String>,
    ) -> Self {
        // Building a rustls client only fails if no TLS backend is available,
        // which the `oidc` feature's reqwest configuration rules out.
        let http = http_client().expect("HTTP client");
        Self::build(
            provider,
            client_id.into(),
            client_secret.into(),
            redirect_uri.into(),
            Vec::new(),
            http,
        )
    }

    /// GitHub (OAuth Apps and GitHub Apps), which isn't an OIDC provider.
    /// Requests `read:user user:email`; [`OAuthUser::subject`] is the numeric
    /// GitHub user id and [`OAuthUser::name`] falls back to the login.
    pub fn github(
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        redirect_uri: impl Into<String>,
    ) -> Self {
        Self::new(
            Provider {
                authorization_url: "https://github.com/login/oauth/authorize".into(),
                token_url: "https://github.com/login/oauth/access_token".into(),
                userinfo_url: Some("https://api.github.com/user".into()),
            },
            client_id,
            client_secret,
            redirect_uri,
        )
        .scopes(["read:user", "user:email"])
    }

    fn build(
        provider: Provider,
        client_id: String,
        client_secret: String,
        redirect_uri: String,
        scopes: Vec<String>,
        http: reqwest::Client,
    ) -> Self {
        Self {
            config: Arc::new(Config {
                provider,
                client_id,
                client_secret,
                redirect_uri,
                scopes,
                basic_auth: false,
                oidc: None,
                on_login: None,
                http,
            }),
        }
    }

    /// Only valid while building (before the client is cloned into handlers).
    fn config_mut(&mut self) -> &mut Config {
        Arc::get_mut(&mut self.config).expect("configure OAuth2 before using its handlers")
    }

    /// Replace the requested scopes. For OIDC providers keep `openid` in the
    /// list, or no ID token is issued.
    pub fn scopes<I, S>(mut self, scopes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.config_mut().scopes = scopes.into_iter().map(Into::into).collect();
        self
    }

    /// Send the client credentials as HTTP Basic auth to the token endpoint
    /// (`client_secret_basic`) instead of in the form body
    /// (`client_secret_post`, the default).
    pub fn client_secret_basic(mut self) -> Self {
        self.config_mut().basic_auth = true;
        self
    }

    /// Decide what a successful login does. By default the user is signed in
    /// with [`sign_in`](Self::sign_in) and redirected to
    /// [`OAuthLogin::return_to`]. A hook can instead look the user up in your
    /// database, refuse them (return an error), keep the tokens, or sign them
    /// in under your own session keys. The session id has already been
    /// rotated when the hook runs.
    ///
    /// ```rust,no_run
    /// # use ultimo::prelude::*;
    /// # use ultimo::auth::oauth2::OAuth2;
    /// # fn f(github: OAuth2) -> OAuth2 {
    /// github.on_login(|ctx, login| async move {
    ///     if login.user.email.as_deref().is_some_and(|e| e.ends_with("@example.com")) {
    ///         OAuth2::sign_in(&ctx, &login.user).await?;
    ///         ctx.redirect(&login.return_to).await
    ///     } else {
    ///         Err(UltimoError::Forbidden("example.com accounts only".into()))
    ///     }
    /// })
    /// # }
    /// ```
    pub fn on_login<F, Fut>(mut self, hook: F) -> Self
    where
        F: Fn(Context, OAuthLogin) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Response>> + Send + 'static,
    {
        self.config_mut().on_login = Some(Arc::new(move |ctx, login| Box::pin(hook(ctx, login))));
        self
    }

    /// The signed-in user, or `None` if nobody signed in through
    /// [`sign_in`](Self::sign_in) (the default login behaviour).
    pub async fn current_user(ctx: &Context) -> Result<Option<OAuthUser>> {
        match ctx.session_opt().await {
            Some(session) => session.get(USER_KEY).await,
            None => Ok(None),
        }
    }

    /// Store `user` in the session so [`current_user`](Self::current_user)
    /// returns it. The callback has already rotated the session id; call this
    /// from an [`on_login`](Self::on_login) hook to keep the default sign-in.
    /// Sign out with `ctx.session().await.destroy()`.
    pub async fn sign_in(ctx: &Context, user: &OAuthUser) -> Result<()> {
        session(ctx).await?.set(USER_KEY, user).await
    }

    /// Route handler that starts a login. Mount it on a `GET` route. An
    /// optional `?return_to=/path` (a same-site path only) is where the user
    /// lands afterwards.
    pub fn login_handler(
        &self,
    ) -> impl Fn(Context) -> BoxFuture<Result<Response>> + Clone + Send + Sync + 'static {
        let client = self.clone();
        move |ctx: Context| {
            let client = client.clone();
            Box::pin(async move { client.login(ctx).await })
        }
    }

    /// Route handler for the provider's redirect back. Mount it on a `GET`
    /// route whose URL is exactly the `redirect_uri` registered with the
    /// provider.
    pub fn callback_handler(
        &self,
    ) -> impl Fn(Context) -> BoxFuture<Result<Response>> + Clone + Send + Sync + 'static {
        let client = self.clone();
        move |ctx: Context| {
            let client = client.clone();
            Box::pin(async move { client.callback(ctx).await })
        }
    }

    async fn login(&self, ctx: Context) -> Result<Response> {
        let cfg = &self.config;
        let session = session(&ctx).await?;
        let pending = Pending {
            state: random_token(),
            verifier: random_token(),
            nonce: cfg.oidc.as_ref().map(|_| random_token()),
            return_to: safe_return_to(ctx.req.query("return_to").as_deref()),
        };
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(pending.verifier.as_bytes()));
        let scope = cfg.scopes.join(" ");

        let mut params: Vec<(&str, &str)> = vec![
            ("response_type", "code"),
            ("client_id", &cfg.client_id),
            ("redirect_uri", &cfg.redirect_uri),
            ("state", &pending.state),
            ("code_challenge", &challenge),
            ("code_challenge_method", "S256"),
        ];
        if !scope.is_empty() {
            params.push(("scope", &scope));
        }
        if let Some(nonce) = &pending.nonce {
            params.push(("nonce", nonce));
        }
        let query = serde_urlencoded::to_string(&params)
            .map_err(|e| UltimoError::Internal(format!("encode authorize URL: {e}")))?;
        let sep = if cfg.provider.authorization_url.contains('?') {
            '&'
        } else {
            '?'
        };
        let location = format!("{}{sep}{query}", cfg.provider.authorization_url);

        session.set(PENDING_KEY, &pending).await?;
        ctx.redirect(&location).await
    }

    async fn callback(&self, ctx: Context) -> Result<Response> {
        let cfg = &self.config;
        let session = session(&ctx).await?;
        // Single use: whatever happens next, this login attempt is consumed.
        let pending: Option<Pending> = session.get(PENDING_KEY).await?;
        session.remove(PENDING_KEY).await;

        if let Some(error) = ctx.req.query("error") {
            return Err(UltimoError::Unauthorized(format!(
                "sign-in was not completed: {}",
                percent_decode(&error)
            )));
        }
        let pending = pending.ok_or_else(|| {
            UltimoError::BadRequest(
                "no sign-in in progress (start again from the login page)".into(),
            )
        })?;
        let state = ctx.req.query("state").map(|s| percent_decode(&s));
        if !state.is_some_and(|s| constant_time_eq(s.as_bytes(), pending.state.as_bytes())) {
            return Err(UltimoError::BadRequest("sign-in state mismatch".into()));
        }
        let code = ctx
            .req
            .query("code")
            .map(|c| percent_decode(&c))
            .ok_or_else(|| UltimoError::BadRequest("missing authorization code".into()))?;

        let tokens = self.exchange(&code, &pending.verifier).await?;
        let user = match &cfg.oidc {
            Some(oidc) => {
                self.verify_id_token(oidc, &tokens, pending.nonce.as_deref())
                    .await?
            }
            None => self.userinfo(&tokens.access_token).await?,
        };

        // Session-fixation defense: a new session id for the signed-in user.
        session.regenerate();
        let login = OAuthLogin {
            user,
            tokens,
            return_to: pending.return_to,
        };
        match &cfg.on_login {
            Some(hook) => hook(ctx, login).await,
            None => {
                Self::sign_in(&ctx, &login.user).await?;
                ctx.redirect(&login.return_to).await
            }
        }
    }

    async fn exchange(&self, code: &str, verifier: &str) -> Result<OAuthTokens> {
        let cfg = &self.config;
        let mut form = vec![
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", cfg.redirect_uri.as_str()),
            ("code_verifier", verifier),
        ];
        let mut request = cfg
            .http
            .post(&cfg.provider.token_url)
            .header("Accept", "application/json");
        if cfg.basic_auth {
            request = request.basic_auth(&cfg.client_id, Some(&cfg.client_secret));
        } else {
            form.push(("client_id", cfg.client_id.as_str()));
            form.push(("client_secret", cfg.client_secret.as_str()));
        }
        let response = request
            .form(&form)
            .send()
            .await
            .map_err(|e| UltimoError::Internal(format!("token request failed: {e}")))?;
        let status = response.status();
        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| UltimoError::Internal(format!("token response is not JSON: {e}")))?;
        // Some providers (GitHub) report errors with a 200 status.
        if let Some(error) = body.get("error").and_then(|e| e.as_str()) {
            return Err(UltimoError::Unauthorized(format!(
                "token exchange rejected: {error}"
            )));
        }
        if !status.is_success() {
            return Err(UltimoError::Internal(format!(
                "token endpoint returned {status}"
            )));
        }
        serde_json::from_value(body)
            .map_err(|e| UltimoError::Internal(format!("unexpected token response: {e}")))
    }

    async fn verify_id_token(
        &self,
        oidc: &Oidc,
        tokens: &OAuthTokens,
        nonce: Option<&str>,
    ) -> Result<OAuthUser> {
        let id_token = tokens.id_token.as_deref().ok_or_else(|| {
            UltimoError::Unauthorized(
                "provider returned no ID token (is `openid` in the scopes?)".into(),
            )
        })?;
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[&oidc.issuer]);
        validation.set_audience(&[&self.config.client_id]);
        validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
        validation.leeway = 60;
        let claims = crate::auth::jwt::verify_with_jwks(&oidc.jwks, id_token, &validation).await?;

        let token_nonce = claims.get("nonce").and_then(|n| n.as_str());
        let nonce_ok = match (nonce, token_nonce) {
            (Some(expected), Some(got)) => constant_time_eq(expected.as_bytes(), got.as_bytes()),
            _ => false,
        };
        if !nonce_ok {
            return Err(UltimoError::Unauthorized("ID token nonce mismatch".into()));
        }
        user_from_claims(claims)
    }

    async fn userinfo(&self, access_token: &str) -> Result<OAuthUser> {
        let url = self
            .config
            .provider
            .userinfo_url
            .as_deref()
            .ok_or_else(|| {
                UltimoError::Internal("provider has no userinfo_url to identify the user".into())
            })?;
        let claims: serde_json::Value = self
            .config
            .http
            .get(url)
            .bearer_auth(access_token)
            .header("Accept", "application/json")
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| UltimoError::Internal(format!("userinfo request failed: {e}")))?
            .json()
            .await
            .map_err(|e| UltimoError::Internal(format!("userinfo is not JSON: {e}")))?;
        user_from_claims(claims)
    }
}

/// The session for `ctx`, as a clear error if the middleware is missing.
async fn session(ctx: &Context) -> Result<Session> {
    ctx.session_opt().await.ok_or_else(|| {
        UltimoError::Internal(
            "OAuth2 needs the session middleware: add `session(store, config)`".into(),
        )
    })
}

fn http_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        // GitHub's API rejects requests without a User-Agent.
        .user_agent(concat!("ultimo/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| UltimoError::Internal(format!("HTTP client: {e}")))
}

fn user_from_claims(claims: serde_json::Value) -> Result<OAuthUser> {
    let text = |key: &str| claims.get(key).and_then(|v| v.as_str()).map(str::to_string);
    let subject = match claims.get("sub").or_else(|| claims.get("id")) {
        Some(serde_json::Value::String(s)) if !s.is_empty() => s.clone(),
        Some(serde_json::Value::Number(n)) => n.to_string(),
        _ => {
            return Err(UltimoError::Unauthorized(
                "provider response has no user id (`sub` or `id`)".into(),
            ))
        }
    };
    Ok(OAuthUser {
        subject,
        email: text("email"),
        name: text("name")
            .or_else(|| text("preferred_username"))
            .or_else(|| text("login")),
        claims,
    })
}

/// 32 random bytes, base64url: used for `state`, `nonce` and the PKCE verifier
/// (43 chars, within RFC 7636's 43–128).
fn random_token() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("OS RNG failure");
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Only same-site absolute paths are allowed as `return_to`, so the login
/// route can't be used as an open redirect (`//evil.com`, `/\evil.com`,
/// `https://evil.com` all fall back to `/`).
fn safe_return_to(raw: Option<&str>) -> String {
    let Some(path) = raw.map(percent_decode) else {
        return "/".into();
    };
    let ok = path.starts_with('/')
        && !path.starts_with("//")
        && !path.contains('\\')
        && !path.chars().any(|c| c.is_control());
    if ok {
        path
    } else {
        "/".into()
    }
}

/// Decode `%XX` escapes and `+` from a query value (`Request::query` returns
/// it raw).
fn percent_decode(raw: &str) -> String {
    serde_urlencoded::from_str::<Vec<(String, String)>>(&format!("v={raw}"))
        .ok()
        .and_then(|mut pairs| pairs.pop())
        .map(|(_, v)| v)
        .unwrap_or_else(|| raw.to_string())
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn return_to_only_allows_same_site_paths() {
        assert_eq!(safe_return_to(None), "/");
        assert_eq!(safe_return_to(Some("/dashboard?tab=1")), "/dashboard?tab=1");
        assert_eq!(safe_return_to(Some("%2Fsettings")), "/settings");
        for evil in [
            "https://evil.com",
            "//evil.com",
            "/\\evil.com",
            "%2F%2Fevil.com",
            "evil.com",
            "/a%0Ab",
        ] {
            assert_eq!(safe_return_to(Some(evil)), "/", "{evil}");
        }
    }

    #[test]
    fn subject_from_sub_or_numeric_id() {
        let oidc =
            user_from_claims(serde_json::json!({ "sub": "abc", "email": "a@x.io" })).unwrap();
        assert_eq!(oidc.subject, "abc");
        assert_eq!(oidc.email.as_deref(), Some("a@x.io"));

        let github =
            user_from_claims(serde_json::json!({ "id": 583231, "login": "octocat" })).unwrap();
        assert_eq!(github.subject, "583231");
        assert_eq!(github.name.as_deref(), Some("octocat"));

        assert!(user_from_claims(serde_json::json!({ "login": "x" })).is_err());
        assert!(user_from_claims(serde_json::json!({ "sub": "" })).is_err());
    }

    #[test]
    fn pkce_challenge_matches_rfc7636_example() {
        // RFC 7636 Appendix B.
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        assert_eq!(challenge, "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }

    #[test]
    fn random_tokens_are_long_and_distinct() {
        let (a, b) = (random_token(), random_token());
        assert_eq!(a.len(), 43);
        assert_ne!(a, b);
    }

    #[test]
    fn constant_time_eq_works() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"ab"));
    }
}
