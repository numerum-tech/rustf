use crate::config::{AppConfig, TemplateEngine, TemplateStorage};
use crate::context::Context;
use crate::error::Result;
use crate::events::{EventContext, EventEmitter};
use crate::http::{Request, Response, Server};
use crate::middleware::{MiddlewareRegistry, MiddlewareResult};
use crate::models::ModelRegistry;
use crate::routing::{Route, Router};
use crate::shared::SharedRegistry;
use crate::views::ViewEngine;
use crate::workers::WorkerManager;
use ipnetwork::IpNetwork;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Memory-safe RustF application with Arc-based component sharing
///
/// This refactored version uses Arc for safe sharing of framework components
/// between concurrent contexts, eliminating all unsafe code and raw pointers.
pub struct RustF {
    router: Router,
    models: Arc<ModelRegistry>,
    views: Arc<ViewEngine>,
    static_dirs: HashMap<String, PathBuf>,
    middleware: MiddlewareRegistry,
    shared: Arc<SharedRegistry>,
    events: Arc<RwLock<EventEmitter>>,
    workers: Option<Arc<WorkerManager>>,
    pub config: Arc<AppConfig>,
    trusted_proxies: Arc<Vec<IpNetwork>>,
}

impl Default for RustF {
    fn default() -> Self {
        Self::new()
    }
}

