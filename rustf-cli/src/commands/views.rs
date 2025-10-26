use std::path::PathBuf;
use anyhow::Result;
use crate::analyzer::{files::ProjectFiles, ProjectAnalyzer};
use crate::analysis::views::{ViewAnalyzer, ViewAnalysis, ViewToControllerMapping};
use rayon::prelude::*;

pub async fn run(project_path: PathBuf, name: Option<String>, layout: bool, security: bool) -> Result<()> {
    log::info!("Analyzing views with optimizations...");
    
    // Use parallel file scanning for better performance
    let files = ProjectFiles::scan_parallel(&project_path)?;
    
    if files.views.is_empty() {
        println!("No views found in the project.");
        return Ok(());
    }
    
    // Parallel analysis of all views
    let view_analyses: Vec<ViewAnalysis> = files.views
        .par_iter()
        .filter_map(|view_path| {
            match ViewAnalyzer::analyze_view(view_path) {
                Ok(analysis) => Some(analysis),
                Err(e) => {
                    log::warn!("Failed to analyze view {}: {}", view_path.display(), e);
                    None
                }
            }
        })
        .collect();
    
    // Get view-to-controller mappings using the analyzer with caching
    let analyzer = ProjectAnalyzer::new(project_path.clone())?;
    let project_analysis = analyzer.analyze_complete(false).await?;
    
    // Build fast lookup map for controller->handler->routes
    let mut controller_view_map = std::collections::HashMap::new();
    for controller in &project_analysis.controllers {
        for handler in &controller.handlers {
            // Check if handler likely renders a view (contains "view" or common view patterns)
            if handler.name.contains("view") || 
               handler.name.contains("show") || 
               handler.name.contains("edit") || 
               handler.name.contains("create") ||
               handler.name.contains("index") ||
               handler.name.contains("form") {
                controller_view_map.insert(
                    format!("{}::{}", controller.name, handler.name),
                    (controller.name.clone(), handler.name.clone())
                );
            }
        }
    }
    
    let mappings = create_optimized_view_mappings(&view_analyses, &controller_view_map);
    
    // Update view analyses with controller mappings (parallel)
    let mut view_analyses = view_analyses; // Make mutable
    view_analyses.par_iter_mut().for_each(|view| {
        if let Some(mapping) = mappings.iter().find(|m| m.view_name == view.name) {
            view.controller_mappings = mapping.controllers.iter()
                .map(|c| format!("{}::{}", c.controller_name, c.handler_name))
                .collect();
        }
    });
    
    // Handle specific view query
    if let Some(target_name) = name {
        if let Some(view) = view_analyses.iter().find(|v| v.name == target_name) {
            display_detailed_view_analysis(view);
        } else {
            println!("View '{}' not found", target_name);
            println!("Available views: {}", 
                view_analyses.iter().map(|v| v.name.as_str()).collect::<Vec<_>>().join(", "));
        }
        return Ok(());
    }
    
    // Handle layout hierarchy display
    if layout {
        display_layout_hierarchy(&view_analyses);
        return Ok(());
    }
    
    // Handle security analysis
    if security {
        display_security_analysis(&view_analyses);
        return Ok(());
    }
    
    // Display overview of all views
    display_views_overview(&view_analyses, &mappings);
    
    Ok(())
}

fn display_views_overview(views: &[ViewAnalysis], _mappings: &[ViewToControllerMapping]) {
    println!("=== Views Analysis ===\n");
    println!("Total Views: {}", views.len());
    
    let layouts = views.iter().filter(|v| matches!(v.template_type, crate::analysis::views::TemplateType::Layout)).count();
    let pages = views.iter().filter(|v| matches!(v.template_type, crate::analysis::views::TemplateType::Page)).count();
    let partials = views.iter().filter(|v| matches!(v.template_type, crate::analysis::views::TemplateType::Partial)).count();
    let components = views.iter().filter(|v| matches!(v.template_type, crate::analysis::views::TemplateType::Component)).count();
    
    println!("Template Types:");
    println!("  - Layouts: {}", layouts);
    println!("  - Pages: {}", pages);
    println!("  - Partials: {}", partials);
    println!("  - Components: {}", components);
    
    let total_variables: usize = views.iter().map(|v| v.template_variables.len()).sum();
    let total_forms: usize = views.iter().map(|v| v.forms.len()).sum();
    let total_security_issues: usize = views.iter().map(|v| v.security_issues.len()).sum();
    
    println!("\nOverall Metrics:");
    println!("  - Template Variables: {}", total_variables);
    println!("  - Forms: {}", total_forms);
    println!("  - Security Issues: {}", total_security_issues);
    
    // Average complexity
    let avg_complexity = if !views.is_empty() {
        views.iter().map(|v| v.complexity_metrics.complexity_score).sum::<u32>() as f64 / views.len() as f64
    } else {
        0.0
    };
    println!("  - Average Complexity: {:.1}", avg_complexity);
    
    println!("\n=== Views Overview ===");
    println!("{:20} | {:10} | {:8} | {:6} | {:6} | Controllers", "Name", "Type", "Lines", "Vars", "Issues");
    println!("{:-<80}", "");
    
    for view in views {
        let template_type = format!("{:?}", view.template_type);
        let controllers = if view.controller_mappings.is_empty() {
            "none".to_string()
        } else {
            view.controller_mappings.join(", ")
        };
        
        println!("{:20} | {:10} | {:8} | {:6} | {:6} | {}", 
            view.name,
            template_type,
            view.complexity_metrics.total_lines,
            view.complexity_metrics.template_variables_count,
            view.security_issues.len(),
            controllers
        );
    }
    
    // Show views without controller mappings
    let unmapped_views: Vec<&ViewAnalysis> = views.iter()
        .filter(|v| v.controller_mappings.is_empty())
        .collect();
    
    if !unmapped_views.is_empty() {
        println!("\n=== Unmapped Views ===");
        println!("The following views are not referenced by any controllers:");
        for view in unmapped_views {
            println!("  - {} ({})", view.name, view.file_path);
        }
    }
}

