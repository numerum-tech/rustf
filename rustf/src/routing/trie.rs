//! High-performance Trie-based router for RustF
//!
//! This module implements a radix trie (compressed trie) for efficient route matching.
//! It provides O(log n) route matching instead of the previous O(n) implementation.

use super::RouteHandler;
use std::collections::HashMap;
use std::fmt::Debug;

/// Route information stored at trie nodes
#[derive(Clone, Debug)]
pub struct RouteInfo {
    pub handler: RouteHandler,
    pub xhr_only: bool,
}

/// A Trie node that can contain route handlers and parameters
#[derive(Debug)]
struct TrieNode {
    /// Exact path segment match
    static_children: HashMap<String, TrieNode>,
    /// Dynamic parameter match (e.g., {id})
    param_child: Option<(String, Box<TrieNode>)>,
    /// Wildcard match (e.g., * or **)
    wildcard_child: Option<Box<TrieNode>>,
    /// Route information for each HTTP method at this node
    handlers: HashMap<String, RouteInfo>,
}

impl TrieNode {
    fn new() -> Self {
        Self {
            static_children: HashMap::new(),
            param_child: None,
            wildcard_child: None,
            handlers: HashMap::new(),
        }
    }
}

/// High-performance Trie-based router
///
/// Provides efficient route matching using a radix trie data structure.
/// Routes are pre-compiled into the trie at startup for O(log n) lookup performance.
pub struct TrieRouter {
    root: TrieNode,
    route_count: usize,
}

impl Default for TrieRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl TrieRouter {
    /// Create a new empty trie router
    pub fn new() -> Self {
        Self {
            root: TrieNode::new(),
            route_count: 0,
        }
    }

    /// Add a route to the trie
    ///
    /// # Arguments
    /// * `method` - HTTP method (GET, POST, etc.)
    /// * `path` - Route path with optional parameters (e.g., "/users/{id}")
    /// * `handler` - Route handler function
    /// * `xhr_only` - Whether this route requires XHR/AJAX requests
    pub fn add_route(&mut self, method: &str, path: &str, handler: RouteHandler, xhr_only: bool) {
        let segments = self.parse_path(path);
        let mut current = &mut self.root;

        // Navigate/create the trie path
        for segment in segments {
            match segment {
                PathSegment::Static(segment_str) => {
                    current = current
                        .static_children
                        .entry(segment_str)
                        .or_insert_with(TrieNode::new);
                }
                PathSegment::Parameter(param_name) => {
                    if current.param_child.is_none() {
                        current.param_child = Some((param_name, Box::new(TrieNode::new())));
                    }
                    current = &mut current.param_child.as_mut().unwrap().1;
                }
                PathSegment::Wildcard => {
                    if current.wildcard_child.is_none() {
                        current.wildcard_child = Some(Box::new(TrieNode::new()));
                    }
                    current = current.wildcard_child.as_mut().unwrap();
                }
            }
        }

        // Add the handler at the final node
        // For XHR routes, we store with "XHR" as method but match on GET/POST
        if method == "XHR" {
            // XHR routes match both GET and POST
            current
                .handlers
                .insert("GET".to_string(), RouteInfo { handler, xhr_only });
            current
                .handlers
                .insert("POST".to_string(), RouteInfo { handler, xhr_only });
            self.route_count += 2;
        } else {
            current
                .handlers
                .insert(method.to_uppercase(), RouteInfo { handler, xhr_only });
            self.route_count += 1;
        }
    }

    /// Match a route in the trie
    ///
    /// Returns the route info and extracted parameters if a match is found.
    pub fn match_route(
        &self,
        method: &str,
        path: &str,
    ) -> Option<(&RouteInfo, HashMap<String, String>)> {
        // Remove query parameters
        let path_only = if let Some(query_start) = path.find('?') {
            &path[..query_start]
        } else {
            path
        };

        let segments: Vec<&str> = path_only
            .trim_start_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();

        let mut params = HashMap::new();

        if let Some(node) = self.match_segments(&self.root, &segments, 0, &mut params) {
            if let Some(route_info) = node.handlers.get(&method.to_uppercase()) {
                return Some((route_info, params));
            }
        }

        None
    }

    /// Get the number of routes registered
    pub fn route_count(&self) -> usize {
        self.route_count
    }

