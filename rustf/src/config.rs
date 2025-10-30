use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

/// Environment type for configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum Environment {
    #[default]
    Development,
    Production,
}

impl Environment {
    /// Get environment from string
    pub fn from_str(env: &str) -> Self {
        match env.to_lowercase().as_str() {
            "production" | "prod" => Environment::Production,
            "development" | "dev" => Environment::Development,
            _ => Environment::Development,
        }
    }

    /// Get environment name as string
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Development => "dev",
            Environment::Production => "prod",
        }
    }

    /// Check if this is a production environment
    pub fn is_production(&self) -> bool {
        matches!(self, Environment::Production)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub environment: Environment,

    #[serde(default)]
    pub server: ServerConfig,

    #[serde(default)]
    pub views: ViewConfig,

    #[serde(default)]
    pub session: SessionConfig,

    #[serde(default)]
    pub static_files: StaticConfig,

    #[serde(default)]
    pub database: DatabaseConfig,

    #[serde(default)]
    pub cors: CorsConfig,

    #[serde(default)]
    pub logging: LoggingConfig,

    #[serde(default)]
    pub uploads: UploadConfig,

    // All other sections - user-defined configuration sections
    // These sections are stored as TOML values and can be deserialized on-demand
    #[serde(flatten)]
    pub sections: HashMap<String, toml::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,

    #[serde(default = "default_port")]
    pub port: u16,

    #[serde(default = "default_timeout")]
    pub timeout: u64,

    #[serde(default)]
    pub ssl_enabled: bool,

    #[serde(default)]
    pub ssl_cert: Option<String>,

    #[serde(default)]
    pub ssl_key: Option<String>,

    #[serde(default = "default_max_connections")]
    pub max_connections: usize,

    #[serde(default = "default_shutdown_timeout")]
    pub shutdown_timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewConfig {
    #[serde(default = "default_views_dir")]
    pub directory: String,

    #[serde(default = "default_layout")]
    pub default_layout: String,

    #[serde(default)]
    pub cache_enabled: bool,

    #[serde(default = "default_extension")]
    pub extension: String,

    #[serde(default)]
    pub engine: TemplateEngine,

    #[serde(default)]
    pub storage: TemplateStorage,

    #[serde(default = "default_root")]
    pub default_root: String,
}

/// Template engine - Total.js is the only supported engine
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum TemplateEngine {
    /// Total.js template engine (the only supported engine)
    #[default]
    TotalJs,
}

/// Template storage method - runtime configurable
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum TemplateStorage {
    /// Load templates from filesystem at runtime (supports --views CLI flag)
    Filesystem,
    /// Templates embedded at compile time (ignores --views CLI flag)
    #[default]
    Embedded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    #[serde(default = "default_session_enabled")]
    pub enabled: bool,

    #[serde(default = "default_cookie_name")]
    pub cookie_name: String,

    #[serde(default = "default_idle_timeout")]
    pub idle_timeout: u64,

    #[serde(default = "default_absolute_timeout")]
    pub absolute_timeout: u64,

    #[serde(default = "default_same_site")]
    pub same_site: String,

    #[serde(default = "default_fingerprint_mode")]
    pub fingerprint_mode: String,

    #[serde(default)]
    pub storage: SessionStorageConfig,

    #[serde(default)]
    pub exempt_routes: Vec<String>,
}

/// Session storage backend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SessionStorageConfig {
    /// In-memory session storage (default)
    Memory {
        #[serde(default = "default_cleanup_interval")]
        cleanup_interval: u64,
    },
    /// Redis-based session storage
    Redis {
        url: String,
        #[serde(default = "default_redis_prefix")]
        prefix: String,
        #[serde(default = "default_redis_pool_size")]
        pool_size: usize,
        #[serde(default = "default_redis_connection_timeout")]
        connection_timeout: u64,
        #[serde(default = "default_redis_command_timeout")]
        command_timeout: u64,
    },
    /// Database-based session storage
    Database {
        #[serde(default = "default_sessions_table")]
        table: String,
        connection_url: Option<String>,
        #[serde(default = "default_cleanup_interval")]
        cleanup_interval: u64,
    },
}

