use syn::{File, Item, ItemFn, Type, Pat, FnArg, ReturnType};
use std::path::Path;
use std::fs;
use anyhow::{Result, Context};
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct HandlerAnalysis {
    pub name: String,
    pub signature: String,
    pub is_async: bool,
    pub parameters: Vec<HandlerParameter>,
    pub return_type: String,
    pub context_usage: Vec<ContextUsage>,
    pub error_handling: ErrorHandling,
    pub complexity_score: u32,
}

#[derive(Debug, Serialize, Clone)]
pub struct HandlerParameter {
    pub name: String,
    pub param_type: String,
    pub is_context: bool,
}

#[derive(Debug, Serialize, Clone)]
pub struct ContextUsage {
    pub method: String,
    pub usage_type: String, // "request", "response", "session", "flash", "validation"
    pub line_number: Option<u32>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ErrorHandling {
    pub has_error_handling: bool,
    pub error_types: Vec<String>,
    pub uses_result_type: bool,
}

pub struct HandlerAnalyzer;

impl HandlerAnalyzer {
    pub fn analyze_handlers(file_path: &Path) -> Result<Vec<HandlerAnalysis>> {
        let content = fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

        let syntax_tree: File = syn::parse_file(&content)
            .with_context(|| format!("Failed to parse Rust file: {}", file_path.display()))?;

        let mut handlers = Vec::new();

        for item in &syntax_tree.items {
            if let Item::Fn(func) = item {
                let func_name = func.sig.ident.to_string();
                
                // Skip the install function as it's not a handler
                if func_name == "install" {
                    continue;
                }

                // Check if it looks like a handler (async function with Context parameter)
                if func.sig.asyncness.is_some() && Self::has_context_parameter(&func.sig) {
                    let analysis = Self::analyze_handler_function(func, &content)?;
                    handlers.push(analysis);
                }
            }
        }

        Ok(handlers)
    }

    fn analyze_handler_function(func: &ItemFn, source_code: &str) -> Result<HandlerAnalysis> {
        let name = func.sig.ident.to_string();
        let signature = Self::build_signature_string(&func.sig);
        let is_async = func.sig.asyncness.is_some();
        let parameters = Self::analyze_parameters(&func.sig);
        let return_type = Self::analyze_return_type(&func.sig);
        let context_usage = Self::analyze_context_usage(func, source_code);
        let error_handling = Self::analyze_error_handling(func, source_code);
        let complexity_score = Self::calculate_complexity(func);

        Ok(HandlerAnalysis {
            name,
            signature,
            is_async,
            parameters,
            return_type,
            context_usage,
            error_handling,
            complexity_score,
        })
    }

    fn has_context_parameter(sig: &syn::Signature) -> bool {
        sig.inputs.iter().any(|arg| {
            if let FnArg::Typed(pat_type) = arg {
                if let Type::Path(type_path) = &*pat_type.ty {
                    if let Some(segment) = type_path.path.segments.last() {
                        return segment.ident == "Context";
                    }
                }
            }
            false
        })
    }

