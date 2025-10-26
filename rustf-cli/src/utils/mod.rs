//! Common utilities and helper functions for RustF CLI

pub mod backup;
pub mod search;
pub mod validators;
pub mod transformers;

pub use transformers::*;

use crate::analyzer::{ProjectAnalysis, ControllerInfo, RouteInfo, HandlerInfo};
use std::collections::HashMap;

/// Common utility functions for analysis data
pub struct AnalysisUtils;

impl AnalysisUtils {
    /// Get analysis statistics summary
    pub fn get_stats_summary(analysis: &ProjectAnalysis) -> AnalysisStats {
        let total_handlers: usize = analysis.controllers.iter().map(|c| c.handlers.len()).sum();
        
        let route_methods: HashMap<String, usize> = analysis.routes.iter()
            .fold(HashMap::new(), |mut acc, route| {
                *acc.entry(route.method.clone()).or_insert(0) += 1;
                acc
            });
        
        let complexity_stats = Self::calculate_complexity_stats(analysis);
        let security_stats = Self::calculate_security_stats(analysis);
        
        AnalysisStats {
            controllers_count: analysis.controllers.len(),
            handlers_count: total_handlers,
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
            total_handlers: all_handlers.len(),
        }
    }
    
    /// Calculate security statistics
    pub fn calculate_security_stats(analysis: &ProjectAnalysis) -> SecurityStats {
        let error_issues = analysis.issues.iter().filter(|i| i.severity == "error").count();
        let warning_issues = analysis.issues.iter().filter(|i| i.severity == "warning").count();
        let info_issues = analysis.issues.iter().filter(|i| i.severity == "info").count();
        
        let views_with_security_issues = analysis.views.iter()
            .filter(|v| !v.security_issues.is_empty())
            .count();
        
        let high_risk_views = analysis.views.iter()
            .filter(|v| v.security_issues.iter().any(|i| i.severity == "high"))
            .count();
        
        SecurityStats {
            error_issues,
            warning_issues,
            info_issues,
            views_with_security_issues,
            high_risk_views,
            total_security_issues: analysis.views.iter()
                .map(|v| v.security_issues.len())
                .sum(),
        }
    }
    
    /// Find components by pattern
    pub fn find_by_pattern<'a, T, F>(items: &'a [T], pattern: &str, extract_name: F) -> Vec<&'a T>
    where
        F: Fn(&T) -> &str,
    {
        if pattern.is_empty() {
            return items.iter().collect();
        }
        
        let pattern_lower = pattern.to_lowercase();
        items.iter()
            .filter(|item| extract_name(item).to_lowercase().contains(&pattern_lower))
            .collect()
    }
    
    /// Group routes by HTTP method
    pub fn group_routes_by_method(routes: &[RouteInfo]) -> HashMap<String, Vec<&RouteInfo>> {
        routes.iter()
            .fold(HashMap::new(), |mut acc, route| {
                acc.entry(route.method.clone()).or_insert_with(Vec::new).push(route);
                acc
            })
    }
    
    /// Find routes with parameters
    pub fn find_parameterized_routes(routes: &[RouteInfo]) -> Vec<&RouteInfo> {
        routes.iter()
            .filter(|route| !route.parameters.is_empty())
            .collect()
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
    pub handlers_count: usize,
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
    pub total_handlers: usize,
}

#[derive(Debug, Clone, Default)]
pub struct SecurityStats {
    pub error_issues: usize,
    pub warning_issues: usize,
    pub info_issues: usize,
    pub views_with_security_issues: usize,
    pub high_risk_views: usize,
    pub total_security_issues: usize,
}

/// Formatting helpers
pub struct FormatUtils;

impl FormatUtils {
    /// Format duration in human-readable form
    pub fn format_duration(duration: std::time::Duration) -> String {
        let total_secs = duration.as_secs();
        let millis = duration.subsec_millis();
        
        if total_secs >= 60 {
            let mins = total_secs / 60;
            let secs = total_secs % 60;
            format!("{}m {}s", mins, secs)
        } else if total_secs > 0 {
            format!("{}.{:03}s", total_secs, millis)
        } else {
            format!("{}ms", millis)
        }
    }
    
    /// Format file size in human-readable form
    pub fn format_file_size(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        const THRESHOLD: f64 = 1024.0;
        
        if bytes == 0 {
            return "0 B".to_string();
        }
        
        let bytes_f64 = bytes as f64;
        let unit_index = (bytes_f64.log10() / THRESHOLD.log10()).floor() as usize;
        let unit_index = unit_index.min(UNITS.len() - 1);
        
        let size = bytes_f64 / THRESHOLD.powi(unit_index as i32);
        
        if size >= 100.0 {
            format!("{:.0} {}", size, UNITS[unit_index])
        } else if size >= 10.0 {
            format!("{:.1} {}", size, UNITS[unit_index])
        } else {
            format!("{:.2} {}", size, UNITS[unit_index])
        }
    }
    
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
    
    /// Generate a progress bar
    pub fn progress_bar(current: usize, total: usize, width: usize) -> String {
        if total == 0 {
            return format!("[{}] 0/0", " ".repeat(width));
        }
        
        let percentage = (current as f64 / total as f64).min(1.0);
        let filled_width = (width as f64 * percentage) as usize;
        let empty_width = width - filled_width;
        
        format!(
            "[{}{}] {}/{} ({:.1}%)",
            "█".repeat(filled_width),
            "░".repeat(empty_width),
            current,
            total,
            percentage * 100.0
        )
    }
}