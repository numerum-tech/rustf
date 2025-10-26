//! Code generation module for RustF schema
//! 
//! This module provides code generation capabilities for various targets:
//! - SQLx models with CRUD operations
//! - SQL migrations
//! - TypeScript interfaces
//! - API documentation

use crate::{Schema, Table, SchemaError, Result};
use handlebars::Handlebars;
use std::collections::HashMap;

pub mod sqlx;
pub mod templates;

pub use sqlx::SqlxGenerator;

/// Code generation context passed to templates
#[derive(Debug, Clone, serde::Serialize)]
pub struct GenerationContext {
    /// Schema being processed
    pub schema: Schema,
    /// Current table being generated
    pub table: Table,
    /// Table name
    pub table_name: String,
    /// Additional template variables
    pub variables: HashMap<String, serde_json::Value>,
}

/// Base trait for code generators
pub trait CodeGenerator {
    /// Generate code for a single table
    fn generate_table(&self, table_name: &str, table: &Table, schema: &Schema) -> Result<String>;
    
    /// Generate code for the entire schema
    fn generate_schema(&self, schema: &Schema) -> Result<HashMap<String, String>> {
        let mut results = HashMap::new();
        
        for (table_name, table) in &schema.tables {
            let code = self.generate_table(table_name, table, schema)?;
            results.insert(table_name.clone(), code);
        }
        
        Ok(results)
    }
}

/// Template-based code generator
pub struct TemplateGenerator {
    handlebars: Handlebars<'static>,
}

impl TemplateGenerator {
    /// Create a new template generator
    pub fn new() -> Self {
        let mut handlebars = Handlebars::new();
        
        // Register helper functions
        handlebars.register_helper("snake_case", Box::new(snake_case_helper));
        handlebars.register_helper("camel_case", Box::new(camel_case_helper));
        handlebars.register_helper("pascal_case", Box::new(pascal_case_helper));
        handlebars.register_helper("pluralize", Box::new(pluralize_helper));
        
        Self { handlebars }
    }
    
    /// Register a template
    pub fn register_template(&mut self, name: &str, template: &str) -> Result<()> {
        self.handlebars.register_template_string(name, template)
            .map_err(|e| SchemaError::CodeGen(format!("Template registration failed: {}", e)))?;
        Ok(())
    }
    
    /// Render a template with context
    pub fn render(&self, template_name: &str, context: &GenerationContext) -> Result<String> {
        self.handlebars.render(template_name, context)
            .map_err(|e| SchemaError::CodeGen(format!("Template rendering failed: {}", e)))
    }
}

impl Default for TemplateGenerator {
    fn default() -> Self {
        Self::new()
    }
}

// Handlebars helper functions

fn snake_case_helper(
    h: &handlebars::Helper,
    _: &Handlebars,
    _: &handlebars::Context,
    _: &mut handlebars::RenderContext,
    out: &mut dyn handlebars::Output,
) -> handlebars::HelperResult {
    let param = h.param(0)
        .ok_or_else(|| handlebars::RenderError::new("snake_case helper requires a parameter"))?;
    
    let input = param.value().as_str()
        .ok_or_else(|| handlebars::RenderError::new("snake_case helper requires a string parameter"))?;
    
    let snake_case = to_snake_case(input);
    out.write(&snake_case)?;
    Ok(())
}

fn camel_case_helper(
    h: &handlebars::Helper,
    _: &Handlebars,
    _: &handlebars::Context,
    _: &mut handlebars::RenderContext,
    out: &mut dyn handlebars::Output,
) -> handlebars::HelperResult {
    let param = h.param(0)
        .ok_or_else(|| handlebars::RenderError::new("camel_case helper requires a parameter"))?;
    
    let input = param.value().as_str()
        .ok_or_else(|| handlebars::RenderError::new("camel_case helper requires a string parameter"))?;
    
    let camel_case = to_camel_case(input);
    out.write(&camel_case)?;
    Ok(())
}

