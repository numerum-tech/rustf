use super::*;
use super::files::ProjectFiles;
use super::ast::AstAnalyzer;
use super::lookup::AnalysisLookup;
use super::analysis_cache::{
    get_cached_project_analysis, 
    cache_project_analysis, invalidate_file_cache
};
use std::path::PathBuf;
use anyhow::{Result, Context};
use std::fs;

#[derive(Debug)]
pub struct ProjectAnalyzer {
    pub project_path: PathBuf,
    ast_analyzer: AstAnalyzer,
}

impl ProjectAnalyzer {
    pub fn new(project_path: PathBuf) -> Result<Self> {
        if !project_path.exists() {
            anyhow::bail!("Project path does not exist: {}", project_path.display());
        }
        
        // Check if it's a Rust project
        let cargo_toml = project_path.join("Cargo.toml");
        if !cargo_toml.exists() {
            anyhow::bail!("Not a Rust project: Cargo.toml not found");
        }
        
        Ok(Self { 
            project_path,
            ast_analyzer: AstAnalyzer::default(),
        })
    }
    
    pub async fn analyze_complete(&self, detailed: bool) -> Result<ProjectAnalysis> {
        log::info!("Analyzing project at: {}", self.project_path.display());
        
        // Check cache first
        if let Some(cached_analysis) = get_cached_project_analysis(&self.project_path, detailed).await {
            log::debug!("Using cached analysis for project: {}", self.project_path.display());
            return Ok(cached_analysis);
        }
        
        // Read Cargo.toml to detect RustF usage
        let cargo_content = fs::read_to_string(self.project_path.join("Cargo.toml"))
            .context("Failed to read Cargo.toml")?;
        
        let project_name = self.extract_project_name(&cargo_content)?;
        let framework_version = self.detect_rustf_version(&cargo_content)?;
        
        // Scan project files using parallel scanning for better performance
        let files = ProjectFiles::scan_parallel(&self.project_path)?;
        
        // Analyze controllers and extract routes with enhanced handler information
        let mut controllers = Vec::new();
        let mut routes = Vec::new();
        
        for controller_path in &files.controllers {
            let controller_name = controller_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            
            // Extract routes for this controller
            let controller_routes = match self.ast_analyzer.extract_routes(controller_path) {
                Ok(routes) => routes,
                Err(e) => {
                    log::warn!("Failed to extract routes from {}: {}", controller_path.display(), e);
                    Vec::new()
                }
            };
            
            // Get handler complexity analysis
            let handler_analyses = if let Ok(analyses) = crate::analysis::handlers::HandlerAnalyzer::analyze_handlers(controller_path) {
                analyses
            } else {
                Vec::new()
            };
            
            // Build enhanced handler information
            let mut enhanced_handlers = Vec::new();
            
            // Get basic handler names from AST
            if let Ok(basic_controller_info) = self.ast_analyzer.analyze_controller(controller_path) {
                for handler_name in &basic_controller_info.handlers {
                    // Find routes handled by this handler
                    let handler_routes: Vec<RouteReference> = controller_routes.iter()
                        .filter(|route| route.handler == *handler_name)
                        .map(|route| RouteReference {
                            method: route.method.clone(),
                            path: route.path.clone(),
                            parameters: route.parameters.clone(),
                        })
                        .collect();
                    
                    // Get complexity from handler analysis
                    let complexity = handler_analyses.iter()
                        .find(|h| h.name == *handler_name)
                        .map(|h| h.complexity_score)
                        .unwrap_or(0);
                    
                    enhanced_handlers.push(HandlerInfo {
                        name: handler_name.clone(),
                        qualified_name: format!("{}::{}", controller_name, handler_name),
                        routes: handler_routes,
                        complexity,
                    });
                }
                
                controllers.push(ControllerInfo {
                    name: controller_name,
                    file_path: controller_path.to_string_lossy().to_string(),
                    handlers: enhanced_handlers,
                });
            }
            
            // Add routes to the global routes list
            routes.extend(controller_routes);
        }
        
        // Analyze models
        let mut models = Vec::new();
        for model_path in &files.models {
            let model_name = model_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            
            models.push(ModelInfo {
                name: model_name,
                file_path: model_path.to_string_lossy().to_string(),
            });
        }
        
        // Analyze middleware
        let mut middleware = Vec::new();
        for middleware_path in &files.middleware {
            let middleware_name = middleware_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            
            middleware.push(MiddlewareInfo {
                name: middleware_name,
                priority: None,
                middleware_type: "custom".to_string(),
            });
        }
        
        // Validate and find issues
        let issues = self.validate_project(&routes, &controllers)?;
        
        // Analyze views (placeholder for now)
        let views = Vec::new(); // TODO: Implement view analysis
        
        let analysis = ProjectAnalysis {
            project_name,
            framework_version,
            controllers,
            routes,
            middleware,
            models,
            views,
            issues,
        };

        // Cache the result for future use
        let mut tracked_files = [&files.controllers[..], &files.models[..], &files.middleware[..]].concat();
        tracked_files.push(self.project_path.join("Cargo.toml"));
        
        if let Err(e) = cache_project_analysis(
            self.project_path.clone(),
            detailed,
            analysis.clone(),
            tracked_files,
        ).await {
            log::warn!("Failed to cache analysis result: {}", e);
        }
        
        Ok(analysis)
    }

