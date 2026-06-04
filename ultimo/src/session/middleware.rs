//! Session middleware. See the spec's Security section for the threat model.

use super::{Session, SessionConfig, SessionData, SessionStore};
use crate::context::Context;
use crate::cookie::Cookie;
use crate::error::Result;
use crate::middleware::{BoxedMiddleware, Next};
use crate::response::Response;
use base64::Engine;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Generate a 256-bit URL-safe session id. Panics if the OS RNG fails — we must
/// never fall back to weak randomness for a security token.
fn generate_id() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("OS RNG failure generating session id");
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn id_cookie(config: &SessionConfig, id: &str) -> Cookie {
    Cookie::new(config.cookie_name.clone(), id.to_string())
        .http_only(config.http_only)
        .secure(config.secure)
        .same_site(config.same_site)
        .path(config.path.clone())
        .max_age(config.ttl.as_secs() as i64)
}

fn expiry_cookie(config: &SessionConfig) -> Cookie {
    Cookie::new(config.cookie_name.clone(), "")
        .http_only(config.http_only)
        .secure(config.secure)
        .same_site(config.same_site)
        .path(config.path.clone())
        .max_age(0)
}

async fn push_cookie(sink: &Arc<RwLock<Vec<String>>>, cookie: Cookie) {
    if let Ok(s) = cookie.to_set_cookie_string() {
        sink.write().await.push(s);
    }
}

/// Build session middleware over `store`. Register with `app.use_middleware(...)`.
pub fn session<S>(store: S, config: SessionConfig) -> BoxedMiddleware
where
    S: SessionStore + 'static,
{
    let store = Arc::new(store);
    let config = Arc::new(config.validated());

    Arc::new(move |ctx: Context, next: Next| {
        let store = store.clone();
        let config = config.clone();
        Box::pin(async move {
            // Load an existing session ONLY if the cookie id is known to the
            // store. Never adopt a client-supplied id (anti session-fixation).
            let cookie_id = ctx.cookie(&config.cookie_name);
            let (id, data) = match &cookie_id {
                Some(cid) => match store.load(cid).await {
                    Some(d) => (cid.clone(), d),
                    None => (generate_id(), SessionData::new()),
                },
                None => (generate_id(), SessionData::new()),
            };

            // Share the session with the handler; capture the cookie sink so we
            // can write Set-Cookie after `next` consumes `ctx`.
            let session = Session::new(id.clone(), data);
            ctx.set_session(session.clone()).await;
            let cookie_sink = ctx.set_cookies_handle();

            let response: Response = next(ctx).await?;

            // Persist per the security rules.
            if session.is_destroyed() {
                store.destroy(&id).await;
                push_cookie(&cookie_sink, expiry_cookie(&config)).await;
            } else if session.is_dirty() && !session.is_empty().await {
                // Only persist dirty, non-empty sessions (anti unbounded-DoS).
                let final_id = if session.wants_regenerate() {
                    store.destroy(&id).await; // fixation: drop the old entry
                    generate_id()
                } else {
                    id.clone()
                };
                store
                    .store(&final_id, &session.snapshot().await, config.ttl)
                    .await;
                push_cookie(&cookie_sink, id_cookie(&config, &final_id)).await;
            }
            // Empty/untouched session: persist nothing, emit no cookie.

            Ok(response)
        }) as Pin<Box<dyn Future<Output = Result<Response>> + Send>>
    })
}
