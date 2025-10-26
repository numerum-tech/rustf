//! Data transformation utilities for converting between formats

use crate::analyzer::{ProjectAnalysis, ControllerInfo, RouteInfo, Issue};
use crate::utils::AnalysisStats;
use serde_json::Value;
use std::collections::HashMap;

/// Transform analysis data into different representations
pub struct DataTransformer;

impl DataTransformer {
    /// Convert analysis to summary statistics
    pub fn to_summary_stats(analysis: &ProjectAnalysis) -> AnalysisStats {
        crate::utils::AnalysisUtils::get_stats_summary(analysis)
    }
    
    /// Convert analysis to flat key-value pairs for easy processing
    pub fn to_flat_map(analysis: &ProjectAnalysis) -> HashMap<String, String> {
        let mut map = HashMap::new();
        
        // Basic counts
        map.insert("project_name".to_string(), analysis.project_name.clone());
        map.insert("framework_version".to_string(), analysis.framework_version.clone());
        map.insert("controllers_count".to_string(), analysis.controllers.len().to_string());
        map.insert("routes_count".to_string(), analysis.routes.len().to_string());
        map.insert("middleware_count".to_string(), analysis.middleware.len().to_string());
        map.insert("models_count".to_string(), analysis.models.len().to_string());
        map.insert("views_count".to_string(), analysis.views.len().to_string());
        map.insert("issues_count".to_string(), analysis.issues.len().to_string());
        
        // Complexity stats
        let complexity_stats = crate::utils::AnalysisUtils::calculate_complexity_stats(analysis);
        map.insert("avg_complexity".to_string(), format!("{:.2}", complexity_stats.avg_complexity));
        map.insert("max_complexity".to_string(), complexity_stats.max_complexity.to_string());
        map.insert("high_complexity_handlers".to_string(), complexity_stats.high_complexity.to_string());
        
        // Security stats
        let security_stats = crate::utils::AnalysisUtils::calculate_security_stats(analysis);
        map.insert("error_issues".to_string(), security_stats.error_issues.to_string());
        map.insert("warning_issues".to_string(), security_stats.warning_issues.to_string());
        map.insert("risky_views".to_string(), security_stats.views_with_security_issues.to_string());
        
        // Route method distribution
        let method_counts = Self::count_route_methods(&analysis.routes);
        for (method, count) in method_counts {
            map.insert(format!("routes_{}", method.to_lowercase()), count.to_string());
        }
        
        map
    }
    
    /// Convert analysis to JSON suitable for external APIs
    pub fn to_api_json(analysis: &ProjectAnalysis) -> Value {
        let stats = Self::to_summary_stats(analysis);
        
        serde_json::json!({
            "project": {
                "name": analysis.project_name,
                "framework_version": analysis.framework_version
            },
            "summary": {
                "controllers": stats.controllers_count,
                "routes": stats.routes_count,
                "middleware": stats.middleware_count,
                "models": stats.models_count,
                "views": stats.views_count,
                "issues": stats.issues_count
            },
            "complexity": {
                "average": stats.complexity_stats.avg_complexity,
                "maximum": stats.complexity_stats.max_complexity,
                "distribution": {
                    "low": stats.complexity_stats.low_complexity,
                    "medium": stats.complexity_stats.medium_complexity,
                    "high": stats.complexity_stats.high_complexity
                }
            },
            "security": {
                "total_issues": stats.security_stats.error_issues + stats.security_stats.warning_issues,
                "critical_issues": stats.security_stats.error_issues,
                "risky_views": stats.security_stats.views_with_security_issues
            },
            "routes": {
                "by_method": stats.route_methods,
                "with_parameters": analysis.routes.iter().filter(|r| !r.parameters.is_empty()).count()
            }
        })
    }
    
    /// Convert analysis to CSV-compatible rows
    pub fn to_csv_data(analysis: &ProjectAnalysis) -> Vec<Vec<String>> {
        let mut rows = Vec::new();
        
        // Header
        rows.push(vec![
            "Type".to_string(),
            "Name".to_string(),
            "File Path".to_string(),
            "Complexity".to_string(),
            "Routes Count".to_string(),
            "Issues Count".to_string(),
        ]);
        
        // Controllers
        for controller in &analysis.controllers {
            let avg_complexity = if controller.handlers.is_empty() {
                0.0
            } else {
                controller.handlers.iter().map(|h| h.complexity).sum::<u32>() as f64 / controller.handlers.len() as f64
            };
            
            let routes_count: usize = controller.handlers.iter().map(|h| h.routes.len()).sum();
            
            rows.push(vec![
                "Controller".to_string(),
                controller.name.clone(),
                controller.file_path.clone(),
                format!("{:.1}", avg_complexity),
                routes_count.to_string(),
                "0".to_string(), // Controllers don't have direct issues
            ]);
        }
        
        // Models
        for model in &analysis.models {
            rows.push(vec![
                "Model".to_string(),
                model.name.clone(),
                model.file_path.clone(),
                "0".to_string(), // Models don't have complexity
                "0".to_string(), // Models don't have routes
                "0".to_string(), // Models don't have direct issues
            ]);
        }
        
        // Views
        for view in &analysis.views {
            rows.push(vec![
                "View".to_string(),
                view.name.clone(),
                view.file_path.clone(),
                view.complexity_metrics.complexity_score.to_string(),
                "0".to_string(), // Views don't have routes
                view.security_issues.len().to_string(),
            ]);
        }
        
        rows
    }
    
