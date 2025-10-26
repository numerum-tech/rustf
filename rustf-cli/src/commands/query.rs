use std::path::PathBuf;
use anyhow::Result;
use crate::analyzer::{ProjectAnalyzer, files::ProjectFiles};
use crate::analysis::{
    // handlers::HandlerAnalyzer, // unused
    middleware::MiddlewareAnalyzer,
    views::ViewAnalyzer,
};

pub async fn run(project_path: PathBuf, item_type: String, item_name: String) -> Result<()> {
    log::info!("Querying {} '{}'...", item_type, item_name);
    
    match item_type.to_lowercase().as_str() {
        "route" => query_route(project_path, item_name).await,
        "handler" => query_handler(project_path, item_name).await,
        "view" => query_view(project_path, item_name).await,
        "middleware" => query_middleware(project_path, item_name).await,
        "model" => query_model(project_path, item_name).await,
        "controller" => query_controller(project_path, item_name).await,
        _ => {
            println!("❌ Unknown item type: {}", item_type);
            println!("Supported types: route, handler, view, middleware, model, controller");
            Ok(())
        }
    }
}

async fn query_route(project_path: PathBuf, route_path: String) -> Result<()> {
    let analyzer = ProjectAnalyzer::new(project_path.clone())?;
    let analysis = analyzer.analyze_complete(false).await?;
    
    // Create fast lookup indexes
    let lookup = analyzer.create_lookup_indexes(&analysis);
    
    // Fast lookup by path pattern
    let lookup_result = lookup.find_routes_by_path(&route_path);
    let mut matching_routes = lookup_result.results;
    
    // If no exact match, try fuzzy matching from all routes
    if matching_routes.is_empty() {
        matching_routes = analysis.routes.iter()
            .filter(|route| {
                route.path.contains(&route_path) ||
                fuzzy_match(&route.path, &route_path)
            })
            .cloned()
            .collect();
    }
    
    if matching_routes.is_empty() {
        println!("❌ Route '{}' not found", route_path);
        suggest_similar_routes(&analysis.routes, &route_path);
        return Ok(());
    }
    
    println!("=== Route Query Results ===");
    println!("Query time: {}ms (cache hit: {})\n", lookup_result.query_time_ms, lookup_result.cache_hit);
    
    for route in matching_routes {
        println!("📍 Route: {} {}", route.method, route.path);
        println!("   Handler: {}", route.handler);
        
        if !route.parameters.is_empty() {
            println!("   Parameters: {}", route.parameters.join(", "));
        }
        
        // Find the controller and handler details
        if let Some(controller) = analysis.controllers.iter()
            .find(|c| c.handlers.iter().any(|h| h.name == route.handler)) {
            
            println!("   Controller: {} ({})", controller.name, controller.file_path);
            
            // Get handler details from the enhanced structure
            if let Some(handler) = controller.handlers.iter().find(|h| h.name == route.handler) {
                println!("   Handler Details:");
                println!("     - Qualified Name: {}", handler.qualified_name);
                println!("     - Complexity: {}", handler.complexity);
                
                if !handler.routes.is_empty() {
                    println!("     - Total Routes: {}", handler.routes.len());
                }
            }
        }
        
        // Find middleware that applies to this route
        let files = ProjectFiles::scan(&project_path)?;
        let mut middleware_analyses = Vec::new();
        
        for middleware_path in &files.middleware {
            if let Ok(analysis) = MiddlewareAnalyzer::analyze_middleware(middleware_path) {
                middleware_analyses.push(analysis);
            }
        }
        
        let middleware_chain = MiddlewareAnalyzer::build_middleware_chain(&middleware_analyses, &analysis.routes);
        
        if !middleware_chain.middleware_list.is_empty() {
            println!("   Middleware Chain: {}", middleware_chain.execution_order.join(" → "));
        }
        
        println!();
    }
    
    Ok(())
}

