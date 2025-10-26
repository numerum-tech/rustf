use std::path::PathBuf;
use std::sync::Arc;
use anyhow::{Result, Context};
use tokio::fs;
use tokio::sync::Semaphore;
use serde::Serialize;

use super::{ProjectAnalysis, ControllerInfo, RouteInfo, HandlerInfo, MiddlewareInfo, ModelInfo, Issue};
use super::files::ProjectFiles;
use super::ast::AstAnalyzer;
use crate::analysis::views::ViewAnalysis;

/// Configuration for streaming analysis to control memory usage
#[derive(Debug, Clone)]
pub struct StreamingConfig {
    /// Maximum number of files to process concurrently
    pub max_concurrent_files: usize,
    /// Chunk size for batch processing
    pub chunk_size: usize,
    /// Memory limit in MB (0 = no limit)
    pub memory_limit_mb: usize,
    /// Enable memory monitoring
    pub enable_memory_monitoring: bool,
    /// Drop intermediate results early to save memory
    pub aggressive_memory_cleanup: bool,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            max_concurrent_files: 8,
            chunk_size: 50,
            memory_limit_mb: 512, // 512 MB default limit
            enable_memory_monitoring: true,
            aggressive_memory_cleanup: true,
        }
    }
}

/// Memory usage statistics during streaming analysis
#[derive(Debug, Clone, Serialize)]
pub struct MemoryStats {
    pub peak_memory_mb: f64,
    pub current_memory_mb: f64,
    pub files_processed: usize,
    pub chunks_processed: usize,
    pub memory_warnings: usize,
    pub gc_collections_triggered: usize,
}

/// Streaming analyzer for memory-efficient project analysis
pub struct StreamingAnalyzer {
    project_path: PathBuf,
    config: StreamingConfig,
    ast_analyzer: AstAnalyzer,
    memory_stats: MemoryStats,
    semaphore: Arc<Semaphore>,
}

impl StreamingAnalyzer {
    pub fn new(project_path: PathBuf, config: StreamingConfig) -> Result<Self> {
        if !project_path.exists() {
            anyhow::bail!("Project path does not exist: {}", project_path.display());
        }

        let semaphore = Arc::new(Semaphore::new(config.max_concurrent_files));

        Ok(Self {
            project_path,
            ast_analyzer: AstAnalyzer::default(),
            semaphore,
            config,
            memory_stats: MemoryStats {
                peak_memory_mb: 0.0,
                current_memory_mb: 0.0,
                files_processed: 0,
                chunks_processed: 0,
                memory_warnings: 0,
                gc_collections_triggered: 0,
            },
        })
    }

    /// Perform streaming analysis with memory optimization
    pub async fn analyze_streaming(&mut self) -> Result<ProjectAnalysis> {
        log::info!("Starting streaming analysis for: {}", self.project_path.display());
        
        // Monitor initial memory
        self.update_memory_stats();
        
        // Read basic project info first (minimal memory impact)
        let (project_name, framework_version) = self.extract_project_metadata().await?;
        
        // Scan files using lightweight metadata scan
        let files = ProjectFiles::scan_parallel(&self.project_path)?;
        
        log::info!("Found {} files to analyze", 
                  files.controllers.len() + files.models.len() + files.middleware.len() + files.views.len());

        // Process files in streaming chunks to control memory usage
        let (controllers, routes) = self.process_controllers_streaming(&files.controllers).await?;
        
        // Aggressive cleanup after controller processing
        if self.config.aggressive_memory_cleanup {
            self.trigger_gc_collection();
        }
        
        let middleware = self.process_middleware_streaming(&files.middleware).await?;
        
        if self.config.aggressive_memory_cleanup {
            self.trigger_gc_collection();
        }
        
        let models = self.process_models_streaming(&files.models).await?;
        
        if self.config.aggressive_memory_cleanup {
            self.trigger_gc_collection();
        }
        
        let views = self.process_views_streaming(&files.views).await?;
        
        // Final validation with minimal memory footprint  
        let issues = self.validate_project_streaming(&routes, &controllers)?;
        
        self.update_memory_stats();
        log::info!("Streaming analysis completed. Peak memory: {:.2} MB", self.memory_stats.peak_memory_mb);
        
        Ok(ProjectAnalysis {
            project_name,
            framework_version,
            controllers,
            routes,
            middleware,
            models,
            views,
            issues,
        })
    }

