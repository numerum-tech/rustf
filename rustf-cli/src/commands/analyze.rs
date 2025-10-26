use crate::analyzer::ProjectAnalyzer;
use crate::utils::{AnalysisUtils, FormatUtils, DataTransformer};
use std::path::PathBuf;
use anyhow::Result;

pub async fn run(project_path: PathBuf, format: String, detailed: bool) -> Result<()> {
    log::info!("Starting complete project analysis...");
    
    let analyzer = ProjectAnalyzer::new(project_path)?;
    let analysis = analyzer.analyze_complete(detailed).await?;
    
    match format.as_str() {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&analysis)?);
        }
        "yaml" => {
            println!("{}", serde_yaml::to_string(&analysis)?);
        }
        "api" => {
            // Use new transformer for API-friendly JSON
            let api_json = DataTransformer::to_api_json(&analysis);
            println!("{}", serde_json::to_string_pretty(&api_json)?);
        }
        "csv" => {
            // Use new transformer for CSV output
            let csv_data = DataTransformer::to_csv_data(&analysis);
            for row in csv_data {
                println!("{}", row.join(","));
            }
        }
        "table" | _ => {
            // Enhanced table output using new utilities
            let stats = AnalysisUtils::get_stats_summary(&analysis);
            
            println!("=== RustF Project Analysis ===\n");
            println!("📊 Project Overview:");
            println!("   Name: {}", analysis.project_name);
            println!("   Framework: {}", analysis.framework_version);
            println!();
            
            println!("📈 Component Summary:");
            println!("   🎮 Controllers: {}", stats.controllers_count);
            println!("   🛣️  Routes: {}", stats.routes_count);
            println!("   🛡️  Middleware: {}", stats.middleware_count);
            println!("   📦 Models: {}", stats.models_count);
            println!("   👁️  Views: {}", stats.views_count);
            println!("   ⚠️  Issues: {}", stats.issues_count);
            println!();
            
            // Complexity analysis using new utilities
            println!("🎯 Complexity Analysis:");
            println!("   Average: {:.1}", stats.complexity_stats.avg_complexity);
            println!("   Maximum: {}", stats.complexity_stats.max_complexity);
            println!("   {} {} Low (≤5) {} {} Medium (6-15) {} {} High (>15)",
                    FormatUtils::complexity_indicator(3), stats.complexity_stats.low_complexity,
                    FormatUtils::complexity_indicator(10), stats.complexity_stats.medium_complexity,
                    FormatUtils::complexity_indicator(20), stats.complexity_stats.high_complexity);
            println!();
            
            // Route method distribution
            if !stats.route_methods.is_empty() {
                println!("🌐 Route Methods:");
                for (method, count) in &stats.route_methods {
                    println!("   {}: {}", method, count);
                }
                println!();
            }
            
            // Security overview using new utilities
            if stats.security_stats.error_issues > 0 || stats.security_stats.warning_issues > 0 {
                println!("🔒 Security Overview:");
                if stats.security_stats.error_issues > 0 {
                    println!("   {} {} Critical Issues", FormatUtils::severity_indicator("error"), stats.security_stats.error_issues);
                }
                if stats.security_stats.warning_issues > 0 {
                    println!("   {} {} Warnings", FormatUtils::severity_indicator("warning"), stats.security_stats.warning_issues);
                }
                if stats.security_stats.views_with_security_issues > 0 {
                    println!("   {} {} Views with Security Issues", FormatUtils::severity_indicator("warning"), stats.security_stats.views_with_security_issues);
                }
                println!();
            }
            
            // Issues with enhanced formatting
            if !analysis.issues.is_empty() {
                println!("⚠️  Issues Found:");
                for issue in &analysis.issues {
                    println!("   {} {}", FormatUtils::severity_indicator(&issue.severity), issue.message);
                }
                println!();
            }
            
            // High complexity handlers using new utilities
            let high_complexity_handlers = AnalysisUtils::find_high_complexity_handlers(&analysis.controllers, 15);
            if !high_complexity_handlers.is_empty() {
                println!("🔥 High Complexity Handlers:");
                for handler in &high_complexity_handlers {
                    println!("   {} {} (complexity: {})", 
                            FormatUtils::complexity_indicator(handler.complexity),
                            handler.qualified_name, 
                            handler.complexity);
                }
                println!();
            }
            
            if detailed {
                println!("=== Route Details ===");
                // Use enhanced route grouping
                let routes_by_method = AnalysisUtils::group_routes_by_method(&analysis.routes);
                for (method, routes) in routes_by_method {
                    println!("\n{} Routes:", method);
                    for route in routes {
                        let params_str = if route.parameters.is_empty() {
                            String::new()
                        } else {
                            format!(" ({})", route.parameters.join(", "))
                        };
                        println!("   {} -> {}{}", route.path, route.handler, params_str);
                    }
                }
            }
        }
    }
    
    // Show cache performance statistics
    let cache_stats = analyzer.get_cache_stats();
    if cache_stats.total_requests > 0 {
        println!("\n=== Cache Performance ===");
        println!("Hit Rate: {:.1}% ({}/{} requests)", 
                cache_stats.hit_rate, 
                cache_stats.cache_hits, 
                cache_stats.total_requests);
        if cache_stats.hit_rate >= 50.0 {
            println!("✅ Cache is improving performance significantly!");
        }
    }
    
    log::info!("Analysis complete");
    Ok(())
}