use std::path::PathBuf;
use anyhow::Result;
use crate::analyzer::ProjectAnalyzer;
use crate::routes::RouteAnalyzer;

pub async fn run(project_path: PathBuf, conflicts_only: bool, validate: bool) -> Result<()> {
    log::info!("Analyzing routes...");
    
    let analyzer = ProjectAnalyzer::new(project_path)?;
    let analysis = analyzer.analyze_complete(false).await?;
    
    let route_tree = RouteAnalyzer::build_route_tree(&analysis.routes, &analysis.controllers);
    
    if conflicts_only {
        println!("=== Route Conflicts ===");
        if route_tree.conflicts.is_empty() {
            println!("✅ No route conflicts found");
        } else {
            for conflict in &route_tree.conflicts {
                println!("❌ {} {} -> {}", 
                    conflict.method, 
                    conflict.path,
                    conflict.conflicting_handlers.join(", ")
                );
            }
        }
        return Ok(());
    }
    
    println!("=== Route Analysis ===\n");
    println!("Total Routes: {}", route_tree.routes.len());
    println!("Conflicts: {}", route_tree.conflicts.len());
    println!("Missing Handlers: {}", route_tree.missing_handlers.len());
    println!("Controller Groups: {}", route_tree.route_groups.len());
    
    if !route_tree.routes.is_empty() {
        println!("\n=== All Routes ===");
        for route in &route_tree.routes {
            let params = if route.parameters.is_empty() {
                String::new()
            } else {
                format!(" [params: {}]", route.parameters.join(", "))
            };
            println!("{:>6} {:<30} -> {}{}", 
                route.method, 
                route.path, 
                route.handler,
                params
            );
        }
    }
    
    if !route_tree.conflicts.is_empty() {
        println!("\n=== ❌ Route Conflicts ===");
        for conflict in &route_tree.conflicts {
            println!("{} {} -> {} handlers: {}", 
                conflict.method, 
                conflict.path,
                conflict.conflicting_handlers.len(),
                conflict.conflicting_handlers.join(", ")
            );
        }
    }
    
    if validate && !route_tree.missing_handlers.is_empty() {
        println!("\n=== ❌ Missing Handlers ===");
        for handler in &route_tree.missing_handlers {
            println!("- {}", handler);
        }
    }
    
    if !route_tree.route_groups.is_empty() {
        println!("\n=== Routes by Controller ===");
        for (controller, routes) in &route_tree.route_groups {
            println!("Controller: {} ({} routes)", controller, routes.len());
            for route in routes {
                println!("  {:>6} {} -> {}", route.method, route.path, route.handler);
            }
            println!();
        }
    }
    
    // Show parameter analysis
    let params = RouteAnalyzer::analyze_route_parameters(&route_tree.routes);
    if !params.is_empty() {
        println!("=== Route Parameters ===");
        for (route, parameters) in params {
            println!("{} -> [{}]", route, parameters.join(", "));
        }
    }
    
    Ok(())
}