async fn query_handler(project_path: PathBuf, handler_name: String) -> Result<()> {
    let analyzer = ProjectAnalyzer::new(project_path)?;
    let analysis = analyzer.analyze_complete(false).await?;
    
    // Create fast lookup indexes
    let lookup = analyzer.create_lookup_indexes(&analysis);
    
    // Fast lookup by handler name
    let lookup_result = lookup.find_handlers_by_name(&handler_name);
    let mut found_handlers = lookup_result.results;
    
    // If no fast lookup results, try fuzzy matching
    if found_handlers.is_empty() {
        for controller in &analysis.controllers {
            for handler in &controller.handlers {
                if handler.qualified_name == handler_name ||
                   fuzzy_match(&handler.name, &handler_name) ||
                   fuzzy_match(&handler.qualified_name, &handler_name) {
                    found_handlers.push(handler.clone());
                }
            }
        }
    }
    
    if found_handlers.is_empty() {
        println!("❌ Handler '{}' not found", handler_name);
        suggest_similar_handlers(&analysis.controllers, &handler_name);
        return Ok(());
    }
    
    println!("=== Handler Query Results ===");
    println!("Query time: {}ms (cache hit: {})\n", lookup_result.query_time_ms, lookup_result.cache_hit);
    
    for handler in found_handlers {
        println!("🎯 Handler: {}", handler.qualified_name);
        // Extract controller name from qualified name (format: controller::handler)
        let controller_name = handler.qualified_name.split("::").next().unwrap_or("unknown");
        println!("   Controller: {}", controller_name);
        println!("   Complexity Score: {}", handler.complexity);
        
        if !handler.routes.is_empty() {
            println!("   Routes:");
            for route in &handler.routes {
                let params_str = if route.parameters.is_empty() {
                    String::new()
                } else {
                    format!(" (params: {})", route.parameters.join(", "))
                };
                println!("     - {} {}{}", route.method, route.path, params_str);
            }
        } else {
            println!("   Routes: none");
        }
        
        println!();
    }
    
    Ok(())
}

async fn query_view(project_path: PathBuf, view_name: String) -> Result<()> {
    let files = ProjectFiles::scan(&project_path)?;
    
    // Find matching views
    let mut matching_views = Vec::new();
    
    for view_path in &files.views {
        if let Some(name) = view_path.file_stem().and_then(|s| s.to_str()) {
            if name == view_name || fuzzy_match(name, &view_name) {
                if let Ok(analysis) = ViewAnalyzer::analyze_view(view_path) {
                    matching_views.push(analysis);
                }
            }
        }
    }
    
    if matching_views.is_empty() {
        println!("❌ View '{}' not found", view_name);
        suggest_similar_views(&files.views, &view_name);
        return Ok(());
    }
    
    println!("=== View Query Results ===\n");
    
    for view in matching_views {
        println!("👁️  View: {}", view.name);
        println!("   File: {}", view.file_path);
        println!("   Type: {:?}", view.template_type);
        
        if let Some(layout) = &view.layout {
            println!("   Layout: {}", layout);
        }
        
        println!("   Lines: {}", view.complexity_metrics.total_lines);
        println!("   Template Variables: {}", view.complexity_metrics.template_variables_count);
        println!("   Complexity Score: {}", view.complexity_metrics.complexity_score);
        
        if !view.template_variables.is_empty() {
            println!("   Variables:");
            for var in &view.template_variables {
                println!("     - {} ({:?}) - used {} times", var.name, var.variable_type, var.usage_count);
            }
        }
        
        if !view.forms.is_empty() {
            println!("   Forms: {}", view.forms.len());
            for form in &view.forms {
                println!("     - {} {} (CSRF: {})", form.method, form.action, form.has_csrf_token);
            }
        }
        
        if !view.security_issues.is_empty() {
            println!("   Security Issues: {}", view.security_issues.len());
            for issue in &view.security_issues {
                let severity_emoji = match issue.severity.as_str() {
                    "high" => "🔴",
                    "medium" => "🟡",
                    _ => "🟢",
                };
                println!("     {} {:?}: {}", severity_emoji, issue.issue_type, issue.description);
            }
        }
        
        // Find controllers that use this view
        let controllers_path = project_path.join("src/controllers");
        if let Ok(mappings) = ViewAnalyzer::find_view_controller_mappings(&controllers_path, &[view.clone()]) {
            if let Some(mapping) = mappings.first() {
                if !mapping.controllers.is_empty() {
                    println!("   Used by Controllers:");
                    for controller_usage in &mapping.controllers {
                        println!("     - {}::{} ({:?})", 
                            controller_usage.controller_name,
                            controller_usage.handler_name,
                            controller_usage.usage_type
                        );
                    }
                }
            }
        }
        
        println!();
    }
    
    Ok(())
}