impl RustF {
    fn ensure_private_storage_dirs(config: &AppConfig) {
        for (label, dir) in [
            ("private", config.r#private.directory.as_str()),
            ("uploads", config.uploads.directory.as_str()),
        ] {
            let dir_path = std::path::Path::new(dir);
            if !dir_path.exists() {
                match std::fs::create_dir_all(dir_path) {
                    Ok(()) => log::info!("Created {} directory: {}", label, dir),
                    Err(e) => log::warn!("Could not create {} directory '{}': {}", label, dir, e),
                }
            }
        }
    }

    fn parse_trusted_proxies(config: &AppConfig) -> Vec<IpNetwork> {
        config
            .server
            .trusted_proxies
            .iter()
            .filter_map(|entry| match entry.parse::<IpNetwork>() {
                Ok(network) => Some(network),
                Err(err) => {
                    log::warn!("Ignoring invalid trusted proxy '{}': {}", entry, err);
                    None
                }
            })
            .collect()
    }

    pub fn new() -> Self {
        Self::with_config(AppConfig::default())
    }

    pub fn with_config(config: AppConfig) -> Self {
        // Wrap config in Arc first so we can share it
        let config_arc = Arc::new(config);

        // Create view engine based on configuration: engine + storage
        let views = match (
            config_arc.views.engine.clone(),
            config_arc.views.storage.clone(),
        ) {
            (TemplateEngine::TotalJs, TemplateStorage::Filesystem) => {
                log::info!(
                    "Using Total.js engine with filesystem storage from: {}",
                    config_arc.views.directory
                );
                // Total.js is always available as the built-in engine
                ViewEngine::totaljs_filesystem_with_app_config(
                    &config_arc.views.directory,
                    config_arc.clone(),
                )
            }
            (TemplateEngine::TotalJs, TemplateStorage::Embedded) => {
                log::info!("Using Total.js engine with embedded storage");
                #[cfg(feature = "embedded-views")]
                {
                    ViewEngine::totaljs_embedded_with_app_config(config_arc.clone())
                }
                #[cfg(not(feature = "embedded-views"))]
                {
                    panic!("Embedded templates not available! Enable 'embedded-views' feature")
                }
            }
        };

        let middleware = MiddlewareRegistry::new();

        // Initialize static files from configuration
        let mut static_dirs = HashMap::new();
        static_dirs.insert(
            config_arc.static_files.url_prefix.clone(),
            PathBuf::from(&config_arc.static_files.directory),
        );
        log::debug!(
            "Configured static files: {} -> {}",
            config_arc.static_files.url_prefix,
            config_arc.static_files.directory
        );

        Self::ensure_private_storage_dirs(&config_arc);
        let trusted_proxies = Arc::new(Self::parse_trusted_proxies(&config_arc));

        Self {
            router: Router::new(),
            models: Arc::new(ModelRegistry::new()),
            views: Arc::new(views),
            static_dirs,
            middleware,
            shared: Arc::new(SharedRegistry::new()),
            events: Arc::new(RwLock::new(EventEmitter::new())),
            workers: None,
            config: config_arc,
            trusted_proxies,
        }
    }

    pub fn from_file(config_path: &str) -> Result<Self> {
        let config = AppConfig::from_file(config_path)?;
        Ok(Self::with_config(config))
    }

    pub fn from_env() -> Result<Self> {
        let config = AppConfig::from_env()?;
        Ok(Self::with_config(config))
    }

    /// Create RustF application with enhanced path support
    ///
    /// This method supports both CLI arguments and intelligent path resolution:
    /// - CLI `--config` flag for custom config files
    /// - Development-time auto-detection from target/debug folder
    /// - Full/relative path support in config.toml views.directory
    ///
    /// # Path Resolution Examples
    /// In config.toml:
    /// ```toml
    /// [views]
    /// directory = "views"              # Relative to config file
    /// directory = "./templates"        # Relative to config file
    /// directory = "../shared/views"    # Relative to config file
    /// directory = "/absolute/path"     # Absolute path
    /// ```
    ///
    /// # CLI Examples
    /// ```bash
    /// # Use intelligent config loading (includes dev auto-detection)
    /// ./my-app
    ///
    /// # Specify config file
    /// ./my-app --config /etc/myapp/config.toml
    /// ./my-app -c ../shared-config.toml
    /// ```
    pub fn with_args() -> Result<Self> {
        let args = crate::cli::CliArgs::parse()?;

        let config = if let Some(config_path) = args.config_path() {
            log::info!("Loading configuration from CLI: {}", config_path.display());
            AppConfig::from_file(config_path)?
        } else {
            log::debug!("Using intelligent configuration loading with dev auto-detection");
            AppConfig::load_with_dev_detection()?
        };

        Ok(Self::with_config(config))
    }

    /// Create RustF application with CLI args and a custom default config path
    ///
    /// Similar to `with_args()` but uses a custom default configuration file
    /// when no `--config` flag is provided.
    ///
    /// # Arguments
    /// * `default_config_path` - Default config file to use if no --config flag
    ///
    /// # Examples
    /// ```no_run
    /// # use rustf::prelude::*;
    /// # fn _doctest() -> rustf::Result<()> {
    /// // Use custom default, but allow --config to override
    /// let app = RustF::with_args_and_default("config/production.toml")?;
    /// # Ok(()) }
    /// ```
    pub fn with_args_and_default(default_config_path: &str) -> Result<Self> {
        let args = crate::cli::CliArgs::parse()?;

        let mut config = if let Some(config_path) = args.config_path() {
            log::info!(
                "Loading configuration from CLI arg: {}",
                config_path.display()
            );
            AppConfig::from_file(config_path)?
        } else {
            log::info!(
                "Loading default configuration from: {}",
                default_config_path
            );
            AppConfig::from_file(default_config_path)?
        };

        // Override views directory from CLI if provided and using filesystem storage
        if let Some(views_path) = args.views_path() {
            match config.views.storage {
                crate::config::TemplateStorage::Filesystem => {
                    log::info!(
                        "Overriding views directory from CLI: {}",
                        views_path.display()
                    );
                    config.views.directory = views_path.to_string_lossy().to_string();
                }
                crate::config::TemplateStorage::Embedded => {
                    log::warn!("--views flag ignored: embedded template storage is configured");
                }
            }
        }

        Ok(Self::with_config(config))
    }

    pub fn config(mut self, config: AppConfig) -> Self {
        // Update views directory and rebuild static directory mapping to align with new config
        Arc::get_mut(&mut self.views)
            .expect("Views should not be shared during configuration")
            .set_directory(&config.views.directory);

        self.static_dirs.clear();
        self.static_dirs.insert(
            config.static_files.url_prefix.clone(),
            PathBuf::from(&config.static_files.directory),
        );

        Self::ensure_private_storage_dirs(&config);
        self.trusted_proxies = Arc::new(Self::parse_trusted_proxies(&config));
        self.config = Arc::new(config);
        self
    }

    pub fn controllers(mut self, routes: Vec<Route>) -> Self {
        for route in routes {
            self.router.add_route(route);
        }
        self
    }

    pub fn models<F>(mut self, register_fn: F) -> Self
    where
        F: FnOnce(&mut ModelRegistry),
    {
        let models = Arc::get_mut(&mut self.models)
            .expect("Models should not be shared during configuration");
        register_fn(models);
        self
    }

    /// Register a middleware with the application
    ///
    /// # Example
    /// ```rust,ignore
    /// let app = RustF::new()
    ///     .middleware("auth", AuthMiddleware::new("secret"))
    ///     .middleware("cors", CorsMiddleware::default());
    /// ```
    /// Enable default security middleware (recommended for production)
    ///
    /// This enables:
    /// - Security headers (X-Frame-Options, X-Content-Type-Options, etc.)
    /// - Input validation (SQL injection, XSS detection)
    /// - Content Security Policy with permissive defaults
    ///
    /// Note: Rate limiting is NOT enabled by default as it requires tuning
    ///
    /// # Example
    /// ```rust,ignore
    /// let app = RustF::new()
    ///     .with_default_security()
    ///     .controllers(auto_controllers!());
    /// ```
    pub fn with_default_security(mut self) -> Self {
        use crate::middleware::builtin::{
            CspMiddleware, SecurityHeadersMiddleware, ValidationMiddleware,
        };

        // Add security headers (safe for all apps)
        self.middleware
            .register_outbound("security_headers", SecurityHeadersMiddleware::new());

        // Add input validation (protects against common attacks)
        self.middleware
            .register_inbound("input_validation", ValidationMiddleware::new());

        // Add permissive CSP (report-only mode to avoid breaking existing apps)
        self.middleware
            .register_dual("csp", CspMiddleware::permissive());

        log::info!("Default security middleware enabled (security headers, input validation, CSP in report-only mode)");
        log::info!("For rate limiting, explicitly add: .middleware_from(|r| r.register_dual(\"rate_limit\", RateLimitMiddleware::default()))");

        self
    }

    /// Enable gzip response compression middleware.
    ///
    /// Responses are compressed only when:
    /// - The client sends `Accept-Encoding: gzip`
    /// - The body is at least 256 bytes
    /// - The `Content-Type` is compressible (text, JSON, JS, XML, …)
    /// - The response is not already encoded
    ///
    /// # Example
    /// ```rust,ignore
    /// let app = RustF::new()
    ///     .with_compression()
    ///     .controllers(auto_controllers!());
    /// ```
    pub fn with_compression(mut self) -> Self {
        use crate::middleware::builtin::CompressionMiddleware;
        self.middleware
            .register_outbound("compression", CompressionMiddleware::new());
        log::info!("Gzip compression middleware enabled");
        self
    }

    /// Enable HTTP method-override middleware.
    ///
    /// Upgrades a POST request to PUT/PATCH/DELETE when the request carries
    /// a `_method=<METHOD>` field (in query string or form body). Required
    /// for the HTML forms emitted by `rustf-cli new crud` to work with
    /// their PUT and DELETE routes, since browsers only natively send GET
    /// and POST. Matches the Rails / Express / Phoenix convention.
    ///
    /// # Example
    /// ```rust,ignore
    /// let app = RustF::new()
    ///     .with_method_override()
    ///     .controllers(auto_controllers!());
    /// ```
    pub fn with_method_override(mut self) -> Self {
        use crate::middleware::builtin::MethodOverrideMiddleware;
        self.middleware
            .register_inbound("method_override", MethodOverrideMiddleware::new());
        log::info!("HTTP method-override middleware enabled");
        self
    }

    /// Register multiple middleware from a function (for auto-discovery)
    ///
    /// # Example
    /// ```rust,ignore
    /// let app = RustF::new()
    ///     .middleware_from(auto_middleware!());
    /// ```
    pub fn middleware_from<F>(mut self, register_fn: F) -> Self
    where
        F: FnOnce(&mut MiddlewareRegistry),
    {
        register_fn(&mut self.middleware);
        self
    }

    /// Register multiple shared modules from a function (for auto-discovery)
    ///
    /// # Example
    /// ```rust,ignore
    /// let app = RustF::new()
    ///     .modules_from(auto_modules!());
    /// ```
    pub fn modules_from<F>(mut self, register_fn: F) -> Self
    where
        F: FnOnce(&mut SharedRegistry),
    {
        let shared = Arc::get_mut(&mut self.shared)
            .expect("SharedRegistry should not be shared during configuration");
        register_fn(shared);
        self
    }

    /// Enable workers and initialize the worker manager
    ///
    /// # Example
    /// ```rust,ignore
    /// let app = RustF::new()
    ///     .with_workers();
    /// ```
    pub fn with_workers(mut self) -> Self {
        self.ensure_worker_manager();
        self
    }

    fn ensure_worker_manager(&mut self) {
        if self.workers.is_some() {
            return;
        }

        match WorkerManager::with_config(self.config.clone()) {
            Ok(manager) => {
                let manager = Arc::new(manager);
                if let Err(e) = crate::workers::initialize_with_manager(manager.clone()) {
                    log::error!("Failed to initialize worker system: {}", e);
                } else {
                    self.workers = Some(manager);
                    log::info!("Worker system initialized");
                }
            }
            Err(e) => {
                log::error!("Failed to create worker manager: {}", e);
            }
        }
    }

    /// Register workers from a closure
    ///
    /// # Example
    /// ```rust,ignore
    /// let app = RustF::new()
    ///     .with_workers()
    ///     .workers_from(|manager| {
    ///         manager.spawn("email-sender", email_handler);
    ///         manager.recurring("cleanup", Duration::from_secs(3600), cleanup_handler);
    ///     });
    /// ```
    pub fn workers_from<F, Fut>(mut self, register_fn: F) -> Self
    where
        F: FnOnce(Arc<WorkerManager>) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        // Enable workers if not already enabled
        self.ensure_worker_manager();

        if let Some(manager) = &self.workers {
            let manager = manager.clone();
            tokio::spawn(async move {
                if let Err(e) = register_fn(manager).await {
                    log::error!("Error registering workers: {}", e);
                }
            });
        }

        self
    }

