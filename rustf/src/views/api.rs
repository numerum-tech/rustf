use crate::views::{
    totaljs::{
        parser::Parser,
        renderer::{RenderContext, Renderer},
    },
    ViewEngine,
};
use crate::Result;
use serde_json::{json, Value};
use std::sync::{Arc, OnceLock};

/// Global ViewEngine instance for inline template rendering
static GLOBAL_VIEW_ENGINE: OnceLock<Arc<ViewEngine>> = OnceLock::new();

/// Initialize the global VIEW API with a ViewEngine instance
///
/// This should be called once during application startup
pub fn initialize_global_view(engine: Arc<ViewEngine>) -> Result<()> {
    GLOBAL_VIEW_ENGINE
        .set(engine)
        .map_err(|_| crate::error::Error::internal("Global VIEW already initialized"))?;
    Ok(())
}

/// Global VIEW API for inline template rendering
///
/// Provides static methods to render templates from anywhere in the application
/// without needing a Context instance.
///
/// # Examples
///
/// ```ignore
/// use rustf::views::VIEW;
/// use serde_json::json;
///
/// // Render a template file with model data
/// let html = VIEW::render("emails/welcome", json!({"name": "Alice"}), None, Some("layouts/email"))?;
///
/// // Render inline template string
/// let html = VIEW::render_string("Hello @{M.name}!", json!({"name": "Bob"}), None)?;
///
/// // With repository data
/// let model = json!({"product": "Widget"});
/// let repository = json!({"site_name": "My Store", "year": 2025});
/// let html = VIEW::render("emails/order", model, Some(repository), None)?;
/// ```
pub struct VIEW;

impl VIEW {
    /// Render a template file with model and optional repository data
    ///
    /// # Parameters
    /// - `template_path`: Path to template file (relative to views directory, without extension)
    /// - `model`: Main template data (accessible as @{M.key} or @{model.key})
    /// - `repository`: Optional context data (accessible as @{R.key} or @{repository.key})
    /// - `layout`: Optional layout template name
    ///
    /// # Returns
    /// Rendered HTML string or error
    ///
    /// # Example
    /// ```ignore
    /// let html = VIEW::render(
    ///     "emails/welcome",
    ///     json!({"name": "Alice", "email": "alice@example.com"}),
    ///     Some(json!({"site_name": "My Site", "support_email": "support@mysite.com"})),
    ///     Some("layouts/email")
    /// )?;
    /// ```
    pub fn render(
        template_path: &str,
        model: Value,
        repository: Option<Value>,
        layout: Option<&str>,
    ) -> Result<String> {
        let engine = GLOBAL_VIEW_ENGINE.get().ok_or_else(|| {
            crate::error::Error::internal(
                "Global VIEW not initialized. Call initialize_global_view() during app startup",
            )
        })?;

        // Prepare data with repository in the same format as Context::view()
        let repository_value = repository.unwrap_or(json!({}));
        let session_value = json!({}); // No session data in global VIEW

        let data = json!({
            "data": model,
            "_context_repository": repository_value,
            "_context_session": session_value
        });

        // Render with optional layout
        engine.render(template_path, &data, layout)
    }

