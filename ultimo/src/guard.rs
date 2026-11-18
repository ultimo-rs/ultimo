//! Request guards: allow or block requests with your own rules, before they
//! reach a handler.
//!
//! - [`Guard`] runs a predicate you write — on IP, path, user agent, headers,
//!   anything on the [`Context`] — and allows the request, rejects it with
//!   `403`, or answers with a response of your choosing. A lightweight in-app
//!   "WAF-lite" hook for custom rules.
//! - [`GeoFilter`] allows or blocks countries. The country comes from a CDN
//!   header (e.g. Cloudflare's `CF-IPCountry`) or from a lookup function you
//!   supply (e.g. a MaxMind database). Ultimo bundles no GeoIP data.
//!
//! All three types are also re-exported from
//! [`middleware::builtin`](crate::middleware::builtin), next to
//! [`IpFilter`](crate::middleware::builtin::IpFilter).
//!
//! These are primitives for app-level rules. A full WAF, DDoS protection and
//! bot management belong at the edge (Cloudflare, AWS WAF, …), in front of the
//! app.
//!
//! ```rust
//! use ultimo::guard::{Decision, GeoFilter, Guard};
//! use ultimo::prelude::*;
//!
//! let mut app = Ultimo::new();
//! app.trust_proxy(true); // behind Cloudflare: trust its forwarding headers
//!
//! // Block well-known scanners by user agent.
//! app.use_middleware(
//!     Guard::new(|ctx: &Context| {
//!         let ua = ctx.req.header("user-agent").unwrap_or_default();
//!         Decision::deny_if(ua.contains("sqlmap") || ua.contains("nikto"))
//!     })
//!     .build(),
//! );
//!
//! // Serve only the US and Canada, using Cloudflare's country header.
//! app.use_middleware(GeoFilter::allow(&["US", "CA"]).header("CF-IPCountry").build());
//! ```

use crate::context::Context;
use crate::middleware::BoxedMiddleware;
use crate::response::UltimoBody;
use std::collections::HashSet;
use std::net::IpAddr;
use std::sync::Arc;

/// Context state key holding the country resolved by [`GeoFilter`].
pub(crate) const COUNTRY_STATE_KEY: &str = "ultimo.geo.country";

/// What a [`Guard`] does with a request.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Decision {
    /// Pass the request on to the next middleware and the handler.
    Allow,
    /// Reject it with the guard's deny response (`403 Forbidden` unless
    /// changed with [`Guard::deny_status`]).
    Deny,
    /// Reject it with this status and plain-text body, e.g. a `404` to hide a
    /// route's existence. Build it with [`Decision::respond`].
    Respond {
        /// HTTP status code.
        status: u16,
        /// Plain-text response body.
        body: String,
    },
}

impl Decision {
    /// [`Deny`](Self::Deny) when `condition` holds, otherwise
    /// [`Allow`](Self::Allow).
    pub fn deny_if(condition: bool) -> Self {
        if condition {
            Self::Deny
        } else {
            Self::Allow
        }
    }

    /// [`Allow`](Self::Allow) when `condition` holds, otherwise
    /// [`Deny`](Self::Deny).
    pub fn allow_if(condition: bool) -> Self {
        Self::deny_if(!condition)
    }

    /// Reject with `status` and a plain-text `body`.
    pub fn respond(status: u16, body: impl Into<String>) -> Self {
        Self::Respond {
            status,
            body: body.into(),
        }
    }
}

type Predicate = Arc<dyn Fn(&Context) -> Decision + Send + Sync>;

/// Middleware that runs a predicate on every request and
/// [allows, denies or answers](Decision) it.
///
/// The predicate is synchronous and runs before the handler, so keep it cheap.
/// Register several guards for several rules; they run in registration order.
///
/// ```rust
/// use ultimo::guard::{Decision, Guard};
/// use ultimo::prelude::*;
///
/// let mut app = Ultimo::new();
/// // Hide /admin from requests that don't come from the office network header.
/// app.use_middleware(
///     Guard::new(|ctx: &Context| {
///         if ctx.req.path().starts_with("/admin") && ctx.req.header("x-office").is_none() {
///             Decision::respond(404, "Not Found")
///         } else {
///             Decision::Allow
///         }
///     })
///     .build(),
/// );
/// ```
#[derive(Clone)]
pub struct Guard {
    predicate: Predicate,
    deny_status: u16,
    deny_body: Arc<str>,
}

