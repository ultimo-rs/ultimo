//! Fast HTTP router with path parameter support
//!
//! Implements efficient path-based routing with support for:
//! - Static paths (/users)
//! - Path parameters (/users/:id)
//! - Multiple parameters (/users/:userId/posts/:postId)
//! - HTTP method matching

use std::collections::HashMap;

/// HTTP method enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Method {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    HEAD,
    OPTIONS,
}

impl Method {
    /// Parse method from hyper Method
    pub fn from_hyper(method: &hyper::Method) -> Option<Self> {
        match *method {
            hyper::Method::GET => Some(Method::GET),
            hyper::Method::POST => Some(Method::POST),
            hyper::Method::PUT => Some(Method::PUT),
            hyper::Method::DELETE => Some(Method::DELETE),
            hyper::Method::PATCH => Some(Method::PATCH),
            hyper::Method::HEAD => Some(Method::HEAD),
            hyper::Method::OPTIONS => Some(Method::OPTIONS),
            _ => None,
        }
    }
}

/// Path parameter map
pub type Params = HashMap<String, String>;

/// A single route segment
#[derive(Debug, Clone, PartialEq)]
enum Segment {
    /// Static path segment
    Static(String),
    /// Dynamic parameter segment
    Param(String),
}

/// Route pattern for matching
#[derive(Debug, Clone)]
pub struct Route {
    segments: Vec<Segment>,
    raw_path: String,
}

impl Route {
    /// Create a new route from a path pattern
    pub fn new(path: &str) -> Self {
        let segments = Self::parse_path(path);
        Self {
            segments,
            raw_path: path.to_string(),
        }
    }

    /// Parse a path into segments
    fn parse_path(path: &str) -> Vec<Segment> {
        path.split('/')
            .filter(|s| !s.is_empty())
            .map(|segment| {
                if let Some(stripped) = segment.strip_prefix(':') {
                    Segment::Param(stripped.to_string())
                } else {
                    Segment::Static(segment.to_string())
                }
            })
            .collect()
    }

    /// Match this route against an incoming path
    pub fn matches(&self, path: &str) -> Option<Params> {
        let path_segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        // Must have same number of segments
        if path_segments.len() != self.segments.len() {
            return None;
        }

        let mut params = HashMap::new();

        for (route_seg, path_seg) in self.segments.iter().zip(path_segments.iter()) {
            match route_seg {
                Segment::Static(expected) => {
                    if expected != path_seg {
                        return None;
                    }
                }
                Segment::Param(name) => {
                    params.insert(name.clone(), path_seg.to_string());
                }
            }
        }

        Some(params)
    }

    /// Get the raw path pattern
    pub fn path(&self) -> &str {
        &self.raw_path
    }
}

/// Router entry combining method, route, and handler index
#[derive(Debug, Clone)]
pub struct RouterEntry {
    pub method: Method,
    pub route: Route,
    pub handler_id: usize,
}

/// Main router struct
#[derive(Debug)]
pub struct Router {
    routes: Vec<RouterEntry>,
}

impl Router {
    /// Create a new empty router
    pub fn new() -> Self {
        Self { routes: Vec::new() }
    }

    /// Add a route to the router
    pub fn add_route(&mut self, method: Method, path: &str, handler_id: usize) {
        let route = Route::new(path);
        self.routes.push(RouterEntry {
            method,
            route,
            handler_id,
        });
    }

    /// Find a matching route for the given method and path
    pub fn find_route(&self, method: Method, path: &str) -> Option<(usize, Params)> {
        for entry in &self.routes {
            if entry.method == method {
                if let Some(params) = entry.route.matches(path) {
                    return Some((entry.handler_id, params));
                }
            }
        }
        None
    }

    /// Get all registered routes (useful for debugging)
    pub fn routes(&self) -> &[RouterEntry] {
        &self.routes
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static_route() {
        let route = Route::new("/users");
        assert!(route.matches("/users").is_some());
        assert!(route.matches("/posts").is_none());
        assert!(route.matches("/users/123").is_none());
    }

    #[test]
    fn test_single_param() {
        let route = Route::new("/users/:id");
        let params = route.matches("/users/123");
        assert!(params.is_some());
        let params = params.unwrap();
        assert_eq!(params.get("id"), Some(&"123".to_string()));
    }

    #[test]
    fn test_multiple_params() {
        let route = Route::new("/users/:userId/posts/:postId");
        let params = route.matches("/users/42/posts/100");
        assert!(params.is_some());
        let params = params.unwrap();
        assert_eq!(params.get("userId"), Some(&"42".to_string()));
        assert_eq!(params.get("postId"), Some(&"100".to_string()));
    }

    #[test]
    fn test_no_match_different_length() {
        let route = Route::new("/users/:id");
        assert!(route.matches("/users").is_none());
        assert!(route.matches("/users/123/posts").is_none());
    }

    #[test]
    fn test_router_add_and_find() {
        let mut router = Router::new();
        router.add_route(Method::GET, "/users", 0);
        router.add_route(Method::GET, "/users/:id", 1);
        router.add_route(Method::POST, "/users", 2);

        // Test exact match
        let result = router.find_route(Method::GET, "/users");
        assert!(result.is_some());
        let (handler_id, params) = result.unwrap();
        assert_eq!(handler_id, 0);
        assert!(params.is_empty());

        // Test param match
        let result = router.find_route(Method::GET, "/users/123");
        assert!(result.is_some());
        let (handler_id, params) = result.unwrap();
        assert_eq!(handler_id, 1);
        assert_eq!(params.get("id"), Some(&"123".to_string()));

        // Test method mismatch
        let result = router.find_route(Method::PUT, "/users");
        assert!(result.is_none());
    }

    #[test]
    fn test_root_path() {
        let route = Route::new("/");
        assert!(route.matches("/").is_some());
        assert!(route.matches("/users").is_none());
    }

    #[test]
    fn test_mixed_static_and_params() {
        let route = Route::new("/api/v1/users/:id/profile");
        let params = route.matches("/api/v1/users/123/profile");
        assert!(params.is_some());
        let params = params.unwrap();
        assert_eq!(params.get("id"), Some(&"123".to_string()));
        assert_eq!(params.len(), 1);
    }
}
