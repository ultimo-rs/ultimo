//! HTTP cookie parsing and `Set-Cookie` formatting (RFC 6265).

use crate::error::{Result, UltimoError};
use std::collections::HashMap;

/// `SameSite` cookie attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SameSite {
    Strict,
    Lax,
    None,
}

impl SameSite {
    fn as_str(&self) -> &'static str {
        match self {
            SameSite::Strict => "Strict",
            SameSite::Lax => "Lax",
            SameSite::None => "None",
        }
    }
}

/// Attributes for a `Set-Cookie`.
#[derive(Debug, Clone, Default)]
pub struct CookieOptions {
    pub http_only: bool,
    pub secure: bool,
    pub same_site: Option<SameSite>,
    /// Max-Age in seconds.
    pub max_age: Option<i64>,
    pub path: Option<String>,
    pub domain: Option<String>,
}

/// A cookie to emit via `Set-Cookie`.
#[derive(Debug, Clone)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub options: CookieOptions,
}

impl Cookie {
    /// Create a cookie with default options.
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            options: CookieOptions::default(),
        }
    }

    /// Set the `HttpOnly` attribute.
    pub fn http_only(mut self, v: bool) -> Self {
        self.options.http_only = v;
        self
    }
    /// Set the `Secure` attribute.
    pub fn secure(mut self, v: bool) -> Self {
        self.options.secure = v;
        self
    }
    /// Set the `SameSite` attribute.
    pub fn same_site(mut self, v: SameSite) -> Self {
        self.options.same_site = Some(v);
        self
    }
    /// Set `Max-Age` in seconds.
    pub fn max_age(mut self, secs: i64) -> Self {
        self.options.max_age = Some(secs);
        self
    }
    /// Set the `Path` attribute.
    pub fn path(mut self, p: impl Into<String>) -> Self {
        self.options.path = Some(p.into());
        self
    }
    /// Set the `Domain` attribute.
    pub fn domain(mut self, d: impl Into<String>) -> Self {
        self.options.domain = Some(d.into());
        self
    }

    /// Format as a `Set-Cookie` header value. Rejects names/values containing
    /// control characters (header-injection guard).
    pub fn to_set_cookie_string(&self) -> Result<String> {
        validate_token(&self.name)?;
        validate_value(&self.value)?;
        let mut s = format!("{}={}", self.name, self.value);
        if let Some(p) = &self.options.path {
            validate_value(p)?;
            s.push_str(&format!("; Path={p}"));
        }
        if let Some(d) = &self.options.domain {
            validate_value(d)?;
            s.push_str(&format!("; Domain={d}"));
        }
        if let Some(m) = self.options.max_age {
            s.push_str(&format!("; Max-Age={m}"));
        }
        if let Some(ss) = self.options.same_site {
            s.push_str(&format!("; SameSite={}", ss.as_str()));
        }
        if self.options.secure {
            s.push_str("; Secure");
        }
        if self.options.http_only {
            s.push_str("; HttpOnly");
        }
        Ok(s)
    }
}

fn has_ctl(s: &str) -> bool {
    s.bytes().any(|b| b < 0x20 || b == 0x7f)
}

fn validate_token(name: &str) -> Result<()> {
    if name.is_empty() || has_ctl(name) || name.contains([';', '=', ' ', '\t']) {
        return Err(UltimoError::BadRequest(format!(
            "invalid cookie name: {name:?}"
        )));
    }
    Ok(())
}

fn validate_value(value: &str) -> Result<()> {
    if has_ctl(value) || value.contains([';', '\r', '\n']) {
        return Err(UltimoError::BadRequest("invalid cookie value".to_string()));
    }
    Ok(())
}

/// Parse a request `Cookie:` header into name → value pairs.
pub fn parse_cookie_header(header: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for pair in header.split(';') {
        let pair = pair.trim();
        if let Some((k, v)) = pair.split_once('=') {
            out.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_multiple_cookies() {
        let m = parse_cookie_header("a=1; b=2;  c=3");
        assert_eq!(m.get("a").map(String::as_str), Some("1"));
        assert_eq!(m.get("b").map(String::as_str), Some("2"));
        assert_eq!(m.get("c").map(String::as_str), Some("3"));
    }

    #[test]
    fn formats_all_attributes() {
        let c = Cookie::new("sid", "abc")
            .http_only(true)
            .secure(true)
            .same_site(SameSite::Lax)
            .max_age(3600)
            .path("/");
        let s = c.to_set_cookie_string().unwrap();
        assert!(s.starts_with("sid=abc"));
        assert!(s.contains("; Path=/"));
        assert!(s.contains("; Max-Age=3600"));
        assert!(s.contains("; SameSite=Lax"));
        assert!(s.contains("; Secure"));
        assert!(s.contains("; HttpOnly"));
    }

    #[test]
    fn rejects_header_injection() {
        let c = Cookie::new("sid", "abc\r\nSet-Cookie: evil=1");
        assert!(c.to_set_cookie_string().is_err());
    }
}