fn display_detailed_view_analysis(view: &ViewAnalysis) {
    println!("=== View: {} ===\n", view.name);
    println!("File: {}", view.file_path);
    println!("Type: {:?}", view.template_type);
    
    if let Some(layout) = &view.layout {
        println!("Layout: {}", layout);
    }
    
    println!("\n--- Complexity Metrics ---");
    println!("Lines: {}", view.complexity_metrics.total_lines);
    println!("Template Variables: {}", view.complexity_metrics.template_variables_count);
    println!("Includes: {}", view.complexity_metrics.includes_count);
    println!("Forms: {}", view.complexity_metrics.forms_count);
    println!("Nesting Depth: {}", view.complexity_metrics.nesting_depth);
    println!("Complexity Score: {}", view.complexity_metrics.complexity_score);
    
    if !view.template_variables.is_empty() {
        println!("\n--- Template Variables ---");
        for var in &view.template_variables {
            println!("  {:20} | {:12} | Line {:3} | Uses: {:2} | Escaped: {}", 
                var.name,
                format!("{:?}", var.variable_type),
                var.first_occurrence_line,
                var.usage_count,
                var.is_escaped
            );
        }
    }
    
    if !view.includes.is_empty() {
        println!("\n--- Includes ---");
        for include in &view.includes {
            println!("  - {}", include);
        }
    }
    
    if !view.partials.is_empty() {
        println!("\n--- Partials ---");
        for partial in &view.partials {
            println!("  - {}", partial);
        }
    }
    
    if !view.forms.is_empty() {
        println!("\n--- Forms ---");
        for (i, form) in view.forms.iter().enumerate() {
            println!("  Form {}:", i + 1);
            println!("    Method: {}", form.method);
            println!("    Action: {}", form.action);
            println!("    CSRF Token: {}", if form.has_csrf_token { "Yes" } else { "No" });
            println!("    Fields: {}", form.input_fields.len());
            
            if !form.input_fields.is_empty() {
                for field in &form.input_fields {
                    println!("      - {} ({}){}", 
                        field.name, 
                        field.field_type,
                        if field.required { " [required]" } else { "" }
                    );
                }
            }
        }
    }
    
    if !view.security_issues.is_empty() {
        println!("\n--- Security Issues ---");
        for issue in &view.security_issues {
            let severity_emoji = match issue.severity.as_str() {
                "high" => "🔴",
                "medium" => "🟡",
                _ => "🟢",
            };
            
            println!("{} {:?} (Line {}): {}", 
                severity_emoji,
                issue.issue_type,
                issue.line_number,
                issue.description
            );
            println!("    Recommendation: {}", issue.recommendation);
        }
    }
    
    if !view.controller_mappings.is_empty() {
        println!("\n--- Controller Mappings ---");
        for mapping in &view.controller_mappings {
            println!("  - {}", mapping);
        }
    }
}