impl Default for SessionStorageConfig {
    fn default() -> Self {
        Self::Memory {
            cleanup_interval: default_cleanup_interval(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticConfig {
    #[serde(default = "default_static_dir")]
    pub directory: String,

    #[serde(default = "default_static_prefix")]
    pub url_prefix: String,

    #[serde(default)]
    pub cache_enabled: bool,

    #[serde(default = "default_cache_max_age")]
    pub cache_max_age: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DatabaseConfig {
    #[serde(default)]
    pub url: Option<String>,

    #[serde(default)]
    pub max_connections: Option<u32>,

    #[serde(default)]
    pub timeout: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default)]
    pub allowed_origins: Vec<String>,

    #[serde(default)]
    pub allowed_methods: Vec<String>,

    #[serde(default)]
    pub allowed_headers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,

    #[serde(default)]
    pub file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadConfig {
    #[serde(default = "default_upload_dir")]
    pub directory: String,

    #[serde(default = "default_max_file_size")]
    pub max_file_size: u64,

    #[serde(default = "default_max_files")]
    pub max_files: usize,

    #[serde(default)]
    pub allowed_extensions: Vec<String>,

    #[serde(default)]
    pub blocked_extensions: Vec<String>,

    #[serde(default)]
    pub create_directories: bool,
}

// Default value functions
fn default_host() -> String {
    "127.0.0.1".to_string()
}
fn default_port() -> u16 {
    8000
}
fn default_timeout() -> u64 {
    30
}
fn default_shutdown_timeout() -> u64 {
    30
}
fn default_max_connections() -> usize {
    1000
}
fn default_views_dir() -> String {
    "views".to_string()
}
fn default_layout() -> String {
    "layouts/default".to_string()
}
fn default_extension() -> String {
    "html".to_string()
}
fn default_root() -> String {
    "".to_string()
}
fn default_session_enabled() -> bool {
    true
}
fn default_idle_timeout() -> u64 {
    900
} // 15 minutes
fn default_absolute_timeout() -> u64 {
    28800
} // 8 hours
fn default_same_site() -> String {
    "Lax".to_string()
}
fn default_fingerprint_mode() -> String {
    "soft".to_string()
}
fn default_cookie_name() -> String {
    "rustf_session".to_string()
}
fn default_static_dir() -> String {
    "public".to_string()
}
fn default_static_prefix() -> String {
    "/static".to_string()
}
fn default_cache_max_age() -> u64 {
    86400
}
fn default_log_level() -> String {
    "info".to_string()
}
fn default_upload_dir() -> String {
    "uploads".to_string()
}
fn default_max_file_size() -> u64 {
    10 * 1024 * 1024
} // 10MB
fn default_max_files() -> usize {
    5
}

// Session storage defaults
fn default_cleanup_interval() -> u64 {
    300
} // 5 minutes
fn default_redis_prefix() -> String {
    "rustf:session:".to_string()
}
fn default_redis_pool_size() -> usize {
    10
}
fn default_redis_connection_timeout() -> u64 {
    5000
} // 5 seconds
fn default_redis_command_timeout() -> u64 {
    3000
} // 3 seconds
fn default_sessions_table() -> String {
    "sessions".to_string()
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            timeout: default_timeout(),
            ssl_enabled: false,
            ssl_cert: None,
            ssl_key: None,
            max_connections: default_max_connections(),
            shutdown_timeout: default_shutdown_timeout(),
        }
    }
}

impl Default for ViewConfig {
    fn default() -> Self {
        Self {
            directory: default_views_dir(),
            default_layout: default_layout(),
            cache_enabled: false,
            extension: default_extension(),
            engine: TemplateEngine::default(),
            storage: TemplateStorage::default(),
            default_root: default_root(),
        }
    }
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            enabled: default_session_enabled(),
            cookie_name: default_cookie_name(),
            idle_timeout: default_idle_timeout(),
            absolute_timeout: default_absolute_timeout(),
            same_site: default_same_site(),
            fingerprint_mode: default_fingerprint_mode(),
            storage: SessionStorageConfig::default(),
            exempt_routes: Vec::new(),
        }
    }
}

impl Default for StaticConfig {
    fn default() -> Self {
        Self {
            directory: default_static_dir(),
            url_prefix: default_static_prefix(),
            cache_enabled: true,
            cache_max_age: default_cache_max_age(),
        }
    }
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            allowed_origins: vec!["*".to_string()],
            allowed_methods: vec!["GET".to_string(), "POST".to_string()],
            allowed_headers: vec!["Content-Type".to_string()],
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            file: None,
        }
    }
}

impl Default for UploadConfig {
    fn default() -> Self {
        Self {
            directory: default_upload_dir(),
            max_file_size: default_max_file_size(),
            max_files: default_max_files(),
            allowed_extensions: vec![],
            blocked_extensions: vec![
                "exe".to_string(),
                "bat".to_string(),
                "sh".to_string(),
                "cmd".to_string(),
            ],
            create_directories: true,
        }
    }
}

impl AppConfig {
    /// Load configuration with environment-specific overrides
    pub fn load() -> Result<Self> {
        Self::load_with_base_dir(".")
    }

    /// Load configuration with development-time auto-detection
    pub fn load_with_dev_detection() -> Result<Self> {
        // Check if we're in a development environment and look for config in target/debug
        let base_dir = if cfg!(debug_assertions) && Path::new("target/debug").exists() {
            // Look for config relative to target/debug (where the binary runs)
            if Path::new("../../config.toml").exists() {
                log::debug!("Development mode: Using config.toml from project root");
                "../.."
            } else {
                "."
            }
        } else {
            "."
        };

        Self::load_with_base_dir(base_dir)
    }