    /// Get the worker manager if workers are enabled
    pub fn workers(&self) -> Option<&Arc<WorkerManager>> {
        self.workers.as_ref()
    }

    pub fn static_files(mut self, url_prefix: &str, directory: &str) -> Self {
        self.static_dirs
            .insert(url_prefix.to_string(), PathBuf::from(directory));
        self
    }

    pub fn views(mut self, directory: &str) -> Self {
        Arc::get_mut(&mut self.views)
            .expect("Views should not be shared during configuration")
            .set_directory(directory);
        self
    }

    /// Register an event handler with default priority
    ///
    /// # Example
    /// ```rust,ignore
    /// let app = RustF::new()
    ///     .on("ready", |ctx| Box::pin(async move {
    ///         println!("Application ready!");
    ///         Ok(())
    ///     }));
    /// ```
    pub fn on<F>(self, event: &str, handler: F) -> Self
    where
        F: Fn(
                EventContext,
            )
                -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>>
            + Send
            + Sync
            + 'static,
    {
        // Get mutable access during configuration phase
        if let Ok(mut events) = self.events.try_write() {
            events.on(event, handler);
        } else {
            log::warn!(
                "Could not register event handler for '{}' - events system is locked",
                event
            );
        }
        self
    }

    /// Register an event handler with specific priority
    pub fn on_priority<F>(self, event: &str, priority: i32, handler: F) -> Self
    where
        F: Fn(
                EventContext,
            )
                -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>>
            + Send
            + Sync
            + 'static,
    {
        // Get mutable access during configuration phase
        if let Ok(mut events) = self.events.try_write() {
            events.on_priority(event, priority, handler);
        } else {
            log::warn!(
                "Could not register event handler for '{}' - events system is locked",
                event
            );
        }
        self
    }

    /// Register multiple event handlers from a function
    ///
    /// # Example
    /// ```rust,ignore
    /// let app = RustF::new()
    ///     .events_from(|emitter| {
    ///         emitter.on("ready", handler1);
    ///         emitter.on("startup", handler2);
    ///     });
    /// ```
    pub fn events_from<F>(self, register_fn: F) -> Self
    where
        F: FnOnce(&mut EventEmitter),
    {
        // Get mutable access during configuration phase
        if let Ok(mut events) = self.events.try_write() {
            register_fn(&mut events);
        } else {
            log::warn!("Could not register event handlers - events system is locked");
        }
        self
    }

    /// Configure event system performance settings
    ///
    /// # Example
    /// ```rust,ignore
    /// let app = RustF::new()
    ///     .event_config(EventEmitterConfig::parallel()
    ///         .with_timeout(Duration::from_secs(60))
    ///         .with_max_concurrent(4));
    /// ```
    pub fn event_config(self, config: crate::events::EventEmitterConfig) -> Self {
        if let Ok(mut events) = self.events.try_write() {
            events.set_config(config);
        } else {
            log::warn!("Could not update event configuration - events system is locked");
        }
        self
    }