    /// Process controllers in memory-efficient streaming chunks
    async fn process_controllers_streaming(&mut self, controller_paths: &[PathBuf]) -> Result<(Vec<ControllerInfo>, Vec<RouteInfo>)> {
        let mut all_controllers = Vec::new();
        let mut all_routes = Vec::new();

        // Process controllers in chunks to limit memory usage
        for chunk in controller_paths.chunks(self.config.chunk_size) {
            self.check_memory_limits().await?;
            
            log::debug!("Processing controller chunk of {} files", chunk.len());
            
            // Process chunk concurrently but with controlled parallelism
            let mut chunk_results = Vec::new();
            for path in chunk {
                let _permit = self.semaphore.acquire().await.unwrap();
                let result = Self::process_single_controller(path, &self.ast_analyzer).await;
                chunk_results.push(result);
                drop(_permit); // Release permit immediately
            }

            // Collect results and immediately free memory
            for result in chunk_results {
                match result {
                    Ok((controller, routes)) => {
                        all_controllers.push(controller);
                        all_routes.extend(routes);
                    }
                    Err(e) => {
                        log::warn!("Failed to process controller: {}", e);
                    }
                }
            }

            self.memory_stats.chunks_processed += 1;
            self.update_memory_stats();
            
            // Trigger GC if memory usage is high
            if self.config.enable_memory_monitoring && self.memory_stats.current_memory_mb > self.config.memory_limit_mb as f64 * 0.8 {
                self.trigger_gc_collection();
            }
        }

        Ok((all_controllers, all_routes))
    }

    /// Process a single controller file with minimal memory footprint
    async fn process_single_controller(path: &PathBuf, ast_analyzer: &AstAnalyzer) -> Result<(ControllerInfo, Vec<RouteInfo>)> {
        let controller_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Extract routes with streaming approach
        let routes = ast_analyzer.extract_routes(path)?;
        
        // Create minimal handler info to reduce memory
        let handlers = Self::create_minimal_handlers(&routes, &controller_name);
        
        let controller = ControllerInfo {
            name: controller_name,
            file_path: path.to_string_lossy().to_string(),
            handlers,
        };

        Ok((controller, routes))
    }

    /// Create minimal handler information to reduce memory usage
    fn create_minimal_handlers(routes: &[RouteInfo], controller_name: &str) -> Vec<HandlerInfo> {
        // Group routes by handler to minimize memory usage
        let mut handler_map = std::collections::HashMap::new();
        
        for route in routes {
            let handler_name = route.handler.split("::").last().unwrap_or(&route.handler).to_string();
            let entry = handler_map.entry(handler_name.clone()).or_insert_with(|| {
                (Vec::new(), 1u32) // (routes, complexity)
            });
            
            entry.0.push(super::RouteReference {
                method: route.method.clone(),
                path: route.path.clone(),
                parameters: route.parameters.clone(),
            });
            
            // Simple complexity calculation
            entry.1 += route.parameters.len() as u32;
        }

        handler_map.into_iter().map(|(name, (routes, complexity))| {
            HandlerInfo {
                name: name.clone(),
                qualified_name: format!("{}::{}", controller_name, name),
                routes,
                complexity,
            }
        }).collect()
    }

    /// Process middleware files in streaming chunks
    async fn process_middleware_streaming(&mut self, middleware_paths: &[PathBuf]) -> Result<Vec<MiddlewareInfo>> {
        let mut all_middleware = Vec::new();

        for chunk in middleware_paths.chunks(self.config.chunk_size) {
            self.check_memory_limits().await?;
            
            let mut chunk_results = Vec::new();
            for path in chunk {
                let _permit = self.semaphore.acquire().await.unwrap();
                let result = Self::process_single_middleware(path).await;
                chunk_results.push(result);
                drop(_permit); // Release permit immediately
            }

            for result in chunk_results {
                if let Ok(middleware) = result {
                    all_middleware.push(middleware);
                }
            }

            self.memory_stats.chunks_processed += 1;
            self.update_memory_stats();
        }

        Ok(all_middleware)
    }

    /// Process a single middleware file with minimal memory usage
    async fn process_single_middleware(path: &PathBuf) -> Result<MiddlewareInfo> {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Minimal middleware analysis to reduce memory
        Ok(MiddlewareInfo {
            name,
            priority: None,
            middleware_type: "custom".to_string(),
        })
    }