    /// Convert routes to tabular data
    pub fn routes_to_table_data(routes: &[RouteInfo]) -> Vec<Vec<String>> {
        let mut rows = Vec::new();
        
        // Header
        rows.push(vec![
            "Method".to_string(),
            "Path".to_string(),
            "Handler".to_string(),
            "Parameters".to_string(),
        ]);
        
        // Data rows
        for route in routes {
            rows.push(vec![
                route.method.clone(),
                route.path.clone(),
                route.handler.clone(),
                route.parameters.join(", "),
            ]);
        }
        
        rows
    }
    
    /// Convert controllers to tabular data with detailed handler info
    pub fn controllers_to_table_data(controllers: &[ControllerInfo]) -> Vec<Vec<String>> {
        let mut rows = Vec::new();
        
        // Header
        rows.push(vec![
            "Controller".to_string(),
            "Handler".to_string(),
            "Complexity".to_string(),
            "Routes".to_string(),
            "Methods".to_string(),
        ]);
        
        // Data rows
        for controller in controllers {
            for handler in &controller.handlers {
                let routes_info = if handler.routes.is_empty() {
                    ("0".to_string(), "none".to_string())
                } else {
                    let methods: Vec<String> = handler.routes.iter()
                        .map(|r| r.method.clone())
                        .collect::<std::collections::HashSet<_>>()
                        .into_iter()
                        .collect();
                    (handler.routes.len().to_string(), methods.join(", "))
                };
                
                rows.push(vec![
                    controller.name.clone(),
                    handler.name.clone(),
                    handler.complexity.to_string(),
                    routes_info.0,
                    routes_info.1,
                ]);
            }
        }
        
        rows
    }
    
    /// Convert issues to tabular data
    pub fn issues_to_table_data(issues: &[Issue]) -> Vec<Vec<String>> {
        let mut rows = Vec::new();
        
        // Header
        rows.push(vec![
            "Severity".to_string(),
            "Message".to_string(),
            "File".to_string(),
            "Line".to_string(),
        ]);
        
        // Data rows
        for issue in issues {
            rows.push(vec![
                issue.severity.clone(),
                issue.message.clone(),
                issue.file_path.clone().unwrap_or("N/A".to_string()),
                issue.line.map_or("N/A".to_string(), |l| l.to_string()),
            ]);
        }
        
        rows
    }
    
    /// Transform analysis into metrics suitable for monitoring systems
    pub fn to_metrics(analysis: &ProjectAnalysis) -> HashMap<String, f64> {
        let mut metrics = HashMap::new();
        
        // Basic counts
        metrics.insert("rustf.project.controllers.total".to_string(), analysis.controllers.len() as f64);
        metrics.insert("rustf.project.routes.total".to_string(), analysis.routes.len() as f64);
        metrics.insert("rustf.project.middleware.total".to_string(), analysis.middleware.len() as f64);
        metrics.insert("rustf.project.models.total".to_string(), analysis.models.len() as f64);
        metrics.insert("rustf.project.views.total".to_string(), analysis.views.len() as f64);
        metrics.insert("rustf.project.issues.total".to_string(), analysis.issues.len() as f64);
        
        // Complexity metrics
        let complexity_stats = crate::utils::AnalysisUtils::calculate_complexity_stats(analysis);
        metrics.insert("rustf.complexity.average".to_string(), complexity_stats.avg_complexity);
        metrics.insert("rustf.complexity.maximum".to_string(), complexity_stats.max_complexity as f64);
        metrics.insert("rustf.complexity.high_count".to_string(), complexity_stats.high_complexity as f64);
        
        // Security metrics
        let security_stats = crate::utils::AnalysisUtils::calculate_security_stats(analysis);
        metrics.insert("rustf.security.errors".to_string(), security_stats.error_issues as f64);
        metrics.insert("rustf.security.warnings".to_string(), security_stats.warning_issues as f64);
        metrics.insert("rustf.security.risky_views".to_string(), security_stats.views_with_security_issues as f64);
        
        // Route method distribution
        let method_counts = Self::count_route_methods(&analysis.routes);
        for (method, count) in method_counts {
            metrics.insert(format!("rustf.routes.{}.count", method.to_lowercase()), count as f64);
        }
        
        // Calculated ratios
        if analysis.controllers.len() > 0 {
            metrics.insert("rustf.ratios.routes_per_controller".to_string(), 
                          analysis.routes.len() as f64 / analysis.controllers.len() as f64);
        }
        
        if analysis.routes.len() > 0 {
            let parameterized_routes = analysis.routes.iter().filter(|r| !r.parameters.is_empty()).count();
            metrics.insert("rustf.ratios.parameterized_routes".to_string(), 
                          parameterized_routes as f64 / analysis.routes.len() as f64);
        }
        
        metrics
    }
    
