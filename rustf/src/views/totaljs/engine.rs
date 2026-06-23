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
#[cfg(feature = "embedded-views")]
use crate::views::embed_provider::get_views_provider;
use crate::views::minifier::minify_html;
use crate::views::ViewEngineImpl;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// Cache entry for compiled templates
#[derive(Clone)]
struct CacheEntry {
    template: Arc<Template>,
    _compiled_at: u64,
    version: Option<String>,
}

/// Template cache for performance
#[derive(Clone)]
struct TemplateCache {
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    enable_hot_reload: bool,
    /// If true, skip mtime checks and trust cache entries (production optimization)
    trust_cache: bool,
}

impl TemplateCache {
    fn new(enable_hot_reload: bool) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            enable_hot_reload,
            // In production (release mode), trust cache to avoid blocking filesystem calls
            trust_cache: !enable_hot_reload && !cfg!(debug_assertions),
        }
    }

    fn new_with_trust_cache(enable_hot_reload: bool, trust_cache: bool) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            enable_hot_reload,
            trust_cache: trust_cache && !enable_hot_reload,
        }
    }

    fn get_file_mtime(path: &Path) -> Option<u64> {
        std::fs::metadata(path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
    }

    fn get_cached(&self, cache_key: &str) -> Option<Arc<Template>> {
        if let Ok(cache) = self.cache.read() {
            if let Some(entry) = cache.get(cache_key) {
                return Some(Arc::clone(&entry.template));
            }
        }
        None
    }

    fn get_if_version_matches(
        &self,
        cache_key: &str,
        version: Option<&str>,
    ) -> Option<Arc<Template>> {
        if let Ok(cache) = self.cache.read() {
            if let Some(entry) = cache.get(cache_key) {
                if entry.version.as_deref() == version {
                    return Some(Arc::clone(&entry.template));
                }
            }
        }
        None
    }

    fn compile_and_store(
        &self,
        cache_key: &str,
        content: &str,
        version: Option<String>,
    ) -> Result<Arc<Template>> {
        let mut parser = Parser::new(content)?;
        let template_arc = Arc::new(parser.parse()?);

        if let Ok(mut cache) = self.cache.write() {
            let entry = CacheEntry {
                template: Arc::clone(&template_arc),
                _compiled_at: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                version,
            };
            cache.insert(cache_key.to_string(), entry);
        }

        Ok(template_arc)
    }

    fn clear(&self) {
        if let Ok(mut cache) = self.cache.write() {
            cache.clear();
        }
    }
}

/// Total.js template engine implementation
pub struct TotalJsEngine {
    source: TemplateSource,
    cache: TemplateCache,
    /// Cached JSON representation of CONF to avoid re-serializing config every render.
    conf_value: Arc<RwLock<Arc<Value>>>,
    /// Translation system (legacy JSON-based)
    translator: Arc<RwLock<Option<Arc<TranslationSystem>>>>,
    /// Resource translation system (new .res file based)
    resource_translator: Arc<RwLock<Option<ResourceTranslationSystem>>>,
    /// Whether to minify rendered HTML output
    minify: bool,
}

#[derive(Clone)]
enum TemplateSource {
    Filesystem {
        base_dir: PathBuf,
    },
    #[cfg(feature = "embedded-views")]
    Embedded,
}

struct TemplateAsset {
    content: String,
    version: Option<String>,
}

fn conf_from_view_config(view_config: Option<&ViewConfig>, cache_enabled: bool) -> Arc<Value> {
    let mut root = serde_json::Map::new();
    if let Some(vc) = view_config {
        let mut views = serde_json::Map::new();
        views.insert(
            "default_root".to_string(),
            Value::String(vc.default_root.clone()),
        );
        views.insert(
            "default_layout".to_string(),
            Value::String(vc.default_layout.clone()),
        );
        views.insert("cache_enabled".to_string(), Value::Bool(cache_enabled));
        root.insert("views".to_string(), Value::Object(views));
    }
    Arc::new(Value::Object(root))
}