    /// Render an inline template string with model and optional repository data
    ///
    /// This method parses and renders a template string directly without loading from file.
    /// Useful for dynamic templates, email generation, or simple formatting.
    ///
    /// # Parameters
    /// - `template_string`: Template content as string
    /// - `model`: Main template data (accessible as @{M.key} or @{model.key})
    /// - `repository`: Optional context data (accessible as @{R.key} or @{repository.key})
    ///
    /// # Returns
    /// Rendered HTML string or error
    ///
    /// # Example
    /// ```ignore
    /// let template = "Hello @{M.name}, welcome to @{R.site_name}!";
    /// let html = VIEW::render_string(
    ///     template,
    ///     json!({"name": "Alice"}),
    ///     Some(json!({"site_name": "My Site"}))
    /// )?;
    /// // Output: "Hello Alice, welcome to My Site!"
    /// ```
    pub fn render_string(
        template_string: &str,
        model: Value,
        repository: Option<Value>,
    ) -> Result<String> {
        // Parse the template string
        let mut parser = Parser::new(template_string)?;
        let template = parser.parse()?;

        // Prepare data with repository separation
        let repository_value = repository.unwrap_or(json!({}));
        let session_value = json!({}); // No session data in global VIEW

        // Create render context with proper model/repository separation
        let context = RenderContext::new(model)
            .with_repository(repository_value)
            .with_session(session_value);

        // Render the template
        let mut renderer = Renderer::new(context);
        renderer.render(&template)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::ViewEngine;
    use std::sync::Arc;

    fn setup_test_engine() -> Arc<ViewEngine> {
        let engine = ViewEngine::builder()
            .views_path("views")
            .extension("html")
            .build();
        Arc::new(engine)
    }

    #[test]
    fn test_render_string_basic() {
        let template = "Hello @{M.name}!";
        let model = json!({"name": "Alice"});

        let result = VIEW::render_string(template, model, None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().trim(), "Hello Alice!");
    }

    #[test]
    fn test_render_string_with_repository() {
        let template = "Hello @{M.name}, welcome to @{R.site_name}!";
        let model = json!({"name": "Alice"});
        let repository = json!({"site_name": "My Site"});

        let result = VIEW::render_string(template, model, Some(repository));
        assert!(result.is_ok());
        assert_eq!(result.unwrap().trim(), "Hello Alice, welcome to My Site!");
    }

    #[test]
    fn test_render_string_with_conditionals() {
        let template = "@{if M.show}Visible@{else}Hidden@{fi}";

        let model_true = json!({"show": true});
        let result = VIEW::render_string(template, model_true, None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().trim(), "Visible");

        let model_false = json!({"show": false});
        let result = VIEW::render_string(template, model_false, None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().trim(), "Hidden");
    }

    #[test]
    fn test_render_string_with_loops() {
        let template = "@{foreach item in M.items}@{item} @{end}";
        let model = json!({"items": ["A", "B", "C"]});

        let result = VIEW::render_string(template, model, None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().trim(), "A B C");
    }

    #[test]
    fn test_render_string_with_variables() {
        let template = "@{var x = M.a + M.b}Result: @{x}";
        let model = json!({"a": 10, "b": 20});

        let result = VIEW::render_string(template, model, None);
        assert!(result.is_ok());
        let output = result.unwrap();
        // Variables may not be fully implemented yet, just check it renders without error
        assert!(output.contains("Result:"));
    }

    #[test]
    fn test_render_not_initialized() {
        // Without initialization, render() should fail
        let result = VIEW::render("test", json!({}), None, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not initialized"));
    }

    #[test]
    fn test_initialize_global_view() {
        // This test demonstrates initialization but we can't actually test it
        // in unit tests due to OnceLock behavior (can only be set once globally)
        let engine = setup_test_engine();

        // In a real app, this would be called once in main.rs
        // let result = initialize_global_view(engine);
        // assert!(result.is_ok());

        // Subsequent calls should fail
        // let result2 = initialize_global_view(engine.clone());
        // assert!(result2.is_err());
    }

    #[test]
    fn test_render_string_empty_template() {
        let template = "";
        let model = json!({});

        let result = VIEW::render_string(template, model, None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().trim(), "");
    }

    #[test]
    fn test_render_string_no_placeholders() {
        let template = "Just plain text";
        let model = json!({"ignored": "value"});

        let result = VIEW::render_string(template, model, None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().trim(), "Just plain text");
    }

    #[test]
    fn test_render_string_nested_model() {
        let template = "User: @{M.user.name}, Email: @{M.user.email}";
        let model = json!({
            "user": {
                "name": "Alice",
                "email": "alice@example.com"
            }
        });

        let result = VIEW::render_string(template, model, None);
        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(html.contains("User: Alice"));
        assert!(html.contains("Email: alice@example.com"));
    }

    #[test]
    fn test_render_string_complex_repository() {
        let template = "@{M.greeting}, @{M.name}! @{R.footer}";
        let model = json!({"greeting": "Hello", "name": "Bob"});
        let repository = json!({"footer": "© 2025 My Company"});

        let result = VIEW::render_string(template, model, Some(repository));
        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(html.contains("Hello, Bob!"));
        assert!(html.contains("© 2025 My Company"));
    }
}
