//! Data transformation utilities for converting between formats

use crate::analyzer::ProjectAnalysis;
use crate::utils::AnalysisStats;
use serde_json::Value;

/// Transform analysis data into different representations
pub struct DataTransformer;

impl DataTransformer {
    /// Convert analysis to summary statistics
    pub fn to_summary_stats(analysis: &ProjectAnalysis) -> AnalysisStats {
        crate::utils::AnalysisUtils::get_stats_summary(analysis)
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
}