    /// Register definitions (providers, helpers, validators, interceptors)
    ///
    /// # Example
    /// ```rust,ignore
    /// let app = RustF::new()
    ///     .definitions(|defs| {
    ///         defs.register_provider(MySessionProvider::new());
    ///         defs.register_helper("custom", MyHelper);
    ///         defs.register_validator("custom", MyValidator);
    ///     });
    /// ```
    pub fn definitions<F>(self, install_fn: F) -> Self
    where
        F: FnOnce(&mut crate::definitions::Definitions),
    {
        // Load definitions during the configuration phase
        futures::executor::block_on(async {
            let mut defs = crate::definitions::get_mut().await;
            install_fn(&mut defs);
            if let Err(e) = defs.initialize().await {
                log::error!("Failed to initialize definitions: {}", e);
            }
        });
        self
    }

    /// Register definitions from a module's install function
    ///
    /// # Example
    /// ```rust,ignore
    /// let app = RustF::new()
    ///     .definitions_from(my_app::definitions::install);
    /// ```
    pub fn definitions_from(self, install_fn: crate::definitions::InstallFn) -> Self {
        self.definitions(install_fn)
    }

    /// Auto-load ALL components WITHOUT builtin middleware
    ///
    /// This is the recommended method for basic setup. It automatically loads:
    ///
    /// **Custom components from conventional directories:**
    /// - `src/controllers/*.rs` → Routes
    /// - `src/models/*.rs` → Database models
    /// - `src/workers/*.rs` → Background workers
    /// - `src/middleware/*.rs` → Custom middleware
    /// - `src/events/*.rs` → Event handlers
    /// - `src/definitions/*.rs` → Helpers & validators
    ///
    /// **Note:** Builtin middleware is NOT enabled by default.
    /// Use `auto_load_with()` to explicitly enable builtin middleware.
    ///
    /// **Session middleware** is enabled by default but can be disabled via config.toml.
    ///
    /// # Example
    /// ```rust,ignore
    /// #[rustf::auto_discover]
    /// #[tokio::main]
    /// async fn main() -> rustf::Result<()> {
    ///     RustF::new()
    ///         .auto_load()  // No builtin middleware
    ///         .start().await
    /// }
    /// ```
    ///
    /// # To Enable Builtin Middleware
    /// Use `auto_load_with()` instead:
    ///
    /// ```rust,ignore
    /// RustF::new()
    ///     .auto_load_with(&["logging", "cors", "rate_limit", "csrf"])
    ///     .start().await
    /// ```
    pub fn auto_load(self) -> Self {
        self.auto_load_with(&[])
    }

    /// Auto-load ALL components with specified builtin middleware
    ///
    /// This is the recommended method for production use. Use this to explicitly enable
    /// the builtin middleware you need for your application.
    ///
    /// **Note:** Session middleware is enabled by default (configurable via config.toml).
    ///
    /// # Arguments
    /// * `builtin_middleware` - Slice of middleware names to enable
    ///
    ///   Available builtin middleware:
    ///   - `"logging"` - Request/response logging
    ///   - `"cors"` - Cross-Origin Resource Sharing
    ///   - `"rate_limit"` - Rate limiting
    ///   - `"csrf"` - CSRF protection
    ///
    /// # Examples
    /// ```rust,ignore
    /// // Production setup with all security middleware
    /// RustF::new()
    ///     .auto_load_with(&["logging", "cors", "rate_limit", "csrf"])
    ///     .start().await
    ///
    /// // API server with logging and CORS only
    /// RustF::new()
    ///     .auto_load_with(&["logging", "cors"])
    ///     .start().await
    ///
    /// // Minimal setup (same as auto_load())
    /// RustF::new()
    ///     .auto_load_with(&[])
    ///     .start().await
    ///
    /// // Web app with CSRF protection
    /// RustF::new()
    ///     .auto_load_with(&["logging", "csrf"])
    ///     .start().await
    /// ```
    ///
    /// # Configuration
    /// Each builtin middleware reads its configuration from `config.toml`:
    ///
    /// ```toml
    /// [middleware.logging]
    /// # Logging is configured via RUST_LOG environment variable
    ///
    /// [middleware.cors]
    /// allow_origin = "*"
    /// allow_methods = ["GET", "POST", "PUT", "DELETE", "OPTIONS"]
    /// allow_credentials = false
    ///
    /// [middleware.rate_limit]
    /// max_requests = 100
    /// window_seconds = 60
    ///
    /// [middleware.csrf]
    /// exempt_routes = ["/api/*", "/webhook/*"]
    /// error_message = "CSRF validation failed"
    /// ```
    pub fn auto_load_with(mut self, builtin_middleware: &[&str]) -> Self {
        if let Some(hooks) = crate::auto::hooks() {
            if let Some(controllers) = hooks.controllers {
                self = self.controllers(controllers());
            }

            if let Some(models) = hooks.models {
                self = self.models(models);
            }

            if let Some(shared) = hooks.shared {
                self = self.modules_from(shared);
            }

            if let Some(definitions) = hooks.definitions {
                self = self.definitions(definitions);
            }

            if let Some(middleware) = hooks.middleware {
                self = self.middleware_from(middleware);
            }

            if let Some(events) = hooks.events {
                self = self.events_from(events);
            }

            if let Some(workers_installer) = hooks.workers {
                self = self.with_workers().workers_from(workers_installer);
            }
        } else {
            log::warn!(
                "Auto-discovery hooks not registered. Ensure #[rustf::auto_discover] is applied to your entry point."
            );
        }

        if !builtin_middleware.is_empty() {
            self = self.load_builtin_middleware(builtin_middleware);
        }

        self
    }

