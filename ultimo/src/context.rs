//! Request/Response context for handling HTTP requests and responses
//!
//! Provides a unified interface for handling HTTP requests and building responses.

use crate::{
    error::{Result, UltimoError},
    response::{Response, ResponseBuilder},
    router::Params,
};
use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::{body::Incoming, Request as HyperRequest};
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use tokio::sync::RwLock;

#[cfg(feature = "database")]
use crate::database::Database;

/// Request wraps the incoming HTTP request and provides easy access to request data
pub struct Request {
    method: hyper::Method,
    uri: hyper::Uri,
    headers: hyper::HeaderMap,
    params: Params,
    body: Arc<RwLock<Option<Bytes>>>,
}

impl Request {
    /// Build a Request from already-parsed parts and a buffered body.
    pub(crate) fn from_parts(
        parts: hyper::http::request::Parts,
        body: Bytes,
        params: Params,
    ) -> Self {
        Self {
            method: parts.method,
            uri: parts.uri,
            headers: parts.headers,
            params,
            body: Arc::new(RwLock::new(Some(body))),
        }
    }

    /// Create a new Request from a Hyper request and path parameters
    pub async fn new(req: HyperRequest<Incoming>, params: Params) -> Result<Self> {
        let (parts, body) = req.into_parts();
        let collected = body
            .collect()
            .await
            .map_err(|e| UltimoError::Internal(format!("Failed to read body: {}", e)))?;
        Ok(Self::from_parts(parts, collected.to_bytes(), params))
    }

    /// Get a path parameter by name
    pub fn param(&self, name: &str) -> Result<&str> {
        self.params
            .get(name)
            .map(|s| s.as_str())
            .ok_or_else(|| UltimoError::BadRequest(format!("Missing path parameter: {}", name)))
    }

    /// Get all path parameters at once
    pub fn params(&self) -> &Params {
        &self.params
    }

    /// Get a query parameter by name
    pub fn query(&self, name: &str) -> Option<String> {
        self.uri.query().and_then(|q| {
            q.split('&').find_map(|pair| {
                let (key, value) = pair.split_once('=')?;
                if key == name {
                    Some(value.to_string())
                } else {
                    None
                }
            })
        })
    }

    /// Get all query parameters at once
    pub fn queries(&self) -> HashMap<String, Vec<String>> {
        let mut result: HashMap<String, Vec<String>> = HashMap::new();

        if let Some(query) = self.uri.query() {
            for pair in query.split('&') {
                let mut parts = pair.splitn(2, '=');
                if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
                    result
                        .entry(key.to_string())
                        .or_default()
                        .push(value.to_string());
                }
            }
        }

        result
    }

    /// Get a header value by name
    pub fn header(&self, name: &str) -> Option<String> {
        self.headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
    }

    /// Get the request path
    pub fn path(&self) -> &str {
        self.uri.path()
    }

    /// Get the full URL as a string
    pub fn url(&self) -> String {
        self.uri.to_string()
    }

    /// Get the request method
    pub fn method(&self) -> &hyper::Method {
        &self.method
    }

    /// Parse request body as JSON
    pub async fn json<T: DeserializeOwned>(&self) -> Result<T> {
        let body = self.body.read().await;
        let bytes = body
            .as_ref()
            .ok_or_else(|| UltimoError::BadRequest("Body already consumed".to_string()))?;

        serde_json::from_slice(bytes).map_err(UltimoError::Json)
    }

    /// Parse request body as text
    pub async fn text(&self) -> Result<String> {
        let body = self.body.read().await;
        let bytes = body
            .as_ref()
            .ok_or_else(|| UltimoError::BadRequest("Body already consumed".to_string()))?;

        String::from_utf8(bytes.to_vec())
            .map_err(|e| UltimoError::BadRequest(format!("Invalid UTF-8: {}", e)))
    }

    /// Get request body as bytes.
    ///
    /// The body is buffered and cached, so this (and [`json`](Self::json) /
    /// [`text`](Self::text)) may be called any number of times.
    pub async fn bytes(&self) -> Result<Bytes> {
        let body = self.body.read().await;
        body.as_ref()
            .cloned()
            .ok_or_else(|| UltimoError::BadRequest("Body already consumed".to_string()))
    }

    /// Raw request body bytes (alias for [`bytes`](Self::bytes)). Repeatable.
    pub async fn raw_body(&self) -> Result<Bytes> {
        self.bytes().await
    }
}