impl Guard {
    /// A guard running `predicate` on each request.
    pub fn new<F>(predicate: F) -> Self
    where
        F: Fn(&Context) -> Decision + Send + Sync + 'static,
    {
        Self {
            predicate: Arc::new(predicate),
            deny_status: 403,
            deny_body: Arc::from("Forbidden"),
        }
    }

    /// Response for [`Decision::Deny`] (default `403` / `Forbidden`).
    pub fn deny_status(mut self, status: u16, body: impl Into<String>) -> Self {
        self.deny_status = status;
        self.deny_body = Arc::from(body.into());
        self
    }

    /// Build the middleware.
    pub fn build(self) -> BoxedMiddleware {
        let guard = self;
        Arc::new(move |ctx, next| {
            let guard = guard.clone();
            Box::pin(async move {
                match (guard.predicate)(&ctx) {
                    Decision::Allow => next(ctx).await,
                    Decision::Deny => {
                        log_block(&ctx, guard.deny_status, "guard");
                        Ok(plain(guard.deny_status, guard.deny_body.to_string()))
                    }
                    Decision::Respond { status, body } => {
                        log_block(&ctx, status, "guard");
                        Ok(plain(status, body))
                    }
                }
            })
        })
    }
}

fn plain(status: u16, body: String) -> crate::response::Response {
    hyper::Response::builder()
        .status(hyper::StatusCode::from_u16(status).unwrap_or(hyper::StatusCode::FORBIDDEN))
        .header(hyper::header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(UltimoBody::full(body))
        .unwrap_or_else(|_| hyper::Response::new(UltimoBody::empty()))
}

fn log_block(ctx: &Context, status: u16, by: &'static str) {
    tracing::debug!(
        path = ctx.req.path(),
        client_ip = ?ctx.client_ip(),
        status,
        by,
        "request blocked"
    );
}

type Lookup = Arc<dyn Fn(IpAddr) -> Option<String> + Send + Sync>;

/// Country allow-list or deny-list middleware.
///
/// The country is an ISO 3166-1 alpha-2 code (`US`, `DE`, …), compared
/// case-insensitively, taken from:
///
/// 1. a CDN [`header`](Self::header) such as `CF-IPCountry` — read **only when
///    the app trusts its proxy** (`app.trust_proxy(true)`), since without a
///    proxy in front a client could send that header itself; then
/// 2. your [`lookup`](Self::lookup) function, given the
///    [client IP](crate::Context::client_ip).
///
/// Blocked requests get `403 Forbidden`. A request whose country can't be
/// determined (no header, lookup returns `None`, or Cloudflare's `XX`) is
/// **denied on an allow-list and allowed on a deny-list**; change that with
/// [`allow_unknown`](Self::allow_unknown) / [`deny_unknown`](Self::deny_unknown).
///
/// The resolved country is available to handlers as
/// [`ctx.country()`](crate::Context::country).
///
/// ```rust
/// use ultimo::guard::GeoFilter;
/// # let mut app = ultimo::Ultimo::new();
/// app.trust_proxy(true);
/// app.use_middleware(GeoFilter::deny(&["KP", "IR"]).header("CF-IPCountry").build());
/// ```
#[derive(Clone)]
pub struct GeoFilter {
    countries: Arc<HashSet<String>>,
    allow_list: bool,
    header: Option<Arc<str>>,
    lookup: Option<Lookup>,
    unknown_allowed: bool,
}

impl GeoFilter {
    /// Allow only these countries; everything else gets `403`.
    ///
    /// # Panics
    /// If a code isn't two ASCII letters/digits (e.g. `US`, `T1`).
    pub fn allow(countries: &[&str]) -> Self {
        Self::with(countries, true)
    }

    /// Block these countries; everything else is allowed.
    ///
    /// # Panics
    /// If a code isn't two ASCII letters/digits (e.g. `US`, `T1`).
    pub fn deny(countries: &[&str]) -> Self {
        Self::with(countries, false)
    }

    fn with(countries: &[&str], allow_list: bool) -> Self {
        let countries = countries
            .iter()
            .map(|c| normalize(c).unwrap_or_else(|| panic!("invalid country code {c:?}")))
            .collect();
        Self {
            countries: Arc::new(countries),
            allow_list,
            header: None,
            lookup: None,
            unknown_allowed: !allow_list,
        }
    }

    /// Read the country from this request header, set by your CDN or load
    /// balancer (`CF-IPCountry` on Cloudflare, `CloudFront-Viewer-Country`
    /// on CloudFront, …). Ignored unless `app.trust_proxy(true)`.
    pub fn header(mut self, name: impl Into<String>) -> Self {
        self.header = Some(Arc::from(name.into()));
        self
    }

    /// Resolve the country from the client IP, e.g. with a MaxMind GeoLite2
    /// reader. Used when no [`header`](Self::header) value is available.
    pub fn lookup<F>(mut self, lookup: F) -> Self
    where
        F: Fn(IpAddr) -> Option<String> + Send + Sync + 'static,
    {
        self.lookup = Some(Arc::new(lookup));
        self
    }

    /// Let requests from an unknown country through.
    pub fn allow_unknown(mut self) -> Self {
        self.unknown_allowed = true;
        self
    }

    /// Block requests from an unknown country.
    pub fn deny_unknown(mut self) -> Self {
        self.unknown_allowed = false;
        self
    }

    /// Build the middleware.
    ///
    /// # Panics
    /// If neither a [`header`](Self::header) nor a [`lookup`](Self::lookup)
    /// was given.
    pub fn build(self) -> BoxedMiddleware {
        assert!(
            self.header.is_some() || self.lookup.is_some(),
            "GeoFilter needs a country source: call .header(...) and/or .lookup(...)"
        );
        let geo = self;
        Arc::new(move |ctx, next| {
            let geo = geo.clone();
            Box::pin(async move {
                let country = geo.country(&ctx);
                let allowed = match &country {
                    Some(c) => geo.countries.contains(c) == geo.allow_list,
                    None => geo.unknown_allowed,
                };
                if !allowed {
                    log_block(&ctx, 403, "geo");
                    return Ok(plain(403, "Forbidden".into()));
                }
                if let Some(c) = country {
                    ctx.set(COUNTRY_STATE_KEY, c).await;
                }
                next(ctx).await
            })
        })
    }

    fn country(&self, ctx: &Context) -> Option<String> {
        let from_header = self
            .header
            .as_ref()
            .filter(|_| ctx.trusts_proxy())
            .and_then(|h| ctx.req.header(h))
            .and_then(|v| normalize(&v));
        from_header.or_else(|| {
            let lookup = self.lookup.as_ref()?;
            lookup(ctx.client_ip()?).and_then(|c| normalize(&c))
        })
    }
}

/// Uppercase a two-character code; `None` for anything else and for `XX`
/// (Cloudflare's "unknown").
fn normalize(code: &str) -> Option<String> {
    let code = code.trim();
    if code.len() != 2 || !code.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return None;
    }
    let code = code.to_ascii_uppercase();
    (code != "XX").then_some(code)
}

#[cfg(test)]
mod tests {
    use super::normalize;

    #[test]
    fn normalize_country_codes() {
        assert_eq!(normalize("us").as_deref(), Some("US"));
        assert_eq!(normalize(" de ").as_deref(), Some("DE"));
        assert_eq!(normalize("T1").as_deref(), Some("T1"));
        assert_eq!(normalize("XX"), None);
        assert_eq!(normalize("xx"), None);
        assert_eq!(normalize("USA"), None);
        assert_eq!(normalize(""), None);
        assert_eq!(normalize("U-"), None);
    }
}
