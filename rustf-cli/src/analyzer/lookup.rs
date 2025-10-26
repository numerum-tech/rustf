use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
// use anyhow::Result; // unused
use serde::Serialize;

use crate::analyzer::{RouteInfo, ControllerInfo, HandlerInfo};

/// Fast hash-based lookup system for routes and handlers
#[derive(Debug, Clone)]
pub struct AnalysisLookup {
    // Route lookups
    pub routes_by_method: HashMap<String, Vec<RouteInfo>>,
    pub routes_by_path: HashMap<String, Vec<RouteInfo>>,
    pub routes_by_handler: HashMap<String, Vec<RouteInfo>>,
    pub route_hash_index: HashMap<u64, RouteInfo>,
    
    // Handler lookups
    pub handlers_by_name: HashMap<String, Vec<HandlerInfo>>,
    pub handlers_by_controller: HashMap<String, Vec<HandlerInfo>>,
    pub handler_hash_index: HashMap<u64, HandlerInfo>,
    
    // Controller lookups
    pub controllers_by_name: HashMap<String, ControllerInfo>,
    pub controller_hash_index: HashMap<u64, ControllerInfo>,
    
    // Fast pattern matching indexes
    pub route_patterns: Vec<RoutePattern>,
    pub handler_patterns: Vec<HandlerPattern>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RoutePattern {
    pub pattern: String,
    pub regex: String,
    pub parameters: Vec<String>,
    pub method: String,
    pub handler: String,
    pub hash: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct HandlerPattern {
    pub qualified_name: String,
    pub controller: String,
    pub handler: String,
    pub complexity: u32,
    pub route_count: usize,
    pub hash: u64,
}

/// Query results for fast lookups
#[derive(Debug, Serialize)]
pub struct LookupResult<T> {
    pub results: Vec<T>,
    pub query_time_ms: u64,
    pub cache_hit: bool,
}

impl AnalysisLookup {
    /// Create new lookup system from analysis data
    pub fn new(routes: &[RouteInfo], controllers: &[ControllerInfo]) -> Self {
        let mut lookup = AnalysisLookup {
            routes_by_method: HashMap::new(),
            routes_by_path: HashMap::new(),
            routes_by_handler: HashMap::new(),
            route_hash_index: HashMap::new(),
            handlers_by_name: HashMap::new(),
            handlers_by_controller: HashMap::new(),
            handler_hash_index: HashMap::new(),
            controllers_by_name: HashMap::new(),
            controller_hash_index: HashMap::new(),
            route_patterns: Vec::new(),
            handler_patterns: Vec::new(),
        };
        
        lookup.build_route_indexes(routes);
        lookup.build_controller_indexes(controllers);
        lookup.build_pattern_indexes(routes, controllers);
        
        lookup
    }
    
    /// Fast route lookup by HTTP method
    pub fn find_routes_by_method(&self, method: &str) -> LookupResult<RouteInfo> {
        let start = std::time::Instant::now();
        
        let results = self.routes_by_method
            .get(method)
            .cloned()
            .unwrap_or_default();
        
        LookupResult {
            results,
            query_time_ms: start.elapsed().as_millis() as u64,
            cache_hit: true,
        }
    }
    
    /// Fast route lookup by path pattern
    pub fn find_routes_by_path(&self, path: &str) -> LookupResult<RouteInfo> {
        let start = std::time::Instant::now();
        
        // First try exact match
        if let Some(routes) = self.routes_by_path.get(path) {
            return LookupResult {
                results: routes.clone(),
                query_time_ms: start.elapsed().as_millis() as u64,
                cache_hit: true,
            };
        }
        
        // Then try pattern matching
        let mut results = Vec::new();
        for pattern in &self.route_patterns {
            if Self::path_matches_pattern(&pattern.pattern, path) {
                if let Some(route) = self.route_hash_index.get(&pattern.hash) {
                    results.push(route.clone());
                }
            }
        }
        
        LookupResult {
            results,
            query_time_ms: start.elapsed().as_millis() as u64,
            cache_hit: false,
        }
    }
    
    /// Fast handler lookup by name
    pub fn find_handlers_by_name(&self, name: &str) -> LookupResult<HandlerInfo> {
        let start = std::time::Instant::now();
        
        let results = self.handlers_by_name
            .get(name)
            .cloned()
            .unwrap_or_default();
        
        LookupResult {
            results,
            query_time_ms: start.elapsed().as_millis() as u64,
            cache_hit: true,
        }
    }
    
    /// Fast handler lookup by controller
    pub fn find_handlers_by_controller(&self, controller: &str) -> LookupResult<HandlerInfo> {
        let start = std::time::Instant::now();
        
        let results = self.handlers_by_controller
            .get(controller)
            .cloned()
            .unwrap_or_default();
        
        LookupResult {
            results,
            query_time_ms: start.elapsed().as_millis() as u64,
            cache_hit: true,
        }
    }
    
    /// Fast controller lookup by name
    pub fn find_controller_by_name(&self, name: &str) -> Option<ControllerInfo> {
        self.controllers_by_name.get(name).cloned()
    }
    
    /// Find routes that could conflict (same method and overlapping paths)
    pub fn find_potential_conflicts(&self) -> Vec<(RouteInfo, RouteInfo)> {
        let mut conflicts = Vec::new();
        
        for (_method, routes) in &self.routes_by_method {
            for (i, route1) in routes.iter().enumerate() {
                for route2 in routes.iter().skip(i + 1) {
                    if Self::paths_could_conflict(&route1.path, &route2.path) {
                        conflicts.push((route1.clone(), route2.clone()));
                    }
                }
            }
        }
        
        conflicts
    }
    
    /// Get statistics about the lookup indexes
    pub fn get_stats(&self) -> LookupStats {
        LookupStats {
            total_routes: self.route_hash_index.len(),
            total_handlers: self.handler_hash_index.len(),
            total_controllers: self.controller_hash_index.len(),
            routes_by_method: self.routes_by_method.len(),
            routes_by_path: self.routes_by_path.len(),
            handlers_by_name: self.handlers_by_name.len(),
            handlers_by_controller: self.handlers_by_controller.len(),
            route_patterns: self.route_patterns.len(),
            handler_patterns: self.handler_patterns.len(),
        }
    }
    
    /// Advanced search with multiple criteria
    pub fn search_routes(&self, criteria: &RouteCriteria) -> LookupResult<RouteInfo> {
        let start = std::time::Instant::now();
        let mut results = Vec::new();
        
        // Start with the most selective criteria
        let candidates = if let Some(method) = &criteria.method {
            self.routes_by_method.get(method).cloned().unwrap_or_default()
        } else if let Some(handler) = &criteria.handler {
            self.routes_by_handler.get(handler).cloned().unwrap_or_default()
        } else {
            self.route_hash_index.values().cloned().collect()
        };
        
        // Apply additional filters
        for route in candidates {
            let mut matches = true;
            
            if let Some(path) = &criteria.path {
                if !Self::path_matches_pattern(&route.path, path) {
                    matches = false;
                }
            }
            
            if let Some(min_params) = criteria.min_parameters {
                if route.parameters.len() < min_params {
                    matches = false;
                }
            }
            
            if matches {
                results.push(route);
            }
        }
        
        LookupResult {
            results,
            query_time_ms: start.elapsed().as_millis() as u64,
            cache_hit: false,
        }
    }
    
    // Private helper methods
    
    fn build_route_indexes(&mut self, routes: &[RouteInfo]) {
        for route in routes {
            let hash = Self::hash_route(route);
            
            // Method index
            self.routes_by_method
                .entry(route.method.clone())
                .or_insert_with(Vec::new)
                .push(route.clone());
            
            // Path index
            self.routes_by_path
                .entry(route.path.clone())
                .or_insert_with(Vec::new)
                .push(route.clone());
            
            // Handler index
            self.routes_by_handler
                .entry(route.handler.clone())
                .or_insert_with(Vec::new)
                .push(route.clone());
            
            // Hash index
            self.route_hash_index.insert(hash, route.clone());
        }
    }
    
    fn build_controller_indexes(&mut self, controllers: &[ControllerInfo]) {
        for controller in controllers {
            let hash = Self::hash_controller(controller);
            
            // Name index
            self.controllers_by_name.insert(controller.name.clone(), controller.clone());
            
            // Hash index
            self.controller_hash_index.insert(hash, controller.clone());
            
            // Handler indexes
            for handler in &controller.handlers {
                let handler_hash = Self::hash_handler(handler);
                
                // Handler name index
                self.handlers_by_name
                    .entry(handler.name.clone())
                    .or_insert_with(Vec::new)
                    .push(handler.clone());
                
                // Handler by controller index
                self.handlers_by_controller
                    .entry(controller.name.clone())
                    .or_insert_with(Vec::new)
                    .push(handler.clone());
                
                // Handler hash index
                self.handler_hash_index.insert(handler_hash, handler.clone());
            }
        }
    }
    
    fn build_pattern_indexes(&mut self, routes: &[RouteInfo], controllers: &[ControllerInfo]) {
        // Build route patterns
        for route in routes {
            let pattern = RoutePattern {
                pattern: route.path.clone(),
                regex: Self::path_to_regex(&route.path),
                parameters: route.parameters.clone(),
                method: route.method.clone(),
                handler: route.handler.clone(),
                hash: Self::hash_route(route),
            };
            self.route_patterns.push(pattern);
        }
        
        // Build handler patterns
        for controller in controllers {
            for handler in &controller.handlers {
                let pattern = HandlerPattern {
                    qualified_name: handler.qualified_name.clone(),
                    controller: controller.name.clone(),
                    handler: handler.name.clone(),
                    complexity: handler.complexity,
                    route_count: handler.routes.len(),
                    hash: Self::hash_handler(handler),
                };
                self.handler_patterns.push(pattern);
            }
        }
    }
    
    fn hash_route(route: &RouteInfo) -> u64 {
        let mut hasher = DefaultHasher::new();
        route.method.hash(&mut hasher);
        route.path.hash(&mut hasher);
        route.handler.hash(&mut hasher);
        hasher.finish()
    }
    
    fn hash_handler(handler: &HandlerInfo) -> u64 {
        let mut hasher = DefaultHasher::new();
        handler.qualified_name.hash(&mut hasher);
        handler.complexity.hash(&mut hasher);
        hasher.finish()
    }
    
    fn hash_controller(controller: &ControllerInfo) -> u64 {
        let mut hasher = DefaultHasher::new();
        controller.name.hash(&mut hasher);
        controller.file_path.hash(&mut hasher);
        hasher.finish()
    }
    
    fn path_to_regex(path: &str) -> String {
        // Convert RustF path patterns to regex
        // e.g., "/users/{id}" -> "/users/([^/]+)"
        let mut regex = path.to_string();
        regex = regex.replace("{", "([^/]+)");
        regex = regex.replace("}", "");
        regex = format!("^{}$", regex);
        regex
    }
    
    fn path_matches_pattern(pattern: &str, path: &str) -> bool {
        // Simple pattern matching for paths with parameters
        let pattern_parts: Vec<&str> = pattern.split('/').collect();
        let path_parts: Vec<&str> = path.split('/').collect();
        
        if pattern_parts.len() != path_parts.len() {
            return false;
        }
        
        for (pattern_part, path_part) in pattern_parts.iter().zip(path_parts.iter()) {
            if pattern_part.starts_with('{') && pattern_part.ends_with('}') {
                // Parameter - matches any non-empty string
                if path_part.is_empty() {
                    return false;
                }
            } else if pattern_part != path_part {
                return false;
            }
        }
        
        true
    }
    
    fn paths_could_conflict(path1: &str, path2: &str) -> bool {
        // Check if two paths could match the same request
        Self::path_matches_pattern(path1, path2) || Self::path_matches_pattern(path2, path1)
    }
}

/// Search criteria for advanced route queries
#[derive(Debug, Default)]
pub struct RouteCriteria {
    pub method: Option<String>,
    pub path: Option<String>,
    pub handler: Option<String>,
    pub min_parameters: Option<usize>,
}

/// Statistics about lookup performance
#[derive(Debug, Serialize)]
pub struct LookupStats {
    pub total_routes: usize,
    pub total_handlers: usize,
    pub total_controllers: usize,
    pub routes_by_method: usize,
    pub routes_by_path: usize,
    pub handlers_by_name: usize,
    pub handlers_by_controller: usize,
    pub route_patterns: usize,
    pub handler_patterns: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::HandlerInfo;

    fn create_test_route(method: &str, path: &str, handler: &str) -> RouteInfo {
        RouteInfo {
            method: method.to_string(),
            path: path.to_string(),
            handler: handler.to_string(),
            parameters: vec![],
        }
    }

    fn create_test_handler(name: &str, qualified_name: &str) -> HandlerInfo {
        HandlerInfo {
            name: name.to_string(),
            qualified_name: qualified_name.to_string(),
            routes: vec![],
            complexity: 1,
        }
    }

    fn create_test_controller(name: &str, handlers: Vec<HandlerInfo>) -> ControllerInfo {
        ControllerInfo {
            name: name.to_string(),
            file_path: format!("src/controllers/{}.rs", name),
            handlers,
        }
    }

    #[test]
    fn test_route_lookup_by_method() {
        let routes = vec![
            create_test_route("GET", "/users", "users::index"),
            create_test_route("POST", "/users", "users::create"),
            create_test_route("GET", "/posts", "posts::index"),
        ];
        let controllers = vec![];
        
        let lookup = AnalysisLookup::new(&routes, &controllers);
        
        let get_routes = lookup.find_routes_by_method("GET");
        assert_eq!(get_routes.results.len(), 2);
        assert!(get_routes.cache_hit);
        
        let post_routes = lookup.find_routes_by_method("POST");
        assert_eq!(post_routes.results.len(), 1);
        assert!(post_routes.cache_hit);
    }

    #[test]
    fn test_handler_lookup_by_controller() {
        let handlers = vec![
            create_test_handler("index", "users::index"),
            create_test_handler("show", "users::show"),
        ];
        let controllers = vec![
            create_test_controller("users", handlers),
        ];
        let routes = vec![];
        
        let lookup = AnalysisLookup::new(&routes, &controllers);
        
        let user_handlers = lookup.find_handlers_by_controller("users");
        assert_eq!(user_handlers.results.len(), 2);
        assert!(user_handlers.cache_hit);
    }

    #[test]
    fn test_path_pattern_matching() {
        assert!(AnalysisLookup::path_matches_pattern("/users/{id}", "/users/123"));
        assert!(AnalysisLookup::path_matches_pattern("/users/{id}/posts/{post_id}", "/users/123/posts/456"));
        assert!(!AnalysisLookup::path_matches_pattern("/users/{id}", "/posts/123"));
        assert!(!AnalysisLookup::path_matches_pattern("/users/{id}", "/users"));
    }

    #[test]
    fn test_conflict_detection() {
        let routes = vec![
            create_test_route("GET", "/users/{id}", "users::show"),
            create_test_route("GET", "/users/{user_id}", "users::display"),
            create_test_route("GET", "/posts/{id}", "posts::show"),
        ];
        let controllers = vec![];
        
        let lookup = AnalysisLookup::new(&routes, &controllers);
        let conflicts = lookup.find_potential_conflicts();
        
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].0.handler, "users::show");
        assert_eq!(conflicts[0].1.handler, "users::display");
    }

    #[test]
    fn test_advanced_search() {
        let routes = vec![
            create_test_route("GET", "/users/{id}", "users::show"),
            create_test_route("POST", "/users", "users::create"),
            create_test_route("GET", "/posts/{id}", "posts::show"),
        ];
        let controllers = vec![];
        
        let lookup = AnalysisLookup::new(&routes, &controllers);
        
        let criteria = RouteCriteria {
            method: Some("GET".to_string()),
            ..Default::default()
        };
        
        let results = lookup.search_routes(&criteria);
        assert_eq!(results.results.len(), 2);
    }
}