    /// Load configuration from a specific base directory
    pub fn load_with_base_dir<P: AsRef<Path>>(base_dir: P) -> Result<Self> {
        let base_dir = base_dir.as_ref();

        // Determine environment
        let env = Self::detect_environment();

        // Load base configuration as TOML Value
        let base_config_path = base_dir.join("config.toml");
        let mut merged_value = if base_config_path.exists() {
            Self::load_toml_value(&base_config_path)?
        } else {
            // Use default config as Value
            let default_config = AppConfig::default();
            toml::to_string(&default_config)
                .ok()
                .and_then(|s| toml::from_str(&s).ok())
                .unwrap_or(toml::Value::Table(toml::map::Map::new()))
        };

        // Load and merge environment-specific configuration if it exists
        let env_config_path = base_dir.join(format!("config.{}.toml", env.as_str()));
        if env_config_path.exists() {
            log::debug!(
                "Loading environment-specific config from: {}",
                env_config_path.display()
            );
            let env_value = Self::load_toml_value(&env_config_path)?;

            // Merge environment config into base (environment takes precedence)
            #[cfg(feature = "config")]
            {
                use serde_toml_merge::merge;
                merged_value = merge(merged_value, env_value).map_err(|e| {
                    Error::internal(format!("Failed to merge configuration files: {}", e))
                })?;
            }

            #[cfg(not(feature = "config"))]
            {
                // Fallback if feature is not enabled (shouldn't happen in practice)
                log::warn!(
                    "Config feature not enabled, skipping environment-specific config merge"
                );
            }
        }

        // Deserialize merged TOML value into AppConfig
        // Convert toml::Value to serde_json::Value for deserialization
        let json_value = serde_json::to_value(&merged_value).map_err(|e| {
            Error::internal(format!("Failed to convert merged configuration: {}", e))
        })?;

        let mut config: AppConfig = serde_json::from_value(json_value).map_err(|e| {
            Error::internal(format!("Failed to deserialize merged configuration: {}", e))
        })?;

        // Set detected environment
        config.environment = env.clone();

        // Apply environment variable overrides
        config.apply_env_overrides()?;

        // Resolve views directory path (supports both relative and absolute paths)
        config.resolve_views_directory(base_dir)?;

        // Apply security defaults for production
        if config.environment.is_production() {
            config.apply_security_defaults();
        }

        // Apply performance defaults for production
        if config.environment.is_production() {
            config.apply_performance_defaults();
        }

        // Validate configuration
        config.validate()?;

        log::info!(
            "Configuration loaded and merged successfully (environment: {})",
            config.environment.as_str()
        );

        Ok(config)
    }

    /// Load configuration from TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_ref = path.as_ref();

        // Enhanced error message with path information
        let content = fs::read_to_string(path_ref).map_err(|e| {
            Error::internal(format!(
                "Failed to read config file '{}': {}. Make sure the file exists and is readable.",
                path_ref.display(),
                e
            ))
        })?;

        let mut config: AppConfig = toml::from_str(&content).map_err(|e| {
            Error::internal(format!(
                "Failed to parse config file '{}': {}. Check TOML syntax.",
                path_ref.display(),
                e
            ))
        })?;

        // Resolve paths relative to the config file's directory
        if let Some(parent_dir) = path_ref.parent() {
            config.resolve_views_directory(parent_dir)?;
        } else {
            config.resolve_views_directory(".")?;
        }

