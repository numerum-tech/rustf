use super::trie::{RouteInfo, TrieRouter};
use super::Route;
use std::collections::HashMap;

pub struct Router {
    trie: TrieRouter,
    route_count: usize,
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

impl Router {
    /// Create a new high-performance router using Trie data structure
    pub fn new() -> Self {
        Self {
            trie: TrieRouter::new(),
            route_count: 0,
        }
    }

    /// Add a route to the router
    ///
    /// This now uses the high-performance Trie implementation for O(log n) lookup
    pub fn add_route(&mut self, route: Route) {
        let mut info = RouteInfo::new(route.handler)
            .with_before(route.before)
            .with_socket(route.socket);
        if route.xhr_only {
            info = info.xhr_only();
        }
        self.trie.add_route(&route.method, &route.path, info);
        // XHR routes count as 2 (GET + POST)
        self.route_count += if route.method == "XHR" { 2 } else { 1 };
    }

    /// Match a route in the router
    ///
    /// Returns the route info and extracted parameters if a match is found.
    /// Now provides O(log n) performance instead of the previous O(n) implementation.
    pub fn match_route(
        &self,
        method: &str,
        path: &str,
    ) -> Option<(&RouteInfo, HashMap<String, String>)> {
        self.trie.match_route(method, path)
    }

    /// Get the number of routes registered in this router
    pub fn route_count(&self) -> usize {
        self.route_count
    }

    /// Check if the router has any routes
    pub fn is_empty(&self) -> bool {
        self.route_count == 0
    }
}
