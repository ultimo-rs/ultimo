//! Buffered response wrapper with assertion helpers.

use crate::response::Response;
use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::{HeaderMap, StatusCode};
use serde::de::DeserializeOwned;
use serde::Serialize;

/// A fully-buffered response. All accessors are synchronous.
pub struct TestResponse {
    status: StatusCode,
    headers: HeaderMap,
    body: Bytes,
}

impl TestResponse {
    pub(crate) async fn from_response(resp: Response) -> Self {
        let (parts, body) = resp.into_parts();
        let body = body
            .collect()
            .await
            .map(|c| c.to_bytes())
            .unwrap_or_default();
        Self {
            status: parts.status,
            headers: parts.headers,
            body,
        }
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|v| v.to_str().ok())
    }
    pub fn bytes(&self) -> &Bytes {
        &self.body
    }
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
    pub fn json<T: DeserializeOwned>(&self) -> T {
        serde_json::from_slice(&self.body).unwrap_or_else(|e| {
            panic!(
                "response body is not valid JSON for the target type: {e}\nbody: {}",
                self.text()
            )
        })
    }

    pub fn assert_status(&self, code: u16) -> &Self {
        assert_eq!(
            self.status.as_u16(),
            code,
            "expected status {code}, got {}",
            self.status.as_u16()
        );
        self
    }
    pub fn assert_ok(&self) -> &Self {
        self.assert_status(200)
    }
    pub fn assert_status_is_success(&self) -> &Self {
        assert!(
            self.status.is_success(),
            "expected 2xx status, got {}",
            self.status.as_u16()
        );
        self
    }
    pub fn assert_header(&self, name: &str, value: &str) -> &Self {
        assert_eq!(self.header(name), Some(value), "header {name} mismatch");
        self
    }
    pub fn assert_text(&self, expected: &str) -> &Self {
        assert_eq!(self.text(), expected, "response text mismatch");
        self
    }
    pub fn assert_json<T: Serialize>(&self, expected: &T) -> &Self {
        let got: serde_json::Value = serde_json::from_slice(&self.body)
            .unwrap_or_else(|e| panic!("response is not JSON: {e}"));
        let want = serde_json::to_value(expected).expect("expected is serializable");
        assert_eq!(got, want, "response JSON mismatch");
        self
    }
}
