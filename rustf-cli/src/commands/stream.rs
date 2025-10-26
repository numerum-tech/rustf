//! Streaming analysis command for memory-optimized large project analysis

use crate::analyzer::{StreamingAnalyzer, StreamingConfigBuilder};
use std::path::PathBuf;
use anyhow::Result;

pub async fn run(
    project_path: PathBuf,
    memory_limit: usize,
    chunk_size: usize,
    max_concurrent: usize,
    aggressive_cleanup: bool,
    format: String,
) -> Result<()> {
    println!("🔄 Starting streaming analysis for large project...");
    println!("📁 Project Path: {}", project_path.display());
    println!("💾 Memory Limit: {} MB", memory_limit);
    println!("📦 Chunk Size: {}", chunk_size);
    println!("⚡ Max Concurrent: {}", max_concurrent);
    println!("🧹 Aggressive Cleanup: {}", if aggressive_cleanup { "enabled" } else { "disabled" });
    println!();

    // Build streaming configuration
    let config = StreamingConfigBuilder::new()
        .memory_limit_mb(memory_limit)
        .chunk_size(chunk_size)
        .max_concurrent_files(max_concurrent)
        .aggressive_cleanup(aggressive_cleanup)
        .enable_memory_monitoring(true)
        .build();

    // Create streaming analyzer
    let mut analyzer = StreamingAnalyzer::new(project_path, config)?;

    // Perform streaming analysis
    let start_time = std::time::Instant::now();
    let analysis = analyzer.analyze_streaming().await?;
    let duration = start_time.elapsed();

    // Get memory statistics
    let memory_stats = analyzer.get_memory_stats();

    // Display results based on format
    match format.as_str() {
        "json" => {
            let json_output = serde_json::to_string_pretty(&analysis)?;
            println!("{}", json_output);
        }
        "yaml" => {
            let yaml_output = serde_yaml::to_string(&analysis)?;
            println!("{}", yaml_output);
        }
        "table" | _ => {
            display_streaming_results(&analysis, memory_stats, duration);
        }
    }

    Ok(())
}

fn display_streaming_results(
    analysis: &crate::analyzer::ProjectAnalysis,
    memory_stats: &crate::analyzer::MemoryStats,
    duration: std::time::Duration,
) {
    println!("🎯 === Streaming Analysis Results ===\n");

    // Project Overview
    println!("📋 Project Information:");
    println!("   Name: {}", analysis.project_name);
    println!("   Framework: {}", analysis.framework_version);
    println!();

    // Component Summary
    println!("📊 Component Summary:");
    println!("   Controllers: {}", analysis.controllers.len());
    println!("   Routes: {}", analysis.routes.len());
    println!("   Middleware: {}", analysis.middleware.len());
    println!("   Models: {}", analysis.models.len());
    println!("   Views: {}", analysis.views.len());
    println!("   Issues: {}", analysis.issues.len());
    println!();

    // Performance Metrics
    println!("⚡ Performance Metrics:");
    println!("   Analysis Duration: {:.2}s", duration.as_secs_f64());
    println!("   Peak Memory Usage: {:.2} MB", memory_stats.peak_memory_mb);
    println!("   Files Processed: {}", memory_stats.files_processed);
    println!("   Chunks Processed: {}", memory_stats.chunks_processed);
    println!("   Memory Warnings: {}", memory_stats.memory_warnings);
    println!("   GC Collections: {}", memory_stats.gc_collections_triggered);
    println!();

    // Memory Efficiency
    let memory_efficiency = if memory_stats.peak_memory_mb > 0.0 {
        (memory_stats.files_processed as f64) / memory_stats.peak_memory_mb
    } else {
        0.0
    };
    
    println!("📈 Memory Efficiency:");
    println!("   Files per MB: {:.1}", memory_efficiency);
    if memory_stats.memory_warnings > 0 {
        println!("   ⚠️  Memory limit exceeded {} times", memory_stats.memory_warnings);
    } else {
        println!("   ✅ Memory usage stayed within limits");
    }
    println!();

    // Route Summary
    if !analysis.routes.is_empty() {
        println!("🛣️  Route Summary:");
        let mut methods = std::collections::HashMap::new();
        for route in &analysis.routes {
            *methods.entry(&route.method).or_insert(0) += 1;
        }
        
        for (method, count) in methods {
            println!("   {}: {}", method, count);
        }
        println!();
    }

    // Controllers with Handlers
    if !analysis.controllers.is_empty() {
        println!("🎮 Controllers:");
        for controller in &analysis.controllers {
            println!("   {} ({} handlers)", controller.name, controller.handlers.len());
            for handler in &controller.handlers {
                let route_count = handler.routes.len();
                let complexity_indicator = match handler.complexity {
                    0..=5 => "🟢",
                    6..=15 => "🟡", 
                    _ => "🔴",
                };
                println!("     {} {} ({} routes, complexity: {}) {}", 
                        complexity_indicator, handler.name, route_count, handler.complexity, complexity_indicator);
            }
        }
        println!();
    }

    // Issues
    if !analysis.issues.is_empty() {
        println!("⚠️  Issues Found:");
        for issue in &analysis.issues {
            let severity_icon = match issue.severity.as_str() {
                "error" => "❌",
                "warning" => "⚠️",
                _ => "ℹ️",
            };
            println!("   {} {}", severity_icon, issue.message);
        }
        println!();
    }

    // Recommendations
    println!("💡 Performance Recommendations:");
    if memory_stats.peak_memory_mb > 200.0 {
        println!("   📦 Consider reducing chunk size for better memory control");
    }
    if memory_stats.gc_collections_triggered > 5 {
        println!("   🧹 Frequent GC detected - consider lowering memory limit");
    }
    if memory_stats.files_processed < 100 && memory_stats.peak_memory_mb < 50.0 {
        println!("   🚀 Small project detected - regular analysis might be faster");
    }
    if memory_stats.memory_warnings == 0 && memory_stats.peak_memory_mb < 100.0 {
        println!("   ✅ Excellent memory efficiency - streaming worked well!");
    }
}