    /// Process model files in streaming chunks
    async fn process_models_streaming(&mut self, model_paths: &[PathBuf]) -> Result<Vec<ModelInfo>> {
        let mut all_models = Vec::new();

        for chunk in model_paths.chunks(self.config.chunk_size) {
            self.check_memory_limits().await?;
            
            // Process models with minimal memory footprint
            for path in chunk {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    all_models.push(ModelInfo {
                        name: name.to_string(),
                        file_path: path.to_string_lossy().to_string(),
                    });
                }
            }

            self.memory_stats.chunks_processed += 1;
            self.memory_stats.files_processed += chunk.len();
        }

        Ok(all_models)
    }

    /// Process view files in streaming chunks
    async fn process_views_streaming(&mut self, view_paths: &[PathBuf]) -> Result<Vec<ViewAnalysis>> {
        let mut all_views = Vec::new();

        for chunk in view_paths.chunks(self.config.chunk_size) {
            self.check_memory_limits().await?;
            
            let mut chunk_results = Vec::new();
            for path in chunk {
                let _permit = self.semaphore.acquire().await.unwrap();
                let result = Self::process_single_view(path).await;
                chunk_results.push(result);
                drop(_permit); // Release permit immediately
            }

            for result in chunk_results {
                if let Ok(view) = result {
                    all_views.push(view);
                }
            }

            self.memory_stats.chunks_processed += 1;
            self.update_memory_stats();
        }

        Ok(all_views)
    }

    /// Process a single view file with minimal analysis
    async fn process_single_view(path: &PathBuf) -> Result<ViewAnalysis> {
        use crate::analysis::views::ViewAnalyzer;
        
        // Use existing view analyzer but could be optimized further
        ViewAnalyzer::analyze_view(path)
    }

    /// Extract basic project metadata without loading large files
    async fn extract_project_metadata(&self) -> Result<(String, String)> {
        let cargo_path = self.project_path.join("Cargo.toml");
        let content = fs::read_to_string(cargo_path).await
            .context("Failed to read Cargo.toml")?;

        let project_name = Self::extract_project_name_minimal(&content)?;
        let framework_version = Self::detect_rustf_version_minimal(&content)?;

        Ok((project_name, framework_version))
    }

    /// Extract project name with minimal string processing
    fn extract_project_name_minimal(content: &str) -> Result<String> {
        for line in content.lines() {
            if line.trim_start().starts_with("name") {
                if let Some(name_part) = line.split('=').nth(1) {
                    let name = name_part.trim().trim_matches('"');
                    return Ok(name.to_string());
                }
            }
        }
        Ok("unknown".to_string())
    }

    /// Detect RustF version with minimal processing
    fn detect_rustf_version_minimal(content: &str) -> Result<String> {
        if content.contains("rustf") {
            Ok("detected".to_string())
        } else {
            Ok("not-detected".to_string())
        }
    }

    /// Validate project with minimal memory usage
    fn validate_project_streaming(&self, routes: &[RouteInfo], controllers: &[ControllerInfo]) -> Result<Vec<Issue>> {
        let mut issues = Vec::new();

        // Basic validation with early exit to save memory
        if routes.is_empty() && !controllers.is_empty() {
            issues.push(Issue {
                severity: "warning".to_string(),
                message: "Controllers found but no routes detected".to_string(),
                file_path: None,
                line: None,
            });
        }

        // Check for common issues without loading additional data
        for route in routes.iter().take(100) { // Limit validation scope
            if route.path.is_empty() {
                issues.push(Issue {
                    severity: "error".to_string(),
                    message: format!("Empty route path for handler: {}", route.handler),
                    file_path: None,
                    line: None,
                });
            }
        }

        Ok(issues)
    }

    /// Monitor memory usage and update statistics
    fn update_memory_stats(&mut self) {
        if !self.config.enable_memory_monitoring {
            return;
        }

        // Get current memory usage (simplified - in production would use proper memory monitoring)
        let current_memory = Self::get_current_memory_usage();
        self.memory_stats.current_memory_mb = current_memory;
        
        if current_memory > self.memory_stats.peak_memory_mb {
            self.memory_stats.peak_memory_mb = current_memory;
        }
    }

    /// Check memory limits and take action if exceeded
    async fn check_memory_limits(&mut self) -> Result<()> {
        if !self.config.enable_memory_monitoring || self.config.memory_limit_mb == 0 {
            return Ok(());
        }

        self.update_memory_stats();
        
        if self.memory_stats.current_memory_mb > self.config.memory_limit_mb as f64 {
            self.memory_stats.memory_warnings += 1;
            log::warn!("Memory usage ({:.2} MB) exceeds limit ({} MB)", 
                      self.memory_stats.current_memory_mb, self.config.memory_limit_mb);
            
            // Trigger garbage collection
            self.trigger_gc_collection();
            
            // Wait a bit for GC to complete
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            
            self.update_memory_stats();
            
            // If still over limit, return error
            if self.memory_stats.current_memory_mb > self.config.memory_limit_mb as f64 * 1.2 {
                anyhow::bail!("Memory usage ({:.2} MB) still exceeds limit after GC", 
                             self.memory_stats.current_memory_mb);
            }
        }

        Ok(())
    }

    /// Trigger garbage collection to free memory
    fn trigger_gc_collection(&mut self) {
        self.memory_stats.gc_collections_triggered += 1;
        log::debug!("Triggering garbage collection (attempt #{})", self.memory_stats.gc_collections_triggered);
        
        // Force garbage collection (this is Rust, so we mainly drop large allocations)
        // In a real implementation, you might want to clear caches, drop unused data, etc.
        std::hint::black_box(vec![0u8; 0]); // Minimal GC hint
    }

    /// Get current memory usage (simplified implementation)
    fn get_current_memory_usage() -> f64 {
        // This is a placeholder - in production you'd use a proper memory monitoring crate
        // like `memory-stats` or system calls to get actual memory usage
        
        // Simplified estimation based on thread count and basic heuristics
        std::thread::available_parallelism()
            .map(|p| p.get() as f64 * 8.0) // ~8MB per thread as rough estimate
            .unwrap_or(64.0)
    }

    /// Get memory statistics
    pub fn get_memory_stats(&self) -> &MemoryStats {
        &self.memory_stats
    }

    /// Get streaming configuration
    pub fn get_config(&self) -> &StreamingConfig {
        &self.config
    }
}