    /// Load builtin middleware with configuration from config.toml
    ///
    /// This is a private helper method that instantiates builtin middleware
    /// using their `.from_config()` methods which read from config.toml.
    fn load_builtin_middleware(self, middleware_names: &[&str]) -> Self {
        self.middleware_from(move |registry| {
            use crate::middleware::builtin::{LoggingMiddleware, CorsMiddleware, RateLimitMiddleware};
            use crate::security::CsrfMiddleware;

            for name in middleware_names {
                match *name {
                    "logging" => {
                        registry.register_dual("logging", LoggingMiddleware::new());
                    }
                    "cors" => {
                        let cors = CorsMiddleware::from_config();
                        registry.register_dual("cors", cors);
                    }
                    "rate_limit" => {
                        let rate_limit = RateLimitMiddleware::from_config();
                        registry.register_dual("rate_limit", rate_limit);
                    }
                    "csrf" => {
                        let csrf = CsrfMiddleware::from_config();
                        registry.register_inbound("csrf", csrf);
                    }
                    unknown => {
                        log::warn!("Unknown builtin middleware '{}' - ignoring. Available: logging, cors, rate_limit, csrf", unknown);
                    }
                }
            }
        })
    }

    pub async fn serve(mut self, addr: Option<&str>) -> Result<()> {
        // Initialize global configuration access (CONF)
        if let Err(e) = crate::configuration::CONF::init((*self.config).clone()) {
            log::error!("Failed to initialize global configuration: {}", e);
            return Err(e);
        }
        log::info!("Global configuration (CONF) initialized");

        // Initialize global repository (APP/MAIN)
        if let Err(e) = crate::repository::APP::init(serde_json::json!({})) {
            log::error!("Failed to initialize global repository (APP/MAIN): {}", e);
            return Err(e);
        }
        log::info!("Global repository (APP/MAIN) initialized");

        // Note: MODULE system is NOT initialized by framework anymore.
        // Developers must explicitly call MODULE::init() in their app code
        // to initialize the module system for explicit named registration.
        // This ensures developers have full control over module registration timing.

        // Emit config.loaded event
        if let Err(e) = self
            .events
            .read()
            .await
            .emit(
                crate::events::events::CONFIG_LOADED,
                None,
                self.config.clone(),
            )
            .await
        {
            log::error!("Error emitting config.loaded event: {}", e);
        }

        // Initialize database connection if configured
        #[cfg(feature = "database")]
        {
            use crate::db::DB;
            if let Err(e) = DB::init(self.config.database.url.as_deref()).await {
                log::error!("Database initialization failed: {}", e);
                return Err(e);
            }

            // Emit database.ready event
            if let Err(e) = self
                .events
                .read()
                .await
                .emit(
                    crate::events::events::DATABASE_READY,
                    None,
                    self.config.clone(),
                )
                .await
            {
                log::error!("Error emitting database.ready event: {}", e);
            }
        }

        // Auto-register session middleware if enabled in config (dual-phase)
        // This must be done after database initialization as it may need DB/Redis connections
        if let Some(session_middleware) =
            crate::session::config_adapter::create_session_middleware(&self.config).await
        {
            self.middleware.register_dual("session", session_middleware);
        }

        // Initialize shared modules
        if let Err(e) = self.shared.initialize_all().await {
            log::error!("Shared modules initialization failed: {}", e);
            return Err(crate::error::Error::internal(format!(
                "Module initialization failed: {}",
                e
            )));
        }

        // Emit modules.ready event
        if let Err(e) = self
            .events
            .read()
            .await
            .emit(
                crate::events::events::MODULES_READY,
                None,
                self.config.clone(),
            )
            .await
        {
            log::error!("Error emitting modules.ready event: {}", e);
        }

        // Initialize global VIEW API for inline template rendering
        if let Err(e) = crate::views::api::initialize_global_view(self.views.clone()) {
            log::warn!("Failed to initialize global VIEW API: {}", e);
        } else {
            log::info!("Global VIEW API initialized");
        }

        // Emit middleware.ready event
        if !self.middleware.is_empty() {
            if let Err(e) = self
                .events
                .read()
                .await
                .emit(
                    crate::events::events::MIDDLEWARE_READY,
                    None,
                    self.config.clone(),
                )
                .await
            {
                log::error!("Error emitting middleware.ready event: {}", e);
            }
        }

        // Emit routes.ready event
        if let Err(e) = self
            .events
            .read()
            .await
            .emit(
                crate::events::events::ROUTES_READY,
                None,
                self.config.clone(),
            )
            .await
        {
            log::error!("Error emitting routes.ready event: {}", e);
        }

        // Emit startup event before starting server
        if let Err(e) = self
            .events
            .read()
            .await
            .emit(crate::events::events::STARTUP, None, self.config.clone())
            .await
        {
            log::error!("Error emitting startup event: {}", e);
        }

        // Emit ready event - framework is fully initialized
        if let Err(e) = self
            .events
            .read()
            .await
            .emit(crate::events::events::READY, None, self.config.clone())
            .await
        {
            log::error!("Error emitting ready event: {}", e);
        }

        let config_addr = self.config.server_address();
        let server_addr = addr.unwrap_or(&config_addr);
        let server = Server::new(self);
        server.serve(server_addr).await
    }

    pub async fn start(self) -> Result<()> {
        self.serve(None).await
    }

