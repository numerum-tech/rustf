//! Validation utilities for project structure and conventions

use crate::analyzer::{ProjectAnalysis, RouteInfo};
use std::path::Path;
use std::collections::{HashMap, HashSet};

/// Project validation rules
pub struct ProjectValidator;

impl ProjectValidator {
    /// Validate naming conventions
    pub fn validate_naming_conventions(analysis: &ProjectAnalysis) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        
        // Controller naming
        for controller in &analysis.controllers {
            if !Self::is_valid_controller_name(&controller.name) {
                issues.push(ValidationIssue {
                    severity: ValidationSeverity::Warning,
                    category: ValidationCategory::NamingConvention,
                    message: format!("Controller '{}' should follow snake_case convention", controller.name),
                    location: Some(controller.file_path.clone()),
                    suggestion: Some(Self::to_snake_case(&controller.name)),
                });
            }
        }
        
        // Route naming
        for route in &analysis.routes {
            if !Self::is_valid_route_path(&route.path) {
                issues.push(ValidationIssue {
                    severity: ValidationSeverity::Info,
                    category: ValidationCategory::NamingConvention,
                    message: format!("Route path '{}' should follow REST conventions", route.path),
                    location: None,
                    suggestion: None,
                });
            }
        }
        
        // Handler naming
        for controller in &analysis.controllers {
            for handler in &controller.handlers {
                if !Self::is_valid_handler_name(&handler.name) {
                    issues.push(ValidationIssue {
                        severity: ValidationSeverity::Warning,
                        category: ValidationCategory::NamingConvention,
                        message: format!("Handler '{}' should follow snake_case convention", handler.name),
                        location: Some(controller.file_path.clone()),
                        suggestion: Some(Self::to_snake_case(&handler.name)),
                    });
                }
            }
        }
        