async fn query_middleware(project_path: PathBuf, middleware_name: String) -> Result<()> {
    let files = ProjectFiles::scan(&project_path)?;
    
    // Find matching middleware
    let mut matching_middleware = Vec::new();
    
    for middleware_path in &files.middleware {
        if let Some(name) = middleware_path.file_stem().and_then(|s| s.to_str()) {
            if name == middleware_name || fuzzy_match(name, &middleware_name) {
                if let Ok(analysis) = MiddlewareAnalyzer::analyze_middleware(middleware_path) {
                    matching_middleware.push(analysis);
                }
            }
        }
    }
    
    if matching_middleware.is_empty() {
        println!("❌ Middleware '{}' not found", middleware_name);
        suggest_similar_middleware(&files.middleware, &middleware_name);
        return Ok(());
    }
    
    println!("=== Middleware Query Results ===\n");
    
    for middleware in matching_middleware {
        println!("🔧 Middleware: {}", middleware.name);
        println!("   File: {}", middleware.file_path);
        println!("   Type: {:?}", middleware.middleware_type);
        println!("   Implements Trait: {}", middleware.implements_trait);
        println!("   Has Install Function: {}", middleware.has_install_function);
        
        if let Some(order) = middleware.execution_order {
            println!("   Execution Order: {}", order);
        }
        
        println!("   Scope:");
        println!("     - Global: {}", middleware.scope.global);
        println!("     - Route Specific: {}", middleware.scope.route_specific);
        println!("     - Controller Specific: {}", middleware.scope.controller_specific);
        
        if !middleware.scope.patterns.is_empty() {
            println!("     - Patterns: {}", middleware.scope.patterns.join(", "));
        }
        
        if !middleware.dependencies.is_empty() {
            println!("   Dependencies: {}", middleware.dependencies.join(", "));
        }
        
        // Show routes affected by this middleware
        let analyzer = ProjectAnalyzer::new(project_path.clone())?;
        let analysis = analyzer.analyze_complete(false).await?;
        
        let affected_routes: Vec<_> = if middleware.scope.global {
            analysis.routes
        } else if !middleware.scope.patterns.is_empty() {
            analysis.routes.into_iter()
                .filter(|route| {
                    middleware.scope.patterns.iter().any(|pattern| {
                        let pattern = pattern.replace("*", "");
                        route.path.starts_with(&pattern)
                    })
                })
                .collect()
        } else {
            Vec::new()
        };
        
        if !affected_routes.is_empty() {
            println!("   Affects {} routes:", affected_routes.len());
            for route in affected_routes.iter().take(5) {
                println!("     - {} {}", route.method, route.path);
            }
            if affected_routes.len() > 5 {
                println!("     - ... and {} more", affected_routes.len() - 5);
            }
        }
        
        println!();
    }
    
    Ok(())
}

async fn query_model(project_path: PathBuf, model_name: String) -> Result<()> {
    let files = ProjectFiles::scan(&project_path)?;
    
    // Find matching models
    let mut matching_models = Vec::new();
    
    for model_path in &files.models {
        if let Some(name) = model_path.file_stem().and_then(|s| s.to_str()) {
            if name == model_name || fuzzy_match(name, &model_name) {
                matching_models.push((name.to_string(), model_path.clone()));
            }
        }
    }
    
    if matching_models.is_empty() {
        println!("❌ Model '{}' not found", model_name);
        suggest_similar_models(&files.models, &model_name);
        return Ok(());
    }
    
    println!("=== Model Query Results ===\n");
    
    for (name, model_path) in matching_models {
        println!("📊 Model: {}", name);
        println!("   File: {}", model_path.display());
        
        // Basic file analysis
        if let Ok(content) = std::fs::read_to_string(&model_path) {
            let lines = content.lines().count();
            println!("   Lines: {}", lines);
            
            // Look for struct definitions
            let structs = content.matches("struct ").count();
            if structs > 0 {
                println!("   Structs: {}", structs);
            }
            
            // Look for impl blocks
            let impls = content.matches("impl ").count();
            if impls > 0 {
                println!("   Implementations: {}", impls);
            }
            
            // Look for serde derives
            if content.contains("Serialize") || content.contains("Deserialize") {
                println!("   Serializable: Yes");
            }
            
            // Look for database-related annotations
            if content.contains("sqlx") || content.contains("diesel") {
                println!("   Database Integration: Yes");
            }
        }
        
        println!();
    }
    
    Ok(())
}