    /// Extract unique file paths from analysis
    pub fn extract_file_paths(analysis: &ProjectAnalysis) -> Vec<String> {
        let mut paths = Vec::new();
        
        // Controller files
        for controller in &analysis.controllers {
            paths.push(controller.file_path.clone());
        }
        
        // Model files
        for model in &analysis.models {
            paths.push(model.file_path.clone());
        }
        
        // View files
        for view in &analysis.views {
            paths.push(view.file_path.clone());
        }
        
        // Remove duplicates and sort
        paths.sort();
        paths.dedup();
        
        paths
    }
    
    /// Group components by directory
    pub fn group_by_directory(analysis: &ProjectAnalysis) -> HashMap<String, Vec<String>> {
        let mut groups: HashMap<String, Vec<String>> = HashMap::new();
        
        // Helper to extract directory
        let extract_dir = |path: &str| -> String {
            std::path::Path::new(path)
                .parent()
                .and_then(|p| p.to_str())
                .unwrap_or(".")
                .to_string()
        };
        
        // Group controllers
        for controller in &analysis.controllers {
            let dir = extract_dir(&controller.file_path);
            groups.entry(dir).or_insert_with(Vec::new).push(format!("Controller: {}", controller.name));
        }
        
        // Group models
        for model in &analysis.models {
            let dir = extract_dir(&model.file_path);
            groups.entry(dir).or_insert_with(Vec::new).push(format!("Model: {}", model.name));
        }
        
        // Group views
        for view in &analysis.views {
            let dir = extract_dir(&view.file_path);
            groups.entry(dir).or_insert_with(Vec::new).push(format!("View: {}", view.name));
        }
        
        groups
    }
    
    // Helper methods
    fn count_route_methods(routes: &[RouteInfo]) -> HashMap<String, usize> {
        routes.iter()
            .fold(HashMap::new(), |mut acc, route| {
                *acc.entry(route.method.clone()).or_insert(0) += 1;
                acc
            })
    }
}

/// Utility for converting data to different string formats
pub struct StringTransformer;

impl StringTransformer {
    /// Convert table data to CSV string
    pub fn to_csv_string(data: Vec<Vec<String>>) -> String {
        data.into_iter()
            .map(|row| row.join(","))
            .collect::<Vec<_>>()
            .join("\n")
    }
    
    /// Convert table data to TSV string
    pub fn to_tsv_string(data: Vec<Vec<String>>) -> String {
        data.into_iter()
            .map(|row| row.join("\t"))
            .collect::<Vec<_>>()
            .join("\n")
    }
    
    /// Convert table data to Markdown table
    pub fn to_markdown_table(data: Vec<Vec<String>>) -> String {
        if data.is_empty() {
            return String::new();
        }
        
        let mut result = String::new();
        
        // Header row
        if let Some(header) = data.first() {
            result.push_str("| ");
            result.push_str(&header.join(" | "));
            result.push_str(" |\n");
            
            // Separator row
            result.push_str("|");
            for _ in header {
                result.push_str("---|");
            }
            result.push('\n');
        }
        
        // Data rows
        for row in data.iter().skip(1) {
            result.push_str("| ");
            result.push_str(&row.join(" | "));
            result.push_str(" |\n");
        }
        
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flat_map_conversion() {
        let analysis = ProjectAnalysis {
            project_name: "test".to_string(),
            framework_version: "1.0".to_string(),
            controllers: vec![],
            routes: vec![],
            middleware: vec![],
            models: vec![],
            views: vec![],
            issues: vec![],
        };
        
        let flat_map = DataTransformer::to_flat_map(&analysis);
        assert_eq!(flat_map.get("project_name"), Some(&"test".to_string()));
        assert_eq!(flat_map.get("controllers_count"), Some(&"0".to_string()));
    }
    
    #[test]
    fn test_csv_conversion() {
        let data = vec![
            vec!["Name".to_string(), "Value".to_string()],
            vec!["Test".to_string(), "123".to_string()],
        ];
        
        let csv = StringTransformer::to_csv_string(data);
        assert_eq!(csv, "Name,Value\nTest,123");
    }
    
    #[test]
    fn test_markdown_table() {
        let data = vec![
            vec!["Name".to_string(), "Value".to_string()],
            vec!["Test".to_string(), "123".to_string()],
        ];
        
        let markdown = StringTransformer::to_markdown_table(data);
        assert!(markdown.contains("| Name | Value |"));
        assert!(markdown.contains("| Test | 123 |"));
    }
}