        issues
    }
    
    /// Validate route consistency
    pub fn validate_route_consistency(analysis: &ProjectAnalysis) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        
        // Check for duplicate routes
        let mut route_map: HashMap<String, Vec<&RouteInfo>> = HashMap::new();
        for route in &analysis.routes {
            let key = format!("{} {}", route.method, route.path);
            route_map.entry(key).or_insert_with(Vec::new).push(route);
        }
        
        for (route_key, routes) in route_map {
            if routes.len() > 1 {
                let handlers: Vec<String> = routes.iter().map(|r| r.handler.clone()).collect();
                issues.push(ValidationIssue {
                    severity: ValidationSeverity::Error,
                    category: ValidationCategory::RouteConflict,
                    message: format!("Duplicate route: {} (handlers: {})", route_key, handlers.join(", ")),
                    location: None,
                    suggestion: Some("Ensure each route has a unique method/path combination".to_string()),
                });
            }
        }
        
        // Check for missing handlers
        let handler_names: HashSet<String> = analysis.controllers.iter()
            .flat_map(|c| &c.handlers)
            .map(|h| h.qualified_name.clone())
            .collect();
        
        for route in &analysis.routes {
            if !handler_names.contains(&route.handler) {
                issues.push(ValidationIssue {
                    severity: ValidationSeverity::Error,
                    category: ValidationCategory::MissingHandler,
                    message: format!("Route {} {} references missing handler: {}", route.method, route.path, route.handler),
                    location: None,
                    suggestion: Some(format!("Implement handler function '{}'", route.handler)),
                });
            }
        }
        
        issues
    }
    
    /// Validate complexity thresholds
    pub fn validate_complexity_thresholds(analysis: &ProjectAnalysis) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        
        for controller in &analysis.controllers {
            for handler in &controller.handlers {
                let severity = match handler.complexity {
                    0..=10 => continue, // Good complexity
                    11..=20 => ValidationSeverity::Warning,
                    21..=30 => ValidationSeverity::Error,
                    _ => ValidationSeverity::Critical,
                };
                
                issues.push(ValidationIssue {
                    severity,
                    category: ValidationCategory::Complexity,
                    message: format!("Handler '{}' has high complexity: {}", handler.name, handler.complexity),
                    location: Some(controller.file_path.clone()),
                    suggestion: Some("Consider refactoring into smaller functions".to_string()),
                });
            }
        }
        
        issues
    }
    
    /// Validate REST API conventions
    pub fn validate_rest_conventions(analysis: &ProjectAnalysis) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        
        for route in &analysis.routes {
            let rest_issue = Self::check_rest_conventions(&route.method, &route.path, &route.handler);
            if let Some(issue) = rest_issue {
                issues.push(ValidationIssue {
                    severity: ValidationSeverity::Info,
                    category: ValidationCategory::RestConvention,
                    message: issue,
                    location: None,
                    suggestion: None,
                });
            }
        }
        
        issues
    }
    
    /// Validate security practices
    pub fn validate_security_practices(analysis: &ProjectAnalysis) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        
        // Check for views with security issues
        for view in &analysis.views {
            for security_issue in &view.security_issues {
                let severity = match security_issue.severity.as_str() {
                    "high" => ValidationSeverity::Critical,
                    "medium" => ValidationSeverity::Error,
                    "low" => ValidationSeverity::Warning,
                    _ => ValidationSeverity::Info,
                };
                
                issues.push(ValidationIssue {
                    severity,
                    category: ValidationCategory::Security,
                    message: format!("Security issue in view '{}': {}", view.name, security_issue.description),
                    location: Some(view.file_path.clone()),
                    suggestion: Some(security_issue.recommendation.clone()),
                });
            }
        }
        
        // Check for potential security anti-patterns in routes
        for route in &analysis.routes {
            if route.method == "GET" && route.path.to_lowercase().contains("delete") {
                issues.push(ValidationIssue {
                    severity: ValidationSeverity::Warning,
                    category: ValidationCategory::Security,
                    message: format!("Potential security issue: DELETE operation using GET method in route {}", route.path),
                    location: None,
                    suggestion: Some("Use DELETE or POST method for destructive operations".to_string()),
                });
            }
        }
        
        issues
    }
    
    // Helper methods
    fn is_valid_controller_name(name: &str) -> bool {
        name.chars().all(|c| c.is_ascii_lowercase() || c == '_') && !name.starts_with('_') && !name.ends_with('_')
    }
    
    fn is_valid_handler_name(name: &str) -> bool {
        name.chars().all(|c| c.is_ascii_lowercase() || c == '_') && !name.starts_with('_') && !name.ends_with('_')
    }
    
    fn is_valid_route_path(path: &str) -> bool {
        path.starts_with('/') && !path.ends_with('/') || path == "/"
    }
    
    fn to_snake_case(s: &str) -> String {
        let mut result = String::new();
        let mut chars = s.chars().peekable();
        
        while let Some(c) = chars.next() {
            if c.is_ascii_uppercase() {
                if !result.is_empty() && chars.peek().map_or(false, |&next| next.is_ascii_lowercase()) {
                    result.push('_');
                }
                result.push(c.to_ascii_lowercase());
            } else {
                result.push(c);
            }
        }
        
        result
    }
    
    fn check_rest_conventions(method: &str, path: &str, handler: &str) -> Option<String> {
        let path_segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        
        if path_segments.is_empty() {
            return None;
        }
        
        let resource = path_segments[0];
        let handler_lower = handler.to_lowercase();
        
        match method {
            "GET" => {
                if path_segments.len() == 1 && !handler_lower.contains("list") && !handler_lower.contains("index") {
                    Some(format!("GET {} should typically have a handler like 'list_{}' or 'index_{}'", path, resource, resource))
                } else if path_segments.len() > 1 && !handler_lower.contains("get") && !handler_lower.contains("show") {
                    Some(format!("GET {} should typically have a handler like 'get_{}' or 'show_{}'", path, resource, resource))
                } else {
                    None
                }
            }
            "POST" => {
                if !handler_lower.contains("create") && !handler_lower.contains("store") {
                    Some(format!("POST {} should typically have a handler like 'create_{}' or 'store_{}'", path, resource, resource))
                } else {
                    None
                }
            }
            "PUT" | "PATCH" => {
                if !handler_lower.contains("update") && !handler_lower.contains("edit") {
                    Some(format!("{} {} should typically have a handler like 'update_{}' or 'edit_{}'", method, path, resource, resource))
                } else {
                    None
                }
            }
            "DELETE" => {
                if !handler_lower.contains("delete") && !handler_lower.contains("destroy") {
                    Some(format!("DELETE {} should typically have a handler like 'delete_{}' or 'destroy_{}'", path, resource, resource))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationCategory {
    NamingConvention,
    RouteConflict,
    MissingHandler,
    Complexity,
    RestConvention,
    Security,
    Performance,
    Structure,
}

#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub severity: ValidationSeverity,
    pub category: ValidationCategory,
    pub message: String,
    pub location: Option<String>,
    pub suggestion: Option<String>,
}

impl ValidationIssue {
    pub fn severity_icon(&self) -> &'static str {
        match self.severity {
            ValidationSeverity::Critical => "🚨",
            ValidationSeverity::Error => "❌",
            ValidationSeverity::Warning => "⚠️",
            ValidationSeverity::Info => "ℹ️",
        }
    }
    
    pub fn category_name(&self) -> &'static str {
        match self.category {
            ValidationCategory::NamingConvention => "Naming",
            ValidationCategory::RouteConflict => "Route Conflict",
            ValidationCategory::MissingHandler => "Missing Handler",
            ValidationCategory::Complexity => "Complexity",
            ValidationCategory::RestConvention => "REST Convention",
            ValidationCategory::Security => "Security",
            ValidationCategory::Performance => "Performance",
            ValidationCategory::Structure => "Structure",
        }
    }
}

/// File system validation utilities
pub struct FileValidator;

impl FileValidator {
    /// Check if path exists and is readable
    pub fn validate_file_exists(path: &Path) -> bool {
        path.exists() && path.is_file()
    }
    
    /// Check if directory exists and is readable
    pub fn validate_directory_exists(path: &Path) -> bool {
        path.exists() && path.is_dir()
    }
    
    /// Validate file extension
    pub fn validate_file_extension(path: &Path, expected_ext: &str) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map_or(false, |ext| ext.eq_ignore_ascii_case(expected_ext))
    }
    
    /// Check if file is empty
    pub fn is_empty_file(path: &Path) -> bool {
        std::fs::metadata(path)
            .map(|meta| meta.len() == 0)
            .unwrap_or(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_naming_validation() {
        assert!(ProjectValidator::is_valid_controller_name("user_controller"));
        assert!(ProjectValidator::is_valid_controller_name("auth"));
        assert!(!ProjectValidator::is_valid_controller_name("UserController"));
        assert!(!ProjectValidator::is_valid_controller_name("_private"));
    }

    #[test]
    fn test_snake_case_conversion() {
        assert_eq!(ProjectValidator::to_snake_case("UserController"), "user_controller");
        assert_eq!(ProjectValidator::to_snake_case("HTTPSProxy"), "https_proxy");
        assert_eq!(ProjectValidator::to_snake_case("XMLParser"), "xml_parser");
    }

    #[test]
    fn test_route_path_validation() {
        assert!(ProjectValidator::is_valid_route_path("/users"));
        assert!(ProjectValidator::is_valid_route_path("/"));
        assert!(!ProjectValidator::is_valid_route_path("users"));
        assert!(!ProjectValidator::is_valid_route_path("/users/"));
    }

    #[test]
    fn test_validation_issue() {
        let issue = ValidationIssue {
            severity: ValidationSeverity::Error,
            category: ValidationCategory::Security,
            message: "Test issue".to_string(),
            location: None,
            suggestion: None,
        };
        
        assert_eq!(issue.severity_icon(), "❌");
        assert_eq!(issue.category_name(), "Security");
    }
}