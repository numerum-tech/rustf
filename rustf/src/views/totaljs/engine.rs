use super::{
    ast::Template,
    parser::Parser,
    renderer::{RenderContext, Renderer, TemplateLoader},
    resource_translation::ResourceTranslationSystem,
    translation::TranslationSystem,
};
use crate::config::{AppConfig, ViewConfig};
use crate::error::{Error, Result};
use crate::repository::APP;
use crate::views::ViewEngineImpl;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// Cache entry for compiled templates
#[derive(Clone)]
struct CacheEntry {
    template: Template,
    _compiled_at: u64,
    file_modified: Option<u64>,
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

    fn get_file_mtime(path: &Path) -> Option<u64> {
        std::fs::metadata(path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
    }

    fn get_or_compile(&self, path: &Path, content: &str) -> Result<Template> {
        // If hot reload is enabled (cache disabled), always recompile
        if self.enable_hot_reload {
            let mut parser = Parser::new(content)?;
            return parser.parse();
        }

        let path_str = path.to_string_lossy().to_string();
        let current_mtime = Self::get_file_mtime(path);

        // Check cache
        if let Ok(cache) = self.cache.read() {
            if let Some(entry) = cache.get(&path_str) {
                // Use cached version if file hasn't changed
                if let (Some(cached_mtime), Some(current)) = (entry.file_modified, current_mtime) {
                    if current <= cached_mtime {
                        return Ok(entry.template.clone());
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
                file_modified: current_mtime,
            };
            cache.insert(path_str, entry);
        }

        Ok(template)
    }

    fn clear(&self) {
        if let Ok(mut cache) = self.cache.write() {
            cache.clear();
        }
    }
}

/// Total.js template engine implementation
pub struct TotalJsEngine {
    base_dir: PathBuf,
    cache: TemplateCache,
    /// Global configuration (legacy - for backward compatibility)
    config: Arc<RwLock<HashMap<String, String>>>,
    /// Full application configuration for CONF global
    app_config: Option<Arc<AppConfig>>,
    /// Translation system (legacy JSON-based)
    translator: Arc<RwLock<Option<TranslationSystem>>>,
    /// Resource translation system (new .res file based)
    resource_translator: Arc<RwLock<Option<ResourceTranslationSystem>>>,
}

impl TotalJsEngine {
    /// Create a new Total.js template engine
    pub fn new(base_dir: &str) -> Self {
        Self::with_config(base_dir, None)
    }

    /// Create a new Total.js template engine with configuration
    pub fn with_config(base_dir: &str, view_config: Option<&ViewConfig>) -> Self {
        // Determine caching behavior based on config
        let (enable_hot_reload, cache_enabled) = if let Some(vc) = view_config {
            // If cache is disabled in config, always recompile templates
            (!vc.cache_enabled, vc.cache_enabled)
        } else {
            // Default: hot reload in debug, cache in release
            (cfg!(debug_assertions), !cfg!(debug_assertions))
        };

        let mut config = HashMap::new();

        // If ViewConfig provided, extract relevant settings
        if let Some(vc) = view_config {
            config.insert("default_root".to_string(), vc.default_root.clone());
            config.insert("default_layout".to_string(), vc.default_layout.clone());
            config.insert("cache_enabled".to_string(), cache_enabled.to_string());
        }

        Self {
            base_dir: PathBuf::from(base_dir),
            cache: TemplateCache::new(enable_hot_reload),
            config: Arc::new(RwLock::new(config)),
            app_config: None,
            translator: Arc::new(RwLock::new(None)),
            resource_translator: Arc::new(RwLock::new(None)),
        }
    }

    /// Create a new Total.js template engine with full application configuration
    pub fn with_app_config(base_dir: &str, app_config: Arc<AppConfig>) -> Self {
        // Respect cache_enabled setting from config
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
            base_dir: PathBuf::from(base_dir),
            cache: TemplateCache::new(enable_hot_reload),
            config: Arc::new(RwLock::new(config)),
            app_config: Some(app_config),
            translator: Arc::new(RwLock::new(None)),
            resource_translator: Arc::new(RwLock::new(None)),
        }
    }

    /// Set global repository data (APP/MAIN)
    ///
    /// This method is deprecated. Use APP::set() directly instead.
    #[deprecated(note = "Use APP::set() directly instead")]
    pub fn set_global_repository(&self, data: Value) {
        // Initialize APP if not already initialized
        if !APP::is_initialized() {
            let _ = APP::init(data);
        } else {
            // Clear and repopulate
            let _ = APP::clear();
            if let Value::Object(map) = data {
                for (key, value) in map {
                    let _ = APP::set(&key, value);
                }
            }
        }
    }

    /// Set global configuration
    pub fn set_config(&self, config: HashMap<String, String>) {
        if let Ok(mut cfg) = self.config.write() {
            *cfg = config;
        }
    }

    /// Set translation system (legacy JSON-based)
    pub fn set_translator(&self, translator: TranslationSystem) {
        if let Ok(mut trans) = self.translator.write() {
            *trans = Some(translator);
        }
    }

    /// Set resource translation system (new .res file based)
    pub fn set_resource_translator(&self, translator: ResourceTranslationSystem) {
        if let Ok(mut trans) = self.resource_translator.write() {
            *trans = Some(translator);
        }
    }

    /// Load translations from resources directory
    pub fn load_translations(&self, resources_dir: &Path) -> Result<()> {
        let mut translator = ResourceTranslationSystem::new();
        translator
            .load_resources_dir(resources_dir)
            .map_err(|e| Error::template(format!("Failed to load translations: {}", e)))?;
        self.set_resource_translator(translator);
        Ok(())
    }

    /// Clear template cache
    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    /// Set the full application configuration
    pub fn set_app_config(&mut self, app_config: Arc<AppConfig>) {
        // Update the legacy config HashMap for backward compatibility
        if let Ok(mut cfg) = self.config.write() {
            cfg.insert(
                "default_root".to_string(),
                app_config.views.default_root.clone(),
            );
            cfg.insert(
                "default_layout".to_string(),
                app_config.views.default_layout.clone(),
            );
            cfg.insert(
                "cache_enabled".to_string(),
                app_config.views.cache_enabled.to_string(),
            );
        }
        self.app_config = Some(app_config);
    }

    /// Get the path to a template file
    fn template_path(&self, template: &str) -> PathBuf {
        let mut path = self.base_dir.clone();

        // Simply remove leading slash if present for consistency
        // Both '/someview' and 'someview' should work the same way
        let template_clean = template.strip_prefix('/').unwrap_or(template);

        // Add .html extension if not present
        if template_clean.ends_with(".html") {
            path.push(template_clean);
        } else {
            path.push(format!("{}.html", template_clean));
        }

        path
    }

    /// Get the path to a layout file
    fn layout_path(&self, layout: &str) -> PathBuf {
        let mut path = self.base_dir.clone();

        // If layout contains a path separator, use it as-is
        if layout.contains('/') {
            if layout.ends_with(".html") {
                path.push(layout);
            } else {
                path.push(format!("{}.html", layout));
            }
        } else {
            // For simple names, look in layouts/ subdirectory first
            path.push("layouts");
            if layout.ends_with(".html") {
                path.push(layout);
            } else {
                path.push(format!("{}.html", layout));
            }
        }

        path
    }

    /// Load and compile a template
    fn load_template(&self, path: &Path) -> Result<Template> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| Error::template(format!("Failed to load template {:?}: {}", path, e)))?;

        self.cache.get_or_compile(path, &content)
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

    /// Render a template with context repository data
    pub fn render_with_context(
        &self,
        template: &str,
        data: &Value,
        layout: Option<&str>,
        context_repository: &Value,
    ) -> Result<String> {
        self.render_with_layout(template, data, layout, Some(context_repository))
    }

    /// Render a template with optional layout
    pub fn render_with_layout(
        &self,
        template: &str,
        data: &Value,
        layout: Option<&str>,
        context_repository: Option<&Value>,
    ) -> Result<String> {
        self.render_with_layout_and_session(template, data, layout, context_repository, None)
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

        // Create template loader that uses the cache
        let base_dir = self.base_dir.clone();
        let cache = self.cache.clone();
        let loader: TemplateLoader = Box::new(move |name: &str| {
            let mut path = base_dir.clone();

            // Handle different path types:
            // - Paths starting with '/' are absolute from views directory
            // - Other paths are also from views directory (for now)
            // TODO: In the future, we could make non-'/' paths relative to current template
            let clean_name = name.strip_prefix('/').unwrap_or(name);

            if clean_name.ends_with(".html") {
                path.push(clean_name);
            } else {
                path.push(format!("{}.html", clean_name));
            }

            let content = std::fs::read_to_string(&path).map_err(|e| {
                crate::error::Error::template(format!("Failed to load partial '{}': {}", name, e))
            })?;

            cache.get_or_compile(&path, &content)
        });

        // Add translator to context if available (prefer resource translator over legacy)
        let context = if let Ok(trans) = self.resource_translator.read() {
            if let Some(resource_trans) = trans.as_ref() {
                // Get view-specific translations
                let view_translations = resource_trans.get_view_translations(template);
                // Convert to legacy format temporarily (TODO: update renderer to use resource translator directly)
                let mut legacy_trans = TranslationSystem::new();
                legacy_trans.add_translations("current", (*view_translations).clone());
                context.with_translator(legacy_trans)
            } else {
                context
            }
        } else if let Ok(trans) = self.translator.read() {
            if let Some(translator) = trans.as_ref() {
                context.with_translator(translator.clone())
            } else {
                context
            }
        } else {
            context
        };

        // Render the template with template loader for partials
        let mut renderer = Renderer::new(context)
            .with_template_path(self.base_dir.to_string_lossy().to_string())
            .with_template_loader(std::sync::Arc::new(loader));

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
            let base_dir = self.base_dir.clone();
            let cache = self.cache.clone();
            let loader: TemplateLoader = Box::new(move |name: &str| {
                let mut path = base_dir.clone();

                // Handle different path types (same as main template loader)
                let clean_name = name.strip_prefix('/').unwrap_or(name);

                if clean_name.ends_with(".html") {
                    path.push(clean_name);
                } else {
                    path.push(format!("{}.html", clean_name));
                }

                let content = std::fs::read_to_string(&path).map_err(|e| {
                    Error::template(format!("Failed to load partial '{}': {}", name, e))
                })?;

                cache.get_or_compile(&path, &content)
            });

            // Add translator to layout context
            let layout_context = if let Ok(trans) = self.resource_translator.read() {
                if let Some(resource_trans) = trans.as_ref() {
                    let view_translations = resource_trans.get_view_translations(layout_name);
                    let mut legacy_trans = TranslationSystem::new();
                    legacy_trans.add_translations("current", (*view_translations).clone());
                    layout_context.with_translator(legacy_trans)
                } else {
                    layout_context
                }
            } else if let Ok(trans) = self.translator.read() {
                if let Some(translator) = trans.as_ref() {
                    layout_context.with_translator(translator.clone())
                } else {
                    layout_context
                }
            } else {
                layout_context
            };

            let mut layout_renderer = Renderer::new(layout_context)
                .with_template_path(self.base_dir.to_string_lossy().to_string())
                .with_template_loader(Arc::new(loader));

            layout_renderer.render(&layout_ast)
        } else {
            Ok(content)
        }
    }
}

