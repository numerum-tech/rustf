use std::path::PathBuf;
use anyhow::Result;
use crate::analyzer::ProjectAnalyzer;
// use crate::analysis::handlers::{HandlerAnalyzer, HandlerAnalysis}; // unused

pub async fn run(project_path: PathBuf, name: Option<String>) -> Result<()> {
    log::info!("Analyzing controllers...");
    
    let analyzer = ProjectAnalyzer::new(project_path)?;
    let analysis = analyzer.analyze_complete(false).await?;
    
    // If specific controller requested, filter results
    if let Some(target_name) = name {
        if let Some(controller) = analysis.controllers.iter().find(|c| c.name == target_name) {
            println!("=== Controller: {} ===\n", controller.name);
            display_controller_details(controller);
        } else {
            println!("Controller '{}' not found", target_name);
            println!("Available controllers: {}", 
                analysis.controllers.iter().map(|c| c.name.as_str()).collect::<Vec<_>>().join(", "));
        }
        return Ok(());
    }
    
    // Display overview of all controllers
    println!("=== Controllers Analysis ===\n");
    println!("Total Controllers: {}", analysis.controllers.len());
    
    let total_handlers: usize = analysis.controllers.iter().map(|c| c.handlers.len()).sum();
    println!("Total Handlers: {}", total_handlers);
    
    // Calculate async handlers from routes (all handlers in RustF are async)
    println!("Async Handlers: {}", total_handlers);
    
    println!("\n=== Controllers Overview ===");
    for controller in &analysis.controllers {
        let complexity_avg = if !controller.handlers.is_empty() {
            controller.handlers.iter().map(|h| h.complexity).sum::<u32>() as f64 / controller.handlers.len() as f64
        } else {
            0.0
        };
        
        println!("{:15} | {:2} handlers | avg complexity: {:.1}", 
            controller.name, 
            controller.handlers.len(), 
            complexity_avg
        );
        
        // Show handlers with their routes
        for handler in &controller.handlers {
            let routes_str = if handler.routes.is_empty() {
                "no routes".to_string()
            } else {
                handler.routes.iter()
                    .map(|r| format!("{} {}", r.method, r.path))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            println!("  {:25} → {} (complexity: {})", 
                handler.qualified_name,
                routes_str,
                handler.complexity
            );
        }
        println!();
    }
    
    // Overall patterns analysis
    let all_handlers: Vec<&crate::analyzer::HandlerInfo> = analysis.controllers.iter()
        .flat_map(|c| &c.handlers)
        .collect();
    
    if !all_handlers.is_empty() {
        println!("=== Handler Patterns ===");
        println!("Total handlers: {}", all_handlers.len());
        
        let total_routes: usize = all_handlers.iter().map(|h| h.routes.len()).sum();
        println!("Total routes: {}", total_routes);
        
        let handlers_with_routes = all_handlers.iter().filter(|h| !h.routes.is_empty()).count();
        println!("Handlers with routes: {}/{}", handlers_with_routes, all_handlers.len());
        
        println!("\nComplexity distribution:");
        let low_complexity = all_handlers.iter().filter(|h| h.complexity <= 5).count();
        let medium_complexity = all_handlers.iter().filter(|h| h.complexity > 5 && h.complexity <= 10).count();
        let high_complexity = all_handlers.iter().filter(|h| h.complexity > 10).count();
        let avg_complexity = all_handlers.iter().map(|h| h.complexity).sum::<u32>() as f64 / all_handlers.len() as f64;
        
        println!("  Low (≤5):     {} handlers", low_complexity);
        println!("  Medium (6-10): {} handlers", medium_complexity);
        println!("  High (>10):   {} handlers", high_complexity);
        println!("  Average:      {:.1}", avg_complexity);
    }
    
    Ok(())
}

fn display_controller_details(controller: &crate::analyzer::ControllerInfo) {
    println!("File: {}", controller.file_path);
    println!("Handlers: {}", controller.handlers.len());
    
    for handler in &controller.handlers {
        println!("\n  Handler: {}", handler.qualified_name);
        println!("    Complexity: {}", handler.complexity);
        
        if !handler.routes.is_empty() {
            println!("    Routes:");
            for route in &handler.routes {
                let params_str = if route.parameters.is_empty() {
                    String::new()
                } else {
                    format!(" (params: {})", route.parameters.join(", "))
                };
                println!("      - {} {}{}", route.method, route.path, params_str);
            }
        } else {
            println!("    Routes: none");
        }
    }
}