    /// Perform graceful cleanup during shutdown
    ///
    /// This method is called automatically when the server receives a shutdown signal.
    /// It ensures all resources are properly released in the correct order.
    pub(crate) async fn cleanup(&self) -> Result<()> {
        use std::time::Duration;

        log::info!("Starting graceful shutdown sequence...");

        // 1. Emit shutdown event for custom handlers
        if let Err(e) = self
            .events
            .read()
            .await
            .emit(crate::events::events::SHUTDOWN, None, self.config.clone())
            .await
        {
            log::error!("Error emitting shutdown event: {}", e);
        }

        // Give event handlers a moment to process
        tokio::time::sleep(Duration::from_millis(100)).await;

        // 2. Shutdown workers if enabled
        if let Some(workers) = &self.workers {
            log::info!("Shutting down workers...");
            if let Err(e) = workers.shutdown_all().await {
                log::error!("Error shutting down workers: {}", e);
            }
        }

        // 3. Shutdown shared modules
        log::info!("Shutting down shared modules...");
        if let Err(e) = self.shared.shutdown_all().await {
            log::error!("Error shutting down shared modules: {}", e);
        }

        // 4. Close database connections
        #[cfg(feature = "database")]
        {
            use crate::db::DB;
            if let Err(e) = DB::shutdown().await {
                log::error!("Error closing database connections: {}", e);
            }
        }

        // 4. Cleanup complete
        log::info!("Shutdown complete");

        log::info!("Graceful shutdown complete");
        Ok(())
    }

    pub async fn handle_request<B>(&self, req: hyper::Request<B>) -> Result<Response>
    where
        B: hyper::body::Body,
        B::Error: std::fmt::Display,
    {
        self.handle_request_with_peer(req, None).await
    }

    pub(crate) async fn handle_request_with_peer<B>(
        &self,
        req: hyper::Request<B>,
        peer_addr: Option<SocketAddr>,
    ) -> Result<Response>
    where
        B: hyper::body::Body,
        B::Error: std::fmt::Display,
    {
        let request =
            Request::from_hyper_with_connection(req, peer_addr, Arc::clone(&self.trusted_proxies))
                .await?;

        // Check for static files first (match prefix safely using request path without query)
        let request_path = request.path().to_string();
        for (prefix, dir) in &self.static_dirs {
            if let Some(relative_suffix) = Self::match_static_prefix(&request_path, prefix) {
                // Extract conditional request headers before moving `request`
                let if_none_match = request.headers.get("if-none-match").cloned();
                let if_modified_since = request.headers.get("if-modified-since").cloned();
                return self
                    .serve_static_file(
                        dir,
                        relative_suffix,
                        &request_path,
                        if_none_match.as_deref(),
                        if_modified_since.as_deref(),
                    )
                    .await;
            }
        }

        // Create memory-safe context with Arc references
        let mut context = Context::new(request, Arc::clone(&self.views));

        // Execute middleware chain + route handler
        let result = self.execute_middleware_chain(&mut context).await?;

        // Get response from result
        let response = match result {
            MiddlewareResult::Continue => {
                // This should not happen - the final handler should always return Stop
                log::warn!("Middleware chain returned Continue but no more handlers available");
                Response::internal_error()
            }
            MiddlewareResult::Stop(response) => response,
        };

        Ok(response)
    }

    /// Execute the dual-phase middleware chain followed by the route handler
    ///
    /// This implements the inbound/outbound pattern where middleware can process
    /// requests on the way in and responses on the way out.
    async fn execute_middleware_chain(&self, ctx: &mut Context) -> Result<MiddlewareResult> {
        use crate::middleware::InboundAction;
        // Fast path: no middleware registered
        if self.middleware.is_empty() {
            return self.execute_route_handler(ctx).await;
        }

        // Get sorted middleware (by priority) — zero-cost slice borrow
        let middleware_list = self.middleware.get_sorted();
        let mut outbound_stack = Vec::new();

        // Phase 1: INBOUND - Process request through middleware
        for middleware in middleware_list {
            if middleware.inbound.is_none() && middleware.has_outbound() {
                outbound_stack.push(middleware);
                continue;
            }

            // Only process if middleware has inbound phase and should run
            if let Some(ref inbound) = middleware.inbound {
                if inbound.should_run(ctx) {
                    // Process the request (now async)
                    let action = inbound.process_request(ctx).await?;

                    match action {
                        InboundAction::Continue => {
                            // Check if this middleware wants to process response
                            if middleware.has_outbound() {
                                outbound_stack.push(middleware);
                            }
                        }
                        InboundAction::Stop => {
                            if middleware.has_outbound() {
                                outbound_stack.push(middleware);
                            }

                            // Early return - use response set on context
                            let response =
                                ctx.take_response().unwrap_or_else(Response::internal_error);
                            ctx.set_response(response);
                            self.run_outbound_stack(ctx, &outbound_stack).await?;
                            let final_response =
                                ctx.take_response().unwrap_or_else(Response::internal_error);
                            return Ok(MiddlewareResult::Stop(final_response));
                        }
                        InboundAction::Capture => {
                            // This middleware wants to process the response
                            outbound_stack.push(middleware);
                        }
                    }
                }
            }
        }

        // Phase 2: Execute the route handler
        let result = self.execute_route_handler(ctx).await?;

        let response = match result {
            MiddlewareResult::Stop(resp) => resp,
            MiddlewareResult::Continue => Response::internal_error(),
        };

        // Set the response in context for outbound middleware to access
        ctx.set_response(response);

        // Phase 3: OUTBOUND - Process response through middleware (in reverse order)
        // Context has all modifications from both inbound middleware and handler
        self.run_outbound_stack(ctx, &outbound_stack).await?;

        // Get the final response from context
        let final_response = ctx.take_response().unwrap_or_else(Response::internal_error);

        Ok(MiddlewareResult::Stop(final_response))
    }

    async fn run_outbound_stack(
        &self,
        ctx: &mut Context,
        outbound_stack: &[&crate::middleware::DualPhaseMiddlewareInstance],
    ) -> Result<()> {
        for middleware in outbound_stack.iter().rev() {
            if let Some(ref outbound) = middleware.outbound {
                outbound.process_response(ctx).await?;
            }
        }

        Ok(())
    }

