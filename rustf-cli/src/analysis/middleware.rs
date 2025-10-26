use syn::{File, Item};
use std::path::Path;
use std::fs;
use anyhow::{Result, Context};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize, Clone)]
pub struct MiddlewareAnalysis {
    pub name: String,
    pub file_path: String,
    pub middleware_type: MiddlewareType,
    pub priority: Option<i32>,
    pub implements_trait: bool,
    pub has_install_function: bool,
    pub execution_order: Option<usize>,
    pub scope: MiddlewareScope,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
pub enum MiddlewareType {
    Builtin,
    Custom,
    Authentication,
    Logging,
    CORS,
    RateLimit,
    Security,
    Unknown,
}

#[derive(Debug, Serialize, Clone)]
pub struct MiddlewareScope {
    pub global: bool,
    pub route_specific: bool,
    pub controller_specific: bool,
    pub patterns: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct MiddlewareChain {
    pub middleware_list: Vec<MiddlewareAnalysis>,
    pub execution_order: Vec<String>,
    pub conflicts: Vec<MiddlewareConflict>,
    pub coverage_analysis: CoverageAnalysis,
}

#[derive(Debug, Serialize)]
pub struct MiddlewareConflict {
    pub middleware1: String,
    pub middleware2: String,
    pub conflict_type: String,
    pub description: String,
}

#[derive(Debug, Serialize)]
pub struct CoverageAnalysis {
    pub total_routes: usize,
    pub protected_routes: usize,
    pub logging_coverage: f64,
    pub auth_coverage: f64,
    pub cors_coverage: f64,
}

pub struct MiddlewareAnalyzer;

impl MiddlewareAnalyzer {
    pub fn analyze_middleware(file_path: &Path) -> Result<MiddlewareAnalysis> {
        let content = fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

        let syntax_tree: File = syn::parse_file(&content)
            .with_context(|| format!("Failed to parse Rust file: {}", file_path.display()))?;

        let name = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let middleware_type = Self::determine_middleware_type(&name, &content);
        let implements_trait = Self::implements_middleware_trait(&syntax_tree);
        let has_install_function = Self::has_install_function(&syntax_tree);
        let scope = Self::analyze_scope(&content);
        let dependencies = Self::extract_dependencies(&syntax_tree);

        Ok(MiddlewareAnalysis {
            name,
            file_path: file_path.to_string_lossy().to_string(),
            middleware_type,
            priority: None, // Could be extracted from attributes or comments
            implements_trait,
            has_install_function,
            execution_order: None,
            scope,
            dependencies,
        })
    }

    fn determine_middleware_type(name: &str, content: &str) -> MiddlewareType {
        let name_lower = name.to_lowercase();
        let content_lower = content.to_lowercase();

        if name_lower.contains("auth") || content_lower.contains("authentication") {
            MiddlewareType::Authentication
        } else if name_lower.contains("log") || content_lower.contains("logging") {
            MiddlewareType::Logging
        } else if name_lower.contains("cors") || content_lower.contains("cross-origin") {
            MiddlewareType::CORS
        } else if name_lower.contains("rate") || content_lower.contains("throttle") {
            MiddlewareType::RateLimit
        } else if name_lower.contains("security") || content_lower.contains("csrf") {
            MiddlewareType::Security
        } else if content_lower.contains("rustf::middleware::builtin") {
            MiddlewareType::Builtin
        } else if content_lower.contains("middleware") {
            MiddlewareType::Custom
        } else {
            MiddlewareType::Unknown
        }
    }

