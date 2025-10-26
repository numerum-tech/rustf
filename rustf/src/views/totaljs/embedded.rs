use super::{
    ast::Template,
    parser::Parser,
    renderer::{RenderContext, Renderer, TemplateLoader},
    translation::TranslationSystem,
};
use crate::config::{AppConfig, ViewConfig};
use crate::error::{Error, Result};
use crate::repository::APP;
use crate::views::ViewEngineImpl;
use rust_embed::RustEmbed;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// Embedded templates using rust-embed
///
/// This embeds all templates from the views directory at compile time.
/// You can customize the folder path by changing the folder attribute.
///
/// Note: This path is relative to the crate root where this is compiled.
/// Applications using RustF should create their own embedded templates struct
/// with their specific views folder path.
#[derive(RustEmbed)]
#[folder = "views/"]
struct EmbeddedTemplates;

/// Cache entry for compiled templates
#[derive(Clone)]
struct CacheEntry {
    template: Template,
    _compiled_at: u64,
    #[allow(dead_code)]
    file_hash: Option<String>, // For development hot reload
}

/// Template cache for performance
#[derive(Clone)]
struct TemplateCache {
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    enable_hot_reload: bool,
}

impl TemplateCache {
    fn new(enable_hot_reload: bool) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            enable_hot_reload,
        }
    }

    fn get_or_compile(&self, path: &str, content: &str, hash: Option<String>) -> Result<Template> {
        // Check cache
        if let Ok(cache) = self.cache.read() {
            if let Some(entry) = cache.get(path) {
                // In production mode, always use cached version
                if !self.enable_hot_reload {
                    return Ok(entry.template.clone());
                }

                // In development mode, check if file has changed
                if let Some(current_hash) = &hash {
                    if let Some(cached_hash) = &entry.file_hash {
                        if current_hash == cached_hash {
                            return Ok(entry.template.clone());
                        }
                    }
                }
            }
        }

        // Compile template
        let mut parser = Parser::new(content)?;
        let template = parser.parse()?;

        // Update cache
        if let Ok(mut cache) = self.cache.write() {
            let entry = CacheEntry {
                template: template.clone(),
                _compiled_at: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                file_hash: hash,
            };
            cache.insert(path.to_string(), entry);
        }

        Ok(template)
    }

    fn clear(&self) {
        if let Ok(mut cache) = self.cache.write() {
            cache.clear();
        }
    }
}

/// Total.js template engine with embedded templates
pub struct EmbeddedTotalJsEngine {
    cache: TemplateCache,
    /// Global configuration (legacy - for backward compatibility)
    config: Arc<RwLock<HashMap<String, String>>>,
    /// Full application configuration for CONF global
    app_config: Option<Arc<AppConfig>>,
    /// Translation system
    translator: Arc<RwLock<Option<TranslationSystem>>>,
}

impl EmbeddedTotalJsEngine {
    /// Create a new embedded Total.js engine
    pub fn new() -> Self {
        Self::with_config(None)
    }

    /// Create with optional view configuration
    pub fn with_config(view_config: Option<&ViewConfig>) -> Self {
        // In development, enable hot reload by default; in production, use cache
        let enable_hot_reload = if let Some(config) = view_config {
            // If cache is disabled, enable hot reload
            !config.cache_enabled
        } else {
            cfg!(debug_assertions)
        };

        Self {
            cache: TemplateCache::new(enable_hot_reload),
            config: Arc::new(RwLock::new(HashMap::new())),
            app_config: None,
            translator: Arc::new(RwLock::new(None)),
        }
    }

    /// Create a new embedded Total.js engine with full application configuration
    pub fn with_app_config(app_config: Arc<AppConfig>) -> Self {
        let enable_hot_reload = !app_config.views.cache_enabled;

        let mut config = HashMap::new();
        // Extract view-specific settings for backward compatibility
        config.insert(
            "default_root".to_string(),
            app_config.views.default_root.clone(),
        );
        config.insert(
            "default_layout".to_string(),
            app_config.views.default_layout.clone(),
        );
        config.insert(
            "cache_enabled".to_string(),
            app_config.views.cache_enabled.to_string(),
        );

        Self {
            cache: TemplateCache::new(enable_hot_reload),
            config: Arc::new(RwLock::new(config)),
            app_config: Some(app_config),
            translator: Arc::new(RwLock::new(None)),
        }
    }

    /// Load embedded template content
    fn load_embedded_template(&self, path: &str) -> Result<(String, Option<String>)> {
        // Normalize path - remove leading slash if present
        let normalized_path = if path.starts_with('/') {
            &path[1..]
        } else {
            path
        };

        // Try to load the embedded file
        match EmbeddedTemplates::get(normalized_path) {
            Some(file) => {
                // Get content as string
                let content = std::str::from_utf8(&file.data)
                    .map_err(|e| Error::template(format!("Invalid UTF-8 in template {}: {}", path, e)))?
                    .to_string();

                // Calculate hash for development hot reload
                let hash = if self.cache.enable_hot_reload {
                    use std::collections::hash_map::DefaultHasher;
                    use std::hash::{Hash, Hasher};
                    let mut hasher = DefaultHasher::new();
                    file.data.hash(&mut hasher);
                    Some(format!("{:x}", hasher.finish()))
                } else {
                    None
                };

                Ok((content, hash))
            }
            None => Err(Error::template(format!(
                "Embedded template not found: {}. Make sure the template exists in the views directory at compile time.",
                path
            ))),
        }
    }

