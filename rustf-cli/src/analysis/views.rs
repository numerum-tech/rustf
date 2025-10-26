use std::path::Path;
use std::fs;
use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};
use regex::Regex;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewAnalysis {
    pub name: String,
    pub file_path: String,
    pub template_type: TemplateType,
    pub layout: Option<String>,
    pub template_variables: Vec<TemplateVariable>,
    pub includes: Vec<String>,
    pub partials: Vec<String>,
    pub forms: Vec<FormAnalysis>,
    pub security_issues: Vec<SecurityIssue>,
    pub complexity_metrics: ViewComplexityMetrics,
    pub controller_mappings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TemplateType {
    Layout,
    Page,
    Partial,
    Component,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVariable {
    pub name: String,
    pub variable_type: VariableType,
    pub usage_count: usize,
    pub first_occurrence_line: usize,
    pub is_escaped: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VariableType {
    Simple,      // {{variable}}
    Expression,  // {{user.name}}
    Loop,        // {{#each items}}
    Conditional, // {{#if condition}}
    Helper,      // {{formatDate date}}
    Raw,         // {{{raw_html}}}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormAnalysis {
    pub method: String,
    pub action: String,
    pub has_csrf_token: bool,
    pub input_fields: Vec<FormField>,
    pub validation_attributes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormField {
    pub name: String,
    pub field_type: String,
    pub required: bool,
    pub validation_pattern: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityIssue {
    pub issue_type: SecurityIssueType,
    pub description: String,
    pub line_number: usize,
    pub severity: String,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityIssueType {
    XssVulnerability,
    MissingCsrfToken,
    UnsafeVariable,
    ExternalLink,
    InlineScript,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewComplexityMetrics {
    pub total_lines: usize,
    pub template_variables_count: usize,
    pub includes_count: usize,
    pub forms_count: usize,
    pub nesting_depth: usize,
    pub complexity_score: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewToControllerMapping {
    pub view_name: String,
    pub controllers: Vec<ControllerViewUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerViewUsage {
    pub controller_name: String,
    pub handler_name: String,
    pub usage_type: ViewUsageType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViewUsageType {
    Direct,    // ctx.view("template", data)
    Layout,    // ctx.layout("layout")
    Redirect,  // via redirect with flash
}

pub struct ViewAnalyzer;

impl ViewAnalyzer {
    pub fn analyze_view(file_path: &Path) -> Result<ViewAnalysis> {
        let content = fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read view file: {}", file_path.display()))?;

        let name = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let template_type = Self::determine_template_type(&name, &content);
        let layout = Self::extract_layout(&content);
        let template_variables = Self::extract_template_variables(&content);
        let includes = Self::extract_includes(&content);
        let partials = Self::extract_partials(&content);
        let forms = Self::analyze_forms(&content);
        let security_issues = Self::analyze_security_issues(&content);
        let complexity_metrics = Self::calculate_complexity_metrics(&content, &template_variables, &includes, &forms);

        Ok(ViewAnalysis {
            name,
            file_path: file_path.to_string_lossy().to_string(),
            template_type,
            layout,
            template_variables,
            includes,
            partials,
            forms,
            security_issues,
            complexity_metrics,
            controller_mappings: Vec::new(), // Will be populated during cross-reference analysis
        })
    }

    fn determine_template_type(name: &str, content: &str) -> TemplateType {
        let name_lower = name.to_lowercase();
        
        if name_lower.contains("layout") || content.contains("{% block content %}") || content.contains("{{ content }}") {
            TemplateType::Layout
        } else if name_lower.starts_with("_") || name_lower.contains("partial") {
            TemplateType::Partial
        } else if content.contains("{% macro") || name_lower.contains("component") {
            TemplateType::Component
        } else if content.contains("<html") || content.contains("<!DOCTYPE") {
            TemplateType::Page
        } else {
            TemplateType::Unknown
        }
    }

    fn extract_layout(content: &str) -> Option<String> {
        // Look for Tera layout declarations like {% extends "layout.html" %}
        let layout_patterns = [
            r#"\{%\s*extends\s+"([^"]+)"\s*%\}"#,
            r#"\{%\s*extends\s+'([^']+)'\s*%\}"#,
            r#"\{%\s*extends\s+([^\s%]+)\s*%\}"#,
        ];

        for pattern in &layout_patterns {
            if let Ok(re) = Regex::new(pattern) {
                if let Some(captures) = re.captures(content) {
                    if let Some(layout_name) = captures.get(1) {
                        let layout = layout_name.as_str().trim();
                        // Remove .html extension if present and extract layout name
                        if layout.starts_with("layouts/") {
                            return Some(layout.strip_prefix("layouts/").unwrap_or(layout)
                                      .strip_suffix(".html").unwrap_or(layout).to_string());
                        } else {
                            return Some(layout.strip_suffix(".html").unwrap_or(layout).to_string());
                        }
                    }
                }
            }
        }

        None
    }

    fn extract_template_variables(content: &str) -> Vec<TemplateVariable> {
        let mut variables = HashMap::new();
        let lines: Vec<&str> = content.lines().collect();

        // Patterns for different Tera variable types
        let patterns = [
            (r"\{\{\s*([a-zA-Z_][a-zA-Z0-9_.]*)\s*\|\s*safe\s*\}\}", VariableType::Raw),        // {{ var | safe }}
            (r"\{%\s*for\s+[a-zA-Z_][a-zA-Z0-9_]*\s+in\s+([a-zA-Z_][a-zA-Z0-9_.]+)\s*%\}", VariableType::Loop),  // {% for item in items %}
            (r"\{%\s*if\s+([^%]+)\s*%\}", VariableType::Conditional), // {% if condition %}
            (r"\{\{\s*([a-zA-Z_][a-zA-Z0-9_.]*)\s*\|\s*([a-zA-Z_][a-zA-Z0-9_]+)\s*\}\}", VariableType::Helper), // {{ var | filter }}
            (r"\{\{\s*([a-zA-Z_][a-zA-Z0-9_.]+)\s*\}\}", VariableType::Expression), // {{ user.name }}
            (r"\{\{\s*([a-zA-Z_][a-zA-Z0-9_]*)\s*\}\}", VariableType::Simple), // {{ variable }}
        ];

        for (line_num, line) in lines.iter().enumerate() {
            for (pattern, var_type) in &patterns {
                if let Ok(re) = Regex::new(pattern) {
                    for capture in re.captures_iter(line) {
                        if let Some(var_match) = capture.get(1) {
                            let var_name = var_match.as_str().trim().to_string();
                            let is_escaped = !matches!(var_type, VariableType::Raw);
                            
                            let entry = variables.entry(var_name.clone()).or_insert(TemplateVariable {
                                name: var_name,
                                variable_type: var_type.clone(),
                                usage_count: 0,
                                first_occurrence_line: line_num + 1,
                                is_escaped,
                            });
                            entry.usage_count += 1;
                        }
                    }
                }
            }
        }

        variables.into_values().collect()
    }

    fn extract_includes(content: &str) -> Vec<String> {
        let mut includes = HashSet::new();
        
        let include_patterns = [
            r#"\{%\s*include\s+"([^"]+)"\s*%\}"#,
            r#"\{%\s*include\s+'([^']+)'\s*%\}"#,
            r#"\{%\s*include\s+([^\s%]+)\s*%\}"#,
            r#"<\w+\s+src="([^"]+\.html)""#,
        ];

        for pattern in &include_patterns {
            if let Ok(re) = Regex::new(pattern) {
                for capture in re.captures_iter(content) {
                    if let Some(include_match) = capture.get(1) {
                        includes.insert(include_match.as_str().trim().to_string());
                    }
                }
            }
        }

        includes.into_iter().collect()
    }

    fn extract_partials(content: &str) -> Vec<String> {
        let mut partials = HashSet::new();
        
        // Look for Tera macro and include references
        let partial_patterns = [
            r#"\{%\s*import\s+"([^"]+)"\s*%\}"#,
            r#"\{%\s*import\s+'([^']+)'\s*%\}"#,
            r#"\{\{\s*([a-zA-Z_][a-zA-Z0-9_]*)\s*\(\)"#, // macro calls like {{ my_macro() }}
        ];

        for pattern in &partial_patterns {
            if let Ok(re) = Regex::new(pattern) {
                for capture in re.captures_iter(content) {
                    if let Some(partial_match) = capture.get(1) {
                        partials.insert(partial_match.as_str().trim().to_string());
                    }
                }
            }
        }

        partials.into_iter().collect()
    }

    fn analyze_forms(content: &str) -> Vec<FormAnalysis> {
        let mut forms = Vec::new();
        
        // Extract form tags
        if let Ok(form_re) = Regex::new(r#"<form[^>]*method\s*=\s*["']([^"']+)["'][^>]*action\s*=\s*["']([^"']+)["'][^>]*>(.*?)</form>"#) {
            for capture in form_re.captures_iter(content) {
                let method = capture.get(1).map(|m| m.as_str().to_uppercase()).unwrap_or_default();
                let action = capture.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
                let form_content = capture.get(3).map(|m| m.as_str()).unwrap_or_default();
                
                let has_csrf_token = form_content.contains("csrf") || form_content.contains("_token");
                let input_fields = Self::extract_form_fields(form_content);
                let validation_attributes = Self::extract_validation_attributes(form_content);

                forms.push(FormAnalysis {
                    method,
                    action,
                    has_csrf_token,
                    input_fields,
                    validation_attributes,
                });
            }
        }

        forms
    }

    fn extract_form_fields(form_content: &str) -> Vec<FormField> {
        let mut fields = Vec::new();
        
        if let Ok(input_re) = Regex::new(r#"<input[^>]*name\s*=\s*["']([^"']+)["'][^>]*type\s*=\s*["']([^"']+)["'][^>]*>"#) {
            for capture in input_re.captures_iter(form_content) {
                let name = capture.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
                let field_type = capture.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
                let required = form_content.contains("required");
                
                fields.push(FormField {
                    name,
                    field_type,
                    required,
                    validation_pattern: None, // TODO: extract pattern attribute
                });
            }
        }

        fields
    }

    fn extract_validation_attributes(form_content: &str) -> Vec<String> {
        let mut attributes = Vec::new();
        
        let validation_attrs = ["required", "minlength", "maxlength", "pattern", "min", "max", "step"];
        for attr in &validation_attrs {
            if form_content.contains(attr) {
                attributes.push(attr.to_string());
            }
        }

        attributes
    }

    fn analyze_security_issues(content: &str) -> Vec<SecurityIssue> {
        let mut issues = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        for (line_num, line) in lines.iter().enumerate() {
            // Check for XSS vulnerabilities (unescaped variables with safe filter)
            if let Ok(re) = Regex::new(r"\{\{[^}]*\|\s*safe[^}]*\}\}") {
                if re.is_match(line) {
                    issues.push(SecurityIssue {
                        issue_type: SecurityIssueType::XssVulnerability,
                        description: "Unescaped template variable with 'safe' filter may be vulnerable to XSS".to_string(),
                        line_number: line_num + 1,
                        severity: "high".to_string(),
                        recommendation: "Ensure data is properly sanitized before using the 'safe' filter".to_string(),
                    });
                }
            }

            // Check for inline scripts
            if line.contains("<script") && !line.contains("src=") {
                issues.push(SecurityIssue {
                    issue_type: SecurityIssueType::InlineScript,
                    description: "Inline script detected".to_string(),
                    line_number: line_num + 1,
                    severity: "medium".to_string(),
                    recommendation: "Move scripts to external files for better CSP compliance".to_string(),
                });
            }

            // Check for external links without rel="noopener"
            if let Ok(re) = Regex::new(r#"<a[^>]*href\s*=\s*["']https?://[^"']+["'][^>]*target\s*=\s*["']_blank["'][^>]*>"#) {
                if re.is_match(line) && !line.contains("rel=") {
                    issues.push(SecurityIssue {
                        issue_type: SecurityIssueType::ExternalLink,
                        description: "External link with target='_blank' missing rel='noopener'".to_string(),
                        line_number: line_num + 1,
                        severity: "low".to_string(),
                        recommendation: "Add rel='noopener noreferrer' to prevent window.opener access".to_string(),
                    });
                }
            }
        }

        // Check for missing CSRF tokens in forms
        if content.contains("<form") && content.contains("method") {
            let has_csrf = content.contains("csrf") || content.contains("_token");
            if !has_csrf {
                issues.push(SecurityIssue {
                    issue_type: SecurityIssueType::MissingCsrfToken,
                    description: "Form without CSRF protection detected".to_string(),
                    line_number: 0,
                    severity: "high".to_string(),
                    recommendation: "Add CSRF token to form for protection against cross-site request forgery".to_string(),
                });
            }
        }

        issues
    }

    fn calculate_complexity_metrics(
        content: &str, 
        variables: &[TemplateVariable], 
        includes: &[String], 
        forms: &[FormAnalysis]
    ) -> ViewComplexityMetrics {
        let total_lines = content.lines().count();
        let template_variables_count = variables.len();
        let includes_count = includes.len();
        let forms_count = forms.len();
        
        // Calculate nesting depth by counting maximum nested template blocks
        let nesting_depth = Self::calculate_nesting_depth(content);
        
        // Complexity score calculation
        let mut complexity_score = 0u32;
        complexity_score += total_lines as u32 / 10; // +1 per 10 lines
        complexity_score += template_variables_count as u32 * 2; // +2 per variable
        complexity_score += includes_count as u32 * 3; // +3 per include
        complexity_score += forms_count as u32 * 5; // +5 per form
        complexity_score += nesting_depth as u32 * 2; // +2 per nesting level

        ViewComplexityMetrics {
            total_lines,
            template_variables_count,
            includes_count,
            forms_count,
            nesting_depth,
            complexity_score,
        }
    }

    fn calculate_nesting_depth(content: &str) -> usize {
        let mut max_depth = 0;
        let mut current_depth = 0;
        
        // Track opening and closing Tera template blocks
        let lines = content.lines();
        for line in lines {
            // Count opening blocks ({% if %}, {% for %}, {% block %}, etc.)
            let opens = line.matches("{% if").count() + 
                       line.matches("{% for").count() + 
                       line.matches("{% block").count() +
                       line.matches("{% macro").count();
            let closes = line.matches("{% endif").count() + 
                        line.matches("{% endfor").count() + 
                        line.matches("{% endblock").count() +
                        line.matches("{% endmacro").count();
            
            current_depth += opens;
            max_depth = max_depth.max(current_depth);
            current_depth = current_depth.saturating_sub(closes);
        }
        
        max_depth
    }

    pub fn find_view_controller_mappings(
        controllers_path: &Path,
        views: &[ViewAnalysis]
    ) -> Result<Vec<ViewToControllerMapping>> {
        let mut mappings = Vec::new();
        
        for view in views {
            let mut view_mapping = ViewToControllerMapping {
                view_name: view.name.clone(),
                controllers: Vec::new(),
            };
            
            // Scan controller files for view references
            if controllers_path.exists() {
                for entry in fs::read_dir(controllers_path)? {
                    let entry = entry?;
                    let path = entry.path();
                    
                    if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                        if let Ok(controller_content) = fs::read_to_string(&path) {
                            let controller_name = path.file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("unknown")
                                .to_string();
                            
                            // Look for view references in controller
                            let view_usages = Self::find_view_usage_in_controller(&controller_content, &view.name);
                            
                            for usage in view_usages {
                                view_mapping.controllers.push(ControllerViewUsage {
                                    controller_name: controller_name.clone(),
                                    handler_name: usage.0,
                                    usage_type: usage.1,
                                });
                            }
                        }
                    }
                }
            }
            
            mappings.push(view_mapping);
        }
        
        Ok(mappings)
    }

    fn find_view_usage_in_controller(content: &str, view_name: &str) -> Vec<(String, ViewUsageType)> {
        let mut usages = Vec::new();
        
        // Look for ctx.view() calls
        if let Ok(re) = Regex::new(&format!(r#"ctx\.view\s*\(\s*["']/?{}["']"#, regex::escape(view_name))) {
            if re.is_match(content) {
                // Try to find the handler function name
                let handler_name = Self::find_handler_name_from_view_call(content, view_name)
                    .unwrap_or_else(|| "unknown".to_string());
                usages.push((handler_name, ViewUsageType::Direct));
            }
        }
        
        // Look for ctx.layout() calls
        if content.contains(&format!("ctx.layout(\"{}\")", view_name)) ||
           content.contains(&format!("ctx.layout('{}')", view_name)) {
            let handler_name = Self::find_handler_name_from_layout_call(content, view_name)
                .unwrap_or_else(|| "unknown".to_string());
            usages.push((handler_name, ViewUsageType::Layout));
        }
        
        usages
    }

    fn find_handler_name_from_view_call(content: &str, view_name: &str) -> Option<String> {
        // Simple heuristic: find the function containing the view call
        let lines: Vec<&str> = content.lines().collect();
        
        for (i, line) in lines.iter().enumerate() {
            if line.contains(&format!("ctx.view")) && line.contains(view_name) {
                // Look backwards for function definition
                for j in (0..i).rev() {
                    if let Some(func_name) = Self::extract_function_name(lines[j]) {
                        return Some(func_name);
                    }
                }
            }
        }
        
        None
    }

    fn find_handler_name_from_layout_call(content: &str, layout_name: &str) -> Option<String> {
        // Similar to view call but for layout
        let lines: Vec<&str> = content.lines().collect();
        
        for (i, line) in lines.iter().enumerate() {
            if line.contains("ctx.layout") && line.contains(layout_name) {
                for j in (0..i).rev() {
                    if let Some(func_name) = Self::extract_function_name(lines[j]) {
                        return Some(func_name);
                    }
                }
            }
        }
        
        None
    }

    fn extract_function_name(line: &str) -> Option<String> {
        // Look for async fn or fn declarations
        if let Ok(re) = Regex::new(r"(?:async\s+)?fn\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(") {
            if let Some(captures) = re.captures(line) {
                if let Some(func_name) = captures.get(1) {
                    return Some(func_name.as_str().to_string());
                }
            }
        }
        
        None
    }
}