    /// Invalidate cache entries for a specific file
    pub async fn invalidate_cache_for_file(&self, file_path: &PathBuf) {
        log::debug!("Invalidating cache for file: {}", file_path.display());
        invalidate_file_cache(file_path).await;
    }

    /// Clear all cached analysis results for this project
    pub async fn clear_project_cache(&self) {
        use super::analysis_cache::global_analysis_cache;
        
        let cache_arc = global_analysis_cache();
        let mut cache = cache_arc.lock().await;
        cache.clear();
        log::info!("Cleared all cached analysis results");
    }
    
    fn validate_project(&self, routes: &[RouteInfo], controllers: &[ControllerInfo]) -> Result<Vec<Issue>> {
        let mut issues = Vec::new();
        
        // Check for missing handlers
        for route in routes {
            let handler_exists = controllers.iter().any(|controller| {
                controller.handlers.iter().any(|handler| handler.name == route.handler)
            });
            
            if !handler_exists {
                issues.push(Issue {
                    severity: "error".to_string(),
                    message: format!("Missing handler function: {}", route.handler),
                    file_path: None,
                    line: None,
                });
            }
        }
        
        // Check for route conflicts
        for (i, route1) in routes.iter().enumerate() {
            for route2 in routes.iter().skip(i + 1) {
                if route1.method == route2.method && route1.path == route2.path {
                    issues.push(Issue {
                        severity: "error".to_string(),
                        message: format!("Duplicate route: {} {}", route1.method, route1.path),
                        file_path: None,
                        line: None,
                    });
                }
            }
        }
        
        Ok(issues)
    }
    
    fn extract_project_name(&self, cargo_content: &str) -> Result<String> {
        // Simple regex-based extraction - in real implementation use toml crate
        if let Some(line) = cargo_content.lines().find(|l| l.starts_with("name")) {
            if let Some(name) = line.split('=').nth(1) {
                return Ok(name.trim().trim_matches('"').to_string());
            }
        }
        Ok("unknown".to_string())
    }
    
    fn detect_rustf_version(&self, cargo_content: &str) -> Result<String> {
        if cargo_content.contains("rustf") {
            Ok("detected".to_string())
        } else {
            Ok("not-found".to_string())
        }
    }
    
    /// Get AST cache statistics for performance monitoring
    pub fn get_cache_stats(&self) -> super::cache::CacheStats {
        self.ast_analyzer.cache_stats()
    }
    
    /// Clear AST cache (useful for testing or memory management)
    pub fn clear_cache(&self) {
        self.ast_analyzer.clear_cache();
    }
    
    /// Create fast lookup indexes from analysis results
    pub fn create_lookup_indexes(&self, analysis: &ProjectAnalysis) -> AnalysisLookup {
        AnalysisLookup::new(&analysis.routes, &analysis.controllers)
    }
}