    /// Load and compile a template
    fn load_template(&self, path: &str) -> Result<Template> {
        let (content, hash) = self.load_embedded_template(path)?;
        self.cache.get_or_compile(path, &content, hash)
    }

    /// Resolve template path
    fn template_path(&self, template: &str) -> String {
        // Simply remove leading slash if present for consistency
        // Both '/someview' and 'someview' should work the same way
        let template_clean = template.strip_prefix('/').unwrap_or(template);

        if template_clean.ends_with(".html") {
            template_clean.to_string()
        } else {
            format!("{}.html", template_clean)
        }
    }

    /// Resolve layout path
    fn layout_path(&self, layout: &str) -> String {
        if layout.starts_with("layouts/") || layout.starts_with("/layouts/") {
            self.template_path(layout)
        } else {
            format!("layouts/{}.html", layout)
        }
    }

    /// Create a render context with common data
    fn create_context(
        &self,
        data: &Value,
        context_repository: Option<&Value>,
        session_data: Option<&Value>,
    ) -> RenderContext {
        let mut context = RenderContext::new(data.clone());

        // Add global repository (APP/MAIN)
        if let Some(repo) = APP::get_repository() {
            if let Ok(data) = repo.read() {
                context = context.with_global_repository(data.clone());
            }
        }

        // Add context repository if provided (repository/R)
        if let Some(ctx_repo) = context_repository {
            context = context.with_repository(ctx_repo.clone());
        }

        // Add global config
        if let Ok(cfg) = self.config.read() {
            context = context.with_config(cfg.clone());
        }

        // Set CONF global for templates
        if let Some(app_config) = &self.app_config {
            // Use full AppConfig if available - serialize it to JSON for full access
            if let Ok(conf_json) = serde_json::to_value(app_config.as_ref()) {
                context = context.with_conf(conf_json);
            }
        } else {
            // Fallback to legacy behavior if no AppConfig
            if let Ok(cfg) = self.config.read() {
                let mut conf_obj = serde_json::Map::new();
                for (key, value) in cfg.iter() {
                    conf_obj.insert(key.clone(), Value::String(value.clone()));
                }
                context = context.with_conf(Value::Object(conf_obj));
            }
        }

        // Add translator if available
        if let Ok(trans) = self.translator.read() {
            if let Some(translator) = trans.as_ref() {
                context = context.with_translator(translator.clone());
            }
        }

        // Add session data if provided, otherwise use empty session
        let session_value = session_data
            .cloned()
            .unwrap_or_else(|| Value::Object(serde_json::Map::new()));

        // Add default values for query and user (would be populated from request context)
        context = context
            .with_session(session_value)
            .with_query(Value::Object(serde_json::Map::new()))
            .with_user(Value::Null);

        context
    }