fn display_layout_hierarchy(views: &[ViewAnalysis]) {
    println!("=== Layout Hierarchy ===\n");
    
    // Find layouts
    let layouts: Vec<&ViewAnalysis> = views.iter()
        .filter(|v| matches!(v.template_type, crate::analysis::views::TemplateType::Layout))
        .collect();
    
    if layouts.is_empty() {
        println!("No layouts found.");
        return;
    }
    
    for layout in layouts {
        println!("📋 Layout: {}", layout.name);
        
        // Find views that use this layout
        let views_using_layout: Vec<&ViewAnalysis> = views.iter()
            .filter(|v| v.layout.as_ref() == Some(&layout.name))
            .collect();
        
        if views_using_layout.is_empty() {
            println!("  └── (no views use this layout)");
        } else {
            for (i, view) in views_using_layout.iter().enumerate() {
                let prefix = if i == views_using_layout.len() - 1 { "└──" } else { "├──" };
                println!("  {} 📄 {} ({:?})", prefix, view.name, view.template_type);
                
                // Show includes for each view
                if !view.includes.is_empty() {
                    let sub_prefix = if i == views_using_layout.len() - 1 { "    " } else { "│   " };
                    for (j, include) in view.includes.iter().enumerate() {
                        let inc_prefix = if j == view.includes.len() - 1 { "└──" } else { "├──" };
                        println!("  {}   {} 📝 {}", sub_prefix, inc_prefix, include);
                    }
                }
            }
        }
        println!();
    }
    
    // Show views without layouts
    let views_without_layout: Vec<&ViewAnalysis> = views.iter()
        .filter(|v| v.layout.is_none() && !matches!(v.template_type, crate::analysis::views::TemplateType::Layout))
        .collect();
    
    if !views_without_layout.is_empty() {
        println!("🔍 Views without layouts:");
        for view in views_without_layout {
            println!("  └── 📄 {} ({:?})", view.name, view.template_type);
        }
    }
}

fn display_security_analysis(views: &[ViewAnalysis]) {
    println!("=== Security Analysis ===\n");
    
    let mut total_issues = 0;
    let mut high_severity = 0;
    let mut medium_severity = 0;
    let mut low_severity = 0;
    
    for view in views {
        for issue in &view.security_issues {
            total_issues += 1;
            match issue.severity.as_str() {
                "high" => high_severity += 1,
                "medium" => medium_severity += 1,
                _ => low_severity += 1,
            }
        }
    }
    
    println!("Total Security Issues: {}", total_issues);
    println!("  🔴 High Severity: {}", high_severity);
    println!("  🟡 Medium Severity: {}", medium_severity);
    println!("  🟢 Low Severity: {}", low_severity);
    
    if total_issues == 0 {
        println!("\n✅ No security issues found!");
        return;
    }
    
    println!("\n=== Issues by View ===");
    
    for view in views {
        if !view.security_issues.is_empty() {
            println!("\n📄 {} ({} issues)", view.name, view.security_issues.len());
            
            for issue in &view.security_issues {
                let severity_emoji = match issue.severity.as_str() {
                    "high" => "🔴",
                    "medium" => "🟡",
                    _ => "🟢",
                };
                
                println!("  {} Line {}: {}", 
                    severity_emoji,
                    issue.line_number,
                    issue.description
                );
                println!("    💡 {}", issue.recommendation);
            }
        }
    }
    
    // Security recommendations
    println!("\n=== Security Recommendations ===");
    
    let forms_without_csrf = views.iter()
        .flat_map(|v| &v.forms)
        .filter(|f| !f.has_csrf_token)
        .count();
    
    if forms_without_csrf > 0 {
        println!("🔴 {} forms missing CSRF protection", forms_without_csrf);
    }
    
    let unescaped_variables = views.iter()
        .flat_map(|v| &v.template_variables)
        .filter(|var| !var.is_escaped)
        .count();
    
    if unescaped_variables > 0 {
        println!("🔴 {} unescaped template variables (XSS risk)", unescaped_variables);
    }
    
    if forms_without_csrf == 0 && unescaped_variables == 0 && high_severity == 0 {
        println!("✅ No critical security issues found!");
    }
}

/// Create optimized view-to-controller mappings using pre-computed controller data
fn create_optimized_view_mappings(
    view_analyses: &[ViewAnalysis], 
    controller_map: &std::collections::HashMap<String, (String, String)>
) -> Vec<ViewToControllerMapping> {
    view_analyses.par_iter().filter_map(|view| {
        // Fast pattern matching for view-controller relationships
        let potential_controllers: Vec<_> = controller_map.iter()
            .filter(|(_handler_key, (controller_name, handler_name))| {
                // Simple heuristics for view-controller matching
                view.name.contains(controller_name) ||
                view.name.contains(handler_name) ||
                handler_name.contains(&view.name) ||
                // Check if view name matches common patterns
                (view.name == "index" && handler_name == "index") ||
                (view.name == "show" && handler_name == "show") ||
                (view.name == "edit" && handler_name == "edit") ||
                (view.name == "create" && handler_name == "create")
            })
            .map(|(_, (controller_name, handler_name))| 
                crate::analysis::views::ControllerViewUsage {
                    controller_name: controller_name.clone(),
                    handler_name: handler_name.clone(),
                    usage_type: crate::analysis::views::ViewUsageType::Direct,
                }
            )
            .collect();
            
        if !potential_controllers.is_empty() {
            Some(ViewToControllerMapping {
                view_name: view.name.clone(),
                controllers: potential_controllers,
            })
        } else {
            None
        }
    }).collect()
}