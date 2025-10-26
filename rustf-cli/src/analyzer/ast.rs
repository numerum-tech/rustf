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
    /// Create new analyzer with custom cache settings
    pub fn with_cache(max_entries: usize, max_age_seconds: u64) -> Self {
        Self {
            cache: Arc::new(AstCache::new(max_entries, max_age_seconds)),
        }
    }

    /// Get cache statistics for monitoring
    pub fn cache_stats(&self) -> super::cache::CacheStats {
        self.cache.get_stats()
    }

    /// Clear the AST cache
    pub fn clear_cache(&self) {
        self.cache.clear();
    }
}

pub struct BasicControllerInfo {
    pub name: String,
    pub file_path: String,
    pub handlers: Vec<String>,
}

impl AstAnalyzer {
    pub fn analyze_controller(&self, file_path: &Path) -> Result<BasicControllerInfo> {
        let syntax_tree = self.cache.get_or_parse(file_path)
            .with_context(|| format!("Failed to parse file: {}", file_path.display()))?;

        let controller_name = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let mut controller_info = BasicControllerInfo {
            name: controller_name,
            file_path: file_path.to_string_lossy().to_string(),
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

    pub fn find_handler_functions(&self, file_path: &Path) -> Result<Vec<String>> {
        let syntax_tree = self.cache.get_or_parse(file_path)?;

        let mut handlers = Vec::new();

        for item in &syntax_tree.items {
            if let Item::Fn(func) = item {
                let func_name = func.sig.ident.to_string();
                
                // Check if it's an async function that takes Context parameter
                if func.sig.asyncness.is_some() && Self::has_context_parameter(&func.sig) {
                    handlers.push(func_name);
                }
            }
        }

        Ok(handlers)
    }

    fn has_context_parameter(sig: &syn::Signature) -> bool {
        sig.inputs.iter().any(|arg| {
            if let syn::FnArg::Typed(pat_type) = arg {
                if let syn::Type::Path(type_path) = &*pat_type.ty {
                    if let Some(segment) = type_path.path.segments.last() {
                        return segment.ident == "Context";
                    }
                }
            }
            false
        })
    }
}