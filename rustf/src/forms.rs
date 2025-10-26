//! Form generation and validation utilities
//!
//! This module provides utilities for creating HTML forms with built-in validation,
//! CSRF protection, and error handling. It's designed to be AI-friendly with
//! clear method names and comprehensive examples.

use crate::error::{Error, Result};
use crate::security::validation::{CsrfProtection, InputValidator, ValidationRule};
use std::collections::HashMap;

/// HTML form builder with validation and CSRF protection
pub struct FormBuilder {
    action: String,
    method: String,
    fields: Vec<FormField>,
    csrf_token: Option<String>,
    errors: HashMap<String, String>,
    values: HashMap<String, String>,
}

/// Form field definition
#[derive(Clone, Debug)]
pub struct FormField {
    pub field_type: String,
    pub name: String,
    pub label: Option<String>,
    pub placeholder: Option<String>,
    pub required: bool,
    pub validation_rules: Vec<ValidationRule>,
    pub attributes: HashMap<String, String>,
}

impl FormBuilder {
    /// Create a new form builder
    ///
    /// # Arguments
    /// * `action` - Form action URL
    /// * `method` - HTTP method (GET, POST, etc.)
    ///
    /// # Example
    /// ```rust,ignore
    /// let form = FormBuilder::new("/login", "POST");
    /// ```
    pub fn new(action: &str, method: &str) -> Self {
        Self {
            action: action.to_string(),
            method: method.to_uppercase(),
            fields: Vec::new(),
            csrf_token: None,
            errors: HashMap::new(),
            values: HashMap::new(),
        }
    }

    /// Add CSRF token to the form
    pub fn with_csrf_token(mut self, token: String) -> Self {
        self.csrf_token = Some(token);
        self
    }

    /// Set form errors (typically from previous validation)
    pub fn with_errors(mut self, errors: HashMap<String, String>) -> Self {
        self.errors = errors;
        self
    }

    /// Set form values (typically from previous submission)
    pub fn with_values(mut self, values: HashMap<String, String>) -> Self {
        self.values = values;
        self
    }

    /// Add a text input field
    pub fn text(self, name: &str) -> FormFieldBuilder {
        FormFieldBuilder::new("text", name, self)
    }

    /// Add an email input field
    pub fn email(self, name: &str) -> FormFieldBuilder {
        FormFieldBuilder::new("email", name, self)
    }

    /// Add a password input field
    pub fn password(self, name: &str) -> FormFieldBuilder {
        FormFieldBuilder::new("password", name, self)
    }

    /// Add a textarea field
    pub fn textarea(self, name: &str) -> FormFieldBuilder {
        FormFieldBuilder::new("textarea", name, self)
    }

    /// Add a select field
    pub fn select(self, name: &str) -> FormFieldBuilder {
        FormFieldBuilder::new("select", name, self)
    }

    /// Add a checkbox field
    pub fn checkbox(self, name: &str) -> FormFieldBuilder {
        FormFieldBuilder::new("checkbox", name, self)
    }

    /// Generate the complete HTML form
    pub fn render(&self) -> String {
        let mut html = format!(
            r#"<form action="{}" method="{}">"#,
            self.action, self.method
        );

        // Add CSRF token if present
        if let Some(token) = &self.csrf_token {
            html.push_str(&format!(
                r#"<input type="hidden" name="_csrf_token" value="{}" />"#,
                html_escape(token)
            ));
        }

        // Render each field
        for field in &self.fields {
            html.push_str(&self.render_field(field));
        }

        html.push_str("</form>");
        html
    }