    fn implements_middleware_trait(syntax_tree: &File) -> bool {
        for item in &syntax_tree.items {
            if let Item::Impl(impl_item) = item {
                if let Some((_, trait_path, _)) = &impl_item.trait_ {
                    if let Some(segment) = trait_path.segments.last() {
                        if segment.ident == "Middleware" {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    fn has_install_function(syntax_tree: &File) -> bool {
        for item in &syntax_tree.items {
            if let Item::Fn(func) = item {
                if func.sig.ident == "install" {
                    return true;
                }
            }
        }
        false
    }

    fn analyze_scope(content: &str) -> MiddlewareScope {
        let global = content.contains("app.middleware") || content.contains("global");
        let route_specific = content.contains("route") && !global;
        let controller_specific = content.contains("controller") && !global;

        let mut patterns = Vec::new();
        
        // Look for route patterns in the middleware
        if content.contains("/api/") {
            patterns.push("/api/*".to_string());
        }
        if content.contains("/admin/") {
            patterns.push("/admin/*".to_string());
        }
        if content.contains("/auth/") {
            patterns.push("/auth/*".to_string());
        }

        MiddlewareScope {
            global,
            route_specific,
            controller_specific,
            patterns,
        }
    }

    fn extract_dependencies(syntax_tree: &File) -> Vec<String> {
        let mut dependencies = Vec::new();

        // Look for common dependencies in use statements
        for item in &syntax_tree.items {
            if let Item::Use(use_item) = item {
                let use_string = quote::quote!(#use_item).to_string();
                
                if use_string.contains("tokio") {
                    dependencies.push("tokio".to_string());
                }
                if use_string.contains("serde") {
                    dependencies.push("serde".to_string());
                }
                if use_string.contains("rustf") {
                    dependencies.push("rustf".to_string());
                }
                if use_string.contains("log") {
                    dependencies.push("log".to_string());
                }
            }
        }

        dependencies.sort();
        dependencies.dedup();
        dependencies
    }

    pub fn build_middleware_chain(middleware_list: &[MiddlewareAnalysis], routes: &[crate::analyzer::RouteInfo]) -> MiddlewareChain {
        let mut ordered_middleware = middleware_list.to_vec();
        
        // Sort by priority and type (builtin first, then by name)
        ordered_middleware.sort_by(|a, b| {
            match (&a.middleware_type, &b.middleware_type) {
                (MiddlewareType::Builtin, MiddlewareType::Custom) => std::cmp::Ordering::Less,
                (MiddlewareType::Custom, MiddlewareType::Builtin) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });

        // Set execution order
        for (index, middleware) in ordered_middleware.iter_mut().enumerate() {
            middleware.execution_order = Some(index);
        }

        let execution_order: Vec<String> = ordered_middleware.iter().map(|m| m.name.clone()).collect();
        let conflicts = Self::detect_middleware_conflicts(&ordered_middleware);
        let coverage_analysis = Self::analyze_coverage(&ordered_middleware, routes);

        MiddlewareChain {
            middleware_list: ordered_middleware,
            execution_order,
            conflicts,
            coverage_analysis,
        }
    }

    fn detect_middleware_conflicts(middleware_list: &[MiddlewareAnalysis]) -> Vec<MiddlewareConflict> {
        let mut conflicts = Vec::new();

        // Check for multiple auth middleware
        let auth_middleware: Vec<&MiddlewareAnalysis> = middleware_list
            .iter()
            .filter(|m| matches!(m.middleware_type, MiddlewareType::Authentication))
            .collect();

        if auth_middleware.len() > 1 {
            for i in 0..auth_middleware.len() {
                for j in i+1..auth_middleware.len() {
                    conflicts.push(MiddlewareConflict {
                        middleware1: auth_middleware[i].name.clone(),
                        middleware2: auth_middleware[j].name.clone(),
                        conflict_type: "duplicate_auth".to_string(),
                        description: "Multiple authentication middleware may conflict".to_string(),
                    });
                }
            }
        }

        // Check for CORS conflicts
        let cors_middleware: Vec<&MiddlewareAnalysis> = middleware_list
            .iter()
            .filter(|m| matches!(m.middleware_type, MiddlewareType::CORS))
            .collect();

        if cors_middleware.len() > 1 {
            for i in 0..cors_middleware.len() {
                for j in i+1..cors_middleware.len() {
                    conflicts.push(MiddlewareConflict {
                        middleware1: cors_middleware[i].name.clone(),
                        middleware2: cors_middleware[j].name.clone(),
                        conflict_type: "duplicate_cors".to_string(),
                        description: "Multiple CORS middleware may set conflicting headers".to_string(),
                    });
                }
            }
        }

        conflicts
    }

    fn analyze_coverage(middleware_list: &[MiddlewareAnalysis], routes: &[crate::analyzer::RouteInfo]) -> CoverageAnalysis {
        let total_routes = routes.len();
        
        // Count routes that might be protected by auth middleware
        let auth_middleware_count = middleware_list
            .iter()
            .filter(|m| matches!(m.middleware_type, MiddlewareType::Authentication))
            .count();

        let api_routes = routes.iter().filter(|r| r.path.starts_with("/api/")).count();
        let protected_routes = if auth_middleware_count > 0 { api_routes } else { 0 };

        // Calculate coverage percentages
        let logging_coverage = if middleware_list.iter().any(|m| matches!(m.middleware_type, MiddlewareType::Logging)) {
            100.0
        } else {
            0.0
        };

        let auth_coverage = if total_routes > 0 {
            (protected_routes as f64 / total_routes as f64) * 100.0
        } else {
            0.0
        };

        let cors_coverage = if middleware_list.iter().any(|m| matches!(m.middleware_type, MiddlewareType::CORS)) {
            100.0
        } else {
            0.0
        };

        CoverageAnalysis {
            total_routes,
            protected_routes,
            logging_coverage,
            auth_coverage,
            cors_coverage,
        }
    }

    pub fn analyze_middleware_patterns(middleware_list: &[MiddlewareAnalysis]) -> MiddlewarePatterns {
        let total_middleware = middleware_list.len();
        
        let mut type_distribution = HashMap::new();
        for middleware in middleware_list {
            let type_name = format!("{:?}", middleware.middleware_type);
            *type_distribution.entry(type_name).or_insert(0) += 1;
        }

        let custom_middleware = middleware_list
            .iter()
            .filter(|m| matches!(m.middleware_type, MiddlewareType::Custom))
            .count();

        let global_middleware = middleware_list
            .iter()
            .filter(|m| m.scope.global)
            .count();

        MiddlewarePatterns {
            total_middleware,
            custom_middleware,
            global_middleware,
            type_distribution,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MiddlewarePatterns {
    pub total_middleware: usize,
    pub custom_middleware: usize,
    pub global_middleware: usize,
    pub type_distribution: HashMap<String, usize>,
}