#[cfg(test)]
mod request_body_tests {
    use super::*;

    fn req_with_body(body: &'static [u8]) -> Request {
        let r = HyperRequest::builder()
            .method("POST")
            .uri("/")
            .body(())
            .unwrap();
        let (parts, ()) = r.into_parts();
        Request::from_parts(parts, Bytes::from_static(body), Params::new())
    }

    #[tokio::test]
    async fn body_is_readable_multiple_times() {
        let req = req_with_body(br#"{"n":1}"#);
        // json twice
        let a: serde_json::Value = req.json().await.unwrap();
        let b: serde_json::Value = req.json().await.unwrap();
        assert_eq!(a, b);
        assert_eq!(a, serde_json::json!({ "n": 1 }));
        // then text + raw_body still work
        assert_eq!(req.text().await.unwrap(), r#"{"n":1}"#);
        assert_eq!(
            req.raw_body().await.unwrap(),
            Bytes::from_static(br#"{"n":1}"#)
        );
    }
}

/// Extract the IP from the first `for=` element of an RFC 7239 `Forwarded` header.
fn parse_forwarded_for(header: &str) -> Option<IpAddr> {
    let first = header.split(',').next()?;
    for part in first.split(';') {
        let part = part.trim();
        if part.len() >= 4 && part[..4].eq_ignore_ascii_case("for=") {
            let v = part[4..].trim().trim_matches('"');
            // IPv6 in brackets, optionally with port: [::1]:1234
            if let Some(rest) = v.strip_prefix('[') {
                if let Some(end) = rest.find(']') {
                    return rest[..end].parse().ok();
                }
            }
            // Bare IP, or IPv4 with :port
            if let Ok(ip) = v.parse::<IpAddr>() {
                return Some(ip);
            }
            if let Some((host, _)) = v.rsplit_once(':') {
                return host.parse().ok();
            }
        }
    }
    None
}

/// Context holds request data and provides response building methods
pub struct Context {
    pub req: Request,
    state: Arc<RwLock<HashMap<String, String>>>,
    response_status: Arc<RwLock<Option<u16>>>,
    response_headers: Arc<RwLock<HashMap<String, String>>>,
    set_cookies: Arc<RwLock<Vec<String>>>,
    /// Peer address of the connection (set by the server; None for in-process dispatch).
    client_addr: Option<SocketAddr>,
    /// Whether to trust `X-Forwarded-For` / `Forwarded` headers for `client_ip()`.
    trust_proxy: bool,
    #[cfg(feature = "session")]
    session: Arc<RwLock<Option<crate::session::Session>>>,
    #[cfg(feature = "jwt")]
    jwt_claims: Arc<RwLock<Option<serde_json::Value>>>,
    #[cfg(feature = "api-key")]
    api_key: Arc<RwLock<Option<crate::auth::api_key::ApiKeyIdentity>>>,

    #[cfg(feature = "database")]
    database: Option<Database>,
}

impl Context {
    /// Build a Context from already-parsed parts and a buffered body.
    pub(crate) fn from_parts(
        parts: hyper::http::request::Parts,
        body: Bytes,
        params: Params,
    ) -> Self {
        Self {
            req: Request::from_parts(parts, body, params),
            state: Arc::new(RwLock::new(HashMap::new())),
            response_status: Arc::new(RwLock::new(None)),
            response_headers: Arc::new(RwLock::new(HashMap::new())),
            set_cookies: Arc::new(RwLock::new(Vec::new())),
            client_addr: None,
            trust_proxy: false,
            #[cfg(feature = "session")]
            session: Arc::new(RwLock::new(None)),
            #[cfg(feature = "jwt")]
            jwt_claims: Arc::new(RwLock::new(None)),
            #[cfg(feature = "api-key")]
            api_key: Arc::new(RwLock::new(None)),
            #[cfg(feature = "database")]
            database: None,
        }
    }

    /// Create a new context from a request and params
    pub async fn new(req: HyperRequest<Incoming>, params: Params) -> Result<Self> {
        let (parts, body) = req.into_parts();
        let collected = body
            .collect()
            .await
            .map_err(|e| UltimoError::Internal(format!("Failed to read body: {}", e)))?;
        Ok(Self::from_parts(parts, collected.to_bytes(), params))
    }

    /// Attach a database to this context (internal use)
    #[cfg(feature = "database")]
    pub(crate) fn attach_database(&mut self, db: Database) {
        self.database = Some(db);
    }

    /// Get the database pool (SQLx)
    #[cfg(feature = "sqlx")]
    pub fn sqlx<DB: sqlx::Database>(&self) -> Result<&sqlx::Pool<DB>> {
        let db = self
            .database
            .as_ref()
            .ok_or(crate::database::DatabaseError::NotConfigured)?;

        let sqlx_pool = db.as_sqlx::<DB>()?;
        Ok(sqlx_pool.pool())
    }

    /// Get a Diesel connection from the pool
    #[cfg(feature = "diesel")]
    pub fn diesel<Conn>(
        &self,
    ) -> Result<diesel::r2d2::PooledConnection<diesel::r2d2::ConnectionManager<Conn>>>
    where
        Conn: diesel::Connection + diesel::r2d2::R2D2Connection + 'static,
    {
        let db = self
            .database
            .as_ref()
            .ok_or(crate::database::DatabaseError::NotConfigured)?;

        let diesel_pool = db.as_diesel::<Conn>()?;
        diesel_pool.get().map_err(Into::into)
    }

    /// Get the database (generic access)
    #[cfg(feature = "database")]
    pub fn database(&self) -> Result<&Database> {
        self.database
            .as_ref()
            .ok_or(crate::database::DatabaseError::NotConfigured.into())
    }

    /// Set a value in the context state (shared between middleware)
    pub async fn set(&self, key: impl Into<String>, value: impl Into<String>) {
        let mut state = self.state.write().await;
        state.insert(key.into(), value.into());
    }

    /// Get a value from the context state
    pub async fn get(&self, key: &str) -> Option<String> {
        let state = self.state.read().await;
        state.get(key).cloned()
    }

    /// Read a request cookie by name.
    pub fn cookie(&self, name: &str) -> Option<String> {
        self.req
            .header("cookie")
            .and_then(|h| crate::cookie::parse_cookie_header(&h).remove(name))
    }

    /// All request cookies.
    pub fn cookies(&self) -> HashMap<String, String> {
        self.req
            .header("cookie")
            .map(|h| crate::cookie::parse_cookie_header(&h))
            .unwrap_or_default()
    }

    /// Queue a `Set-Cookie` for the response. Errors if the cookie is invalid.
    pub async fn set_cookie(&self, cookie: crate::cookie::Cookie) -> Result<()> {
        let s = cookie.to_set_cookie_string()?;
        self.set_cookies.write().await.push(s);
        Ok(())
    }

    /// Queue a deletion of the named cookie (`Max-Age=0`).
    pub async fn remove_cookie(&self, name: &str) -> Result<()> {
        let c = crate::cookie::Cookie::new(name, "").max_age(0).path("/");
        self.set_cookie(c).await
    }

    /// Shared handle to the queued Set-Cookie values (drained by the dispatcher).
    pub(crate) fn set_cookies_handle(&self) -> Arc<RwLock<Vec<String>>> {
        self.set_cookies.clone()
    }

    /// Set the connection peer address + proxy-trust (used by the server).
    pub(crate) fn set_client(&mut self, addr: Option<SocketAddr>, trust_proxy: bool) {
        self.client_addr = addr;
        self.trust_proxy = trust_proxy;
    }

    /// The peer address of the underlying connection, if known. This is the
    /// direct socket peer — for the originating client behind a proxy, use
    /// [`client_ip`](Self::client_ip).
    pub fn peer_addr(&self) -> Option<SocketAddr> {
        self.client_addr
    }

    /// Best-effort originating client IP.
    ///
    /// When proxy trust is enabled (`app.trust_proxy(true)`) this honors the
    /// left-most `X-Forwarded-For` entry, then `Forwarded: for=…`; otherwise (or
    /// if no such header) it falls back to the connection peer. **Only enable
    /// proxy trust when the app is actually behind a trusted proxy** — these
    /// headers are client-spoofable.
    pub fn client_ip(&self) -> Option<IpAddr> {
        if self.trust_proxy {
            if let Some(xff) = self.req.header("x-forwarded-for") {
                if let Some(ip) = xff
                    .split(',')
                    .next()
                    .and_then(|s| s.trim().parse::<IpAddr>().ok())
                {
                    return Some(ip);
                }
            }
            if let Some(fwd) = self.req.header("forwarded") {
                if let Some(ip) = parse_forwarded_for(&fwd) {
                    return Some(ip);
                }
            }
        }
        self.client_addr.map(|a| a.ip())
    }

    /// The current session. Panics if the session middleware isn't installed.
    #[cfg(feature = "session")]
    pub async fn session(&self) -> crate::session::Session {
        self.session
            .read()
            .await
            .clone()
            .expect("session middleware not installed (add `session(store, config)`)")
    }

    /// Attach a session to this context (used by the session middleware).
    #[cfg(feature = "session")]
    pub(crate) async fn set_session(&self, s: crate::session::Session) {
        *self.session.write().await = Some(s);
    }

    /// The validated JWT claims for this request, if the `jwt` middleware ran
    /// and accepted a token. Returns a clone of the raw claims object.
    #[cfg(feature = "jwt")]
    pub async fn jwt_claims(&self) -> Option<serde_json::Value> {
        self.jwt_claims.read().await.clone()
    }

    /// Deserialize the validated JWT claims into a typed struct. Errors if no
    /// claims are present (unauthenticated) or the shape doesn't match.
    #[cfg(feature = "jwt")]
    pub async fn jwt<T: serde::de::DeserializeOwned>(&self) -> Result<T> {
        let claims =
            self.jwt_claims.read().await.clone().ok_or_else(|| {
                crate::error::UltimoError::Unauthorized("no JWT claims".to_string())
            })?;
        serde_json::from_value(claims).map_err(crate::error::UltimoError::from)
    }

    /// Store validated claims on the context (used by the jwt middleware).
    #[cfg(feature = "jwt")]
    pub(crate) async fn set_jwt_claims(&self, claims: serde_json::Value) {
        *self.jwt_claims.write().await = Some(claims);
    }

    /// The API-key identity for this request, if the `api-key` middleware ran and
    /// accepted a key. `None` if unauthenticated (or in optional mode).
    #[cfg(feature = "api-key")]
    pub async fn api_key(&self) -> Option<crate::auth::api_key::ApiKeyIdentity> {
        self.api_key.read().await.clone()
    }

    /// Store the resolved API-key identity (used by the api-key middleware).
    #[cfg(feature = "api-key")]
    pub(crate) async fn set_api_key(&self, identity: crate::auth::api_key::ApiKeyIdentity) {
        *self.api_key.write().await = Some(identity);
    }

    /// Set the response status code
    pub async fn status(&self, code: u16) {
        let mut status = self.response_status.write().await;
        *status = Some(code);
    }

    /// Add a response header
    pub async fn header(&self, name: impl Into<String>, value: impl Into<String>) {
        let mut headers = self.response_headers.write().await;
        headers.insert(name.into(), value.into());
    }

    /// Build response with collected status and headers
    async fn build_response(&self, mut builder: ResponseBuilder) -> ResponseBuilder {
        // Apply status if set
        if let Some(status) = *self.response_status.read().await {
            builder = builder.status(status);
        }

        // Apply headers
        let headers = self.response_headers.read().await;
        for (name, value) in headers.iter() {
            builder = builder.header(name.clone(), value.clone());
        }

        builder
    }

    /// Return a JSON response
    pub async fn json<T: Serialize>(&self, value: T) -> Result<Response> {
        let builder = self.build_response(ResponseBuilder::new()).await;
        builder.json(&value)?.build()
    }

    /// Return a text response
    pub async fn text(&self, text: impl Into<String>) -> Result<Response> {
        let builder = self.build_response(ResponseBuilder::new()).await;
        builder.text(text).build()
    }

    /// Return an HTML response
    pub async fn html(&self, html: impl Into<String>) -> Result<Response> {
        let builder = self.build_response(ResponseBuilder::new()).await;
        builder.html(html).build()
    }

    /// Return a redirect response
    pub async fn redirect(&self, location: &str) -> Result<Response> {
        let status = self.response_status.read().await.unwrap_or(302);
        let builder = ResponseBuilder::new()
            .status(status)
            .header("Location", location);
        builder.build()
    }

    /// Return a not found response
    pub async fn not_found(&self) -> Result<Response> {
        self.status(404).await;
        self.json(serde_json::json!({
            "error": "NotFound",
            "message": "The requested resource was not found"
        }))
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_params_api() {
        // Test the Params type directly
        let mut params = Params::new();
        params.insert("id".to_string(), "123".to_string());
        params.insert("name".to_string(), "test".to_string());

        assert_eq!(params.get("id"), Some(&"123".to_string()));
        assert_eq!(params.get("name"), Some(&"test".to_string()));
        assert_eq!(params.get("missing"), None);
    }

    #[test]
    fn test_query_parsing() {
        // Test query string parsing logic
        let uri: hyper::Uri = "/search?q=rust&page=2".parse().unwrap();

        let query_str = uri.query().unwrap();
        let mut found_q = false;
        let mut found_page = false;

        for pair in query_str.split('&') {
            let parts: Vec<&str> = pair.splitn(2, '=').collect();
            if parts.len() == 2 {
                if parts[0] == "q" && parts[1] == "rust" {
                    found_q = true;
                }
                if parts[0] == "page" && parts[1] == "2" {
                    found_page = true;
                }
            }
        }

        assert!(found_q);
        assert!(found_page);
    }

    #[test]
    fn test_queries_parsing_with_duplicates() {
        // Test parsing multiple values for same key
        let uri: hyper::Uri = "/search?tags=rust&tags=web&page=1".parse().unwrap();

        let mut result: HashMap<String, Vec<String>> = HashMap::new();
        if let Some(query) = uri.query() {
            for pair in query.split('&') {
                let mut parts = pair.splitn(2, '=');
                if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
                    result
                        .entry(key.to_string())
                        .or_default()
                        .push(value.to_string());
                }
            }
        }

        assert_eq!(
            result.get("tags").unwrap(),
            &vec!["rust".to_string(), "web".to_string()]
        );
        assert_eq!(result.get("page").unwrap(), &vec!["1".to_string()]);
    }

    #[test]
    fn test_uri_path() {
        let uri: hyper::Uri = "/api/users/123".parse().unwrap();
        assert_eq!(uri.path(), "/api/users/123");
    }

    #[test]
    fn test_uri_to_string() {
        let uri: hyper::Uri = "/api/users?page=1".parse().unwrap();
        assert_eq!(uri.to_string(), "/api/users?page=1");
    }

    #[test]
    fn test_method_types() {
        let get = hyper::Method::GET;
        let post = hyper::Method::POST;
        let put = hyper::Method::PUT;
        let delete = hyper::Method::DELETE;

        assert_eq!(get, hyper::Method::GET);
        assert_eq!(post, hyper::Method::POST);
        assert_eq!(put, hyper::Method::PUT);
        assert_eq!(delete, hyper::Method::DELETE);
    }

    #[tokio::test]
    async fn test_state_operations() {
        // Test state HashMap operations
        let state = Arc::new(RwLock::new(HashMap::new()));

        {
            let mut s = state.write().await;
            s.insert("user_id".to_string(), "123".to_string());
            s.insert("role".to_string(), "admin".to_string());
        }

        {
            let s = state.read().await;
            assert_eq!(s.get("user_id"), Some(&"123".to_string()));
            assert_eq!(s.get("role"), Some(&"admin".to_string()));
            assert_eq!(s.get("missing"), None);
        }
    }

    #[tokio::test]
    async fn test_response_status_tracking() {
        let status = Arc::new(RwLock::new(None));

        {
            let mut s = status.write().await;
            *s = Some(404);
        }

        {
            let s = status.read().await;
            assert_eq!(*s, Some(404));
        }
    }

    #[tokio::test]
    async fn test_response_headers_tracking() {
        let headers = Arc::new(RwLock::new(HashMap::new()));

        {
            let mut h = headers.write().await;
            h.insert("x-custom".to_string(), "value".to_string());
            h.insert("content-type".to_string(), "application/json".to_string());
        }

        {
            let h = headers.read().await;
            assert_eq!(h.get("x-custom"), Some(&"value".to_string()));
            assert_eq!(h.get("content-type"), Some(&"application/json".to_string()));
        }
    }

    #[test]
    fn test_json_deserialization() {
        // Test JSON parsing
        #[derive(serde::Deserialize, Debug, PartialEq)]
        struct User {
            name: String,
            age: u32,
        }

        let json_str = r#"{"name":"John","age":30}"#;
        let user: User = serde_json::from_str(json_str).unwrap();

        assert_eq!(user.name, "John");
        assert_eq!(user.age, 30);
    }

    #[test]
    fn test_text_parsing() {
        let text = "Hello, World!";
        let bytes = Bytes::from(text);
        let parsed = String::from_utf8(bytes.to_vec()).unwrap();

        assert_eq!(parsed, "Hello, World!");
    }

    #[test]
    fn test_bytes_operations() {
        let data = "binary data";
        let bytes = Bytes::from(data);

        assert_eq!(bytes, Bytes::from("binary data"));
        assert_eq!(bytes.len(), 11);
    }
}

#[cfg(test)]
mod from_parts_tests {
    use super::*;

    #[tokio::test]
    async fn request_from_parts_exposes_method_path_query_body() {
        let req = HyperRequest::builder()
            .method("POST")
            .uri("/users?team=core")
            .body(())
            .unwrap();
        let (parts, ()) = req.into_parts();
        let body = Bytes::from_static(br#"{"name":"ada"}"#);

        let r = Request::from_parts(parts, body, Params::new());

        assert_eq!(r.method(), &hyper::Method::POST);
        assert_eq!(r.path(), "/users");
        assert_eq!(r.query("team").as_deref(), Some("core"));
        assert_eq!(r.text().await.unwrap(), r#"{"name":"ada"}"#);
    }
}

#[cfg(test)]
mod context_response_tests {
    use super::*;
    use http_body_util::BodyExt;

    fn ctx() -> Context {
        let req = HyperRequest::builder()
            .method("GET")
            .uri("/x?a=1&a=2&b=3")
            .header("cookie", "t=v; u=w")
            .body(())
            .unwrap();
        let (parts, ()) = req.into_parts();
        Context::from_parts(parts, Bytes::from_static(b"{\"k\":1}"), Params::new())
    }

    async fn body(resp: Response) -> String {
        let b = resp.into_body().collect().await.unwrap().to_bytes();
        String::from_utf8(b.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn state_set_get() {
        let c = ctx();
        c.set("k", "v").await;
        assert_eq!(c.get("k").await, Some("v".to_string()));
        assert_eq!(c.get("missing").await, None);
    }

    #[tokio::test]
    async fn json_text_html_responses() {
        let c = ctx();
        let r = c.json(serde_json::json!({ "x": 1 })).await.unwrap();
        assert_eq!(r.status(), 200);
        assert_eq!(body(r).await, "{\"x\":1}");

        let c = ctx();
        assert_eq!(body(c.text("hi").await.unwrap()).await, "hi");

        let c = ctx();
        assert_eq!(body(c.html("<p>").await.unwrap()).await, "<p>");
    }

    #[tokio::test]
    async fn status_and_header_applied_to_response() {
        let c = ctx();
        c.status(201).await;
        c.header("x-test", "1").await;
        let r = c.text("ok").await.unwrap();
        assert_eq!(r.status(), 201);
        assert_eq!(r.headers().get("x-test").unwrap(), "1");
    }

    #[tokio::test]
    async fn redirect_and_not_found() {
        let r = ctx().redirect("/login").await.unwrap();
        assert_eq!(r.status(), 302);
        assert_eq!(r.headers().get("location").unwrap(), "/login");

        let r = ctx().not_found().await.unwrap();
        assert_eq!(r.status(), 404);
    }

    #[tokio::test]
    async fn cookies_query_and_body() {
        let c = ctx();
        assert_eq!(c.cookie("t"), Some("v".to_string()));
        assert_eq!(c.cookie("u"), Some("w".to_string()));
        assert_eq!(c.cookies().len(), 2);

        c.set_cookie(crate::cookie::Cookie::new("s", "1"))
            .await
            .unwrap();
        c.remove_cookie("old").await.unwrap();
        assert_eq!(c.set_cookies_handle().read().await.len(), 2);

        assert_eq!(c.req.query("b"), Some("3".to_string()));
        assert_eq!(c.req.queries().get("a").unwrap().len(), 2);
        assert_eq!(c.req.path(), "/x");
        assert_eq!(c.req.method(), &hyper::Method::GET);
    }
}

#[cfg(all(test, feature = "jwt"))]
mod jwt_claims_tests {
    use super::*;
    use serde::Deserialize;

    fn ctx() -> Context {
        let req = HyperRequest::builder()
            .method("GET")
            .uri("/")
            .body(())
            .unwrap();
        let (parts, _) = req.into_parts();
        Context::from_parts(parts, Bytes::new(), Params::new())
    }

    #[derive(Deserialize, PartialEq, Debug)]
    struct Claims {
        sub: String,
    }

    #[tokio::test]
    async fn claims_absent_by_default() {
        let c = ctx();
        assert!(c.jwt_claims().await.is_none());
        assert!(c.jwt::<Claims>().await.is_err());
    }

    #[tokio::test]
    async fn set_then_read_typed_claims() {
        let c = ctx();
        c.set_jwt_claims(serde_json::json!({ "sub": "ada" })).await;
        assert_eq!(
            c.jwt_claims().await,
            Some(serde_json::json!({ "sub": "ada" }))
        );
        assert_eq!(
            c.jwt::<Claims>().await.unwrap(),
            Claims { sub: "ada".into() }
        );
    }
}

#[cfg(test)]
mod forwarded_tests {
    use super::*;

    #[test]
    fn parses_forwarded_for_variants() {
        assert_eq!(
            parse_forwarded_for("for=192.0.2.1"),
            Some("192.0.2.1".parse().unwrap())
        );
        assert_eq!(
            parse_forwarded_for("For=\"[2001:db8::1]:4711\";proto=https"),
            Some("2001:db8::1".parse().unwrap())
        );
        assert_eq!(
            parse_forwarded_for("for=192.0.2.1:8080"),
            Some("192.0.2.1".parse().unwrap())
        );
        assert_eq!(parse_forwarded_for("proto=https;by=10.0.0.1"), None);
    }
}