    /// Execute the route handler (final step in the chain)
    async fn execute_route_handler(&self, ctx: &mut Context) -> Result<MiddlewareResult> {
        // Try to match route. Use `path()` (not the raw `uri`) so HTTP/2 — where
        // the URI is the absolute `http://host/path` form — routes like HTTP/1.
        if let Some((route_info, params)) = self.router.match_route(&ctx.req.method, ctx.req.path())
        {
            // Check XHR constraint if the route requires it
            if route_info.xhr_only && !ctx.is_xhr() {
                return Ok(MiddlewareResult::Stop(
                    Response::forbidden(Some("This endpoint requires an AJAX (XHR) request"))
                        .with_header("Content-Type", "application/json")
                        .with_body(
                            r#"{"error":"This endpoint requires an AJAX (XHR) request"}"#.into(),
                        ),
                ));
            }

            ctx.req.params = params;

            // Run controller-level `before` hook if any (wired via the
            // `routes![before: ..., ...]` macro arm). If it returns
            // `Stop`, skip the handler and return the response the hook
            // already wrote to ctx.res.
            if let Some(before) = route_info.before {
                use crate::routing::BeforeAction;
                match before(ctx).await? {
                    BeforeAction::Stop => {
                        let response = ctx.take_response().unwrap_or_else(Response::internal_error);
                        return Ok(MiddlewareResult::Stop(response));
                    }
                    BeforeAction::Continue => { /* fall through to handler */ }
                }
            }

            // Handler modifies context in place (sets response)
            (route_info.handler)(ctx).await?;

            // Get the response from context or return 500 if not set
            let response = ctx.take_response().unwrap_or_else(Response::internal_error);

            Ok(MiddlewareResult::Stop(response))
        } else {
            Ok(MiddlewareResult::Stop(Response::not_found()))
        }
    }

    async fn serve_static_file(
        &self,
        base_dir: &PathBuf,
        relative_suffix: &str,
        full_request_path: &str,
        if_none_match: Option<&str>,
        if_modified_since: Option<&str>,
    ) -> Result<Response> {
        let cache_enabled = self.config.static_files.cache_enabled;
        let cache_max_age = self.config.static_files.cache_max_age;

        // 1. Try the embedded assets provider first (when feature is active)
        if let Some(provider) = crate::views::embed_provider::get_assets_provider() {
            // Normalise the lookup key: prefer the suffix without prefix,
            // fall back to the full path (without leading slash).
            let candidates: &[&str] = &[relative_suffix, full_request_path.trim_start_matches('/')];
            for key in candidates {
                if key.is_empty() {
                    continue;
                }
                if let Some(data) = provider.get(key) {
                    let content_type = Self::infer_content_type(Path::new(key));
                    let response = if cache_enabled {
                        use std::collections::hash_map::DefaultHasher;
                        use std::hash::{Hash, Hasher};
                        let mut hasher = DefaultHasher::new();
                        data.hash(&mut hasher);
                        let etag = format!("\"embed-{:x}\"", hasher.finish());
                        if if_none_match == Some(etag.as_str()) {
                            Response::not_modified()
                                .with_header("ETag", &etag)
                                .with_header("X-Content-Type-Options", "nosniff")
                                .with_header(
                                    "Cache-Control",
                                    &format!("public, max-age={}", cache_max_age),
                                )
                        } else {
                            Response::ok()
                                .with_header("Content-Type", content_type)
                                .with_header("X-Content-Type-Options", "nosniff")
                                .with_header(
                                    "Cache-Control",
                                    &format!("public, max-age={}", cache_max_age),
                                )
                                .with_header("ETag", &etag)
                                .with_header("Content-Length", &data.len().to_string())
                                .with_body(data)
                        }
                    } else {
                        Response::ok()
                            .with_header("Content-Type", content_type)
                            .with_header("X-Content-Type-Options", "nosniff")
                            .with_header("Cache-Control", "no-store, no-cache")
                            .with_body(data)
                    };
                    return Ok(response);
                }
            }
        }

        let canonical_base = match base_dir.canonicalize() {
            Ok(path) => path,
            Err(_) => return Ok(Response::not_found()),
        };

        // 2. Try the suffix after the prefix first (preferred behaviour)
        if let Some(candidate) = Self::sanitize_and_join(&canonical_base, relative_suffix) {
            if let Some(response) = Self::try_read_static_file(
                &candidate,
                &canonical_base,
                cache_enabled,
                cache_max_age,
                if_none_match,
                if_modified_since,
            )
            .await?
            {
                return Ok(response);
            }
        }

        // 3. Fall back to historical behaviour (prefix included) for compatibility
        let trimmed_full = full_request_path.trim_start_matches('/');
        if let Some(candidate) = Self::sanitize_and_join(&canonical_base, trimmed_full) {
            if let Some(response) = Self::try_read_static_file(
                &candidate,
                &canonical_base,
                cache_enabled,
                cache_max_age,
                if_none_match,
                if_modified_since,
            )
            .await?
            {
                return Ok(response);
            }
        }

        Ok(Response::not_found())
    }

    fn match_static_prefix<'a>(path: &'a str, prefix: &'a str) -> Option<&'a str> {
        if prefix.is_empty() {
            return None;
        }

        let mut normalized_prefix = prefix;
        if normalized_prefix == "/" {
            return Some(path.trim_start_matches('/'));
        }

        if !normalized_prefix.starts_with('/') {
            return None;
        }

        if normalized_prefix.len() > 1 && normalized_prefix.ends_with('/') {
            normalized_prefix = &normalized_prefix[..normalized_prefix.len() - 1];
        }

        if path == normalized_prefix {
            return Some("");
        }

        if let Some(rest) = path.strip_prefix(normalized_prefix) {
            if rest.starts_with('/') {
                return Some(&rest[1..]);
            }
        }

        None
    }