    /// Render a template with layout, context repository, and session data
    pub fn render_with_layout_and_session(
        &self,
        template: &str,
        data: &Value,
        layout: Option<&str>,
        context_repository: Option<&Value>,
        session_data: Option<&Value>,
    ) -> Result<String> {
        let template_path = self.template_path(template);
        let template_ast = self.load_template(&template_path)?;

        // Create render context with session data
        let context = self.create_context(data, context_repository, session_data);

        // Create template loader that uses embedded templates
        let cache = self.cache.clone();
        let loader: TemplateLoader = Box::new(move |name: &str| {
            // Handle different path types (consistent with filesystem engine)
            let clean_name = name.strip_prefix('/').unwrap_or(name);

            // Resolve partial path
            let path = if clean_name.ends_with(".html") {
                clean_name.to_string()
            } else {
                format!("{}.html", clean_name)
            };

            // Load from embedded templates
            match EmbeddedTemplates::get(&path) {
                Some(file) => {
                    let content = std::str::from_utf8(&file.data).map_err(|e| {
                        Error::template(format!("Invalid UTF-8 in partial {}: {}", name, e))
                    })?;

                    // Calculate hash for cache
                    let hash = if cache.enable_hot_reload {
                        use std::collections::hash_map::DefaultHasher;
                        use std::hash::{Hash, Hasher};
                        let mut hasher = DefaultHasher::new();
                        file.data.hash(&mut hasher);
                        Some(format!("{:x}", hasher.finish()))
                    } else {
                        None
                    };

                    cache.get_or_compile(&path, content, hash)
                }
                None => Err(Error::template(format!(
                    "Embedded partial not found: {}",
                    name
                ))),
            }
        });

        // Render the template with template loader for partials
        let mut renderer = Renderer::new(context)
            .with_template_path("embedded://views".to_string())
            .with_template_loader(Arc::new(loader));
        let content = renderer.render(&template_ast)?;

        // Apply layout if specified
        if let Some(layout_name) = layout {
            let layout_path = self.layout_path(layout_name);
            let layout_ast = self.load_template(&layout_path)?;

            // Create new context with content
            let mut layout_data = data.clone();
            if let Value::Object(ref mut map) = layout_data {
                map.insert("content".to_string(), Value::String(content));
            } else {
                let mut map = serde_json::Map::new();
                map.insert("content".to_string(), Value::String(content));
                layout_data = Value::Object(map);
            }

            let mut layout_context =
                self.create_context(&layout_data, context_repository, session_data);

            // Transfer child template sections to layout context
            // This allows child views to define sections that parent layouts can render
            layout_context = layout_context.with_sections(template_ast.sections.clone());

            // Create template loader for layout
            let cache = self.cache.clone();
            let loader: TemplateLoader = Box::new(move |name: &str| {
                // Handle different path types (consistent with filesystem engine)
                let clean_name = name.strip_prefix('/').unwrap_or(name);

                let path = if clean_name.ends_with(".html") {
                    clean_name.to_string()
                } else {
                    format!("{}.html", clean_name)
                };

                match EmbeddedTemplates::get(&path) {
                    Some(file) => {
                        let content = std::str::from_utf8(&file.data).map_err(|e| {
                            Error::template(format!(
                                "Invalid UTF-8 in layout partial {}: {}",
                                name, e
                            ))
                        })?;

                        let hash = if cache.enable_hot_reload {
                            use std::collections::hash_map::DefaultHasher;
                            use std::hash::{Hash, Hasher};
                            let mut hasher = DefaultHasher::new();
                            file.data.hash(&mut hasher);
                            Some(format!("{:x}", hasher.finish()))
                        } else {
                            None
                        };

                        cache.get_or_compile(&path, content, hash)
                    }
                    None => Err(Error::template(format!(
                        "Embedded layout partial not found: {}",
                        name
                    ))),
                }
            });

            let mut layout_renderer = Renderer::new(layout_context)
                .with_template_path("embedded://views".to_string())
                .with_template_loader(Arc::new(loader));
            layout_renderer.render(&layout_ast)
        } else {
            Ok(content)
        }
    }

    /// Set global configuration value
    pub fn set_config(&self, key: &str, value: &str) {
        if let Ok(mut config) = self.config.write() {
            config.insert(key.to_string(), value.to_string());
        }
    }

    /// Set translation system
    pub fn set_translator(&self, translator: TranslationSystem) {
        if let Ok(mut trans) = self.translator.write() {
            *trans = Some(translator);
        }
    }

    /// Clear template cache
    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, bool) {
        let count = self.cache.cache.read().map(|c| c.len()).unwrap_or(0);
        (count, self.cache.enable_hot_reload)
    }
}

impl Default for EmbeddedTotalJsEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ViewEngineImpl for EmbeddedTotalJsEngine {
    fn set_directory(&mut self, _dir: &str) {
        // Embedded views don't support changing directory at runtime
        log::warn!(
            "Cannot change directory for embedded views - templates are compiled into the binary"
        );
    }

    fn render(&self, template: &str, data: &Value, layout: Option<&str>) -> Result<String> {
        // Extract session data if provided in the data object
        let session_data = if let Value::Object(map) = data {
            map.get("_context_session")
        } else {
            None
        };

        // Extract context repository if provided
        let context_repository = if let Value::Object(map) = data {
            map.get("_context_repository")
        } else {
            None
        };

        self.render_with_layout_and_session(
            template,
            data,
            layout,
            context_repository,
            session_data,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_engine_creation() {
        let engine = EmbeddedTotalJsEngine::new();
        let (count, hot_reload) = engine.cache_stats();
        assert_eq!(count, 0);
        assert_eq!(hot_reload, cfg!(debug_assertions));
    }

    #[test]
    fn test_template_path_resolution() {
        let engine = EmbeddedTotalJsEngine::new();

        assert_eq!(engine.template_path("index"), "index.html");
        assert_eq!(engine.template_path("/index"), "index.html");
        assert_eq!(engine.template_path("index.html"), "index.html");
        assert_eq!(engine.template_path("/index.html"), "index.html");
    }

    #[test]
    fn test_layout_path_resolution() {
        let engine = EmbeddedTotalJsEngine::new();

        assert_eq!(engine.layout_path("main"), "layouts/main.html");
        assert_eq!(engine.layout_path("layouts/main"), "layouts/main.html");
        assert_eq!(engine.layout_path("/layouts/main"), "layouts/main.html");
    }

    #[test]
    fn test_config_management() {
        let engine = EmbeddedTotalJsEngine::new();

        engine.set_config("app_name", "TestApp");
        engine.set_config("version", "1.0.0");

        // Verify config is stored
        let config = engine.config.read().unwrap();
        assert_eq!(config.get("app_name"), Some(&"TestApp".to_string()));
        assert_eq!(config.get("version"), Some(&"1.0.0".to_string()));
    }

    #[test]
    fn test_cache_clearing() {
        let engine = EmbeddedTotalJsEngine::new();

        // Clear cache should work without errors
        engine.clear_cache();

        let (count, _) = engine.cache_stats();
        assert_eq!(count, 0);
    }
}
