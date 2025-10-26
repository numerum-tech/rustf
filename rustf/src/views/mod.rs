use crate::config::{AppConfig, ViewConfig};
use crate::error::Result;
use serde_json::Value;
use std::sync::Arc;

pub mod api;
pub mod totaljs; // Total.js is the default built-in template engine // Global VIEW API for inline template rendering

/// Trait for view engine implementations
pub trait ViewEngineImpl: Send + Sync {
    fn render(&self, template: &str, data: &Value, layout: Option<&str>) -> Result<String>;
    fn set_directory(&mut self, dir: &str);
}

/// Main ViewEngine that delegates to different implementations
pub struct ViewEngine {
    engine: Box<dyn ViewEngineImpl>,
}

// Re-export the Total.js engine (always available as default)
pub use totaljs::TotalJsEngine;

// Re-export the global VIEW API
pub use api::VIEW;

impl Default for ViewEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ViewEngine {
    /// Create a new ViewEngine with default directory
    pub fn new() -> Self {
        Self::from_directory("views")
    }

    /// Create a new ViewEngine with specified directory
    /// Defaults to Total.js engine with filesystem storage
    pub fn from_directory(directory: &str) -> Self {
        // Total.js is the default built-in engine
        Self::totaljs_filesystem(directory)
    }

    /// Create a builder for configuring a ViewEngine
    pub fn builder() -> ViewEngineBuilder {
        ViewEngineBuilder::new()
    }

    // Total.js methods (always available as built-in engine)
    pub fn totaljs_filesystem(base_dir: &str) -> Self {
        Self::totaljs_filesystem_with_config(base_dir, None)
    }

    pub fn totaljs_filesystem_with_config(
        base_dir: &str,
        view_config: Option<&ViewConfig>,
    ) -> Self {
        // Use the full Total.js engine implementation
        Self {
            engine: Box::new(totaljs::TotalJsEngine::with_config(base_dir, view_config)),
        }
    }

    pub fn totaljs_filesystem_with_app_config(base_dir: &str, app_config: Arc<AppConfig>) -> Self {
        // Use the full Total.js engine implementation with full AppConfig
        Self {
            engine: Box::new(totaljs::TotalJsEngine::with_app_config(
                base_dir, app_config,
            )),
        }
    }

    #[cfg(feature = "embedded-views")]
    pub fn totaljs_embedded() -> Self {
        // Use the embedded Total.js engine that loads templates from rust-embed
        Self {
            engine: Box::new(totaljs::EmbeddedTotalJsEngine::new()),
        }
    }

    #[cfg(feature = "embedded-views")]
    pub fn totaljs_embedded_with_config(view_config: Option<&ViewConfig>) -> Self {
        // Use the embedded Total.js engine with custom configuration
        Self {
            engine: Box::new(totaljs::EmbeddedTotalJsEngine::with_config(view_config)),
        }
    }

    #[cfg(feature = "embedded-views")]
    pub fn totaljs_embedded_with_app_config(app_config: Arc<AppConfig>) -> Self {
        // Use the embedded Total.js engine with full AppConfig
        Self {
            engine: Box::new(totaljs::EmbeddedTotalJsEngine::with_app_config(app_config)),
        }
    }

    #[cfg(not(feature = "embedded-views"))]
    pub fn totaljs_embedded() -> Self {
        panic!("Embedded views not available! Enable 'embedded-views' feature.")
    }

    #[cfg(not(feature = "embedded-views"))]
    pub fn totaljs_embedded_with_config(_view_config: Option<&ViewConfig>) -> Self {
        panic!("Embedded views not available! Enable 'embedded-views' feature.")
    }

    /// Legacy method - now defaults to Total.js embedded
    #[deprecated(note = "Use totaljs_embedded() instead")]
    pub fn embedded() -> Self {
        Self::totaljs_embedded()
    }

    pub fn set_directory(&mut self, dir: &str) {
        self.engine.set_directory(dir);
    }

    pub fn render(&self, template: &str, data: &Value, layout: Option<&str>) -> Result<String> {
        self.engine.render(template, data, layout)
    }
}

/// Builder for ViewEngine configuration
pub struct ViewEngineBuilder {
    views_path: String,
    extension: String,
}

impl Default for ViewEngineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ViewEngineBuilder {
    pub fn new() -> Self {
        Self {
            views_path: "views".to_string(),
            extension: "html".to_string(),
        }
    }

    pub fn views_path(mut self, path: &str) -> Self {
        self.views_path = path.to_string();
        self
    }

    pub fn extension(mut self, ext: &str) -> Self {
        self.extension = ext.to_string();
        self
    }

    pub fn build(self) -> ViewEngine {
        ViewEngine::totaljs_filesystem(&self.views_path)
    }
}

// FilesystemViewEngine implementation moved to filesystem.rs
