use std::path::{Path, PathBuf};
use std::sync::Arc;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::{mpsc, RwLock};
use anyhow::{Result, Context};
use serde::Serialize;

use crate::analyzer::ProjectAnalyzer;

pub mod events;
pub mod dependencies;

pub use events::*;
pub use dependencies::*;

#[derive(Debug, Clone, Serialize)]
pub struct FileChangeEvent {
    pub event_type: FileChangeType,
    pub file_path: PathBuf,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub affected_components: Vec<AffectedComponent>,
}

#[derive(Debug, Clone, Serialize)]
pub enum FileChangeType {
    Created,
    Modified,
    Deleted,
    Renamed { from: PathBuf, to: PathBuf },
}

#[derive(Debug, Clone, Serialize)]
pub enum AffectedComponent {
    Controller { name: String },
    Route { method: String, path: String },
    Handler { qualified_name: String },
    Middleware { name: String },
    Model { name: String },
    View { name: String },
    Config,
}

pub struct ProjectWatcher {
    project_path: PathBuf,
    analyzer: Arc<RwLock<ProjectAnalyzer>>,
    dependency_tracker: Arc<RwLock<DependencyTracker>>,
    event_sender: mpsc::UnboundedSender<FileChangeEvent>,
    _watcher: RecommendedWatcher, // Keep alive
}

impl ProjectWatcher {
    pub fn new(
        project_path: PathBuf,
        analyzer: Arc<RwLock<ProjectAnalyzer>>,
    ) -> Result<(Self, mpsc::UnboundedReceiver<FileChangeEvent>)> {
        let dependency_tracker = Arc::new(RwLock::new(DependencyTracker::new()));
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        
        let sender_clone = event_sender.clone();
        let project_path_clone = project_path.clone();
        let analyzer_clone = analyzer.clone();
        let dependency_tracker_clone = dependency_tracker.clone();
        
        let mut watcher = notify::recommended_watcher(move |res| {
            match res {
                Ok(event) => {
                    let sender = sender_clone.clone();
                    let project_path = project_path_clone.clone();
                    let analyzer = analyzer_clone.clone();
                    let dependency_tracker = dependency_tracker_clone.clone();
                    
                    // Handle in current thread context - convert to sync processing
                    std::thread::spawn(move || {
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        rt.block_on(async move {
                            if let Some(change_event) = Self::process_notify_event(event, &project_path, &analyzer, &dependency_tracker).await {
                                if let Err(e) = sender.send(change_event) {
                                    log::error!("Failed to send file change event: {}", e);
                                }
                            }
                        });
                    });
                }
                Err(e) => {
                    log::error!("File watcher error: {:?}", e);
                }
            }
        })?;
        
        // Watch the project directory recursively
        watcher.watch(&project_path, RecursiveMode::Recursive)
            .with_context(|| format!("Failed to watch directory: {}", project_path.display()))?;
        
        // Also watch specific directories if they exist
        let watch_dirs = ["src", "views", "config.toml"];
        for dir in &watch_dirs {
            let path = project_path.join(dir);
            if path.exists() {
                if let Err(e) = watcher.watch(&path, RecursiveMode::Recursive) {
                    log::warn!("Failed to watch {}: {}", path.display(), e);
                }
            }
        }
        
        Ok((
            Self {
                project_path,
                analyzer,
                dependency_tracker,
                event_sender,
                _watcher: watcher,
            },
            event_receiver,
        ))
    }
    
    async fn process_notify_event(
        event: Event,
        project_path: &Path,
        analyzer: &Arc<RwLock<ProjectAnalyzer>>,
        dependency_tracker: &Arc<RwLock<DependencyTracker>>,
    ) -> Option<FileChangeEvent> {
        // Filter out non-relevant events
        if !Self::is_relevant_event(&event) {
            return None;
        }
        
        let paths = event.paths;
        if paths.is_empty() {
            return None;
        }
        
        let primary_path = &paths[0];
        
        // Skip if not a Rust, HTML, or config file
        if !Self::is_watched_file_type(primary_path) {
            return None;
        }
        
        // Skip temporary files and hidden files
        if Self::is_temporary_file(primary_path) {
            return None;
        }
        
        let event_type = match event.kind {
            EventKind::Create(_) => FileChangeType::Created,
            EventKind::Remove(_) => FileChangeType::Deleted,
            EventKind::Modify(notify::event::ModifyKind::Name(notify::event::RenameMode::Both)) => {
                if paths.len() >= 2 {
                    FileChangeType::Renamed {
                        from: paths[0].clone(),
                        to: paths[1].clone(),
                    }
                } else {
                    FileChangeType::Modified
                }
            }
            EventKind::Modify(_) => FileChangeType::Modified,
            _ => return None,
        };
        
        // Analyze what components are affected using dependency tracking
        let affected_components = Self::analyze_affected_components_with_dependencies(
            primary_path, 
            project_path, 
            analyzer, 
            dependency_tracker
        ).await;
        
        Some(FileChangeEvent {
            event_type,
            file_path: primary_path.clone(),
            timestamp: chrono::Utc::now(),
            affected_components,
        })
    }
    
    fn is_relevant_event(event: &Event) -> bool {
        matches!(
            event.kind,
            EventKind::Create(_) | 
            EventKind::Modify(_) | 
            EventKind::Remove(_)
        )
    }
    
    fn is_watched_file_type(path: &Path) -> bool {
        if let Some(extension) = path.extension().and_then(|s| s.to_str()) {
            matches!(extension, "rs" | "html" | "toml" | "md")
        } else {
            // Watch files without extensions that might be important
            path.file_name()
                .and_then(|s| s.to_str())
                .map(|name| matches!(name, "Cargo.toml" | "config.toml" | ".env"))
                .unwrap_or(false)
        }
    }
    