async fn query_controller(project_path: PathBuf, controller_name: String) -> Result<()> {
    let analyzer = ProjectAnalyzer::new(project_path)?;
    let analysis = analyzer.analyze_complete(false).await?;
    
    // Find matching controllers
    let matching_controllers: Vec<_> = analysis.controllers.iter()
        .filter(|c| c.name == controller_name || fuzzy_match(&c.name, &controller_name))
        .collect();
    
    if matching_controllers.is_empty() {
        println!("❌ Controller '{}' not found", controller_name);
        suggest_similar_controllers(&analysis.controllers, &controller_name);
        return Ok(());
    }
    
    println!("=== Controller Query Results ===\n");
    
    for controller in matching_controllers {
        println!("🎮 Controller: {}", controller.name);
        println!("   File: {}", controller.file_path);
        println!("   Handlers: {}", controller.handlers.len());
        
        let avg_complexity = if !controller.handlers.is_empty() {
            controller.handlers.iter().map(|h| h.complexity).sum::<u32>() as f64 / controller.handlers.len() as f64
        } else {
            0.0
        };
        
        println!("   Average Complexity: {:.1}", avg_complexity);
        println!("   Total Handlers: {}", controller.handlers.len());
        
        println!("   Handler Details:");
        for handler in &controller.handlers {
            let routes_str = if handler.routes.is_empty() {
                "no routes".to_string()
            } else {
                handler.routes.iter()
                    .map(|r| format!("{} {}", r.method, r.path))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            println!("     - {} (complexity: {}) → {}", 
                handler.qualified_name,
                handler.complexity,
                routes_str
            );
        }
        
        // Count total routes for this controller
        let total_routes: usize = controller.handlers.iter().map(|h| h.routes.len()).sum();
        if total_routes > 0 {
            println!("   Total Routes: {}", total_routes);
        }
        
        println!();
    }
    
    Ok(())
}

// Helper functions for fuzzy matching and suggestions
fn fuzzy_match(text: &str, pattern: &str) -> bool {
    let text_lower = text.to_lowercase();
    let pattern_lower = pattern.to_lowercase();
    
    // Simple fuzzy matching - contains pattern or similar
    text_lower.contains(&pattern_lower) || 
    pattern_lower.contains(&text_lower) ||
    levenshtein_distance(&text_lower, &pattern_lower) <= 2
}

fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let len1 = s1.chars().count();
    let len2 = s2.chars().count();
    
    if len1 == 0 { return len2; }
    if len2 == 0 { return len1; }
    
    let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];
    
    for i in 0..=len1 { matrix[i][0] = i; }
    for j in 0..=len2 { matrix[0][j] = j; }
    
    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();
    
    for (i, c1) in s1_chars.iter().enumerate() {
        for (j, c2) in s2_chars.iter().enumerate() {
            let cost = if c1 == c2 { 0 } else { 1 };
            matrix[i + 1][j + 1] = std::cmp::min(
                std::cmp::min(
                    matrix[i][j + 1] + 1,     // deletion
                    matrix[i + 1][j] + 1      // insertion
                ),
                matrix[i][j] + cost           // substitution
            );
        }
    }
    
    matrix[len1][len2]
}

fn suggest_similar_routes(routes: &[crate::analyzer::RouteInfo], target: &str) {
    let similar: Vec<_> = routes.iter()
        .filter(|r| fuzzy_match(&r.path, target))
        .take(3)
        .collect();
    
    if !similar.is_empty() {
        println!("💡 Did you mean:");
        for route in similar {
            println!("   - {} {}", route.method, route.path);
        }
    }
}

fn suggest_similar_handlers(controllers: &[crate::analyzer::ControllerInfo], target: &str) {
    let similar: Vec<_> = controllers.iter()
        .flat_map(|c| &c.handlers)
        .filter(|h| fuzzy_match(&h.name, target) || fuzzy_match(&h.qualified_name, target))
        .take(3)
        .collect();
    
    if !similar.is_empty() {
        println!("💡 Did you mean:");
        for handler in similar {
            println!("   - {}", handler.qualified_name);
        }
    }
}

fn suggest_similar_views(views: &[PathBuf], target: &str) {
    let similar: Vec<_> = views.iter()
        .filter_map(|v| v.file_stem().and_then(|s| s.to_str()))
        .filter(|name| fuzzy_match(name, target))
        .take(3)
        .collect();
    
    if !similar.is_empty() {
        println!("💡 Did you mean:");
        for view in similar {
            println!("   - {}", view);
        }
    }
}

fn suggest_similar_middleware(middleware: &[PathBuf], target: &str) {
    let similar: Vec<_> = middleware.iter()
        .filter_map(|m| m.file_stem().and_then(|s| s.to_str()))
        .filter(|name| fuzzy_match(name, target))
        .take(3)
        .collect();
    
    if !similar.is_empty() {
        println!("💡 Did you mean:");
        for mw in similar {
            println!("   - {}", mw);
        }
    }
}

fn suggest_similar_models(models: &[PathBuf], target: &str) {
    let similar: Vec<_> = models.iter()
        .filter_map(|m| m.file_stem().and_then(|s| s.to_str()))
        .filter(|name| fuzzy_match(name, target))
        .take(3)
        .collect();
    
    if !similar.is_empty() {
        println!("💡 Did you mean:");
        for model in similar {
            println!("   - {}", model);
        }
    }
}

fn suggest_similar_controllers(controllers: &[crate::analyzer::ControllerInfo], target: &str) {
    let similar: Vec<_> = controllers.iter()
        .filter(|c| fuzzy_match(&c.name, target))
        .take(3)
        .collect();
    
    if !similar.is_empty() {
        println!("💡 Did you mean:");
        for controller in similar {
            println!("   - {}", controller.name);
        }
    }
}