use syn::{Item, ItemFn, Expr, Macro};
// use quote::ToTokens; // unused
use std::path::Path;
// use std::fs; // unused
use std::sync::Arc;
use anyhow::{Result, Context};
use crate::analyzer::RouteInfo;
use super::cache::AstCache;

#[derive(Debug)]
pub struct AstAnalyzer {
    cache: Arc<AstCache>,
}

impl Default for AstAnalyzer {
    fn default() -> Self {
        Self {
            cache: Arc::new(AstCache::default()),
        }
    }
}

impl AstAnalyzer {
    /// Get cache statistics for monitoring
    pub fn cache_stats(&self) -> super::cache::CacheStats {
        self.cache.get_stats()
    }
}

pub struct BasicControllerInfo {
    pub handlers: Vec<String>,
}

impl AstAnalyzer {
    pub fn analyze_controller(&self, file_path: &Path) -> Result<BasicControllerInfo> {
        let syntax_tree = self.cache.get_or_parse(file_path)
            .with_context(|| format!("Failed to parse file: {}", file_path.display()))?;

        let mut controller_info = BasicControllerInfo {
            handlers: Vec::new(),
        };

        // Find all function definitions to identify handlers
        for item in &syntax_tree.items {
            if let Item::Fn(func) = item {
                let handler_name = func.sig.ident.to_string();
                
                // Skip the install function as it's not a handler
                if handler_name != "install" {
                    controller_info.handlers.push(handler_name);
                }
            }
        }

        Ok(controller_info)
    }

    pub fn extract_routes(&self, file_path: &Path) -> Result<Vec<RouteInfo>> {
        let syntax_tree = self.cache.get_or_parse(file_path)
            .with_context(|| format!("Failed to parse file: {}", file_path.display()))?;

        let mut routes = Vec::new();

        // Look for the install function
        for item in &syntax_tree.items {
            if let Item::Fn(func) = item {
                if func.sig.ident == "install" {
                    routes.extend(Self::extract_routes_from_function(func)?);
                }
            }
        }

        Ok(routes)
    }

    fn extract_routes_from_function(func: &ItemFn) -> Result<Vec<RouteInfo>> {
        let mut routes = Vec::new();

        // Look for routes![] macro invocations in the function body
        for stmt in &func.block.stmts {
            if let syn::Stmt::Expr(expr, None) = stmt {
                routes.extend(Self::extract_routes_from_expr(expr)?);
            }
        }

        Ok(routes)
    }

    fn extract_routes_from_expr(expr: &Expr) -> Result<Vec<RouteInfo>> {
        let mut routes = Vec::new();

        match expr {
            Expr::Macro(macro_expr) => {
                if Self::is_routes_macro(&macro_expr.mac) {
                    routes.extend(Self::parse_routes_macro(&macro_expr.mac)?);
                }
            }
            // Handle nested expressions (like in blocks)
            Expr::Block(block_expr) => {
                for stmt in &block_expr.block.stmts {
                    if let syn::Stmt::Expr(nested_expr, None) = stmt {
                        routes.extend(Self::extract_routes_from_expr(nested_expr)?);
                    }
                }
            }
            _ => {}
        }

        Ok(routes)
    }

    fn is_routes_macro(mac: &Macro) -> bool {
        if let Some(segment) = mac.path.segments.last() {
            segment.ident == "routes"
        } else {
            false
        }
    }

    fn parse_routes_macro(mac: &Macro) -> Result<Vec<RouteInfo>> {
        let mut routes = Vec::new();

        // Get the macro tokens as a string to parse manually
        let tokens_string = mac.tokens.to_string();
        
        // Simple regex-based parsing for routes like: GET "/path" => handler
        let route_pattern = regex::Regex::new(r#"(\w+)\s+"([^"]+)"\s*=>\s*(\w+)"#)?;
        
        for cap in route_pattern.captures_iter(&tokens_string) {
            let method = cap[1].to_string();
            let path = cap[2].to_string();
            let handler = cap[3].to_string();
            
            let parameters = Self::extract_path_parameters(&path);
            
            routes.push(RouteInfo {
                method,
                path,
                handler,
                parameters,
            });
        }

        Ok(routes)
    }

    fn extract_path_parameters(path: &str) -> Vec<String> {
        let mut parameters = Vec::new();
        let param_pattern = regex::Regex::new(r"\{([^}]+)\}").unwrap();
        
        for cap in param_pattern.captures_iter(path) {
            parameters.push(cap[1].to_string());
        }
        
        parameters
    }

}