    fn sanitize_and_join(base: &Path, candidate: &str) -> Option<PathBuf> {
        if candidate.is_empty() {
            return None;
        }

        let mut clean = PathBuf::new();
        for component in Path::new(candidate).components() {
            match component {
                Component::Normal(segment) => clean.push(segment),
                Component::CurDir => continue,
                _ => return None, // Reject parent dirs or root prefixes
            }
        }

        Some(base.join(clean))
    }

    async fn try_read_static_file(
        path: &Path,
        canonical_base: &Path,
        cache_enabled: bool,
        cache_max_age: u64,
        if_none_match: Option<&str>,
        if_modified_since: Option<&str>,
    ) -> Result<Option<Response>> {
        use std::time::{SystemTime, UNIX_EPOCH};
        use tokio::io::AsyncReadExt;

        // Open the file once. This resolves the path a single time and gives us
        // a file descriptor we can both stat and read — no TOCTOU race, one fewer
        // syscall on cache misses compared to separate metadata() + read() calls.
        let mut file = match tokio::fs::File::open(path).await {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        let canonical_path = match tokio::fs::canonicalize(path).await {
            Ok(path) => path,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        if !canonical_path.starts_with(canonical_base) {
            return Ok(None);
        }

        // fstat() on the open fd — no second path lookup
        let metadata = file.metadata().await?;
        if !metadata.is_file() {
            return Ok(None);
        }

        let content_type = Self::infer_content_type(&canonical_path);

        if cache_enabled {
            let mtime = metadata
                .modified()
                .unwrap_or(SystemTime::UNIX_EPOCH)
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let etag = format!("\"{:x}-{:x}\"", mtime, metadata.len());
            let last_modified = Self::format_http_date(mtime);

            // 304 checks — file is already open but we return before reading content
            if let Some(client_etag) = if_none_match {
                if client_etag == etag {
                    return Ok(Some(
                        Response::not_modified()
                            .with_header("ETag", &etag)
                            .with_header("X-Content-Type-Options", "nosniff")
                            .with_header(
                                "Cache-Control",
                                &format!("public, max-age={}", cache_max_age),
                            ),
                    ));
                }
            }
            if let Some(ims_header) = if_modified_since {
                if let Some(ims_ts) = Self::parse_http_date(ims_header) {
                    if mtime <= ims_ts {
                        return Ok(Some(
                            Response::not_modified()
                                .with_header("ETag", &etag)
                                .with_header("X-Content-Type-Options", "nosniff")
                                .with_header(
                                    "Cache-Control",
                                    &format!("public, max-age={}", cache_max_age),
                                ),
                        ));
                    }
                }
            }

            // Cache miss — read from the already-open fd, pre-allocate from known size
            let mut content = Vec::with_capacity(metadata.len() as usize);
            file.read_to_end(&mut content).await?;

            Ok(Some(
                Response::ok()
                    .with_header("Content-Type", content_type)
                    .with_header("X-Content-Type-Options", "nosniff")
                    .with_header(
                        "Cache-Control",
                        &format!("public, max-age={}", cache_max_age),
                    )
                    .with_header("ETag", &etag)
                    .with_header("Last-Modified", &last_modified)
                    .with_header("Content-Length", &content.len().to_string())
                    .with_body(content),
            ))
        } else {
            // No caching — read from the already-open fd, pre-allocate from known size
            let mut content = Vec::with_capacity(metadata.len() as usize);
            file.read_to_end(&mut content).await?;

            Ok(Some(
                Response::ok()
                    .with_header("Content-Type", content_type)
                    .with_header("X-Content-Type-Options", "nosniff")
                    .with_header("Cache-Control", "no-store, no-cache")
                    .with_body(content),
            ))
        }
    }

    /// Format a Unix timestamp as an RFC 7231 HTTP date string.
    fn format_http_date(timestamp: u64) -> String {
        crate::utils::http_date::format_http_date(timestamp)
    }

    /// Parse an HTTP date header (RFC 7231, RFC 850, or asctime) to a Unix timestamp.
    fn parse_http_date(date_str: &str) -> Option<u64> {
        crate::utils::http_date::parse_http_date(date_str)
    }

    fn infer_content_type(path: &Path) -> &'static str {
        let ext = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase())
            .unwrap_or_default();

        match ext.as_str() {
            "html" => "text/html",
            "css" => "text/css",
            "js" => "application/javascript",
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "svg" => "image/svg+xml",
            "webp" => "image/webp",
            "json" => "application/json",
            "txt" => "text/plain",
            "ico" => "image/x-icon",
            _ => "application/octet-stream",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RustF;
    use tempfile::TempDir;

    #[tokio::test]
    async fn app_static_serving_sets_nosniff_header() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("app.js");
        tokio::fs::write(&file_path, b"console.log('ok');")
            .await
            .unwrap();

        let canonical_base = temp_dir.path().canonicalize().unwrap();
        let response =
            RustF::try_read_static_file(&file_path, &canonical_base, false, 0, None, None)
                .await
                .unwrap()
                .unwrap();

        assert!(response
            .headers
            .iter()
            .any(|(name, value)| name == "X-Content-Type-Options" && value == "nosniff"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn app_static_serving_rejects_symlink_escape() {
        use std::os::unix::fs::symlink;

        let public_dir = TempDir::new().unwrap();
        let outside_dir = TempDir::new().unwrap();
        let outside_file = outside_dir.path().join("secret.txt");
        tokio::fs::write(&outside_file, b"secret").await.unwrap();

        let link_path = public_dir.path().join("outside-link");
        symlink(outside_dir.path(), &link_path).unwrap();

        let canonical_base = public_dir.path().canonicalize().unwrap();
        let escaped_path = link_path.join("secret.txt");
        let response =
            RustF::try_read_static_file(&escaped_path, &canonical_base, false, 0, None, None)
                .await
                .unwrap();

        assert!(response.is_none());
    }
}
