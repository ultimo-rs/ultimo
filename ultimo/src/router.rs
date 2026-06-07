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

    /// Route specificity: the number of static segments. Higher is more
    /// specific, so a fully-static route outranks one with parameters.
    fn specificity(&self) -> usize {
        self.segments
            .iter()
            .filter(|s| matches!(s, Segment::Static(_)))
            .count()
    }

    /// The normalized lookup key for a fully-static route (segments joined by
    /// `/`), or `None` if the route has any parameter. Matches `normalize_path`.
    fn static_key(&self) -> Option<String> {
        let mut parts: Vec<&str> = Vec::with_capacity(self.segments.len());
        for seg in &self.segments {
            match seg {
                Segment::Static(s) => parts.push(s),
                Segment::Param(_) => return None,
            }
        }
        Some(parts.join("/"))
    }
}

/// Normalize a request path to a static-route key: non-empty segments joined by
/// `/`. Trailing and duplicate slashes are ignored, matching `Route::matches`.
fn normalize_path(path: &str) -> String {
    path.split('/')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("/")
}

/// Router entry combining method, route, and handler index
#[derive(Debug, Clone)]
pub struct RouterEntry {
    pub method: Method,
    pub route: Route,
    pub handler_id: usize,
}

/// Main router struct.
///
/// Lookup is split for speed: fully-static routes go in an O(1) hash index
/// keyed by `(method, normalized-path)`, and only parameterized routes are
/// scanned. Because a fully-static match is always the most specific possible
/// for a path, a hit in the static index wins outright — so the precedence
/// guarantee (static beats param, ties by registration order) is preserved
/// while avoiding an O(N) scan over every registered route.
#[derive(Debug)]
pub struct Router {
    /// All routes in registration order — for `routes()` / introspection.
    routes: Vec<RouterEntry>,
    /// O(1) exact lookup for fully-static routes. First registration wins.
    static_index: HashMap<(Method, String), usize>,
    /// Parameterized routes only, scanned when there's no static match.
    dynamic: Vec<RouterEntry>,
}

impl Router {
    /// Create a new empty router
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            static_index: HashMap::new(),
            dynamic: Vec::new(),
        }
    }

    /// Add a route to the router
    pub fn add_route(&mut self, method: Method, path: &str, handler_id: usize) {
        let route = Route::new(path);
        let entry = RouterEntry {
            method,
            route: route.clone(),
            handler_id,
        };
        match route.static_key() {
            // First registration wins (preserves the prior tie-break semantics).
            Some(key) => {
                self.static_index.entry((method, key)).or_insert(handler_id);
            }
            None => self.dynamic.push(entry.clone()),
        }
        self.routes.push(entry);
    }

    /// Find the best-matching route for the given method and path.
    ///
    /// A fully-static match is the most specific possible for a path, so it wins
    /// outright (O(1) via the static index). Otherwise only parameterized routes
    /// are scanned; the most specific wins, ties broken by registration order.
    pub fn find_route(&self, method: Method, path: &str) -> Option<(usize, Params)> {
        // Fast path: exact static match.
        let key = normalize_path(path);
        if let Some(&handler_id) = self.static_index.get(&(method, key)) {
            return Some((handler_id, Params::new()));
        }
        // Slow path: scan only the parameterized routes.
        let mut best: Option<(usize, Params, usize)> = None;
        for entry in &self.dynamic {
            if entry.method == method {
                if let Some(params) = entry.route.matches(path) {
                    let spec = entry.route.specificity();
                    let better = match &best {
                        Some((_, _, best_spec)) => spec > *best_spec,
                        None => true,
                    };
                    if better {
                        best = Some((entry.handler_id, params, spec));
                    }
                }
            }
        }
        best.map(|(id, params, _)| (id, params))
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

    #[test]
    fn static_route_beats_param_regardless_of_order() {
        // Param registered FIRST, static SECOND — static must still win.
        let mut r = Router::new();
        r.add_route(Method::GET, "/users/:id", 1);
        r.add_route(Method::GET, "/users/me", 2);
        let (id, _) = r.find_route(Method::GET, "/users/me").unwrap();
        assert_eq!(id, 2, "static /users/me should win over /users/:id");
        // and the param route still matches other values
        let (id, params) = r.find_route(Method::GET, "/users/42").unwrap();
        assert_eq!(id, 1);
        assert_eq!(params.get("id"), Some(&"42".to_string()));
    }

    #[test]
    fn trailing_slash_is_ignored() {
        let mut r = Router::new();
        r.add_route(Method::GET, "/users", 1);
        assert!(r.find_route(Method::GET, "/users/").is_some());
        assert!(r.find_route(Method::GET, "/users").is_some());
    }

    #[test]
    fn no_match_returns_none() {
        let mut r = Router::new();
        r.add_route(Method::GET, "/users/me", 1);
        assert!(r.find_route(Method::GET, "/posts").is_none());
        assert!(r.find_route(Method::POST, "/users/me").is_none());
    }

    #[test]
    fn many_static_routes_resolve_correctly() {
        // The static index must return the right handler regardless of table size.
        let mut r = Router::new();
        for i in 0..500 {
            r.add_route(Method::GET, &format!("/route/{i}"), i);
        }
        let (id, params) = r.find_route(Method::GET, "/route/250").unwrap();
        assert_eq!(id, 250);
        assert!(params.is_empty());
        assert!(r.find_route(Method::GET, "/route/999").is_none());
    }

    #[test]
    fn duplicate_static_route_keeps_first_registration() {
        // Two registrations of the same static path: the first wins.
        let mut r = Router::new();
        r.add_route(Method::GET, "/dup", 1);
        r.add_route(Method::GET, "/dup", 2);
        let (id, _) = r.find_route(Method::GET, "/dup").unwrap();
        assert_eq!(id, 1);
    }

    #[test]
    fn root_path_uses_static_index() {
        let mut r = Router::new();
        r.add_route(Method::GET, "/", 7);
        let (id, _) = r.find_route(Method::GET, "/").unwrap();
        assert_eq!(id, 7);
    }

    #[test]
    fn method_is_part_of_the_static_key() {
        let mut r = Router::new();
        r.add_route(Method::GET, "/x", 1);
        r.add_route(Method::POST, "/x", 2);
        assert_eq!(r.find_route(Method::GET, "/x").unwrap().0, 1);
        assert_eq!(r.find_route(Method::POST, "/x").unwrap().0, 2);
    }
}