    fn is_temporary_file(path: &Path) -> bool {
        path.file_name()
            .and_then(|s| s.to_str())
            .map(|name| {
                name.starts_with('.') && name != ".env" ||
                name.ends_with('~') ||
                name.ends_with(".tmp") ||
                name.ends_with(".swp") ||
                name.contains(".#")
            })
            .unwrap_or(false)
    }
    
    async fn analyze_affected_components_with_dependencies(
        file_path: &Path,
        project_path: &Path,
        analyzer: &Arc<RwLock<ProjectAnalyzer>>,
        dependency_tracker: &Arc<RwLock<DependencyTracker>>,
    ) -> Vec<AffectedComponent> {
        let mut affected = Vec::new();
        
        // Update dependency tracking for this file
        if let Ok(mut tracker) = dependency_tracker.try_write() {
            if let Ok(_dep_info) = tracker.analyze_file_dependencies(file_path, project_path).await {
                // Get all files that depend on this changed file
                let dependent_files = tracker.get_affected_files(file_path);
                
                // Analyze the original file
                let direct_components = Self::analyze_affected_components(file_path, project_path, analyzer).await;
                affected.extend(direct_components);
                
                // Analyze dependent files for additional affected components
                for dependent_file in dependent_files {
                    let dependent_components = Self::analyze_affected_components(&dependent_file, project_path, analyzer).await;
                    affected.extend(dependent_components);
                }
            }
        }
        
        // If dependency tracking fails, fall back to original method
        if affected.is_empty() {
            affected = Self::analyze_affected_components(file_path, project_path, analyzer).await;
        }
        
        // Remove duplicates
        affected.sort_by(|a, b| format!("{:?}", a).cmp(&format!("{:?}", b)));
        affected.dedup_by(|a, b| format!("{:?}", a) == format!("{:?}", b));
        
        affected
    }

    async fn analyze_affected_components(
        file_path: &Path,
        project_path: &Path,
        analyzer: &Arc<RwLock<ProjectAnalyzer>>,
    ) -> Vec<AffectedComponent> {
        let mut affected = Vec::new();
        
        // Determine the relative path from project root
        let relative_path = if let Ok(rel) = file_path.strip_prefix(project_path) {
            rel
        } else {
            return affected;
        };
        
        // Analyze based on file location and type
        if let Some(component) = relative_path.components().next() {
            let component_str = component.as_os_str().to_string_lossy();
            
            match component_str.as_ref() {
                "src" => {
                    if let Some(subcomponent) = relative_path.components().nth(1) {
                        let sub_str = subcomponent.as_os_str().to_string_lossy();
                        match sub_str.as_ref() {
                            "controllers" => {
                                if let Some(name) = file_path.file_stem().and_then(|s| s.to_str()) {
                                    affected.push(AffectedComponent::Controller {
                                        name: name.to_string(),
                                    });
                                    
                                    // Try to analyze handlers and routes in this controller
                                    if let Ok(analyzer_guard) = analyzer.try_read() {
                                        if let Ok(analysis) = analyzer_guard.analyze_complete(false).await {
                                            for controller in &analysis.controllers {
                                                if controller.name == name {
                                                    for handler in &controller.handlers {
                                                        affected.push(AffectedComponent::Handler {
                                                            qualified_name: handler.qualified_name.clone(),
                                                        });
                                                        
                                                        for route in &handler.routes {
                                                            affected.push(AffectedComponent::Route {
                                                                method: route.method.clone(),
                                                                path: route.path.clone(),
                                                            });
                                                        }
                                                    }
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            "middleware" => {
                                if let Some(name) = file_path.file_stem().and_then(|s| s.to_str()) {
                                    affected.push(AffectedComponent::Middleware {
                                        name: name.to_string(),
                                    });
                                }
                            }
                            "models" => {
                                if let Some(name) = file_path.file_stem().and_then(|s| s.to_str()) {
                                    affected.push(AffectedComponent::Model {
                                        name: name.to_string(),
                                    });
                                }
                            }
                            _ => {}
                        }
                    }
                }
                "views" => {
                    if let Some(name) = file_path.file_stem().and_then(|s| s.to_str()) {
                        affected.push(AffectedComponent::View {
                            name: name.to_string(),
                        });
                    }
                }
                _ => {
                    if file_path.file_name() == Some(std::ffi::OsStr::new("config.toml")) ||
                       file_path.file_name() == Some(std::ffi::OsStr::new("Cargo.toml")) {
                        affected.push(AffectedComponent::Config);
                    }
                }
            }
        }
        
        affected
    }
    
    pub async fn start_watching(&self) -> Result<()> {
        log::info!("Started watching project: {}", self.project_path.display());
        Ok(())
    }
    
    pub fn stop_watching(&self) {
        log::info!("Stopped watching project: {}", self.project_path.display());
    }

    pub async fn get_dependency_statistics(&self) -> DependencyStatistics {
        let tracker = self.dependency_tracker.read().await;
        tracker.get_statistics()
    }

    pub async fn analyze_project_dependencies(&self) -> Result<()> {
        use crate::analyzer::files::ProjectFiles;
        
        let files = ProjectFiles::scan(&self.project_path)?;
        let mut tracker = self.dependency_tracker.write().await;
        
        // Analyze all Rust files
        let all_files = [
            &files.controllers[..],
            &files.models[..],
            &files.middleware[..],
        ].concat();
        
        for file_path in all_files {
            if let Err(e) = tracker.analyze_file_dependencies(&file_path, &self.project_path).await {
                log::warn!("Failed to analyze dependencies for {}: {}", file_path.display(), e);
            }
        }
        
        log::info!("Completed dependency analysis for {} files", tracker.analysis_cache.len());
        Ok(())
    }
}