fn pascal_case_helper(
    h: &handlebars::Helper,
    _: &Handlebars,
    _: &handlebars::Context,
    _: &mut handlebars::RenderContext,
    out: &mut dyn handlebars::Output,
) -> handlebars::HelperResult {
    let param = h.param(0)
        .ok_or_else(|| handlebars::RenderError::new("pascal_case helper requires a parameter"))?;
    
    let input = param.value().as_str()
        .ok_or_else(|| handlebars::RenderError::new("pascal_case helper requires a string parameter"))?;
    
    let pascal_case = to_pascal_case(input);
    out.write(&pascal_case)?;
    Ok(())
}

fn pluralize_helper(
    h: &handlebars::Helper,
    _: &Handlebars,
    _: &handlebars::Context,
    _: &mut handlebars::RenderContext,
    out: &mut dyn handlebars::Output,
) -> handlebars::HelperResult {
    let param = h.param(0)
        .ok_or_else(|| handlebars::RenderError::new("pluralize helper requires a parameter"))?;
    
    let input = param.value().as_str()
        .ok_or_else(|| handlebars::RenderError::new("pluralize helper requires a string parameter"))?;
    
    let plural = pluralize(input);
    out.write(&plural)?;
    Ok(())
}

// String transformation utilities

pub fn to_snake_case(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();
    
    while let Some(ch) = chars.next() {
        if ch.is_uppercase() && !result.is_empty() {
            result.push('_');
        }
        result.push(ch.to_lowercase().next().unwrap_or(ch));
    }
    
    result
}

pub fn to_camel_case(input: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;
    
    for ch in input.chars() {
        if ch == '_' || ch == '-' || ch == ' ' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(ch.to_uppercase().next().unwrap_or(ch));
            capitalize_next = false;
        } else {
            result.push(ch.to_lowercase().next().unwrap_or(ch));
        }
    }
    
    result
}

pub fn to_pascal_case(input: &str) -> String {
    let mut camel = to_camel_case(input);
    if let Some(first_char) = camel.chars().next() {
        camel = first_char.to_uppercase().collect::<String>() + &camel[1..];
    }
    camel
}

pub fn pluralize(input: &str) -> String {
    // Simple pluralization rules
    if input.ends_with('y') && !input.ends_with("ay") && !input.ends_with("ey") && !input.ends_with("iy") && !input.ends_with("oy") && !input.ends_with("uy") {
        format!("{}ies", &input[..input.len()-1])
    } else if input.ends_with('s') || input.ends_with("sh") || input.ends_with("ch") || input.ends_with('x') || input.ends_with('z') {
        format!("{}es", input)
    } else if input.ends_with("fe") {
        format!("{}ves", &input[..input.len()-2])
    } else if input.ends_with('f') {
        format!("{}ves", &input[..input.len()-1])
    } else {
        format!("{}s", input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_snake_case() {
        assert_eq!(to_snake_case("UserAccount"), "user_account");
        assert_eq!(to_snake_case("XMLHttpRequest"), "x_m_l_http_request");
        assert_eq!(to_snake_case("already_snake"), "already_snake");
    }
    
    #[test]
    fn test_camel_case() {
        assert_eq!(to_camel_case("user_account"), "userAccount");
        assert_eq!(to_camel_case("xml-http-request"), "xmlHttpRequest");
        assert_eq!(to_camel_case("already camelCase"), "alreadyCamelcase");
    }
    
    #[test]
    fn test_pascal_case() {
        assert_eq!(to_pascal_case("user_account"), "UserAccount");
        assert_eq!(to_pascal_case("xml-http-request"), "XmlHttpRequest");
    }
    
    #[test]
    fn test_pluralize() {
        assert_eq!(pluralize("user"), "users");
        assert_eq!(pluralize("category"), "categories");
        assert_eq!(pluralize("box"), "boxes");
        assert_eq!(pluralize("knife"), "knives");
        assert_eq!(pluralize("leaf"), "leaves");
    }
}