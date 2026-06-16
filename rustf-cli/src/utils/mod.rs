//! Common utilities and helper functions for RustF CLI

pub mod backup;
pub mod transformers;

pub use transformers::*;

use crate::analyzer::{ProjectAnalysis, ControllerInfo, RouteInfo, HandlerInfo};
use std::collections::HashMap;

/// Common utility functions for analysis data
pub struct AnalysisUtils;

impl AnalysisUtils {
    /// Get analysis statistics summary
    pub fn get_stats_summary(analysis: &ProjectAnalysis) -> AnalysisStats {
        let route_methods: HashMap<String, usize> = analysis.routes.iter()
            .fold(HashMap::new(), |mut acc, route| {
                *acc.entry(route.method.clone()).or_insert(0) += 1;
                acc
            });
        
        let complexity_stats = Self::calculate_complexity_stats(analysis);
        let security_stats = Self::calculate_security_stats(analysis);
        
        AnalysisStats {
            controllers_count: analysis.controllers.len(),
            routes_count: analysis.routes.len(),
            middleware_count: analysis.middleware.len(),
            models_count: analysis.models.len(),
            views_count: analysis.views.len(),
            issues_count: analysis.issues.len(),
            route_methods,
            complexity_stats,
            security_stats,
        }
    }
    
    /// Calculate complexity statistics
    pub fn calculate_complexity_stats(analysis: &ProjectAnalysis) -> ComplexityStats {
        let all_handlers: Vec<&HandlerInfo> = analysis.controllers.iter()
            .flat_map(|c| &c.handlers)
            .collect();
        
        if all_handlers.is_empty() {
            return ComplexityStats::default();
        }
        
        let complexities: Vec<u32> = all_handlers.iter().map(|h| h.complexity).collect();
        let low_complexity = complexities.iter().filter(|&&c| c <= 5).count();
        let medium_complexity = complexities.iter().filter(|&&c| c > 5 && c <= 15).count();
        let high_complexity = complexities.iter().filter(|&&c| c > 15).count();
        let avg_complexity = complexities.iter().sum::<u32>() as f64 / complexities.len() as f64;
        let max_complexity = *complexities.iter().max().unwrap_or(&0);
        
        ComplexityStats {
            low_complexity,
            medium_complexity,
            high_complexity,
            avg_complexity,
            max_complexity,
        }
    }
    
    /// Calculate security statistics
    pub fn calculate_security_stats(analysis: &ProjectAnalysis) -> SecurityStats {
        let error_issues = analysis.issues.iter().filter(|i| i.severity == "error").count();
        let warning_issues = analysis.issues.iter().filter(|i| i.severity == "warning").count();

        let views_with_security_issues = analysis.views.iter()
            .filter(|v| !v.security_issues.is_empty())
            .count();

        SecurityStats {
            error_issues,
            warning_issues,
            views_with_security_issues,
        }
    }
    
    /// Group routes by HTTP method
    pub fn group_routes_by_method(routes: &[RouteInfo]) -> HashMap<String, Vec<&RouteInfo>> {
        routes.iter()
            .fold(HashMap::new(), |mut acc, route| {
                acc.entry(route.method.clone()).or_insert_with(Vec::new).push(route);
                acc
            })
    }
    
    /// Find high complexity handlers
    pub fn find_high_complexity_handlers(controllers: &[ControllerInfo], threshold: u32) -> Vec<&HandlerInfo> {
        controllers.iter()
            .flat_map(|c| &c.handlers)
            .filter(|h| h.complexity > threshold)
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct AnalysisStats {
    pub controllers_count: usize,
    pub routes_count: usize,
    pub middleware_count: usize,
    pub models_count: usize,
    pub views_count: usize,
    pub issues_count: usize,
    pub route_methods: HashMap<String, usize>,
    pub complexity_stats: ComplexityStats,
    pub security_stats: SecurityStats,
}

#[derive(Debug, Clone, Default)]
pub struct ComplexityStats {
    pub low_complexity: usize,
    pub medium_complexity: usize,
    pub high_complexity: usize,
    pub avg_complexity: f64,
    pub max_complexity: u32,
}

#[derive(Debug, Clone, Default)]
pub struct SecurityStats {
    pub error_issues: usize,
    pub warning_issues: usize,
    pub views_with_security_issues: usize,
}

/// Formatting helpers
pub struct FormatUtils;

impl FormatUtils {
    /// Get complexity indicator emoji
    pub fn complexity_indicator(complexity: u32) -> &'static str {
        match complexity {
            0..=5 => "🟢",
            6..=15 => "🟡",
            16..=30 => "🟠",
            _ => "🔴",
        }
    }
    
    /// Get severity indicator emoji
    pub fn severity_indicator(severity: &str) -> &'static str {
        match severity {
            "error" => "❌",
            "warning" => "⚠️",
            "info" => "ℹ️",
            _ => "•",
        }
    }
}