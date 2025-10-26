use std::path::PathBuf;
use anyhow::Result;
use tokio::time::{Duration, sleep};

use crate::analyzer::ProjectAnalyzer;
use crate::watcher::{ProjectWatcher, EventAggregator};
use tokio::sync::RwLock;
use std::sync::Arc;

pub async fn run(project_path: PathBuf) -> Result<()> {
    println!("🔍 Starting file watcher for RustF project: {}", project_path.display());
    
    // Initialize project analyzer
    let analyzer = ProjectAnalyzer::new(project_path.clone())?;
    let analyzer_arc = Arc::new(RwLock::new(analyzer));
    
    // Create file watcher
    let (watcher, mut receiver) = ProjectWatcher::new(project_path.clone(), analyzer_arc.clone())?;
    let watcher = Arc::new(watcher);
    
    // Analyze initial project dependencies
    println!("🔍 Analyzing project dependencies...");
    watcher.analyze_project_dependencies().await?;
    
    // Display dependency statistics
    let stats = watcher.get_dependency_statistics().await;
    println!("📊 Dependency Analysis Complete:");
    println!("   📁 Files analyzed: {}", stats.total_files);
    println!("   🔗 Dependencies found: {}", stats.total_dependencies);
    if !stats.files_by_type.is_empty() {
        println!("   📋 File types:");
        for (file_type, count) in &stats.files_by_type {
            println!("      {}: {}", file_type, count);
        }
    }
    println!();

    // Start watching
    watcher.start_watching().await?;
    
    // Create event aggregator for summarizing changes
    let aggregator = Arc::new(RwLock::new(EventAggregator::new(5))); // 5-second window
    
    println!("✅ File watcher started successfully!");
    println!("📁 Watching for changes in:");
    println!("   • Rust source files (*.rs)");
    println!("   • HTML templates (*.html)"); 
    println!("   • Configuration files (*.toml)");
    println!("   • Documentation files (*.md)");
    println!("⏹️  Press Ctrl+C to stop watching");
    println!();
    
    // Event processing loop
    let aggregator_clone = aggregator.clone();
    let processing_task = tokio::spawn(async move {
        while let Some(event) = receiver.recv().await {
            println!("🔔 File change detected:");
            println!("   📄 Path: {}", event.file_path.display());
            println!("   🔄 Type: {:?}", event.event_type);
            println!("   🕐 Time: {}", event.timestamp.format("%H:%M:%S"));
            
            if !event.affected_components.is_empty() {
                println!("   📋 Affected components:");
                for component in &event.affected_components {
                    match component {
                        crate::watcher::AffectedComponent::Controller { name } => {
                            println!("      🎮 Controller: {}", name);
                        }
                        crate::watcher::AffectedComponent::Route { method, path } => {
                            println!("      🛤️  Route: {} {}", method, path);
                        }
                        crate::watcher::AffectedComponent::Handler { qualified_name } => {
                            println!("      ⚡ Handler: {}", qualified_name);
                        }
                        crate::watcher::AffectedComponent::Middleware { name } => {
                            println!("      🔗 Middleware: {}", name);
                        }
                        crate::watcher::AffectedComponent::Model { name } => {
                            println!("      📊 Model: {}", name);
                        }
                        crate::watcher::AffectedComponent::View { name } => {
                            println!("      👁️  View: {}", name);
                        }
                        crate::watcher::AffectedComponent::Config => {
                            println!("      ⚙️  Configuration");
                        }
                    }
                }
            }
            println!();
            
            // Add to aggregator
            {
                let mut agg = aggregator_clone.write().await;
                agg.add_event(event);
            }
        }
    });
    
    // Summary reporting task
    let aggregator_clone = aggregator.clone();
    let summary_task = tokio::spawn(async move {
        loop {
            sleep(Duration::from_secs(10)).await;
            
            let summary = {
                let agg = aggregator_clone.read().await;
                agg.get_summary()
            };
            
            if summary.total_events > 0 {
                println!("📊 Change Summary (last 5 seconds):");
                println!("   📈 Total events: {}", summary.total_events);
                
                if !summary.event_types.is_empty() {
                    println!("   📋 Event types:");
                    for (event_type, count) in &summary.event_types {
                        println!("      {} events: {}", event_type, count);
                    }
                }
                
                if !summary.affected_controllers.is_empty() {
                    println!("   🎮 Controllers affected: {}", summary.affected_controllers.join(", "));
                }
                
                if !summary.affected_routes.is_empty() {
                    println!("   🛤️  Routes affected: {}", summary.affected_routes.len());
                }
                
                if !summary.affected_handlers.is_empty() {
                    println!("   ⚡ Handlers affected: {}", summary.affected_handlers.join(", "));
                }
                
                if summary.config_changed {
                    println!("   ⚙️  Configuration changed");
                }
                
                println!("   🕐 Summary time: {}", summary.timestamp.format("%H:%M:%S"));
                println!();
                
                // Clear old events
                {
                    let mut agg = aggregator_clone.write().await;
                    agg.clear();
                }
            }
        }
    });
    
    // Wait for Ctrl+C
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            println!("🛑 Received interrupt signal, stopping file watcher...");
        }
        _ = processing_task => {
            println!("⚠️  Event processing task ended unexpectedly");
        }
        _ = summary_task => {
            println!("⚠️  Summary task ended unexpectedly");
        }
    }
    
    watcher.stop_watching();
    println!("✅ File watcher stopped successfully");
    
    Ok(())
}