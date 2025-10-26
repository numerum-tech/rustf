//! Search utilities for finding components and patterns

use crate::analyzer::{ProjectAnalysis, ControllerInfo, RouteInfo, HandlerInfo, MiddlewareInfo, ModelInfo, Issue};
use crate::analysis::views::ViewAnalysis;
use regex::Regex;

/// Search builder for complex queries
pub struct SearchBuilder {
    query: SearchQuery,
}

#[derive(Debug, Clone, Default)]
pub struct SearchQuery {
    pub name_pattern: Option<String>,
    pub method_filter: Option<String>,
    pub complexity_min: Option<u32>,
    pub complexity_max: Option<u32>,
    pub has_parameters: Option<bool>,
    pub severity_filter: Option<String>,
    pub case_sensitive: bool,
    pub use_regex: bool,
}

impl SearchBuilder {
    pub fn new() -> Self {
        Self {
            query: SearchQuery::default(),
        }
    }
    
    pub fn name_pattern(mut self, pattern: &str) -> Self {
        self.query.name_pattern = Some(pattern.to_string());
        self
    }
    
    pub fn method_filter(mut self, method: &str) -> Self {
        self.query.method_filter = Some(method.to_uppercase());
        self
    }
    
    pub fn complexity_range(mut self, min: u32, max: u32) -> Self {
        self.query.complexity_min = Some(min);
        self.query.complexity_max = Some(max);
        self
    }
    
    pub fn has_parameters(mut self, has_params: bool) -> Self {
        self.query.has_parameters = Some(has_params);
        self
    }
    
    pub fn severity_filter(mut self, severity: &str) -> Self {
        self.query.severity_filter = Some(severity.to_string());
        self
    }
    
    pub fn case_sensitive(mut self, sensitive: bool) -> Self {
        self.query.case_sensitive = sensitive;
        self
    }
    
    pub fn use_regex(mut self, regex: bool) -> Self {
        self.query.use_regex = regex;
        self
    }
    
    pub fn search_controllers(self, controllers: &[ControllerInfo]) -> Vec<&ControllerInfo> {
        controllers.iter()
            .filter(|controller| self.matches_controller(controller))
            .collect()
    }
    
    pub fn search_routes(self, routes: &[RouteInfo]) -> Vec<&RouteInfo> {
        routes.iter()
            .filter(|route| self.matches_route(route))
            .collect()
    }
    
    pub fn search_handlers(self, controllers: &[ControllerInfo]) -> Vec<&HandlerInfo> {
        controllers.iter()
            .flat_map(|c| &c.handlers)
            .filter(|handler| self.matches_handler(handler))
            .collect()
    }
    
    pub fn search_middleware(self, middleware: &[MiddlewareInfo]) -> Vec<&MiddlewareInfo> {
        middleware.iter()
            .filter(|mw| self.matches_middleware(mw))
            .collect()
    }
    
    pub fn search_models(self, models: &[ModelInfo]) -> Vec<&ModelInfo> {
        models.iter()
            .filter(|model| self.matches_model(model))
            .collect()
    }
    
    pub fn search_views(self, views: &[ViewAnalysis]) -> Vec<&ViewAnalysis> {
        views.iter()
            .filter(|view| self.matches_view(view))
            .collect()
    }
    
    pub fn search_issues(self, issues: &[Issue]) -> Vec<&Issue> {
        issues.iter()
            .filter(|issue| self.matches_issue(issue))
            .collect()
    }
    
    fn matches_controller(&self, controller: &ControllerInfo) -> bool {
        if let Some(ref pattern) = self.query.name_pattern {
            if !self.matches_string_pattern(&controller.name, pattern) {
                return false;
            }
        }
        
        // Check complexity range against handler complexity
        if let (Some(min), Some(max)) = (self.query.complexity_min, self.query.complexity_max) {
            let has_handler_in_range = controller.handlers.iter()
                .any(|h| h.complexity >= min && h.complexity <= max);
            if !has_handler_in_range {
                return false;
            }
        }
        
        true
    }
    