    /// Parse a path into segments
    fn parse_path(&self, path: &str) -> Vec<PathSegment> {
        let path_clean = path.trim_start_matches('/');
        if path_clean.is_empty() {
            return vec![];
        }

        path_clean
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|segment| {
                if segment == "*" {
                    PathSegment::Wildcard
                } else if segment.starts_with('{') && segment.ends_with('}') {
                    let param_name = segment[1..segment.len() - 1].to_string();
                    PathSegment::Parameter(param_name)
                } else {
                    PathSegment::Static(segment.to_string())
                }
            })
            .collect()
    }

    /// Recursively match segments in the trie
    fn match_segments<'a>(
        &'a self,
        node: &'a TrieNode,
        segments: &[&str],
        index: usize,
        params: &mut HashMap<String, String>,
    ) -> Option<&'a TrieNode> {
        // If we've consumed all segments, return current node
        if index >= segments.len() {
            return Some(node);
        }

        let current_segment = segments[index];

        // Try static match first (most specific)
        if let Some(child) = node.static_children.get(current_segment) {
            if let Some(result) = self.match_segments(child, segments, index + 1, params) {
                return Some(result);
            }
        }

        // Try parameter match
        if let Some((param_name, child)) = &node.param_child {
            params.insert(param_name.clone(), current_segment.to_string());
            if let Some(result) = self.match_segments(child, segments, index + 1, params) {
                return Some(result);
            }
            // Remove parameter if match failed
            params.remove(param_name);
        }

        // Try wildcard match (least specific)
        if let Some(child) = &node.wildcard_child {
            // Wildcard matches remaining segments
            return Some(child);
        }

        None
    }
}

/// Path segment types for trie construction
#[derive(Debug, Clone)]
enum PathSegment {
    /// Static segment (exact match)
    Static(String),
    /// Parameter segment (e.g., {id})
    Parameter(String),
    /// Wildcard segment (*)
    Wildcard,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::Context;
    use crate::error::Result;
    use std::future::Future;
    use std::pin::Pin;

    // Mock handler for testing
    fn mock_handler(_ctx: &mut Context) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }

    #[test]
    fn test_static_routes() {
        let mut router = TrieRouter::new();
        router.add_route("GET", "/", mock_handler as RouteHandler, false);
        router.add_route("GET", "/users", mock_handler as RouteHandler, false);
        router.add_route("GET", "/users/profile", mock_handler as RouteHandler, false);

        assert!(router.match_route("GET", "/").is_some());
        assert!(router.match_route("GET", "/users").is_some());
        assert!(router.match_route("GET", "/users/profile").is_some());
        assert!(router.match_route("GET", "/nonexistent").is_none());
        assert!(router.match_route("POST", "/users").is_none());
    }

    #[test]
    fn test_parameter_routes() {
        let mut router = TrieRouter::new();
        router.add_route("GET", "/users/{id}", mock_handler as RouteHandler, false);
        router.add_route(
            "GET",
            "/users/{id}/posts/{post_id}",
            mock_handler as RouteHandler,
            false,
        );

        let (_, params) = router.match_route("GET", "/users/123").unwrap();
        assert_eq!(params.get("id"), Some(&"123".to_string()));

        let (_, params) = router.match_route("GET", "/users/456/posts/789").unwrap();
        assert_eq!(params.get("id"), Some(&"456".to_string()));
        assert_eq!(params.get("post_id"), Some(&"789".to_string()));
    }

    #[test]
    fn test_route_priority() {
        let mut router = TrieRouter::new();
        router.add_route("GET", "/users/special", mock_handler as RouteHandler, false);
        router.add_route("GET", "/users/{id}", mock_handler as RouteHandler, false);

        // Static route should take priority over parameter route
        let (_, params) = router.match_route("GET", "/users/special").unwrap();
        assert!(params.is_empty()); // Should match static route, not parameter

        let (_, params) = router.match_route("GET", "/users/123").unwrap();
        assert_eq!(params.get("id"), Some(&"123".to_string()));
    }

    #[test]
    fn test_query_parameters_ignored() {
        let mut router = TrieRouter::new();
        router.add_route("GET", "/search", mock_handler as RouteHandler, false);

        assert!(router
            .match_route("GET", "/search?q=test&limit=10")
            .is_some());
        assert!(router.match_route("GET", "/search?").is_some());
    }

    #[test]
    fn test_route_count() {
        let mut router = TrieRouter::new();
        assert_eq!(router.route_count(), 0);

        router.add_route("GET", "/", mock_handler, false);
        assert_eq!(router.route_count(), 1);

        router.add_route("POST", "/", mock_handler as RouteHandler, false);
        assert_eq!(router.route_count(), 2);

        router.add_route("GET", "/users", mock_handler as RouteHandler, false);
        assert_eq!(router.route_count(), 3);
    }
}