impl ViewEngineImpl for TotalJsEngine {
    fn set_directory(&mut self, dir: &str) {
        self.base_dir = PathBuf::from(dir);
        self.cache.clear();
    }

    fn render(&self, template: &str, data: &Value, layout: Option<&str>) -> Result<String> {
        // Extract context repository and session from data if present
        let (context_repository, session_data) = if let Value::Object(map) = data {
            (map.get("_context_repository"), map.get("_context_session"))
        } else {
            (None, None)
        };

        // Create clean data without the internal fields
        let clean_data = if let Value::Object(mut map) = data.clone() {
            map.remove("_context_repository");
            map.remove("_context_session");
            Value::Object(map)
        } else {
            data.clone()
        };

        self.render_with_layout_and_session(
            template,
            &clean_data,
            layout,
            context_repository,
            session_data,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_engine() -> (TotalJsEngine, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let engine = TotalJsEngine::new(temp_dir.path().to_str().unwrap());
        (engine, temp_dir)
    }

    #[test]
    fn test_simple_template() {
        let (engine, temp_dir) = create_test_engine();

        // Create a simple template
        let template_content = "Hello @{M.name}!";
        fs::write(temp_dir.path().join("test.html"), template_content).unwrap();

        // Render the template
        let data = json!({ "name": "World" });
        let result = engine.render("test", &data, None).unwrap();

        assert_eq!(result, "Hello World!");
    }

    #[test]
    fn test_conditional_template() {
        let (engine, temp_dir) = create_test_engine();

        // Create a template with conditional
        let template_content = "@{if M.logged_in}Welcome back!@{else}Please sign in@{fi}";
        fs::write(temp_dir.path().join("auth.html"), template_content).unwrap();

        // Test with logged_in = true
        let data = json!({ "logged_in": true });
        let result = engine.render("auth", &data, None).unwrap();
        assert_eq!(result, "Welcome back!");

        // Test with logged_in = false
        let data = json!({ "logged_in": false });
        let result = engine.render("auth", &data, None).unwrap();
        assert_eq!(result, "Please sign in");
    }

    #[test]
    fn test_loop_template() {
        let (engine, temp_dir) = create_test_engine();

        // Create a template with loop
        let template_content = "@{foreach item in M.items}@{item} @{end}";
        fs::write(temp_dir.path().join("list.html"), template_content).unwrap();

        // Render with array
        let data = json!({ "items": ["A", "B", "C"] });
        let result = engine.render("list", &data, None).unwrap();
        assert_eq!(result, "A B C ");
    }

    #[test]
    fn test_layout() {
        let (engine, temp_dir) = create_test_engine();

        // Create layouts directory
        fs::create_dir(temp_dir.path().join("layouts")).unwrap();

        // Create a layout using @{body} (standard Total.js placeholder)
        let layout_content = "<html><body>@{body}</body></html>";
        fs::write(temp_dir.path().join("layouts/main.html"), layout_content).unwrap();

        // Create a template
        let template_content = "<h1>@{M.title}</h1>";
        fs::write(temp_dir.path().join("page.html"), template_content).unwrap();

        // Render with layout
        let data = json!({ "title": "Test Page" });
        let result = engine.render("page", &data, Some("main")).unwrap();

        assert_eq!(result, "<html><body><h1>Test Page</h1></body></html>");
    }

    #[test]
    fn test_layout_with_content_tag() {
        // Test for backward compatibility - @{content} is our extension, not standard Total.js
        let (engine, temp_dir) = create_test_engine();

        // Create layouts directory
        fs::create_dir(temp_dir.path().join("layouts")).unwrap();

        // Create a layout using @{content} (our extension for backward compatibility)
        let layout_content = "<html><body>@{content}</body></html>";
        fs::write(
            temp_dir.path().join("layouts/content_layout.html"),
            layout_content,
        )
        .unwrap();

        // Create a template
        let template_content = "<h1>@{M.title}</h1>";
        fs::write(temp_dir.path().join("page.html"), template_content).unwrap();

        // Render with layout
        let data = json!({ "title": "Test Content" });
        let result = engine
            .render("page", &data, Some("layouts/content_layout"))
            .unwrap();

        assert_eq!(result, "<html><body><h1>Test Content</h1></body></html>");
    }

    #[test]
    fn test_sections_in_layouts() {
        let (engine, temp_dir) = create_test_engine();

        // Create layouts directory
        fs::create_dir(temp_dir.path().join("layouts")).unwrap();

        // Create a layout with multiple section placeholders
        let layout_content = r#"<html>
<head>
    <title>@{M.title}</title>
    @{section('styles')}
</head>
<body>
    <header>@{section('header')}</header>
    <main>@{body}</main>
    <footer>@{section('footer')}</footer>
</body>
</html>"#;
        fs::write(
            temp_dir.path().join("layouts/with_sections.html"),
            layout_content,
        )
        .unwrap();

        // Create a child view that defines sections
        let page_content = r#"@{section styles}
<link rel="stylesheet" href="/css/page.css">
<style>.custom { color: red; }</style>
@{end}

@{section header}
<h1>@{M.heading}</h1>
<nav>Navigation Here</nav>
@{end}

<p>Main content: @{M.content}</p>

@{section footer}
<p>Copyright 2025 - @{M.author}</p>
@{end}"#;
        fs::write(
            temp_dir.path().join("page_with_sections.html"),
            page_content,
        )
        .unwrap();

        // Render with layout
        let data = json!({
            "title": "Test Page",
            "heading": "Welcome",
            "content": "This is the main content",
            "author": "RustF Team"
        });
        let result = engine
            .render("page_with_sections", &data, Some("with_sections"))
            .unwrap();

        // Verify structure
        assert!(result.contains("<html>"));
        assert!(result.contains("<title>Test Page</title>"));

        // Verify styles section was rendered in head
        assert!(result.contains(r#"<link rel="stylesheet" href="/css/page.css">"#));
        assert!(result.contains("<style>.custom { color: red; }</style>"));

        // Verify header section was rendered
        assert!(result.contains("<header>"));
        assert!(result.contains("<h1>Welcome</h1>"));
        assert!(result.contains("<nav>Navigation Here</nav>"));
        assert!(result.contains("</header>"));

        // Verify main content (body) was rendered
        assert!(result.contains("<main>"));
        assert!(result.contains("<p>Main content: This is the main content</p>"));
        assert!(result.contains("</main>"));

        // Verify footer section was rendered
        assert!(result.contains("<footer>"));
        assert!(result.contains("<p>Copyright 2025 - RustF Team</p>"));
        assert!(result.contains("</footer>"));
    }

    #[test]
    fn test_missing_sections_in_layout() {
        let (engine, temp_dir) = create_test_engine();

        // Create layouts directory
        fs::create_dir(temp_dir.path().join("layouts")).unwrap();

        // Layout expects sections that child doesn't provide
        let layout_content = r#"<html>
<head>@{section('styles')}</head>
<body>
    <header>@{section('header')}</header>
    <main>@{body}</main>
</body>
</html>"#;
        fs::write(
            temp_dir.path().join("layouts/optional_sections.html"),
            layout_content,
        )
        .unwrap();

        // Child only provides body content, no sections
        let page_content = "<p>Just main content, no sections</p>";
        fs::write(temp_dir.path().join("simple_page.html"), page_content).unwrap();

        let data = json!({});
        let result = engine
            .render("simple_page", &data, Some("optional_sections"))
            .unwrap();

        // Should render successfully with empty section placeholders
        assert!(result.contains("<html>"));
        assert!(result.contains("<head></head>")); // Empty styles section
        assert!(result.contains("<header></header>")); // Empty header section
        assert!(result.contains("<p>Just main content, no sections</p>"));
    }

    #[test]
    fn test_html_escaping() {
        let (engine, temp_dir) = create_test_engine();

        // Create template with escaped and raw variables
        let template_content = "Escaped: @{M.html}\nRaw: @{!M.html}";
        fs::write(temp_dir.path().join("escape.html"), template_content).unwrap();

        let data = json!({ "html": "<script>alert('xss')</script>" });
        let result = engine.render("escape", &data, None).unwrap();

        assert!(result.contains("&lt;script&gt;"));
        assert!(result.contains("<script>alert"));
    }

    #[test]
    fn test_global_repository_data() {
        let (engine, temp_dir) = create_test_engine();

        // Set global repository data
        engine.set_global_repository(json!({ "site_name": "MyApp" }));

        // Create template using APP/MAIN repository
        let template_content = "Welcome to @{APP.site_name}";
        fs::write(temp_dir.path().join("repo.html"), template_content).unwrap();

        let data = json!({});
        let result = engine.render("repo", &data, None).unwrap();

        assert_eq!(result, "Welcome to MyApp");
    }

    #[test]
    fn test_config_values() {
        let (engine, temp_dir) = create_test_engine();

        // Set config
        let mut config = HashMap::new();
        config.insert("app_version".to_string(), "1.0.0".to_string());
        engine.set_config(config);

        // Create template using config
        let template_content = "Version: @{'%app_version'}";
        fs::write(temp_dir.path().join("config.html"), template_content).unwrap();

        let data = json!({});
        let result = engine.render("config", &data, None).unwrap();

        assert_eq!(result, "Version: 1.0.0");
    }

    #[test]
    fn test_url_and_root_handling() {
        let (engine, temp_dir) = create_test_engine();

        // Set config with default_root
        let mut config = HashMap::new();
        config.insert("default_root".to_string(), "/app".to_string());
        engine.set_config(config);

        // Create template that uses URL variables
        let template_content = "Root: @{root}";
        fs::write(temp_dir.path().join("urls.html"), template_content).unwrap();

        let data = json!({});
        let result = engine.render("urls", &data, None).unwrap();

        // Check that root was set correctly
        assert_eq!(result, "Root: /app");
    }

    #[test]
    fn test_context_repository_with_r_alias() {
        let (engine, temp_dir) = create_test_engine();

        // Create template using R alias for context repository
        let template_content = "User: @{R.username}, Theme: @{R.theme}";
        fs::write(temp_dir.path().join("context_repo.html"), template_content).unwrap();

        // Simulate context repository data passed through _context_repository
        let data = json!({
            "title": "Page Title",
            "_context_repository": {
                "username": "john_doe",
                "theme": "dark"
            }
        });

        let result = engine.render("context_repo", &data, None).unwrap();
        assert_eq!(result, "User: john_doe, Theme: dark");
    }

    #[test]
    fn test_model_data_with_m_alias() {
        let (engine, temp_dir) = create_test_engine();

        // Create template using M alias for model data
        let template_content = "Title: @{M.title}, Count: @{M.count}";
        fs::write(temp_dir.path().join("model_alias.html"), template_content).unwrap();

        let data = json!({
            "title": "Test Page",
            "count": 42
        });

        let result = engine.render("model_alias", &data, None).unwrap();
        assert_eq!(result, "Title: Test Page, Count: 42");
    }

    #[test]
    fn test_mixed_repository_access() {
        let (engine, temp_dir) = create_test_engine();

        // Set global repository
        engine.set_global_repository(json!({
            "app_name": "GlobalApp",
            "version": "1.0.0"
        }));

        // Create template using both APP/MAIN and R repositories
        let template_content = r#"
App: @{APP.app_name} v@{MAIN.version}
User: @{R.current_user}
Page: @{M.page_title}"#;
        fs::write(temp_dir.path().join("mixed.html"), template_content).unwrap();

        let data = json!({
            "page_title": "Dashboard",
            "_context_repository": {
                "current_user": "admin"
            }
        });

        let result = engine.render("mixed", &data, None).unwrap();
        assert!(result.contains("App: GlobalApp v1.0.0"));
        assert!(result.contains("User: admin"));
        assert!(result.contains("Page: Dashboard"));
    }

    #[test]
    fn test_repository_vs_r_alias() {
        let (engine, temp_dir) = create_test_engine();

        // Create template using both repository.key and R.key syntax
        let template_content = "Via repository: @{repository.value}, Via R: @{R.value}";
        fs::write(temp_dir.path().join("repo_alias.html"), template_content).unwrap();

        let data = json!({
            "_context_repository": {
                "value": "test_value"
            }
        });

        let result = engine.render("repo_alias", &data, None).unwrap();
        assert_eq!(result, "Via repository: test_value, Via R: test_value");
    }
}