    fn matches_route(&self, route: &RouteInfo) -> bool {
        if let Some(ref pattern) = self.query.name_pattern {
            let matches_path = self.matches_string_pattern(&route.path, pattern);
            let matches_handler = self.matches_string_pattern(&route.handler, pattern);
            if !matches_path && !matches_handler {
                return false;
            }
        }
        
        if let Some(ref method) = self.query.method_filter {
            if route.method.to_uppercase() != *method {
                return false;
            }
        }
        
        if let Some(has_params) = self.query.has_parameters {
            let route_has_params = !route.parameters.is_empty();
            if route_has_params != has_params {
                return false;
            }
        }
        
        true
    }
    
    fn matches_handler(&self, handler: &HandlerInfo) -> bool {
        if let Some(ref pattern) = self.query.name_pattern {
            let matches_name = self.matches_string_pattern(&handler.name, pattern);
            let matches_qualified = self.matches_string_pattern(&handler.qualified_name, pattern);
            if !matches_name && !matches_qualified {
                return false;
            }
        }
        
        if let Some(min) = self.query.complexity_min {
            if handler.complexity < min {
                return false;
            }
        }
        
        if let Some(max) = self.query.complexity_max {
            if handler.complexity > max {
                return false;
            }
        }
        
        true
    }
    
    fn matches_middleware(&self, middleware: &MiddlewareInfo) -> bool {
        if let Some(ref pattern) = self.query.name_pattern {
            let matches_name = self.matches_string_pattern(&middleware.name, pattern);
            let matches_type = self.matches_string_pattern(&middleware.middleware_type, pattern);
            if !matches_name && !matches_type {
                return false;
            }
        }
        
        true
    }
    
    fn matches_model(&self, model: &ModelInfo) -> bool {
        if let Some(ref pattern) = self.query.name_pattern {
            if !self.matches_string_pattern(&model.name, pattern) {
                return false;
            }
        }
        
        true
    }
    
    fn matches_view(&self, view: &ViewAnalysis) -> bool {
        if let Some(ref pattern) = self.query.name_pattern {
            if !self.matches_string_pattern(&view.name, pattern) {
                return false;
            }
        }
        
        if let Some(min) = self.query.complexity_min {
            if view.complexity_metrics.complexity_score < min {
                return false;
            }
        }
        
        if let Some(max) = self.query.complexity_max {
            if view.complexity_metrics.complexity_score > max {
                return false;
            }
        }
        
        true
    }
    
    fn matches_issue(&self, issue: &Issue) -> bool {
        if let Some(ref pattern) = self.query.name_pattern {
            if !self.matches_string_pattern(&issue.message, pattern) {
                return false;
            }
        }
        
        if let Some(ref severity) = self.query.severity_filter {
            if !issue.severity.eq_ignore_ascii_case(severity) {
                return false;
            }
        }
        
        true
    }
    
    fn matches_string_pattern(&self, text: &str, pattern: &str) -> bool {
        if self.query.use_regex {
            if let Ok(regex) = Regex::new(pattern) {
                return regex.is_match(text);
            }
        }
        
        if self.query.case_sensitive {
            text.contains(pattern)
        } else {
            text.to_lowercase().contains(&pattern.to_lowercase())
        }
    }
}

/// Quick search functions for common queries
pub struct QuickSearch;

impl QuickSearch {
    /// Find routes by path pattern
    pub fn routes_by_path<'a>(routes: &'a [RouteInfo], path_pattern: &str) -> Vec<&'a RouteInfo> {
        SearchBuilder::new()
            .name_pattern(path_pattern)
            .search_routes(routes)
    }
    
    /// Find routes by HTTP method
    pub fn routes_by_method<'a>(routes: &'a [RouteInfo], method: &str) -> Vec<&'a RouteInfo> {
        SearchBuilder::new()
            .method_filter(method)
            .search_routes(routes)
    }
    
    /// Find high complexity handlers
    pub fn high_complexity_handlers<'a>(controllers: &'a [ControllerInfo], threshold: u32) -> Vec<&'a HandlerInfo> {
        SearchBuilder::new()
            .complexity_range(threshold, u32::MAX)
            .search_handlers(controllers)
    }
    
    /// Find controllers by name
    pub fn controllers_by_name<'a>(controllers: &'a [ControllerInfo], name_pattern: &str) -> Vec<&'a ControllerInfo> {
        SearchBuilder::new()
            .name_pattern(name_pattern)
            .search_controllers(controllers)
    }
    
    /// Find parameterized routes
    pub fn parameterized_routes<'a>(routes: &'a [RouteInfo]) -> Vec<&'a RouteInfo> {
        SearchBuilder::new()
            .has_parameters(true)
            .search_routes(routes)
    }
    
    /// Find error-level issues
    pub fn error_issues<'a>(issues: &'a [Issue]) -> Vec<&'a Issue> {
        SearchBuilder::new()
            .severity_filter("error")
            .search_issues(issues)
    }
    
    /// Find views with security issues
    pub fn risky_views<'a>(views: &'a [ViewAnalysis]) -> Vec<&'a ViewAnalysis> {
        views.iter()
            .filter(|view| !view.security_issues.is_empty())
            .collect()
    }
}