        log::debug!(
            "Successfully loaded configuration from: {}",
            path_ref.display()
        );
        Ok(config)
    }

    /// Load TOML file as a Value (for merging before deserialization)
    ///
    /// This is used when merging multiple config files at the TOML level
    /// rather than at the AppConfig struct level.
    fn load_toml_value<P: AsRef<Path>>(path: P) -> Result<toml::Value> {
        let path_ref = path.as_ref();

        let content = fs::read_to_string(path_ref).map_err(|e| {
            Error::internal(format!(
                "Failed to read config file '{}': {}. Make sure the file exists and is readable.",
                path_ref.display(),
                e
            ))
        })?;

        let value: toml::Value = toml::from_str(&content).map_err(|e| {
            Error::internal(format!(
                "Failed to parse config file '{}': {}. Check TOML syntax.",
                path_ref.display(),
                e
            ))
        })?;

        log::debug!(
            "Successfully loaded TOML value from: {}",
            path_ref.display()
        );
        Ok(value)
    }

    /// Create configuration with environment variable overrides
    pub fn from_env() -> Result<Self> {
        let mut config = AppConfig::default();
        config.apply_env_overrides()?;
        Ok(config)
    }

    /// Detect current environment from environment variables
    pub fn detect_environment() -> Environment {
        // Check RUSTF_ENV first, then RAILS_ENV, then NODE_ENV for compatibility
        if let Ok(env) = env::var("RUSTF_ENV") {
            return Environment::from_str(&env);
        }
        if let Ok(env) = env::var("RAILS_ENV") {
            return Environment::from_str(&env);
        }
        if let Ok(env) = env::var("NODE_ENV") {
            return Environment::from_str(&env);
        }

        // Default to development
        Environment::Development
    }

    /// Apply security defaults for production environments
    pub fn apply_security_defaults(&mut self) {
        // Sessions are configured via new secure middleware
        // No longer need to override session settings here

        // Disable CORS by default in production (must be explicitly enabled)
        if !self.cors.enabled {
            self.cors.allowed_origins = vec![];
        }

        // Force SSL in production if not explicitly disabled
        if self.environment.is_production() && !self.server.ssl_enabled {
            eprintln!("WARNING: SSL is not enabled in production environment");
        }
    }

    /// Apply performance defaults for production environments
    pub fn apply_performance_defaults(&mut self) {
        // Enable view caching in production
        self.views.cache_enabled = true;

        // Enable static file caching
        self.static_files.cache_enabled = true;

        // Increase connection limits for production
        if self.server.max_connections == default_max_connections() {
            self.server.max_connections = 2000; // Higher default for production
        }

        // Set reasonable timeout for production
        if self.server.timeout == default_timeout() {
            self.server.timeout = 60; // 60 seconds for production
        }
    }

    /// Validate configuration for the current environment
    pub fn validate(&self) -> Result<()> {
        // Validate template engine configuration against enabled features
        self.validate_template_engine()?;

        // Validate server configuration
        if self.server.port == 0 {
            return Err(Error::internal("Server port cannot be 0"));
        }

        if self.server.ssl_enabled {
            if self.server.ssl_cert.is_none() {
                return Err(Error::internal(
                    "SSL certificate path required when SSL is enabled",
                ));
            }
            if self.server.ssl_key.is_none() {
                return Err(Error::internal(
                    "SSL private key path required when SSL is enabled",
                ));
            }
        }

        // Validate production-specific requirements
        if self.environment.is_production() && self.database.url.is_none() {
            eprintln!("WARNING: No database configured in production environment");
        }

        // Validate paths exist
        // Note: For views directory, we only validate for filesystem storage
        // and only warn if it doesn't exist (might be overridden by CLI)
        if matches!(self.views.storage, TemplateStorage::Filesystem)
            && !Path::new(&self.views.directory).exists()
        {
            // Only warn, don't error - might be overridden by CLI --views flag
            log::warn!(
                "Views directory does not exist: {} (can be overridden with --views flag)",
                self.views.directory
            );
        }

        if !Path::new(&self.static_files.directory).exists() {
            // Static files directory is more critical, so we still error
            return Err(Error::internal(format!(
                "Static files directory does not exist: {}",
                self.static_files.directory
            )));
        }

        Ok(())
    }

    /// Apply environment variable overrides
    fn apply_env_overrides(&mut self) -> Result<()> {
        // Environment override (highest priority)
        if let Ok(env) = env::var("RUSTF_ENV") {
            self.environment = Environment::from_str(&env);
        }

        // Server overrides
        if let Ok(host) = env::var("RUSTF_HOST") {
            self.server.host = host;
        }
        if let Ok(port) = env::var("RUSTF_PORT") {
            self.server.port = port
                .parse()
                .map_err(|_| Error::internal("Invalid RUSTF_PORT value"))?;
        }
        if let Ok(timeout) = env::var("RUSTF_TIMEOUT") {
            self.server.timeout = timeout
                .parse()
                .map_err(|_| Error::internal("Invalid RUSTF_TIMEOUT value"))?;
        }
        if let Ok(ssl) = env::var("RUSTF_SSL_ENABLED") {
            self.server.ssl_enabled = ssl
                .parse()
                .map_err(|_| Error::internal("Invalid RUSTF_SSL_ENABLED value"))?;
        }
        if let Ok(ssl_cert) = env::var("RUSTF_SSL_CERT") {
            self.server.ssl_cert = Some(ssl_cert);
        }
        if let Ok(ssl_key) = env::var("RUSTF_SSL_KEY") {
            self.server.ssl_key = Some(ssl_key);
        }
        if let Ok(max_conn) = env::var("RUSTF_MAX_CONNECTIONS") {
            self.server.max_connections = max_conn
                .parse()
                .map_err(|_| Error::internal("Invalid RUSTF_MAX_CONNECTIONS value"))?;
        }

        // View overrides
        if let Ok(views_dir) = env::var("RUSTF_VIEWS_DIR") {
            self.views.directory = views_dir;
        }
        if let Ok(layout) = env::var("RUSTF_DEFAULT_LAYOUT") {
            self.views.default_layout = layout;
        }
        if let Ok(cache) = env::var("RUSTF_VIEW_CACHE") {
            self.views.cache_enabled = cache
                .parse()
                .map_err(|_| Error::internal("Invalid RUSTF_VIEW_CACHE value"))?;
        }

        // New storage configuration
        if let Ok(storage) = env::var("RUSTF_TEMPLATE_STORAGE") {
            self.views.storage = match storage.to_lowercase().as_str() {
                "filesystem" => TemplateStorage::Filesystem,
                "embedded" => TemplateStorage::Embedded,
                _ => {
                    return Err(Error::internal(
                        "Invalid RUSTF_TEMPLATE_STORAGE value. Use 'filesystem' or 'embedded'.",
                    ))
                }
            };
        }

        // Legacy engine override for backward compatibility - DEPRECATED
        if let Ok(engine) = env::var("RUSTF_VIEW_ENGINE") {
            // Map legacy engine values to storage method
            self.views.storage =
                match engine.to_lowercase().as_str() {
                    "filesystem" | "totaljs" | "tera" | "auto" => TemplateStorage::Filesystem,
                    "embedded" => TemplateStorage::Embedded,
                    _ => return Err(Error::internal(
                        "Invalid RUSTF_VIEW_ENGINE value (DEPRECATED - use RUSTF_TEMPLATE_STORAGE)",
                    )),
                };

            log::warn!("RUSTF_VIEW_ENGINE is deprecated. Use RUSTF_TEMPLATE_STORAGE instead.");
        }

        // Session configuration is now handled via config file only
        // No environment variable overrides for sessions

        // Database overrides
        if let Ok(db_url) = env::var("DATABASE_URL") {
            self.database.url = Some(db_url);
        }
        if let Ok(db_url) = env::var("RUSTF_DATABASE_URL") {
            self.database.url = Some(db_url);
        }
        if let Ok(max_conn) = env::var("RUSTF_DB_MAX_CONNECTIONS") {
            self.database.max_connections = Some(
                max_conn
                    .parse()
                    .map_err(|_| Error::internal("Invalid RUSTF_DB_MAX_CONNECTIONS value"))?,
            );
        }
        if let Ok(timeout) = env::var("RUSTF_DB_TIMEOUT") {
            self.database.timeout = Some(
                timeout
                    .parse()
                    .map_err(|_| Error::internal("Invalid RUSTF_DB_TIMEOUT value"))?,
            );
        }

        // Static files overrides
        if let Ok(static_dir) = env::var("RUSTF_STATIC_DIR") {
            self.static_files.directory = static_dir;
        }
        if let Ok(url_prefix) = env::var("RUSTF_STATIC_PREFIX") {
            self.static_files.url_prefix = url_prefix;
        }
        if let Ok(cache) = env::var("RUSTF_STATIC_CACHE") {
            self.static_files.cache_enabled = cache
                .parse()
                .map_err(|_| Error::internal("Invalid RUSTF_STATIC_CACHE value"))?;
        }
        if let Ok(max_age) = env::var("RUSTF_STATIC_MAX_AGE") {
            self.static_files.cache_max_age = max_age
                .parse()
                .map_err(|_| Error::internal("Invalid RUSTF_STATIC_MAX_AGE value"))?;
        }

        // CORS overrides
        if let Ok(cors) = env::var("RUSTF_CORS_ENABLED") {
            self.cors.enabled = cors
                .parse()
                .map_err(|_| Error::internal("Invalid RUSTF_CORS_ENABLED value"))?;
        }
        if let Ok(origins) = env::var("RUSTF_CORS_ORIGINS") {
            self.cors.allowed_origins = origins.split(',').map(|s| s.trim().to_string()).collect();
        }
        if let Ok(methods) = env::var("RUSTF_CORS_METHODS") {
            self.cors.allowed_methods = methods
                .split(',')
                .map(|s| s.trim().to_uppercase())
                .collect();
        }
        if let Ok(headers) = env::var("RUSTF_CORS_HEADERS") {
            self.cors.allowed_headers = headers.split(',').map(|s| s.trim().to_string()).collect();
        }

        // Logging overrides
        if let Ok(level) = env::var("RUSTF_LOG_LEVEL") {
            self.logging.level = level;
        }
        if let Ok(log_file) = env::var("RUSTF_LOG_FILE") {
            self.logging.file = Some(log_file);
        }

        // Upload overrides
        if let Ok(upload_dir) = env::var("RUSTF_UPLOAD_DIR") {
            self.uploads.directory = upload_dir;
        }
        if let Ok(max_size) = env::var("RUSTF_MAX_FILE_SIZE") {
            self.uploads.max_file_size = max_size
                .parse()
                .map_err(|_| Error::internal("Invalid RUSTF_MAX_FILE_SIZE value"))?;
        }
        if let Ok(max_files) = env::var("RUSTF_MAX_FILES") {
            self.uploads.max_files = max_files
                .parse()
                .map_err(|_| Error::internal("Invalid RUSTF_MAX_FILES value"))?;
        }

        Ok(())
    }

    /// Validate template configuration matches enabled features
    fn validate_template_engine(&self) -> Result<()> {
        // Total.js is always available, no validation needed for it

        // Validate embedded storage requires embedded-views feature when used
        match self.views.storage {
            TemplateStorage::Embedded => {
                #[cfg(not(feature = "embedded-views"))]
                {
                    return Err(Error::internal(
                        "Embedded template storage selected but 'embedded-views' feature not enabled. Enable the feature or change storage to 'filesystem'."
                    ));
                }
            }
            TemplateStorage::Filesystem => {
                // Filesystem storage works with any engine feature
            }
        }

        Ok(())
    }

    /// Resolve views directory path to handle relative and absolute paths properly
    ///
    /// This method converts relative paths to be relative to the config base directory
    /// and normalizes absolute paths. This allows the config to specify paths like:
    /// - "views" (relative to config location)
    /// - "./templates" (relative to config location)
    /// - "../shared/views" (relative to config location)
    /// - "/absolute/path/to/views" (absolute path)
    fn resolve_views_directory<P: AsRef<Path>>(&mut self, base_dir: P) -> Result<()> {
        // Only resolve for filesystem storage
        if !matches!(self.views.storage, TemplateStorage::Filesystem) {
            return Ok(());
        }

        let views_path = Path::new(&self.views.directory);

        if views_path.is_absolute() {
            // Absolute path - use as-is but normalize
            self.views.directory = views_path.to_string_lossy().to_string();
            log::debug!("Using absolute views directory: {}", self.views.directory);
        } else {
            // Relative path - make it relative to the config base directory
            let resolved_path = base_dir.as_ref().join(views_path);

            // Canonicalize if the directory exists, otherwise store the logical path
            if resolved_path.exists() {
                match resolved_path.canonicalize() {
                    Ok(canonical) => {
                        self.views.directory = canonical.to_string_lossy().to_string();
                        log::debug!(
                            "Resolved views directory (canonical): {}",
                            self.views.directory
                        );
                    }
                    Err(_) => {
                        self.views.directory = resolved_path.to_string_lossy().to_string();
                        log::debug!(
                            "Resolved views directory (logical): {}",
                            self.views.directory
                        );
                    }
                }
            } else {
                self.views.directory = resolved_path.to_string_lossy().to_string();
                log::debug!(
                    "Resolved views directory (non-existent): {}",
                    self.views.directory
                );
            }
        }

        Ok(())
    }

    /// Get server address string
    pub fn server_address(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }

    /// Check if running in debug mode
    pub fn is_debug(&self) -> bool {
        cfg!(debug_assertions) || env::var("RUSTF_DEBUG").is_ok()
    }

    /// Get a custom configuration section by name
    ///
    /// Deserializes a custom section from the configuration into the specified type.
    /// This allows type-safe access to application-defined configuration sections.
    ///
    /// # Arguments
    /// * `name` - The name of the configuration section
    ///
    /// # Returns
    /// * `Ok(T)` - The deserialized configuration section
    /// * `Err` - If the section doesn't exist or deserialization fails
    ///
    /// # Example
    /// ```ignore
    /// #[derive(serde::Deserialize)]
    /// struct AppSettings {
    ///     api_key: String,
    ///     max_connections: usize,
    /// }
    ///
    /// let settings = config.section::<AppSettings>("app")?;
    /// println!("API Key: {}", settings.api_key);
    /// ```
    pub fn section<T: serde::de::DeserializeOwned>(&self, name: &str) -> Result<T> {
        self.sections
            .get(name)
            .ok_or_else(|| {
                Error::internal(format!(
                    "Configuration section '{}' not found. Check your config.toml.",
                    name
                ))
            })
            .and_then(|value| {
                serde_json::from_value(serde_json::to_value(value).map_err(|e| {
                    Error::internal(format!(
                        "Failed to serialize section '{}' to JSON: {}",
                        name, e
                    ))
                })?)
                .map_err(|e| {
                    Error::internal(format!("Failed to deserialize section '{}': {}", name, e))
                })
            })
    }

    /// Get a raw configuration value from a custom section
    ///
    /// Returns the TOML value at the specified section, or None if not found.
    /// This allows access to configuration values without requiring full deserialization.
    ///
    /// # Arguments
    /// * `section_name` - The name of the configuration section
    ///
    /// # Returns
    /// * `Some(&toml::Value)` - The configuration value
    /// * `None` - If the section doesn't exist
    ///
    /// # Example
    /// ```ignore
    /// if let Some(value) = config.get_value("app.name") {
    ///     println!("App name: {:?}", value);
    /// }
    /// ```
    pub fn get_value(&self, section_name: &str) -> Option<&toml::Value> {
        self.sections.get(section_name)
    }

    /// Check if a custom configuration section exists
    ///
    /// Returns true if the specified section is defined in the configuration.
    ///
    /// # Arguments
    /// * `name` - The name of the configuration section
    ///
    /// # Example
    /// ```ignore
    /// if config.has_section("payment") {
    ///     let payment = config.section::<PaymentConfig>("payment")?;
    /// }
    /// ```
    pub fn has_section(&self, name: &str) -> bool {
        self.sections.contains_key(name)
    }
}

