use std::path::PathBuf;
use anyhow::Result;
use crate::analyzer::{files::ProjectFiles, ProjectAnalyzer};
use std::time::Instant;

pub async fn run(project_path: PathBuf, _filter: Option<String>) -> Result<()> {
    let start_time = Instant::now();
    log::info!("Quick project discovery with parallel scanning...");
    
    // Fast parallel file scanning
    let files = ProjectFiles::scan_parallel(&project_path)?;
    let file_scan_time = start_time.elapsed();
    
    // Quick analysis with caching
    let analyzer = ProjectAnalyzer::new(project_path.clone())?;
    let analysis = analyzer.analyze_complete(false).await?;
    let total_time = start_time.elapsed();
    
    println!("=== 🚀 RustF Project Discovery ===\n");
    println!("📁 Project: {}", project_path.display());
    println!("⏱️  Discovery time: {}ms (file scan: {}ms)", 
             total_time.as_millis(), 
             file_scan_time.as_millis());
    
    // File counts
    println!("\n📊 File Structure:");
    println!("  Controllers: {}", files.controllers.len());
    println!("  Models: {}", files.models.len());
    println!("  Middleware: {}", files.middleware.len());
    println!("  Views: {}", files.views.len());
    println!("  Config files: {}", files.config_files.len());
    
    // Analysis summary
    println!("\n🔍 Code Analysis:");
    println!("  Routes: {}", analysis.routes.len());
    println!("  Handlers: {}", analysis.controllers.iter().map(|c| c.handlers.len()).sum::<usize>());
    println!("  Issues: {}", analysis.issues.len());
    
    // Quick metrics
    let total_routes = analysis.routes.len();
    let total_handlers = analysis.controllers.iter().map(|c| c.handlers.len()).sum::<usize>();
    let avg_routes_per_controller = if !analysis.controllers.is_empty() {
        total_routes as f64 / analysis.controllers.len() as f64
    } else {
        0.0
    };
    
    println!("\n📈 Quick Metrics:");
    println!("  Avg routes per controller: {:.1}", avg_routes_per_controller);
    println!("  Handler coverage: {:.1}%", 
             if total_routes > 0 { 
                 (total_handlers as f64 / total_routes as f64) * 100.0 
             } else { 
                 0.0 
             });
    
    // Framework detection
    println!("\n🔧 Framework:");
    println!("  Type: {}", analysis.framework_version);
    if analysis.framework_version.contains("detected") {
        println!("  Status: ✅ RustF detected");
    } else {
        println!("  Status: ❌ RustF not detected");
    }
    
    // Cache performance
    let cache_stats = analyzer.get_cache_stats();
    if cache_stats.total_requests > 0 {
        println!("\n💾 Cache Performance:");
        println!("  Hit rate: {:.1}%", cache_stats.hit_rate);
        println!("  Entries: {}", cache_stats.entries_count);
    }
    
    // Quick recommendations
    println!("\n💡 Quick Recommendations:");
    if analysis.controllers.is_empty() {
        println!("  • Add controllers in src/controllers/");
    }
    if analysis.routes.is_empty() {
        println!("  • Define routes in controller install() functions");
    }
    if !analysis.issues.is_empty() {
        println!("  • Fix {} issues found during analysis", analysis.issues.len());
    }
    if total_time.as_millis() > 1000 {
        println!("  • Run 'rustf-cli cache-stats' to check cache performance");
    } else {
        println!("  ✅ Fast analysis - caching is working well!");
    }
    
    Ok(())
}