/// Search result aggregator
pub struct SearchResults<'a> {
    pub controllers: Vec<&'a ControllerInfo>,
    pub routes: Vec<&'a RouteInfo>,
    pub handlers: Vec<&'a HandlerInfo>,
    pub middleware: Vec<&'a MiddlewareInfo>,
    pub models: Vec<&'a ModelInfo>,
    pub views: Vec<&'a ViewAnalysis>,
    pub issues: Vec<&'a Issue>,
}

impl<'a> SearchResults<'a> {
    pub fn new() -> Self {
        Self {
            controllers: Vec::new(),
            routes: Vec::new(),
            handlers: Vec::new(),
            middleware: Vec::new(),
            models: Vec::new(),
            views: Vec::new(),
            issues: Vec::new(),
        }
    }
    
    pub fn is_empty(&self) -> bool {
        self.controllers.is_empty() &&
        self.routes.is_empty() &&
        self.handlers.is_empty() &&
        self.middleware.is_empty() &&
        self.models.is_empty() &&
        self.views.is_empty() &&
        self.issues.is_empty()
    }
    
    pub fn total_results(&self) -> usize {
        self.controllers.len() +
        self.routes.len() +
        self.handlers.len() +
        self.middleware.len() +
        self.models.len() +
        self.views.len() +
        self.issues.len()
    }
}

/// Global search across all components
pub fn global_search(analysis: &ProjectAnalysis, query: SearchQuery) -> SearchResults {
    let builder = SearchBuilder { query };
    
    SearchResults {
        controllers: builder.query.clone().into(),
        routes: SearchBuilder { query: builder.query.clone() }.search_routes(&analysis.routes),
        handlers: SearchBuilder { query: builder.query.clone() }.search_handlers(&analysis.controllers),
        middleware: SearchBuilder { query: builder.query.clone() }.search_middleware(&analysis.middleware),
        models: SearchBuilder { query: builder.query.clone() }.search_models(&analysis.models),
        views: SearchBuilder { query: builder.query.clone() }.search_views(&analysis.views),
        issues: SearchBuilder { query: builder.query }.search_issues(&analysis.issues),
    }
}

impl From<SearchQuery> for Vec<&ControllerInfo> {
    fn from(_: SearchQuery) -> Self {
        Vec::new() // Placeholder - would need actual implementation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_builder() {
        let routes = vec![
            RouteInfo {
                method: "GET".to_string(),
                path: "/users/{id}".to_string(),
                handler: "get_user".to_string(),
                parameters: vec!["id".to_string()],
            },
            RouteInfo {
                method: "POST".to_string(),
                path: "/users".to_string(),
                handler: "create_user".to_string(),
                parameters: vec![],
            },
        ];
        
        let results = SearchBuilder::new()
            .method_filter("GET")
            .has_parameters(true)
            .search_routes(&routes);
        
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, "/users/{id}");
    }
    
    #[test]
    fn test_quick_search() {
        let routes = vec![
            RouteInfo {
                method: "GET".to_string(),
                path: "/api/users".to_string(),
                handler: "get_users".to_string(),
                parameters: vec![],
            },
        ];
        
        let results = QuickSearch::routes_by_path(&routes, "/api/");
        assert_eq!(results.len(), 1);
        
        let get_routes = QuickSearch::routes_by_method(&routes, "GET");
        assert_eq!(get_routes.len(), 1);
    }
}