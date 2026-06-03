//! In-process test client and request builder.

use crate::testing::response::TestResponse;
use crate::Ultimo;
use bytes::Bytes;
use http_body_util::Full;
use hyper::{HeaderMap, Method, Request as HyperRequest};
use serde::Serialize;

/// Drives an [`Ultimo`] app in-process for testing.
pub struct TestClient {
    app: Ultimo,
}

impl TestClient {
    /// Wrap a built app.
    pub fn new(app: Ultimo) -> Self {
        Self { app }
    }

    /// Start building a request with an explicit method.
    pub fn request(&self, method: Method, path: &str) -> TestRequest<'_> {
        TestRequest {
            client: self,
            method,
            path: path.to_string(),
            headers: HeaderMap::new(),
            query: Vec::new(),
            body: Bytes::new(),
        }
    }

    pub fn get(&self, path: &str) -> TestRequest<'_> {
        self.request(Method::GET, path)
    }
    pub fn post(&self, path: &str) -> TestRequest<'_> {
        self.request(Method::POST, path)
    }
    pub fn put(&self, path: &str) -> TestRequest<'_> {
        self.request(Method::PUT, path)
    }
    pub fn delete(&self, path: &str) -> TestRequest<'_> {
        self.request(Method::DELETE, path)
    }
    pub fn patch(&self, path: &str) -> TestRequest<'_> {
        self.request(Method::PATCH, path)
    }
    pub fn head(&self, path: &str) -> TestRequest<'_> {
        self.request(Method::HEAD, path)
    }
    pub fn options(&self, path: &str) -> TestRequest<'_> {
        self.request(Method::OPTIONS, path)
    }
}

/// A fluent request builder. Terminate with [`send`](TestRequest::send).
pub struct TestRequest<'a> {
    client: &'a TestClient,
    method: Method,
    path: String,
    headers: HeaderMap,
    query: Vec<(String, String)>,
    body: Bytes,
}

impl<'a> TestRequest<'a> {
    pub fn header(mut self, name: &str, value: &str) -> Self {
        let n = hyper::header::HeaderName::from_bytes(name.as_bytes()).expect("valid header name");
        let v = hyper::header::HeaderValue::from_str(value).expect("valid header value");
        self.headers.insert(n, v);
        self
    }

    pub fn bearer(self, token: &str) -> Self {
        self.header("authorization", &format!("Bearer {token}"))
    }

    pub fn query(mut self, pairs: &[(&str, &str)]) -> Self {
        for (k, v) in pairs {
            self.query.push((k.to_string(), v.to_string()));
        }
        self
    }

    pub fn body(mut self, body: impl Into<Bytes>) -> Self {
        self.body = body.into();
        self
    }

    pub fn text(self, text: &str) -> Self {
        self.body(Bytes::copy_from_slice(text.as_bytes()))
    }

    pub fn json<T: Serialize>(mut self, value: &T) -> Self {
        let bytes = serde_json::to_vec(value).expect("serializable JSON body");
        self.body = Bytes::from(bytes);
        self.header("content-type", "application/json")
    }

    /// Dispatch the request through the app in-process.
    pub async fn send(self) -> TestResponse {
        let uri = build_uri(&self.path, &self.query);
        let mut builder = HyperRequest::builder().method(self.method).uri(uri);
        if let Some(h) = builder.headers_mut() {
            *h = self.headers;
        }
        let req = builder
            .body(Full::new(self.body))
            .expect("valid test request");
        let resp = self.client.app.oneshot(req).await;
        TestResponse::from_response(resp).await
    }
}

fn build_uri(path: &str, query: &[(String, String)]) -> String {
    if query.is_empty() {
        return path.to_string();
    }
    let qs = query
        .iter()
        .map(|(k, v)| format!("{}={}", urlencode(k), urlencode(v)))
        .collect::<Vec<_>>()
        .join("&");
    format!("{path}?{qs}")
}

fn urlencode(s: &str) -> String {
    s.replace('%', "%25")
        .replace(' ', "%20")
        .replace('&', "%26")
        .replace('=', "%3D")
}