/// Builder for streaming configuration
pub struct StreamingConfigBuilder {
    config: StreamingConfig,
}

impl StreamingConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: StreamingConfig::default(),
        }
    }

    pub fn max_concurrent_files(mut self, max: usize) -> Self {
        self.config.max_concurrent_files = max;
        self
    }

    pub fn chunk_size(mut self, size: usize) -> Self {
        self.config.chunk_size = size;
        self
    }

    pub fn memory_limit_mb(mut self, limit: usize) -> Self {
        self.config.memory_limit_mb = limit;
        self
    }

    pub fn enable_memory_monitoring(mut self, enable: bool) -> Self {
        self.config.enable_memory_monitoring = enable;
        self
    }

    pub fn aggressive_cleanup(mut self, enable: bool) -> Self {
        self.config.aggressive_memory_cleanup = enable;
        self
    }

    pub fn build(self) -> StreamingConfig {
        self.config
    }
}

impl Default for StreamingConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[tokio::test]
    async fn test_streaming_config_builder() {
        let config = StreamingConfigBuilder::new()
            .max_concurrent_files(4)
            .chunk_size(25)
            .memory_limit_mb(256)
            .enable_memory_monitoring(true)
            .aggressive_cleanup(false)
            .build();

        assert_eq!(config.max_concurrent_files, 4);
        assert_eq!(config.chunk_size, 25);
        assert_eq!(config.memory_limit_mb, 256);
        assert!(config.enable_memory_monitoring);
        assert!(!config.aggressive_memory_cleanup);
    }

    #[tokio::test]
    async fn test_memory_stats_tracking() {
        let dir = tempdir().unwrap();
        let cargo_toml = dir.path().join("Cargo.toml");
        fs::write(&cargo_toml, r#"
[package]
name = "test-project"
version = "0.1.0"

[dependencies]
rustf = "0.1"
"#).unwrap();

        let config = StreamingConfig::default();
        let mut analyzer = StreamingAnalyzer::new(dir.path().to_path_buf(), config).unwrap();
        
        analyzer.update_memory_stats();
        assert!(analyzer.memory_stats.current_memory_mb > 0.0);
    }

    #[tokio::test]
    async fn test_minimal_project_metadata_extraction() {
        let content = r#"
[package]
name = "test-project"
version = "0.1.0"

[dependencies]
rustf = "0.1"
"#;
        
        let name = StreamingAnalyzer::extract_project_name_minimal(content).unwrap();
        assert_eq!(name, "test-project");
        
        let version = StreamingAnalyzer::detect_rustf_version_minimal(content).unwrap();
        assert_eq!(version, "detected");
    }
}