    /// Render a single form field
    fn render_field(&self, field: &FormField) -> String {
        let mut html = String::new();

        // Field wrapper
        html.push_str(r#"<div class="form-field">"#);

        // Label
        if let Some(label) = &field.label {
            html.push_str(&format!(
                r#"<label for="{}">{}{}</label>"#,
                field.name,
                html_escape(label),
                if field.required { " *" } else { "" }
            ));
        }

        // Input field
        let empty_string = String::new();
        let value = self.values.get(&field.name).unwrap_or(&empty_string);
        let error_class = if self.errors.contains_key(&field.name) {
            " error"
        } else {
            ""
        };

        match field.field_type.as_str() {
            "textarea" => {
                html.push_str(&format!(
                    r#"<textarea name="{}" id="{}" class="form-control{}"{}{}>{}</textarea>"#,
                    field.name,
                    field.name,
                    error_class,
                    if field.required { " required" } else { "" },
                    field
                        .placeholder
                        .as_ref()
                        .map(|p| format!(r#" placeholder="{}""#, html_escape(p)))
                        .unwrap_or_default(),
                    html_escape(value)
                ));
            }
            "select" => {
                html.push_str(&format!(
                    r#"<select name="{}" id="{}" class="form-control{}"{}>"#,
                    field.name,
                    field.name,
                    error_class,
                    if field.required { " required" } else { "" }
                ));
                // Note: Options would be added via attributes or separate method
                html.push_str("</select>");
            }
            "checkbox" => {
                let checked = value == "true" || value == "1" || value == "on";
                html.push_str(&format!(
                    r#"<input type="{}" name="{}" id="{}" value="1" class="form-control{}"{}{}>"#,
                    field.field_type,
                    field.name,
                    field.name,
                    error_class,
                    if field.required { " required" } else { "" },
                    if checked { " checked" } else { "" }
                ));
            }
            _ => {
                // text, email, password, etc.
                html.push_str(&format!(
                    r#"<input type="{}" name="{}" id="{}" value="{}" class="form-control{}"{}{}>"#,
                    field.field_type,
                    field.name,
                    field.name,
                    html_escape(value),
                    error_class,
                    if field.required { " required" } else { "" },
                    field
                        .placeholder
                        .as_ref()
                        .map(|p| format!(r#" placeholder="{}""#, html_escape(p)))
                        .unwrap_or_default()
                ));
            }
        }

        // Error message
        if let Some(error) = self.errors.get(&field.name) {
            html.push_str(&format!(
                r#"<div class="form-error">{}</div>"#,
                html_escape(error)
            ));
        }

        html.push_str("</div>");
        html
    }

    /// Create a validator for this form
    pub fn validator(&self) -> InputValidator {
        let mut validator = InputValidator::new();

        for field in &self.fields {
            for rule in &field.validation_rules {
                validator = validator.add_rule(rule.clone());
            }
        }

        validator
    }
}

/// Builder for individual form fields
pub struct FormFieldBuilder {
    field: FormField,
    form: FormBuilder,
}

impl FormFieldBuilder {
    fn new(field_type: &str, name: &str, form: FormBuilder) -> Self {
        Self {
            field: FormField {
                field_type: field_type.to_string(),
                name: name.to_string(),
                label: None,
                placeholder: None,
                required: false,
                validation_rules: Vec::new(),
                attributes: HashMap::new(),
            },
            form,
        }
    }

    /// Set field label
    pub fn label(mut self, label: &str) -> Self {
        self.field.label = Some(label.to_string());
        self
    }

    /// Set field placeholder
    pub fn placeholder(mut self, placeholder: &str) -> Self {
        self.field.placeholder = Some(placeholder.to_string());
        self
    }

    /// Mark field as required
    pub fn required(mut self) -> Self {
        self.field.required = true;
        // Add required validation rule
        self.field
            .validation_rules
            .push(ValidationRule::new(&self.field.name).required());
        self
    }

    /// Set minimum length validation
    pub fn min_length(mut self, min: usize) -> Self {
        self.field
            .validation_rules
            .push(ValidationRule::new(&self.field.name).min_length(min));
        self
    }

    /// Set maximum length validation
    pub fn max_length(mut self, max: usize) -> Self {
        self.field
            .validation_rules
            .push(ValidationRule::new(&self.field.name).max_length(max));
        self
    }

    /// Add email validation (for email fields)
    pub fn email_validation(mut self) -> Self {
        self.field.validation_rules.push(ValidationRule::email());
        self
    }

    /// Add password validation (for password fields)
    pub fn password_validation(mut self) -> Self {
        self.field.validation_rules.push(ValidationRule::password());
        self
    }

    /// Add custom validation rule
    pub fn validate_with(mut self, rule: ValidationRule) -> Self {
        self.field.validation_rules.push(rule);
        self
    }

    /// Add HTML attribute
    pub fn attribute(mut self, name: &str, value: &str) -> Self {
        self.field
            .attributes
            .insert(name.to_string(), value.to_string());
        self
    }

    /// Finish building this field and return to form builder
    pub fn end(self) -> FormBuilder {
        let mut form = self.form;
        form.fields.push(self.field);
        form
    }
}

/// Form validation and processing helper
pub struct FormProcessor {
    csrf_protection: Option<CsrfProtection>,
}

impl Default for FormProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl FormProcessor {
    /// Create new form processor
    pub fn new() -> Self {
        Self {
            csrf_protection: None,
        }
    }

    /// Enable CSRF protection
    pub fn with_csrf_protection(mut self, secret_key: &str) -> Self {
        self.csrf_protection = Some(CsrfProtection::new(secret_key));
        self
    }

    /// Process form submission with validation
    pub fn process_form(
        &self,
        form_data: &HashMap<String, String>,
        validator: &InputValidator,
        session_id: Option<&str>,
    ) -> Result<HashMap<String, String>> {
        // CSRF validation if enabled
        if let Some(csrf) = &self.csrf_protection {
            if let Some(session_id) = session_id {
                let token = form_data
                    .get("_csrf_token")
                    .ok_or_else(|| Error::template("CSRF token missing".to_string()))?;

                if !csrf.validate_token(token, session_id) {
                    return Err(Error::template("Invalid CSRF token".to_string()));
                }
            }
        }

        // Input validation
        validator.validate(form_data)
    }

    /// Generate CSRF token for session
    pub fn generate_csrf_token(&self, session_id: &str) -> Option<String> {
        self.csrf_protection
            .as_ref()
            .map(|csrf| csrf.generate_token(session_id))
    }
}

/// Utility function to escape HTML content
fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// Built-in form templates for common use cases
pub struct FormTemplates;

impl FormTemplates {
    /// Generate a login form
    pub fn login_form(action: &str, csrf_token: Option<String>) -> FormBuilder {
        let mut form = FormBuilder::new(action, "POST");

        if let Some(token) = csrf_token {
            form = form.with_csrf_token(token);
        }

        form.email("email")
            .label("Email Address")
            .placeholder("Enter your email")
            .required()
            .email_validation()
            .end()
            .password("password")
            .label("Password")
            .placeholder("Enter your password")
            .required()
            .min_length(8)
            .end()
    }

    /// Generate a registration form
    pub fn registration_form(action: &str, csrf_token: Option<String>) -> FormBuilder {
        let mut form = FormBuilder::new(action, "POST");

        if let Some(token) = csrf_token {
            form = form.with_csrf_token(token);
        }

        form.text("username")
            .label("Username")
            .placeholder("Choose a username")
            .required()
            .min_length(3)
            .max_length(32)
            .validate_with(ValidationRule::username())
            .end()
            .email("email")
            .label("Email Address")
            .placeholder("Enter your email")
            .required()
            .email_validation()
            .end()
            .password("password")
            .label("Password")
            .placeholder("Create a password")
            .required()
            .password_validation()
            .end()
            .password("password_confirmation")
            .label("Confirm Password")
            .placeholder("Confirm your password")
            .required()
            .end()
    }

    /// Generate a contact form
    pub fn contact_form(action: &str, csrf_token: Option<String>) -> FormBuilder {
        let mut form = FormBuilder::new(action, "POST");

        if let Some(token) = csrf_token {
            form = form.with_csrf_token(token);
        }

        form.text("name")
            .label("Full Name")
            .placeholder("Enter your full name")
            .required()
            .min_length(2)
            .max_length(100)
            .end()
            .email("email")
            .label("Email Address")
            .placeholder("Enter your email")
            .required()
            .email_validation()
            .end()
            .text("subject")
            .label("Subject")
            .placeholder("What is this about?")
            .required()
            .min_length(5)
            .max_length(200)
            .end()
            .textarea("message")
            .label("Message")
            .placeholder("Enter your message...")
            .required()
            .min_length(20)
            .max_length(5000)
            .end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_form_builder_basic() {
        let form = FormBuilder::new("/test", "POST")
            .text("name")
            .label("Name")
            .required()
            .end();

        let html = form.render();
        assert!(html.contains(r#"action="/test""#));
        assert!(html.contains(r#"method="POST""#));
        assert!(html.contains(r#"name="name""#));
        assert!(html.contains("required"));
    }

    #[test]
    fn test_csrf_token_inclusion() {
        let form = FormBuilder::new("/test", "POST")
            .with_csrf_token("test_token_123".to_string())
            .text("field")
            .end();

        let html = form.render();
        assert!(html.contains(r#"name="_csrf_token""#));
        assert!(html.contains(r#"value="test_token_123""#));
    }

    #[test]
    fn test_form_with_errors() {
        let mut errors = HashMap::new();
        errors.insert("email".to_string(), "Invalid email format".to_string());

        let form = FormBuilder::new("/test", "POST")
            .with_errors(errors)
            .email("email")
            .label("Email")
            .end();

        let html = form.render();
        assert!(html.contains("Invalid email format"));
        assert!(html.contains("error"));
    }

    #[test]
    fn test_form_processor_csrf() {
        let processor = FormProcessor::new().with_csrf_protection("test_secret");

        let token = processor.generate_csrf_token("session_123");
        assert!(token.is_some());

        let mut form_data = HashMap::new();
        form_data.insert("_csrf_token".to_string(), token.unwrap());
        form_data.insert("name".to_string(), "Test".to_string());

        let validator = InputValidator::new().add_rule(ValidationRule::new("name").required());

        let result = processor.process_form(&form_data, &validator, Some("session_123"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_login_form_template() {
        let form = FormTemplates::login_form("/login", Some("token123".to_string()));
        let html = form.render();

        assert!(html.contains(r#"name="email""#));
        assert!(html.contains(r#"name="password""#));
        assert!(html.contains(r#"type="email""#));
        assert!(html.contains(r#"type="password""#));
        assert!(html.contains("token123"));
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(
            html_escape("<script>alert('xss')</script>"),
            "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;"
        );
        assert_eq!(html_escape("Normal text"), "Normal text");
        assert_eq!(html_escape("\"quoted\""), "&quot;quoted&quot;");
    }
}