// TOML support
#[cfg(feature = "config")]
use toml;

#[cfg(not(feature = "config"))]
mod toml {
    use crate::error::Error;
    use serde::de::DeserializeOwned;

    pub fn from_str<T: DeserializeOwned>(_: &str) -> Result<T, Error> {
        Err(Error::internal(
            "TOML support not enabled. Add 'config' feature.",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_storage_merge_redis() {
        // Test that Redis storage configuration from env config overrides base config
        let mut base = AppConfig::default();
        base.session.storage = SessionStorageConfig::Memory {
            cleanup_interval: 300,
        };

        let mut dev = AppConfig::default();
        dev.session.storage = SessionStorageConfig::Redis {
            url: "redis://localhost:6379".to_string(),
            prefix: "test:session:".to_string(),
            pool_size: 10,
            connection_timeout: 5000,
            command_timeout: 3000,
        };

        // Merge dev config into base
        base.merge_with(dev);

        // Verify Redis storage is now active
        assert!(
            matches!(base.session.storage, SessionStorageConfig::Redis { .. }),
            "Session storage should be Redis after merge"
        );
    }

    #[test]
    fn test_session_storage_merge_memory() {
        // Test that Memory storage configuration is preserved if set
        let mut base = AppConfig::default();
        base.session.storage = SessionStorageConfig::Redis {
            url: "redis://localhost:6379".to_string(),
            prefix: "test:".to_string(),
            pool_size: 10,
            connection_timeout: 5000,
            command_timeout: 3000,
        };

        let mut dev = AppConfig::default();
        dev.session.storage = SessionStorageConfig::Memory {
            cleanup_interval: 600,
        };

        // Merge dev config into base
        base.merge_with(dev);

        // Verify Memory storage is now active
        assert!(
            matches!(
                base.session.storage,
                SessionStorageConfig::Memory {
                    cleanup_interval: 600
                }
            ),
            "Session storage should be Memory after merge"
        );
    }

    #[test]
    fn test_session_fingerprint_mode_merge() {
        // Test that fingerprint_mode is merged from env config
        let mut base = AppConfig::default();
        base.session.fingerprint_mode = "strict".to_string();

        let mut dev = AppConfig::default();
        dev.session.fingerprint_mode = "soft".to_string();

        base.merge_with(dev);

        assert_eq!(
            base.session.fingerprint_mode, "soft",
            "Fingerprint mode should be updated from dev config"
        );
    }

    #[test]
    fn test_session_exempt_routes_merge() {
        // Test that exempt routes are merged (not replaced)
        let mut base = AppConfig::default();
        base.session.exempt_routes = vec!["/health".to_string(), "/status".to_string()];

        let mut dev = AppConfig::default();
        dev.session.exempt_routes = vec!["/test".to_string()];

        base.merge_with(dev);

        // Verify both base and dev routes are present
        assert!(
            base.session.exempt_routes.contains(&"/health".to_string()),
            "Base exempt route /health should be preserved"
        );
        assert!(
            base.session.exempt_routes.contains(&"/status".to_string()),
            "Base exempt route /status should be preserved"
        );
        assert!(
            base.session.exempt_routes.contains(&"/test".to_string()),
            "Dev exempt route /test should be added"
        );
        assert_eq!(
            base.session.exempt_routes.len(),
            3,
            "Should have 3 exempt routes total"
        );
    }

    #[test]
    fn test_session_other_fields_merge() {
        // Test that other session fields are properly merged
        let mut base = AppConfig::default();
        base.session.enabled = true;
        base.session.idle_timeout = 1800;
        base.session.cookie_name = "base_session".to_string();

        let mut dev = AppConfig::default();
        dev.session.enabled = false;
        dev.session.idle_timeout = 3600;
        dev.session.cookie_name = "dev_session".to_string();

        base.merge_with(dev);

        assert_eq!(
            base.session.enabled, false,
            "session.enabled should be merged"
        );
        assert_eq!(
            base.session.idle_timeout, 3600,
            "session.idle_timeout should be merged"
        );
        assert_eq!(
            base.session.cookie_name, "dev_session",
            "session.cookie_name should be merged"
        );
    }

    #[test]
    #[cfg(feature = "config")]
    fn test_toml_level_merging_simple() {
        // Test simple TOML value merging
        use serde_toml_merge::merge;

        let base_toml = r#"
[server]
port = 8000
host = "127.0.0.1"
"#;

        let dev_toml = r#"
[server]
port = 3000
"#;

        let base_value: toml::Value = toml::from_str(base_toml).unwrap();
        let dev_value: toml::Value = toml::from_str(dev_toml).unwrap();

        let merged = merge(base_value, dev_value).unwrap();

        // Verify port was overridden
        assert_eq!(merged["server"]["port"].as_integer().unwrap(), 3000);
        // Verify host was preserved from base
        assert_eq!(merged["server"]["host"].as_str().unwrap(), "127.0.0.1");
    }

    #[test]
    #[cfg(feature = "config")]
    fn test_toml_level_merging_nested_tables() {
        // Test that nested tables are properly merged
        use serde_toml_merge::merge;

        let base_toml = r#"
[session.storage]
type = "memory"
cleanup_interval = 300

[session]
enabled = true
cookie_name = "base_session"
"#;

        let dev_toml = r#"
[session.storage]
type = "redis"
url = "redis://localhost:6379"
"#;

        let base_value: toml::Value = toml::from_str(base_toml).unwrap();
        let dev_value: toml::Value = toml::from_str(dev_toml).unwrap();

        let merged = merge(base_value, dev_value).unwrap();

        // Verify session.storage was merged properly
        assert_eq!(
            merged["session"]["storage"]["type"].as_str().unwrap(),
            "redis"
        );
        assert_eq!(
            merged["session"]["storage"]["url"].as_str().unwrap(),
            "redis://localhost:6379"
        );
        // Verify session.enabled was preserved
        assert_eq!(merged["session"]["enabled"].as_bool().unwrap(), true);
    }

    #[test]
    #[cfg(feature = "config")]
    fn test_toml_level_merging_custom_sections() {
        // Test that custom config sections merge properly
        use serde_toml_merge::merge;

        let base_toml = r#"
[custom.email]
smtp_host = "smtp.example.com"
smtp_port = 587

[custom.app]
name = "MyApp"
"#;

        let dev_toml = r#"
[custom.email]
smtp_port = 2525

[custom.logging]
debug = true
"#;

        let base_value: toml::Value = toml::from_str(base_toml).unwrap();
        let dev_value: toml::Value = toml::from_str(dev_toml).unwrap();

        let merged = merge(base_value, dev_value).unwrap();

        // Verify email port was overridden
        assert_eq!(
            merged["custom"]["email"]["smtp_port"].as_integer().unwrap(),
            2525
        );
        // Verify email host was preserved
        assert_eq!(
            merged["custom"]["email"]["smtp_host"].as_str().unwrap(),
            "smtp.example.com"
        );
        // Verify app section was preserved
        assert_eq!(merged["custom"]["app"]["name"].as_str().unwrap(), "MyApp");
        // Verify logging section was added
        assert_eq!(
            merged["custom"]["logging"]["debug"].as_bool().unwrap(),
            true
        );
    }

    #[test]
    fn test_custom_section_access() {
        // Test that custom sections are properly accessible
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Deserialize, Serialize, PartialEq)]
        struct AppSettings {
            name: String,
            version: String,
            debug: bool,
        }

        let mut config = AppConfig::default();

        // Add a custom section to the config
        let app_value = toml::Value::Table(toml::map::Map::from_iter(vec![
            (
                "name".to_string(),
                toml::Value::String("TestApp".to_string()),
            ),
            (
                "version".to_string(),
                toml::Value::String("1.0.0".to_string()),
            ),
            ("debug".to_string(), toml::Value::Boolean(true)),
        ]));

        config.sections.insert("app".to_string(), app_value);

        // Test has_section
        assert!(config.has_section("app"));
        assert!(!config.has_section("nonexistent"));

        // Test get_value
        assert!(config.get_value("app").is_some());
        assert!(config.get_value("nonexistent").is_none());

        // Test section() deserialization
        let app_settings: AppSettings = config.section("app").unwrap();
        assert_eq!(app_settings.name, "TestApp");
        assert_eq!(app_settings.version, "1.0.0");
        assert_eq!(app_settings.debug, true);
    }

    #[test]
    fn test_custom_section_error_handling() {
        // Test error handling when section is missing
        use serde::Deserialize;

        #[derive(Debug, Deserialize)]
        struct PaymentConfig {
            stripe_key: String,
        }

        let config = AppConfig::default();

        // Should return error for missing section
        let result: Result<PaymentConfig> = config.section("payment");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_custom_sections_with_toml_loading() {
        // Test that custom sections are properly loaded and accessible
        let config_toml = r#"
[server]
port = 8000

[app]
name = "TestApp"
version = "2.0.0"

[payment]
stripe_key = "sk_test_123"
enabled = true
"#;

        let config: AppConfig = toml::from_str(config_toml).unwrap();

        // Verify framework section is typed
        assert_eq!(config.server.port, 8000);

        // Verify custom sections are in sections
        assert!(config.has_section("app"));
        assert!(config.has_section("payment"));
        assert!(!config.has_section("nonexistent"));

        // Verify can access values
        assert!(config.get_value("app").is_some());
        assert!(config.get_value("payment").is_some());

        // Verify can access specific values within sections
        let app_name = config
            .get_value("app")
            .and_then(|v| v.get("name"))
            .and_then(|v| v.as_str());
        assert_eq!(app_name, Some("TestApp"));

        let stripe_key = config
            .get_value("payment")
            .and_then(|v| v.get("stripe_key"))
            .and_then(|v| v.as_str());
        assert_eq!(stripe_key, Some("sk_test_123"));
    }
}
