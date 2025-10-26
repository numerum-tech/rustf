//! Cache statistics command for monitoring analysis cache performance

use crate::analyzer::analysis_cache::{get_cache_statistics, global_analysis_cache};
use std::path::PathBuf;
use anyhow::Result;

pub async fn run(_project_path: PathBuf) -> Result<()> {
    println!("=== Analysis Cache Statistics ===\n");
    
    if let Some(stats) = get_cache_statistics().await {
        println!("📊 Cache Performance:");
        println!("   Total Requests: {}", stats.total_requests);
        println!("   Cache Hits: {}", stats.hits);
        println!("   Cache Misses: {}", stats.misses);
        println!("   Hit Rate: {:.2}%", stats.hit_rate * 100.0);
        println!("   Invalidations: {}", stats.invalidations);
        println!();
        
        println!("💾 Cache Size:");
        println!("   Current Entries: {}", stats.cache_size);
        println!();
        
        // Get detailed utilization metrics
        {
            let cache_arc = global_analysis_cache();
            let cache = cache_arc.lock().await;
            let utilization = cache.get_utilization_metrics();
            
            println!("🔍 Cache Utilization:");
            println!("   Capacity: {}", utilization.capacity);
            println!("   Current Size: {}", utilization.current_size);
            println!("   Utilization: {:.1}%", utilization.utilization_percentage);
            println!("   Average Entry Age: {:.1}s", utilization.average_age_seconds);
            println!("   Oldest Entry Age: {:.1}s", utilization.oldest_entry_age_seconds);
            println!("   Tracked Files: {}", utilization.tracked_files_count);
            println!();
            
            if !utilization.most_accessed_keys.is_empty() {
                println!("🔥 Most Accessed Cache Keys:");
                for (i, key) in utilization.most_accessed_keys.iter().take(5).enumerate() {
                    println!("   {}. {}", i + 1, key);
                }
                println!();
            }
        }
        
        // Performance recommendations
        println!("💡 Performance Recommendations:");
        if stats.hit_rate < 0.5 {
            println!("   ⚠️  Low hit rate ({:.1}%) - consider increasing cache TTL", stats.hit_rate * 100.0);
        } else if stats.hit_rate > 0.8 {
            println!("   ✅ Excellent hit rate ({:.1}%)", stats.hit_rate * 100.0);
        } else {
            println!("   ✨ Good hit rate ({:.1}%)", stats.hit_rate * 100.0);
        }
        
        if stats.cache_size == 0 {
            println!("   ℹ️  Cache is empty - run some analysis commands to populate it");
        }
        
        if stats.invalidations > stats.hits {
            println!("   ⚠️  High invalidation rate - files are changing frequently");
        }
        
    } else {
        println!("❌ Failed to retrieve cache statistics");
        println!("   Cache may not be initialized or accessible");
    }
    
    Ok(())
}