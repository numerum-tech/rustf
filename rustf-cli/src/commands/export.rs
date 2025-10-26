use std::path::PathBuf;
use std::fs;
use anyhow::{Result, Context};
use serde::Serialize;
use crate::analyzer::ProjectAnalyzer;
use crate::analysis::handlers::{HandlerAnalysis};
use crate::analysis::middleware::{MiddlewareAnalyzer, MiddlewareAnalysis, MiddlewareChain};
use crate::analyzer::files::ProjectFiles;
use rayon::prelude::*;

#[derive(Debug, Serialize)]
pub struct CompleteProjectExport {
    pub metadata: ProjectMetadata,
    pub analysis: crate::analyzer::ProjectAnalysis,
    pub detailed_handlers: Vec<DetailedControllerAnalysis>,
    pub middleware_chain: MiddlewareChain,
    pub recommendations: Vec<Recommendation>,
    pub ai_insights: AIInsights,
}

#[derive(Debug, Serialize)]
pub struct ProjectMetadata {
    pub export_timestamp: String,
    pub cli_version: String,
    pub analysis_scope: AnalysisScope,
    pub file_counts: FileCounts,
}

#[derive(Debug, Serialize)]
pub struct AnalysisScope {
    pub include_code_samples: bool,
    pub analyzed_files: Vec<String>,
    pub skipped_files: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct FileCounts {
    pub controllers: usize,
    pub middleware: usize,
    pub models: usize,
    pub views: usize,
}

#[derive(Debug, Serialize)]
pub struct DetailedControllerAnalysis {
    pub controller_name: String,
    pub file_path: String,
    pub handlers: Vec<HandlerAnalysis>,
    pub route_coverage: f64,
    pub complexity_metrics: ComplexityMetrics,
}

#[derive(Debug, Serialize)]
pub struct ComplexityMetrics {
    pub average_complexity: f64,
    pub highest_complexity: u32,
    pub total_handlers: usize,
    pub async_handler_ratio: f64,
}

#[derive(Debug, Serialize)]
pub struct Recommendation {
    pub category: String,
    pub priority: String,
    pub description: String,
    pub affected_files: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct AIInsights {
    pub architecture_patterns: Vec<String>,
    pub common_practices: Vec<String>,
    pub potential_improvements: Vec<String>,
    pub framework_usage_score: u32,
}

pub async fn run(project_path: PathBuf, format: String, output: Option<PathBuf>, include_code: bool) -> Result<()> {
    log::info!("Exporting project analysis in {} format...", format);
    
    let analyzer = ProjectAnalyzer::new(project_path.clone())?;
    let basic_analysis = analyzer.analyze_complete(true).await?;
    
    // Gather detailed analysis data with parallel processing
    let files = ProjectFiles::scan_parallel(&project_path)?;
    let detailed_handlers = collect_detailed_handler_analysis(&basic_analysis.controllers).await?;
    let middleware_chain = collect_middleware_analysis(&files.middleware, &basic_analysis.routes).await?;
    
    let complete_export = CompleteProjectExport {
        metadata: ProjectMetadata {
            export_timestamp: chrono::Utc::now().to_rfc3339(),
            cli_version: env!("CARGO_PKG_VERSION").to_string(),
            analysis_scope: AnalysisScope {
                include_code_samples: include_code,
                analyzed_files: collect_analyzed_files(&files),
                skipped_files: Vec::new(), // TODO: track skipped files during analysis
            },
            file_counts: FileCounts {
                controllers: files.controllers.len(),
                middleware: files.middleware.len(),
                models: files.models.len(),
                views: files.views.len(),
            },
        },
        analysis: basic_analysis,
        detailed_handlers,
        recommendations: generate_recommendations(&files, &middleware_chain).await?,
        ai_insights: generate_ai_insights(&files, &middleware_chain).await?,
        middleware_chain,
    };
    
    // Export in requested format
    let exported_content = match format.to_lowercase().as_str() {
        "json" => export_as_json(&complete_export)?,
        "yaml" | "yml" => export_as_yaml(&complete_export)?,
        "markdown" | "md" => export_as_markdown(&complete_export).await?,
        _ => anyhow::bail!("Unsupported export format: {}. Supported: json, yaml, markdown", format),
    };
    
    // Write to file or stdout
    match output {
        Some(output_path) => {
            fs::write(&output_path, exported_content)
                .with_context(|| format!("Failed to write export to {}", output_path.display()))?;
            println!("✅ Exported project analysis to: {}", output_path.display());
        }
        None => {
            println!("{}", exported_content);
        }
    }
    
    Ok(())
}

async fn collect_detailed_handler_analysis(controllers: &[crate::analyzer::ControllerInfo]) -> Result<Vec<DetailedControllerAnalysis>> {
    let mut detailed_controllers = Vec::new();
    
    for controller in controllers {
        // Use the enhanced handler information directly
        let handlers: Vec<HandlerAnalysis> = controller.handlers.iter().map(|h| {
            HandlerAnalysis {
                name: h.qualified_name.clone(), // Use qualified name
                signature: format!("async fn {}(ctx: Context) -> Result<Response>", h.name), // Placeholder
                is_async: true, // All RustF handlers are async
                parameters: vec![], // TODO: could extract from route parameters
                return_type: "Result<Response>".to_string(),
                context_usage: vec![], // TODO: could analyze context usage
                error_handling: crate::analysis::handlers::ErrorHandling {
                    has_error_handling: true,
                    uses_result_type: true,
                    error_types: vec![],
                },
                complexity_score: h.complexity,
            }
        }).collect();
        
        let complexity_metrics = calculate_complexity_metrics(&handlers);
        
        detailed_controllers.push(DetailedControllerAnalysis {
            controller_name: controller.name.clone(),
            file_path: controller.file_path.clone(),
            handlers,
            route_coverage: 100.0, // All routes are covered by design
            complexity_metrics,
        });
    }
    
    Ok(detailed_controllers)
}

async fn collect_middleware_analysis(middleware_files: &[PathBuf], routes: &[crate::analyzer::RouteInfo]) -> Result<MiddlewareChain> {
    // Parallel analysis of middleware files
    let middleware_analyses: Vec<MiddlewareAnalysis> = middleware_files
        .par_iter()
        .filter_map(|middleware_path| {
            match MiddlewareAnalyzer::analyze_middleware(middleware_path) {
                Ok(analysis) => Some(analysis),
                Err(e) => {
                    log::warn!("Failed to analyze middleware {}: {}", middleware_path.display(), e);
                    None
                }
            }
        })
        .collect();
    
    Ok(MiddlewareAnalyzer::build_middleware_chain(&middleware_analyses, routes))
}

fn calculate_complexity_metrics(handlers: &[HandlerAnalysis]) -> ComplexityMetrics {
    if handlers.is_empty() {
        return ComplexityMetrics {
            average_complexity: 0.0,
            highest_complexity: 0,
            total_handlers: 0,
            async_handler_ratio: 0.0,
        };
    }
    
    let total_complexity: u32 = handlers.iter().map(|h| h.complexity_score).sum();
    let async_count = handlers.iter().filter(|h| h.is_async).count();
    let highest_complexity = handlers.iter().map(|h| h.complexity_score).max().unwrap_or(0);
    
    ComplexityMetrics {
        average_complexity: total_complexity as f64 / handlers.len() as f64,
        highest_complexity,
        total_handlers: handlers.len(),
        async_handler_ratio: async_count as f64 / handlers.len() as f64,
    }
}

fn collect_analyzed_files(files: &ProjectFiles) -> Vec<String> {
    let mut analyzed_files = Vec::new();
    
    for controller in &files.controllers {
        analyzed_files.push(controller.to_string_lossy().to_string());
    }
    
    for middleware in &files.middleware {
        analyzed_files.push(middleware.to_string_lossy().to_string());
    }
    
    for model in &files.models {
        analyzed_files.push(model.to_string_lossy().to_string());
    }
    
    analyzed_files.sort();
    analyzed_files
}

async fn generate_recommendations(files: &ProjectFiles, middleware_chain: &MiddlewareChain) -> Result<Vec<Recommendation>> {
    let mut recommendations = Vec::new();
    
    // Security recommendations
    if middleware_chain.coverage_analysis.auth_coverage < 50.0 {
        recommendations.push(Recommendation {
            category: "Security".to_string(),
            priority: "High".to_string(),
            description: "Consider adding authentication middleware for API routes".to_string(),
            affected_files: files.controllers.iter().map(|p| p.to_string_lossy().to_string()).collect(),
        });
    }
    
    // Performance recommendations
    if middleware_chain.coverage_analysis.logging_coverage < 100.0 {
        recommendations.push(Recommendation {
            category: "Monitoring".to_string(),
            priority: "Medium".to_string(),
            description: "Add comprehensive logging middleware for better observability".to_string(),
            affected_files: vec!["src/middleware/".to_string()],
        });
    }
    
    // CORS recommendations for web APIs
    if middleware_chain.coverage_analysis.cors_coverage == 0.0 && !files.controllers.is_empty() {
        recommendations.push(Recommendation {
            category: "Web API".to_string(),
            priority: "Medium".to_string(),
            description: "Add CORS middleware for browser compatibility".to_string(),
            affected_files: vec!["src/middleware/".to_string()],
        });
    }
    
    Ok(recommendations)
}

async fn generate_ai_insights(files: &ProjectFiles, middleware_chain: &MiddlewareChain) -> Result<AIInsights> {
    let mut architecture_patterns = Vec::new();
    let mut common_practices = Vec::new();
    let mut potential_improvements = Vec::new();
    
    // Analyze architecture patterns
    if !files.controllers.is_empty() && !files.middleware.is_empty() {
        architecture_patterns.push("MVC with Middleware Pattern".to_string());
    }
    
    if files.models.len() > 0 {
        architecture_patterns.push("Data Model Layer".to_string());
    }
    
    if files.views.len() > 0 {
        architecture_patterns.push("Template-based Views".to_string());
    }
    
    // Analyze common practices
    if middleware_chain.middleware_list.iter().any(|m| matches!(m.middleware_type, crate::analysis::middleware::MiddlewareType::Logging)) {
        common_practices.push("Structured Logging".to_string());
    }
    
    if middleware_chain.middleware_list.iter().any(|m| matches!(m.middleware_type, crate::analysis::middleware::MiddlewareType::Authentication)) {
        common_practices.push("Authentication Layer".to_string());
    }
    
    // Generate improvement suggestions
    if middleware_chain.conflicts.len() > 0 {
        potential_improvements.push("Resolve middleware conflicts to prevent unexpected behavior".to_string());
    }
    
    if files.controllers.len() > 10 {
        potential_improvements.push("Consider controller organization into modules for better maintainability".to_string());
    }
    
    let framework_usage_score = calculate_framework_usage_score(files, middleware_chain);
    
    Ok(AIInsights {
        architecture_patterns,
        common_practices,
        potential_improvements,
        framework_usage_score,
    })
}

fn calculate_framework_usage_score(files: &ProjectFiles, middleware_chain: &MiddlewareChain) -> u32 {
    let mut score = 0u32;
    
    // Base points for having essential components
    if !files.controllers.is_empty() { score += 25; }
    if !files.middleware.is_empty() { score += 20; }
    if !files.models.is_empty() { score += 15; }
    if !files.views.is_empty() { score += 15; }
    
    // Bonus points for best practices
    if middleware_chain.coverage_analysis.logging_coverage > 80.0 { score += 10; }
    if middleware_chain.coverage_analysis.auth_coverage > 50.0 { score += 10; }
    if middleware_chain.conflicts.is_empty() { score += 5; }
    
    score.min(100)
}

fn export_as_json(export: &CompleteProjectExport) -> Result<String> {
    serde_json::to_string_pretty(export)
        .context("Failed to serialize export data to JSON")
}

fn export_as_yaml(export: &CompleteProjectExport) -> Result<String> {
    serde_yaml::to_string(export)
        .context("Failed to serialize export data to YAML")
}

async fn export_as_markdown(export: &CompleteProjectExport) -> Result<String> {
    let mut md = String::new();
    
    // Header
    md.push_str(&format!("# RustF Project Analysis: {}\n\n", export.analysis.project_name));
    md.push_str(&format!("**Generated:** {} | **CLI Version:** {}\n\n", 
        export.metadata.export_timestamp,
        export.metadata.cli_version
    ));
    
    // Overview section
    md.push_str("## 📊 Project Overview\n\n");
    md.push_str(&format!("- **Framework Version:** {}\n", export.analysis.framework_version));
    md.push_str(&format!("- **Controllers:** {}\n", export.metadata.file_counts.controllers));
    md.push_str(&format!("- **Routes:** {}\n", export.analysis.routes.len()));
    md.push_str(&format!("- **Middleware:** {}\n", export.metadata.file_counts.middleware));
    md.push_str(&format!("- **Models:** {}\n", export.metadata.file_counts.models));
    md.push_str(&format!("- **Framework Usage Score:** {}/100\n\n", export.ai_insights.framework_usage_score));
    
    // Controllers section
    md.push_str("## 🎯 Controllers Analysis\n\n");
    for controller in &export.detailed_handlers {
        md.push_str(&format!("### {}\n", controller.controller_name));
        md.push_str(&format!("- **File:** `{}`\n", controller.file_path));
        md.push_str(&format!("- **Handlers:** {}\n", controller.handlers.len()));
        md.push_str(&format!("- **Average Complexity:** {:.1}\n", controller.complexity_metrics.average_complexity));
        md.push_str(&format!("- **Async Ratio:** {:.1}%\n\n", controller.complexity_metrics.async_handler_ratio * 100.0));
        
        if !controller.handlers.is_empty() {
            md.push_str("**Handlers:**\n");
            // Find the controller in the original analysis to get route information
            if let Some(original_controller) = export.analysis.controllers.iter()
                .find(|c| c.name == controller.controller_name) {
                for handler_info in &original_controller.handlers {
                    let routes_str = if handler_info.routes.is_empty() {
                        "no routes".to_string()
                    } else {
                        handler_info.routes.iter()
                            .map(|r| format!("{} {}", r.method, r.path))
                            .collect::<Vec<_>>()
                            .join(", ")
                    };
                    md.push_str(&format!("- `{}` → {} (complexity: {})\n", 
                        handler_info.qualified_name, 
                        routes_str,
                        handler_info.complexity
                    ));
                }
            }
            md.push_str("\n");
        }
    }
    
    // Routes section
    md.push_str("## 🛣️ Routes\n\n");
    md.push_str("| Method | Path | Handler |\n");
    md.push_str("|--------|------|--------|\n");
    for route in &export.analysis.routes {
        // Find the qualified name for this handler
        let qualified_handler = export.analysis.controllers.iter()
            .flat_map(|c| &c.handlers)
            .find(|h| h.name == route.handler)
            .map(|h| h.qualified_name.clone())
            .unwrap_or_else(|| route.handler.clone());
        
        md.push_str(&format!("| {} | `{}` | `{}` |\n", route.method, route.path, qualified_handler));
    }
    md.push_str("\n");
    
    // Middleware section
    md.push_str("## 🔧 Middleware Analysis\n\n");
    md.push_str(&format!("**Execution Order:** {}\n\n", export.middleware_chain.execution_order.join(" → ")));
    
    md.push_str("### Coverage Analysis\n");
    md.push_str(&format!("- **Logging Coverage:** {:.1}%\n", export.middleware_chain.coverage_analysis.logging_coverage));
    md.push_str(&format!("- **Auth Coverage:** {:.1}%\n", export.middleware_chain.coverage_analysis.auth_coverage));
    md.push_str(&format!("- **CORS Coverage:** {:.1}%\n\n", export.middleware_chain.coverage_analysis.cors_coverage));
    
    // Issues section
    if !export.analysis.issues.is_empty() {
        md.push_str("## ⚠️ Issues\n\n");
        for issue in &export.analysis.issues {
            let emoji = match issue.severity.as_str() {
                "error" => "❌",
                "warning" => "⚠️",
                _ => "ℹ️",
            };
            md.push_str(&format!("{} **{}:** {}\n", emoji, issue.severity.to_uppercase(), issue.message));
        }
        md.push_str("\n");
    }
    
    // Recommendations section
    if !export.recommendations.is_empty() {
        md.push_str("## 💡 Recommendations\n\n");
        for rec in &export.recommendations {
            let priority_emoji = match rec.priority.as_str() {
                "High" => "🔴",
                "Medium" => "🟡",
                _ => "🟢",
            };
            md.push_str(&format!("{} **{}** ({}): {}\n", priority_emoji, rec.category, rec.priority, rec.description));
        }
        md.push_str("\n");
    }
    
    // AI Insights section
    md.push_str("## 🤖 AI Insights\n\n");
    
    if !export.ai_insights.architecture_patterns.is_empty() {
        md.push_str("### Architecture Patterns\n");
        for pattern in &export.ai_insights.architecture_patterns {
            md.push_str(&format!("- {}\n", pattern));
        }
        md.push_str("\n");
    }
    
    if !export.ai_insights.common_practices.is_empty() {
        md.push_str("### Common Practices\n");
        for practice in &export.ai_insights.common_practices {
            md.push_str(&format!("- {}\n", practice));
        }
        md.push_str("\n");
    }
    
    if !export.ai_insights.potential_improvements.is_empty() {
        md.push_str("### Potential Improvements\n");
        for improvement in &export.ai_insights.potential_improvements {
            md.push_str(&format!("- {}\n", improvement));
        }
        md.push_str("\n");
    }
    
    Ok(md)
}