    fn build_signature_string(sig: &syn::Signature) -> String {
        let asyncness = if sig.asyncness.is_some() { "async " } else { "" };
        let name = &sig.ident;
        let inputs: Vec<String> = sig.inputs.iter().map(|arg| {
            match arg {
                FnArg::Receiver(_) => "self".to_string(),
                FnArg::Typed(pat_type) => {
                    let pat = match &*pat_type.pat {
                        Pat::Ident(ident) => ident.ident.to_string(),
                        _ => "_".to_string(),
                    };
                    let ty = quote::quote!(#pat_type.ty).to_string();
                    format!("{}: {}", pat, ty)
                }
            }
        }).collect();
        
        let output = match &sig.output {
            ReturnType::Default => String::new(),
            ReturnType::Type(_, ty) => format!(" -> {}", quote::quote!(#ty)),
        };

        format!("{}fn {}({}){}",
            asyncness,
            name,
            inputs.join(", "),
            output
        )
    }

    fn analyze_parameters(sig: &syn::Signature) -> Vec<HandlerParameter> {
        let mut parameters = Vec::new();

        for input in &sig.inputs {
            if let FnArg::Typed(pat_type) = input {
                let name = match &*pat_type.pat {
                    Pat::Ident(ident) => ident.ident.to_string(),
                    _ => "unknown".to_string(),
                };

                let param_type = quote::quote!(#pat_type.ty).to_string();
                let is_context = param_type.contains("Context");

                parameters.push(HandlerParameter {
                    name,
                    param_type,
                    is_context,
                });
            }
        }

        parameters
    }

    fn analyze_return_type(sig: &syn::Signature) -> String {
        match &sig.output {
            ReturnType::Default => "()".to_string(),
            ReturnType::Type(_, ty) => quote::quote!(#ty).to_string(),
        }
    }

    fn analyze_context_usage(func: &ItemFn, _source_code: &str) -> Vec<ContextUsage> {
        let mut usages = Vec::new();
        let func_source = quote::quote!(#func).to_string();

        // Common Context API methods and their types
        let context_methods = [
            ("param", "request"),
            ("query", "request"),
            ("header", "request"),
            ("body_json", "request"),
            ("body_form", "request"),
            ("json", "response"),
            ("text", "response"),
            ("view", "response"),
            ("redirect", "response"),
            ("session_set", "session"),
            ("session_get", "session"),
            ("session_remove", "session"),
            ("flash_success", "flash"),
            ("flash_error", "flash"),
            ("flash_info", "flash"),
            ("flash_warning", "flash"),
            ("layout", "response"),
        ];

        for (method, usage_type) in &context_methods {
            if func_source.contains(&format!("ctx.{}", method)) {
                usages.push(ContextUsage {
                    method: method.to_string(),
                    usage_type: usage_type.to_string(),
                    line_number: None, // Could be enhanced to find actual line numbers
                });
            }
        }

        usages
    }

    fn analyze_error_handling(func: &ItemFn, _source_code: &str) -> ErrorHandling {
        let func_source = quote::quote!(#func).to_string();
        
        let has_error_handling = func_source.contains("Result") || 
                                func_source.contains("Error") ||
                                func_source.contains("?") ||
                                func_source.contains("match") ||
                                func_source.contains("if let Err");

        let uses_result_type = match &func.sig.output {
            ReturnType::Type(_, ty) => {
                let type_str = quote::quote!(#ty).to_string();
                type_str.contains("Result")
            }
            ReturnType::Default => false,
        };

        let mut error_types = Vec::new();
        if func_source.contains("anyhow::Error") {
            error_types.push("anyhow::Error".to_string());
        }
        if func_source.contains("std::error::Error") {
            error_types.push("std::error::Error".to_string());
        }

        ErrorHandling {
            has_error_handling,
            error_types,
            uses_result_type,
        }
    }

    fn calculate_complexity(func: &ItemFn) -> u32 {
        let func_source = quote::quote!(#func).to_string();
        let mut complexity = 1; // Base complexity

        // Add complexity for control flow
        complexity += func_source.matches("if ").count() as u32;
        complexity += func_source.matches("match ").count() as u32 * 2;
        complexity += func_source.matches("for ").count() as u32;
        complexity += func_source.matches("while ").count() as u32;
        complexity += func_source.matches("loop ").count() as u32;

        // Add complexity for async operations
        complexity += func_source.matches(".await").count() as u32;

        // Add complexity for error handling
        complexity += func_source.matches("?").count() as u32;

        complexity
    }

    pub fn analyze_handler_patterns(handlers: &[HandlerAnalysis]) -> HandlerPatterns {
        let total_handlers = handlers.len();
        let async_handlers = handlers.iter().filter(|h| h.is_async).count();
        let context_users = handlers.iter().filter(|h| 
            h.parameters.iter().any(|p| p.is_context)
        ).count();

        let common_patterns = Self::identify_common_patterns(handlers);
        let complexity_distribution = Self::analyze_complexity_distribution(handlers);

        HandlerPatterns {
            total_handlers,
            async_handlers,
            context_users,
            common_patterns,
            complexity_distribution,
        }
    }

    fn identify_common_patterns(handlers: &[HandlerAnalysis]) -> Vec<String> {
        let mut patterns = Vec::new();

        // Check for common Context API usage patterns
        let view_users = handlers.iter().filter(|h| 
            h.context_usage.iter().any(|u| u.method == "view")
        ).count();
        
        let json_users = handlers.iter().filter(|h| 
            h.context_usage.iter().any(|u| u.method == "json")
        ).count();

        let session_users = handlers.iter().filter(|h| 
            h.context_usage.iter().any(|u| u.usage_type == "session")
        ).count();

        if view_users > 0 {
            patterns.push(format!("Template rendering ({} handlers)", view_users));
        }
        if json_users > 0 {
            patterns.push(format!("JSON responses ({} handlers)", json_users));
        }
        if session_users > 0 {
            patterns.push(format!("Session management ({} handlers)", session_users));
        }

        patterns
    }

    fn analyze_complexity_distribution(handlers: &[HandlerAnalysis]) -> ComplexityDistribution {
        let complexities: Vec<u32> = handlers.iter().map(|h| h.complexity_score).collect();
        
        let low = complexities.iter().filter(|&&c| c <= 5).count();
        let medium = complexities.iter().filter(|&&c| c > 5 && c <= 10).count();
        let high = complexities.iter().filter(|&&c| c > 10).count();

        let average = if !complexities.is_empty() {
            complexities.iter().sum::<u32>() as f64 / complexities.len() as f64
        } else {
            0.0
        };

        ComplexityDistribution {
            low,
            medium,
            high,
            average,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct HandlerPatterns {
    pub total_handlers: usize,
    pub async_handlers: usize,
    pub context_users: usize,
    pub common_patterns: Vec<String>,
    pub complexity_distribution: ComplexityDistribution,
}

#[derive(Debug, Serialize)]
pub struct ComplexityDistribution {
    pub low: usize,    // <= 5
    pub medium: usize, // 6-10  
    pub high: usize,   // > 10
    pub average: f64,
}