fn conf_from_app_config(app_config: &AppConfig) -> Arc<Value> {
    Arc::new(
        serde_json::to_value(app_config).unwrap_or_else(|_| Value::Object(serde_json::Map::new())),
    )
}

fn conf_from_flat_config(config: &std::collections::HashMap<String, String>) -> Arc<Value> {
    let mut conf_obj = serde_json::Map::new();
    for (key, value) in config {
        conf_obj.insert(key.clone(), Value::String(value.clone()));
    }
    Arc::new(Value::Object(conf_obj))
}

fn set_conf_entry(conf: &mut Value, key: &str, value: Value) {
    if !conf.is_object() {
        *conf = Value::Object(serde_json::Map::new());
    }

    if let Value::Object(map) = conf {
        map.insert(key.to_string(), value);
    }
}

/// Append `.html` to a template name if it isn't already present.
fn ensure_html_extension(name: &str) -> String {
    if name.ends_with(".html") {
        name.to_string()
    } else {
        format!("{}.html", name)
    }
}

/// Reject a relative template/partial path that would escape the views base
/// directory. Purely lexical (no filesystem access) so it works for
/// not-yet-created files and can't be defeated by a missing-file race. Blocks
/// `..` traversal and absolute paths; view names are otherwise developer-
/// supplied, but this contains the damage if one is ever built from input.
pub(crate) fn reject_traversal(relative: &str) -> Result<()> {
    let mut depth: i32 = 0;
    for comp in Path::new(relative).components() {
        match comp {
            Component::Normal(_) => depth += 1,
            Component::CurDir => {}
            Component::ParentDir => {
                depth -= 1;
                if depth < 0 {
                    return Err(Error::template(format!(
                        "template name '{}' escapes the views directory",
                        relative
                    )));
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(Error::template(format!(
                    "template name '{}' must be a relative path",
                    relative
                )));
            }
        }
    }
    Ok(())
}

/// Load a template asset (raw content + cache version) from either backend.
/// Shared by the main-template loader and the partial loader so filesystem and
/// embedded sources go through exactly one code path. `name` is only used for
/// error messages; `relative` is the already-validated relative path.
fn load_asset(source: &TemplateSource, relative: &str, name: &str) -> Result<TemplateAsset> {
    match source {
        TemplateSource::Filesystem { base_dir } => {
            let path = base_dir.join(relative);
            let content = std::fs::read_to_string(&path).map_err(|e| {
                Error::template(format!("Failed to load template '{}': {}", name, e))
            })?;
            Ok(TemplateAsset {
                content,
                version: TemplateCache::get_file_mtime(&path).map(|mtime| mtime.to_string()),
            })
        }
        #[cfg(feature = "embedded-views")]
        TemplateSource::Embedded => {
            let provider = get_views_provider().ok_or_else(|| {
                Error::template(
                    "No embedded views provider registered. Make sure auto_discover!() is called \
                     with the embedded-views feature enabled."
                        .to_string(),
                )
            })?;
            let data = provider.get(relative).ok_or_else(|| {
                Error::template(format!(
                    "Embedded template not found: {}. Make sure the template exists in the views \
                     directory at compile time.",
                    name
                ))
            })?;
            let content = std::str::from_utf8(&data)
                .map_err(|e| Error::template(format!("Invalid UTF-8 in template {}: {}", name, e)))?
                .to_string();

            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            data.hash(&mut hasher);

            Ok(TemplateAsset {
                content,
                version: Some(format!("{:x}", hasher.finish())),
            })
        }
    }
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

        let minify = view_config.map(|vc| vc.minify).unwrap_or(false);
        let conf_value = conf_from_view_config(view_config, cache_enabled);

        Self {
            source: TemplateSource::Filesystem {
                base_dir: PathBuf::from(base_dir),
            },
            cache: TemplateCache::new(enable_hot_reload),
            conf_value: Arc::new(RwLock::new(conf_value)),
            translator: Arc::new(RwLock::new(None)),
            resource_translator: Arc::new(RwLock::new(None)),
            minify,
        }
    }

    /// Create a new Total.js template engine with full application configuration
    pub fn with_app_config(base_dir: &str, app_config: Arc<AppConfig>) -> Self {
        // Respect cache_enabled setting from config
        let enable_hot_reload = !app_config.views.cache_enabled;

        // In production with caching enabled, trust the cache (skip mtime checks)
        // This eliminates blocking filesystem operations on every render
        let trust_cache = app_config.views.cache_enabled && app_config.environment.is_production();

        let minify = app_config.views.minify;
        let conf_value = conf_from_app_config(app_config.as_ref());

        Self {
            source: TemplateSource::Filesystem {
                base_dir: PathBuf::from(base_dir),
            },
            cache: TemplateCache::new_with_trust_cache(enable_hot_reload, trust_cache),
            conf_value: Arc::new(RwLock::new(conf_value)),
            translator: Arc::new(RwLock::new(None)),
            resource_translator: Arc::new(RwLock::new(None)),
            minify,
        }
    }

    #[cfg(feature = "embedded-views")]
    pub fn new_embedded() -> Self {
        Self::with_embedded_config(None)
    }

    #[cfg(feature = "embedded-views")]
    pub fn with_embedded_config(view_config: Option<&ViewConfig>) -> Self {
        let (enable_hot_reload, cache_enabled) = if let Some(vc) = view_config {
            (!vc.cache_enabled, vc.cache_enabled)
        } else {
            (cfg!(debug_assertions), !cfg!(debug_assertions))
        };

        let minify = view_config.map(|vc| vc.minify).unwrap_or(false);
        let conf_value = conf_from_view_config(view_config, cache_enabled);

        Self {
            source: TemplateSource::Embedded,
            cache: TemplateCache::new(enable_hot_reload),
            conf_value: Arc::new(RwLock::new(conf_value)),
            translator: Arc::new(RwLock::new(None)),
            resource_translator: Arc::new(RwLock::new(None)),
            minify,
        }
    }

    #[cfg(feature = "embedded-views")]
    pub fn with_embedded_app_config(app_config: Arc<AppConfig>) -> Self {
        let enable_hot_reload = !app_config.views.cache_enabled;
        let trust_cache = app_config.views.cache_enabled && app_config.environment.is_production();

        let minify = app_config.views.minify;
        let conf_value = conf_from_app_config(app_config.as_ref());

        Self {
            source: TemplateSource::Embedded,
            cache: TemplateCache::new_with_trust_cache(enable_hot_reload, trust_cache),
            conf_value: Arc::new(RwLock::new(conf_value)),
            translator: Arc::new(RwLock::new(None)),
            resource_translator: Arc::new(RwLock::new(None)),
            minify,
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
    pub fn set_config(&self, config: std::collections::HashMap<String, String>) {
        if let Ok(mut conf) = self.conf_value.write() {
            *conf = conf_from_flat_config(&config);
        }
    }

    /// Set translation system (legacy JSON-based)
    pub fn set_translator(&self, translator: TranslationSystem) {
        if let Ok(mut trans) = self.translator.write() {
            *trans = Some(Arc::new(translator));
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

    pub fn cache_stats(&self) -> (usize, bool) {
        let count = self.cache.cache.read().map(|c| c.len()).unwrap_or(0);
        (count, self.cache.enable_hot_reload)
    }

    pub fn set_config_value(&self, key: &str, value: &str) {
        if let Ok(mut conf) = self.conf_value.write() {
            let conf_value = Arc::make_mut(&mut *conf);
            set_conf_entry(conf_value, key, Value::String(value.to_string()));
        }
    }

    /// Set the full application configuration
    pub fn set_app_config(&mut self, app_config: Arc<AppConfig>) {
        if let Ok(mut conf) = self.conf_value.write() {
            *conf = conf_from_app_config(app_config.as_ref());
        }
    }

    fn normalized_template_path(&self, template: &str) -> Result<String> {
        // Both '/someview' and 'someview' should work the same way.
        let template_clean = template.strip_prefix('/').unwrap_or(template);
        let relative = ensure_html_extension(template_clean);
        reject_traversal(&relative)?;
        Ok(relative)
    }

    fn normalized_layout_path(&self, layout: &str) -> Result<String> {
        let layout_clean = layout.strip_prefix('/').unwrap_or(layout);
        let relative = if layout_clean.contains('/') {
            // Path given explicitly — use as-is (under base_dir).
            ensure_html_extension(layout_clean)
        } else {
            // Simple name — look in the layouts/ subdirectory.
            format!("layouts/{}", ensure_html_extension(layout_clean))
        };
        reject_traversal(&relative)?;
        Ok(relative)
    }

    /// Test-only: resolve a template name to its full path. Used by the
    /// embedded-engine tests; production code uses `normalized_template_path`
    /// + `load_template` directly.
    #[cfg(test)]
    pub(crate) fn template_path(&self, template: &str) -> Result<PathBuf> {
        let relative = self.normalized_template_path(template)?;
        match &self.source {
            TemplateSource::Filesystem { base_dir } => Ok(base_dir.join(relative)),
            #[cfg(feature = "embedded-views")]
            TemplateSource::Embedded => Ok(PathBuf::from(relative)),
        }
    }

    /// Test-only counterpart to `template_path` for layouts.
    #[cfg(test)]
    pub(crate) fn layout_path(&self, layout: &str) -> Result<PathBuf> {
        let relative = self.normalized_layout_path(layout)?;
        match &self.source {
            TemplateSource::Filesystem { base_dir } => Ok(base_dir.join(relative)),
            #[cfg(feature = "embedded-views")]
            TemplateSource::Embedded => Ok(PathBuf::from(relative)),
        }
    }

    fn template_root_for_renderer(&self) -> Option<String> {
        match &self.source {
            TemplateSource::Filesystem { base_dir } => Some(base_dir.to_string_lossy().to_string()),
            #[cfg(feature = "embedded-views")]
            TemplateSource::Embedded => None,
        }
    }

    fn load_template_from_source(
        source: &TemplateSource,
        cache: &TemplateCache,
        relative_path: &str,
        display_name: &str,
    ) -> Result<Arc<Template>> {
        if cache.trust_cache {
            if let Some(template) = cache.get_cached(relative_path) {
                return Ok(template);
            }
        }

        match source {
            TemplateSource::Filesystem { base_dir } => {
                let path = base_dir.join(relative_path);
                let version = TemplateCache::get_file_mtime(&path).map(|mtime| mtime.to_string());
                if let Some(template) =
                    cache.get_if_version_matches(relative_path, version.as_deref())
                {
                    return Ok(template);
                }

                let content = std::fs::read_to_string(&path).map_err(|e| {
                    Error::template(format!("Failed to load template '{}': {}", display_name, e))
                })?;
                cache.compile_and_store(relative_path, &content, version)
            }
            #[cfg(feature = "embedded-views")]
            TemplateSource::Embedded => {
                let asset = load_asset(source, relative_path, display_name)?;
                if let Some(template) =
                    cache.get_if_version_matches(relative_path, asset.version.as_deref())
                {
                    return Ok(template);
                }
                cache.compile_and_store(relative_path, &asset.content, asset.version)
            }
        }
    }

    /// Build a partial-template loader closure that resolves names through the
    /// same backend + cache as the main template. Used for both view and layout
    /// rendering so there is a single loader implementation.
    fn build_loader(&self) -> TemplateLoader {
        let source = self.source.clone();
        let cache = self.cache.clone();
        Box::new(move |name: &str| {
            let clean_name = name.strip_prefix('/').unwrap_or(name);
            let relative = ensure_html_extension(clean_name);
            reject_traversal(&relative)?;
            Self::load_template_from_source(&source, &cache, &relative, name)
        })
    }

    /// Load and compile a template, returning a shared Arc (zero-copy on cache hit).
    fn load_template(&self, relative_path: &str) -> Result<Arc<Template>> {
        Self::load_template_from_source(&self.source, &self.cache, relative_path, relative_path)
    }

    /// Create a render context with common data
    fn create_context(
        &self,
        data: &Value,
        context_repository: Option<&Value>,
        session_data: Option<&Value>,
    ) -> RenderContext {
        let mut context = RenderContext::new(data.clone());

        context = context.with_global_repository_handle(APP::get_repository());

        // Add context repository if provided (repository/R)
        if let Some(ctx_repo) = context_repository {
            context = context.with_repository(ctx_repo.clone());
        }

        if let Ok(conf) = self.conf_value.read() {
            context = context.with_shared_conf(Arc::clone(&*conf));
        }

        // Add translator if available
        if let Ok(trans) = self.translator.read() {
            if let Some(translator) = trans.as_ref() {
                context = context.with_shared_translator(Arc::clone(translator));
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
        self.render_with_layout_and_session(template, data, layout, context_repository, None, None)
    }

    /// Render a template with layout, context repository, and session data
    pub fn render_with_layout_and_session(
        &self,
        template: &str,
        data: &Value,
        layout: Option<&str>,
        context_repository: Option<&Value>,
        session_data: Option<&Value>,
        request: Option<&crate::views::RequestMeta>,
    ) -> Result<String> {
        let template_path = self.normalized_template_path(template)?;
        let template_ast = self.load_template(&template_path)?;

        // Create render context with session data
        let context = self.create_context(data, context_repository, session_data);
        // Apply per-request metadata (url / hostname / mobile) if provided.
        let context = match request {
            Some(req) => context
                .with_url(req.url.clone())
                .with_hostname(req.hostname.clone())
                .with_mobile(req.mobile),
            None => context,
        };

        // Create template loader that uses the cache (handles fs + embedded).
        let loader: TemplateLoader = self.build_loader();

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
                context.with_shared_translator(Arc::clone(translator))
            } else {
                context
            }
        } else {
            context
        };

        // Render the template with template loader for partials
        let mut renderer = Renderer::new(context).with_template_loader(std::sync::Arc::new(loader));
        if let Some(path) = self.template_root_for_renderer() {
            renderer = renderer.with_template_path(path);
        }

        let content = renderer.render(&template_ast)?;

        // Carry any page title/description set in the view (via @{title('...')}
        // / @{description('...')}) so the layout can output them.
        let page_title = renderer.meta_title();
        let page_description = renderer.meta_description();

        // Apply layout if specified
        if let Some(layout_name) = layout {
            let layout_path = self.normalized_layout_path(layout_name)?;
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

            // Transfer child template sections to the request renderer so the
            // layout can render them as fallbacks/overrides.
            renderer.merge_sections(&template_ast.sections);

            // Carry the view's page title/description into the layout so
            // @{title} / @{description} resolve there.
            if let Some(ref t) = page_title {
                renderer.set_meta_title(t);
            }
            if let Some(ref d) = page_description {
                renderer.set_meta_description(d);
            }

            // Switch translator for the layout render when resource-specific
            // translations are available.
            let saved_translator = renderer.translator_handle();
            if let Ok(trans) = self.resource_translator.read() {
                if let Some(resource_trans) = trans.as_ref() {
                    let view_translations = resource_trans.get_view_translations(layout_name);
                    let mut legacy_trans = TranslationSystem::new();
                    legacy_trans.add_translations("current", (*view_translations).clone());
                    renderer.set_shared_translator(Some(Arc::new(legacy_trans)));
                }
            } else if let Ok(trans) = self.translator.read() {
                if let Some(translator) = trans.as_ref() {
                    renderer.set_shared_translator(Some(Arc::clone(translator)));
                }
            }

            let result = renderer.render_layout_template(&layout_ast, layout_data);
            renderer.set_shared_translator(saved_translator);
            result
        } else {
            Ok(content)
        }
    }
}

impl ViewEngineImpl for TotalJsEngine {
    fn set_directory(&mut self, dir: &str) {
        match &mut self.source {
            TemplateSource::Filesystem { base_dir } => {
                *base_dir = PathBuf::from(dir);
                self.cache.clear();
            }
            #[cfg(feature = "embedded-views")]
            TemplateSource::Embedded => {
                log::warn!(
                    "Cannot change directory for embedded views - templates are compiled into the binary"
                );
            }
        }
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

        let html = self.render_with_layout_and_session(
            template,
            &clean_data,
            layout,
            context_repository,
            session_data,
            None,
        )?;

        if self.minify {
            Ok(minify_html(&html))
        } else {
            Ok(html)
        }
    }

    fn render_rich(
        &self,
        template: &str,
        data: &Value,
        layout: Option<&str>,
        repository: Option<&Value>,
        session: Option<&Value>,
        request: Option<&crate::views::RequestMeta>,
    ) -> Result<String> {
        // Call render_with_layout_and_session directly — no hidden-field packing,
        // no data.clone() needed to strip those fields.
        let html = self
            .render_with_layout_and_session(template, data, layout, repository, session, request)?;
        if self.minify {
            Ok(minify_html(&html))
        } else {
            Ok(html)
        }
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
    fn test_title_set_in_view_reaches_layout() {
        let (engine, temp_dir) = create_test_engine();

        // Layout reads the title via @{title}; view sets it via @{title('...')}
        fs::create_dir_all(temp_dir.path().join("layouts")).unwrap();
        fs::write(
            temp_dir.path().join("layouts/main.html"),
            "<head><title>@{title}</title></head><body>@{body}</body>",
        )
        .unwrap();
        fs::write(
            temp_dir.path().join("page.html"),
            "@{title('My Page')}Hello",
        )
        .unwrap();

        let result = engine
            .render_with_layout("page", &json!({}), Some("main"), None)
            .unwrap();
        assert_eq!(
            result,
            "<head><title>My Page</title></head><body>Hello</body>"
        );
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
        if !APP::is_initialized() {
            APP::init(json!({})).unwrap();
        } else {
            APP::clear().unwrap();
        }
        APP::set("site_name", "MyApp").unwrap();

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
        if !APP::is_initialized() {
            APP::init(json!({})).unwrap();
        } else {
            APP::clear().unwrap();
        }
        APP::set("app_name", "GlobalApp").unwrap();
        APP::set("version", "1.0.0").unwrap();

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
    fn rejects_path_traversal_in_view_and_layout_names() {
        let (engine, _temp) = create_test_engine();

        // `..` that escapes the views directory must be rejected, never read.
        assert!(engine.normalized_template_path("../secret").is_err());
        assert!(engine.normalized_template_path("../../etc/passwd").is_err());
        assert!(engine.normalized_layout_path("../../etc/passwd").is_err());
        // Rendering a traversal name fails rather than reading outside the dir.
        assert!(engine
            .render("../../../../etc/passwd", &json!({}), None)
            .is_err());

        // Normal relative names still resolve.
        assert!(engine.normalized_template_path("home/index").is_ok());
        assert!(engine.normalized_layout_path("default").is_ok());
    }

    #[test]
    fn recursive_partial_include_errors_instead_of_crashing() {
        let (engine, temp_dir) = create_test_engine();
        // A partial that includes itself must hit the depth cap and return an
        // error — NOT recurse until the stack overflows and aborts the process.
        fs::write(temp_dir.path().join("a.html"), "@{view('a')}").unwrap();
        let result = engine.render("a", &json!({}), None);
        assert!(result.is_err(), "recursive partial must error, got Ok");
    }

    #[test]
    fn partial_include_rejects_path_traversal() {
        let (engine, temp_dir) = create_test_engine();
        // The @{view} fs-fallback must enforce the same traversal guard as the
        // loader path — a `..` partial name must not escape the views dir.
        fs::write(
            temp_dir.path().join("main.html"),
            "@{view('../../../../etc/passwd')}",
        )
        .unwrap();
        let result = engine.render("main", &json!({}), None);
        assert!(result.is_err(), "traversal partial must be rejected");
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
