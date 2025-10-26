//! Performance benchmarking for the RustF CLI

use crate::analyzer::ProjectAnalyzer;
use std::path::PathBuf;
use std::time::Instant;
use anyhow::Result;

pub async fn run(project_path: PathBuf, iterations: usize) -> Result<()> {
    println!("=== RustF CLI Performance Benchmark ===\n");
    println!("Project: {}", project_path.display());
    println!("Iterations: {}\n", iterations);
    
    let analyzer = ProjectAnalyzer::new(project_path)?;
    let mut times = Vec::new();
    
    println!("Running benchmark...");
    
    for i in 1..=iterations {
        print!("  Iteration {}/{} ... ", i, iterations);
        
        let start = Instant::now();
        let _analysis = analyzer.analyze_complete(false).await?;
        let duration = start.elapsed();
        
        times.push(duration.as_millis());
        println!("{}ms", duration.as_millis());
    }
    
    // Calculate statistics
    let total_time: u128 = times.iter().sum();
    let avg_time = total_time as f64 / times.len() as f64;
    let min_time = *times.iter().min().unwrap();
    let max_time = *times.iter().max().unwrap();
    
    // Calculate cache effectiveness
    let cache_stats = analyzer.get_cache_stats();
    
    println!("\n=== Performance Results ===");
    println!("Average Time: {:.1}ms", avg_time);
    println!("Min Time: {}ms", min_time);
    println!("Max Time: {}ms", max_time);
    println!("Total Time: {}ms", total_time);
    
    let improvement = if times.len() > 1 {
        let improvement_value = ((max_time as f64 - min_time as f64) / max_time as f64) * 100.0;
        println!("Performance Improvement: {:.1}% (first vs best)", improvement_value);
        improvement_value
    } else {
        0.0
    };
    
    println!("\n=== Cache Statistics ===");
    println!("Total Requests: {}", cache_stats.total_requests);
    println!("Cache Hit Rate: {:.1}%", cache_stats.hit_rate);
    println!("Cache Entries: {}", cache_stats.entries_count);
    
    if cache_stats.hit_rate >= 80.0 {
        println!("\n✅ Excellent caching performance!");
    } else if cache_stats.hit_rate >= 50.0 {
        println!("\n⚠️  Good caching performance, but could be better.");
    } else if cache_stats.total_requests > 10 {
        println!("\n❌ Poor caching performance. Files may be changing or cache is ineffective.");
    }
    
    // Performance recommendations
    println!("\n=== Recommendations ===");
    if avg_time > 1000.0 {
        println!("• Consider using parallel analysis for large projects");
        println!("• Large projects benefit from increased cache size");
    }
    if cache_stats.hit_rate < 80.0 && iterations > 3 {
        println!("• Low cache hit rate suggests files are changing between runs");
        println!("• Consider file watching mode for development");
    }
    if times.len() > 1 && improvement > 50.0 {
        println!("• Significant improvement shows caching is working well!");
        println!("• Repeated analysis of the same project will be much faster");
    }
    
    Ok(())
}