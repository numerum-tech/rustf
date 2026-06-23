use super::{engine::TotalJsEngine, translation::TranslationSystem};
use crate::config::{AppConfig, ViewConfig};
use crate::error::Result;
use crate::views::ViewEngineImpl;
use serde_json::Value;
use std::sync::Arc;

/// Embedded Total.js engine.
///
/// This is intentionally a thin wrapper around `TotalJsEngine`: the rendering,
/// layout, partial, cache, and translation logic all live in the shared engine.
/// The only backend-specific behavior is where template bytes are loaded from.
pub struct EmbeddedTotalJsEngine {
    inner: TotalJsEngine,
}

impl EmbeddedTotalJsEngine {
    pub fn new() -> Self {
        Self {
            inner: TotalJsEngine::new_embedded(),
        }
    }

    pub fn with_config(view_config: Option<&ViewConfig>) -> Self {
        Self {
            inner: TotalJsEngine::with_embedded_config(view_config),
        }
    }

    pub fn with_app_config(app_config: Arc<AppConfig>) -> Self {
        Self {
            inner: TotalJsEngine::with_embedded_app_config(app_config),
        }
    }

    pub fn set_config(&self, key: &str, value: &str) {
        self.inner.set_config_value(key, value);
    }

    pub fn set_translator(&self, translator: TranslationSystem) {
        self.inner.set_translator(translator);
    }

    pub fn clear_cache(&self) {
        self.inner.clear_cache();
    }

    pub fn cache_stats(&self) -> (usize, bool) {
        self.inner.cache_stats()
    }

    #[cfg(test)]
    fn template_path(&self, template: &str) -> String {
        self.inner
            .template_path(template)
            .unwrap()
            .to_string_lossy()
            .to_string()
    }

    #[cfg(test)]
    fn layout_path(&self, layout: &str) -> String {
        self.inner
            .layout_path(layout)
            .unwrap()
            .to_string_lossy()
            .to_string()
    }
}

impl Default for EmbeddedTotalJsEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ViewEngineImpl for EmbeddedTotalJsEngine {
    fn set_directory(&mut self, dir: &str) {
        self.inner.set_directory(dir);
    }

    fn render(&self, template: &str, data: &Value, layout: Option<&str>) -> Result<String> {
        self.inner.render(template, data, layout)
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
        self.inner
            .render_rich(template, data, layout, repository, session, request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::embed_provider::{register_views_provider, EmbeddedFileProvider};
    use serde_json::json;
    use std::collections::HashMap;
    use std::sync::Once;

    struct TestViewsProvider {
        files: HashMap<String, Vec<u8>>,
    }

    impl EmbeddedFileProvider for TestViewsProvider {
        fn get(&self, path: &str) -> Option<Vec<u8>> {
            self.files.get(path).cloned()
        }

        fn files(&self) -> Vec<String> {
            self.files.keys().cloned().collect()
        }
    }

    fn ensure_test_views_provider() {
        static INIT: Once = Once::new();
        INIT.call_once(|| {
            let files = HashMap::from([
                (
                    "page.html".to_string(),
                    b"@{title('Embedded Title')}@{description('Embedded Description')}@{M.body}"
                        .to_vec(),
                ),
                (
                    "leak.html".to_string(),
                    b"@{M._context_repository.secret}|@{R.secret}".to_vec(),
                ),
                (
                    "layouts/main.html".to_string(),
                    b"<title>@{title}</title><meta name=\"description\" content=\"@{description}\">@{body}"
                        .to_vec(),
                ),
            ]);
            register_views_provider(Box::new(TestViewsProvider { files }));
        });
    }

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
        assert_eq!(engine.layout_path("main.html"), "layouts/main.html");
        assert_eq!(engine.layout_path("layouts/main"), "layouts/main.html");
        assert_eq!(engine.layout_path("/layouts/main"), "layouts/main.html");
    }

    #[test]
    fn test_config_management() {
        let engine = EmbeddedTotalJsEngine::new();

        engine.set_config("app_name", "TestApp");
        engine.set_config("version", "1.0.0");
        let (count, _) = engine.cache_stats();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_cache_clearing() {
        let engine = EmbeddedTotalJsEngine::new();
        engine.clear_cache();
        let (count, _) = engine.cache_stats();
        assert_eq!(count, 0);
    }

    #[test]
    fn embedded_render_strips_internal_context_fields_from_model() {
        ensure_test_views_provider();
        let engine = EmbeddedTotalJsEngine::new();

        let html = engine
            .render(
                "leak",
                &json!({
                    "body": "ok",
                    "_context_repository": { "secret": "repo-secret" }
                }),
                None,
            )
            .unwrap();

        assert_eq!(html, "|repo-secret");
    }

    #[test]
    fn embedded_layout_receives_child_meta() {
        ensure_test_views_provider();
        let engine = EmbeddedTotalJsEngine::new();

        let html = engine
            .render("page", &json!({ "body": "Hello" }), Some("main"))
            .unwrap();

        assert!(html.contains("<title>Embedded Title</title>"));
        assert!(html.contains("content=\"Embedded Description\""));
        assert!(html.contains("Hello"));
    }
}
