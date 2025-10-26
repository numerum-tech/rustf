use std::path::PathBuf;
use anyhow::Result;
use crate::analyzer::ProjectAnalyzer;
use crate::analysis::middleware::{MiddlewareAnalyzer, MiddlewareAnalysis};

pub async fn run(project_path: PathBuf, conflicts: bool) -> Result<()> {
    log::info!("Analyzing middleware...");
    
    let analyzer = ProjectAnalyzer::new(project_path)?;
    let analysis = analyzer.analyze_complete(false).await?;
    
    // Analyze middleware files in parallel
    use crate::analyzer::files::ProjectFiles;
    use rayon::prelude::*;
    
    let files = ProjectFiles::scan_parallel(&analyzer.project_path)?;
    
    let middleware_analyses: Vec<MiddlewareAnalysis> = files.middleware
        .par_iter()
        .filter_map(|middleware_path| {
            match MiddlewareAnalyzer::analyze_middleware(middleware_path) {
                Ok(middleware_analysis) => Some(middleware_analysis),
                Err(e) => {
                    log::warn!("Failed to analyze middleware {}: {}", middleware_path.display(), e);
                    None
                }
            }
        })
        .collect();
    
    // Build middleware chain
    let middleware_chain = MiddlewareAnalyzer::build_middleware_chain(&middleware_analyses, &analysis.routes);
    
    if conflicts {
        println!("=== Middleware Conflicts ===");
        if middleware_chain.conflicts.is_empty() {
            println!("✅ No middleware conflicts found");
        } else {
            for conflict in &middleware_chain.conflicts {
                println!("❌ {} <-> {}: {} - {}", 
                    conflict.middleware1,
                    conflict.middleware2,
                    conflict.conflict_type,
                    conflict.description
                );
            }
        }
        return Ok(());
    }
    
    println!("=== Middleware Analysis ===\n");
    println!("Total Middleware: {}", middleware_chain.middleware_list.len());
    println!("Execution Order: {}", middleware_chain.execution_order.join(" -> "));
    println!("Conflicts: {}", middleware_chain.conflicts.len());
    
    if !middleware_chain.middleware_list.is_empty() {
        println!("\n=== Middleware Details ===");
        for middleware in &middleware_chain.middleware_list {
            println!("\n--- {} ---", middleware.name);
            println!("  Type: {:?}", middleware.middleware_type);
            println!("  File: {}", middleware.file_path);
            println!("  Implements trait: {}", middleware.implements_trait);
            println!("  Has install function: {}", middleware.has_install_function);
            
            if let Some(order) = middleware.execution_order {
                println!("  Execution order: {}", order);
            }
            
            println!("  Scope:");
            println!("    Global: {}", middleware.scope.global);
            println!("    Route specific: {}", middleware.scope.route_specific);
            println!("    Controller specific: {}", middleware.scope.controller_specific);
            
            if !middleware.scope.patterns.is_empty() {
                println!("    Patterns: {}", middleware.scope.patterns.join(", "));
            }
            
            if !middleware.dependencies.is_empty() {
                println!("  Dependencies: {}", middleware.dependencies.join(", "));
            }
        }
    }
    
    // Coverage analysis
    println!("\n=== Coverage Analysis ===");
    println!("Total routes: {}", middleware_chain.coverage_analysis.total_routes);
    println!("Protected routes: {}", middleware_chain.coverage_analysis.protected_routes);
    println!("Logging coverage: {:.1}%", middleware_chain.coverage_analysis.logging_coverage);
    println!("Auth coverage: {:.1}%", middleware_chain.coverage_analysis.auth_coverage);
    println!("CORS coverage: {:.1}%", middleware_chain.coverage_analysis.cors_coverage);
    
    // Patterns analysis
    let patterns = MiddlewareAnalyzer::analyze_middleware_patterns(&middleware_chain.middleware_list);
    println!("\n=== Middleware Patterns ===");
    println!("Total middleware: {}", patterns.total_middleware);
    println!("Custom middleware: {}", patterns.custom_middleware);
    println!("Global middleware: {}", patterns.global_middleware);
    
    if !patterns.type_distribution.is_empty() {
        println!("\nType distribution:");
        for (middleware_type, count) in &patterns.type_distribution {
            println!("  {}: {}", middleware_type, count);
        }
    }
    
    // Recommendations
    println!("\n=== Recommendations ===");
    if middleware_chain.coverage_analysis.logging_coverage < 100.0 {
        println!("- Consider adding global logging middleware");
    }
    if middleware_chain.coverage_analysis.auth_coverage < 50.0 && middleware_chain.coverage_analysis.total_routes > 0 {
        println!("- Consider adding authentication middleware for API routes");
    }
    if middleware_chain.coverage_analysis.cors_coverage < 100.0 {  
        println!("- Consider adding CORS middleware for browser compatibility");
    }
    if middleware_chain.conflicts.is_empty() {
        println!("✅ No middleware conflicts detected");
    }
    
    Ok(())
}