# RustF Configuration Guide

**Complete documentation for configuration management in RustF**

## Overview

RustF provides a comprehensive configuration system inspired by Total.js, with a global `CONF` accessor that allows uniform configuration access throughout your application. Configuration can be loaded from TOML files, environment variables, or programmatically, with support for environment-specific settings and hot-reloading during development.

### Key Features
- **Global CONF Access** - Single, uniform way to access configuration anywhere
- **Dot Notation Paths** - Access nested values with `"server.port"` syntax
- **Environment Support** - Development and production environments
- **Multiple Sources** - TOML files, environment variables, and defaults
- **Type-Safe Access** - Typed getters for strings, integers, booleans, and floats
- **Custom Settings** - Extensible with application-specific configuration

## Global Configuration Access (CONF)

RustF provides a global `CONF` singleton that gives uniform access to configuration values using dot notation paths. This is the **ONLY** way to access configuration in RustF - there is no configuration access through Context or other means.

### Basic Usage

```rust
use rustf::prelude::*;  // CONF is included in the prelude

// Access configuration values with dot notation
let port = CONF::get_int("server.port").unwrap_or(8000);
let host = CONF::get_string("server.host").unwrap_or_else(|| "127.0.0.1".to_string());
let db_url = CONF::get_string("database.url");

// Check if a configuration path exists
if CONF::has("database.url") {
    // Database is configured
}

// Get values with defaults
let timeout = CONF::get_or("server.timeout", 30);
let max_files = CONF::get_or("uploads.max_files", 5);

// Environment helpers
if CONF::is_production() {
    // Enable production optimizations
}

if CONF::is_development() {
    // Enable development features
}
```

### CONF Methods

| Method | Description | Example |
|--------|-------------|---------|
| `get<T>(path)` | Get typed value at path | `CONF::get::<u16>("server.port")` |
| `get_string(path)` | Get string value | `CONF::get_string("database.url")` |
| `get_int(path)` | Get integer value | `CONF::get_int("server.port")` |
| `get_bool(path)` | Get boolean value | `CONF::get_bool("server.ssl_enabled")` |
| `get_float(path)` | Get float value | `CONF::get_float("custom.rate")` |
| `get_or(path, default)` | Get with default | `CONF::get_or("server.port", 8000)` |
| `has(path)` | Check if path exists | `CONF::has("database.url")` |
| `env()` | Get current environment | `CONF::env()` |
| `is_production()` | Check if production | `CONF::is_production()` |
| `is_development()` | Check if development | `CONF::is_development()` |
| `all()` | Get entire config | `CONF::all()` |

### Accessing Configuration in Different Contexts

#### In Controllers
```rust
use rustf::prelude::*;  // CONF is included in the prelude

async fn handler(ctx: Context) -> Result<Response> {
    // Configuration is accessed via CONF, not through ctx
    let upload_dir = CONF::get_string("uploads.directory").unwrap_or_else(|| "uploads".to_string());
    let max_size = CONF::get_int("uploads.max_file_size").unwrap_or(10485760);
    
    // Use configuration values
    if file.size > max_size as usize {
        return ctx.throw400(Some("File too large"));
    }
    
    // Save file to upload_dir...
    Ok(Response::ok())
}
```

#### In Modules/Services
```rust
use rustf::prelude::*;  // CONF is included in the prelude

pub struct EmailService;

impl EmailService {
    pub fn send_email(&self, to: &str, subject: &str, body: &str) {
        // Access SMTP configuration without needing Context
        let smtp_host = CONF::get_string("custom.smtp_host").unwrap_or_else(|| "localhost".to_string());
        let smtp_port = CONF::get_int("custom.smtp_port").unwrap_or(25);
        let from_email = CONF::get_string("custom.from_email").unwrap_or_else(|| "noreply@example.com".to_string());
        
        // Send email using configuration...
    }
}
```

#### In Event Handlers
```rust
use rustf::prelude::*;  // CONF is included in the prelude

app.on("startup", |ctx| Box::pin(async move {
    // Access configuration during startup
    let db_url = CONF::get_string("database.url");
    let pool_size = CONF::get_int("database.max_connections").unwrap_or(10);
    
    if let Some(url) = db_url {
        // Initialize database with configuration
        initialize_database(&url, pool_size).await?;
    }
    
    Ok(())
}))
```

#### In Middleware
```rust
use rustf::prelude::*;  // CONF is included in the prelude

pub struct RateLimitMiddleware;

impl Middleware for RateLimitMiddleware {
    fn handle(&self, ctx: &mut Context, next: Next) -> MiddlewareResult {
        // Access rate limit configuration
        let max_requests = CONF::get_int("custom.rate_limit_max").unwrap_or(100);
        let window_seconds = CONF::get_int("custom.rate_limit_window").unwrap_or(60);
        
        // Apply rate limiting...
        next.run(ctx)
    }
}
```

## Configuration Structure

### AppConfig Schema

The main configuration structure in RustF:

```rust
pub struct AppConfig {
    pub environment: Environment,        // development, staging, production, testing
    pub server: ServerConfig,           // Server settings
    pub views: ViewConfig,              // Template engine settings
    pub session: SessionConfig,         // Session management
    pub static_files: StaticConfig,    // Static file serving
    pub database: DatabaseConfig,      // Database connection
    pub cors: CorsConfig,              // CORS settings
    pub logging: LoggingConfig,        // Logging configuration
    pub uploads: UploadConfig,         // File upload settings
    pub custom: HashMap<String, String>, // Custom application settings
}
```

### Configuration Sections

#### Server Configuration
```toml
[server]
host = "127.0.0.1"          # Server bind address
port = 8000                 # Server port
timeout = 30                # Request timeout in seconds
ssl_enabled = false         # Enable HTTPS
ssl_cert = "cert.pem"       # SSL certificate path (if ssl_enabled)
ssl_key = "key.pem"         # SSL private key path (if ssl_enabled)
max_connections = 1000      # Maximum concurrent connections
```

#### Views Configuration
```toml
[views]
directory = "views"         # Template directory path
default_layout = "layouts/default"  # Default layout template
cache_enabled = false       # Enable template caching
extension = "html"          # Template file extension
storage = "filesystem"      # Storage method: "filesystem" or "embedded"
```

#### Session Configuration
```toml
[session]
secret = "change-me-in-production"  # Session encryption secret
timeout = 3600              # Session timeout in seconds
cookie_name = "rustf_session"       # Session cookie name
secure = false              # Secure cookies (HTTPS only)
http_only = true            # HttpOnly flag for cookies

[session.storage]
type = "memory"             # Storage backend: "memory", "redis", or "database"
cleanup_interval = 300      # Cleanup interval in seconds (for memory storage)
```

#### Database Configuration
```toml
[database]
url = "postgresql://user:pass@localhost/myapp"  # Database connection URL
max_connections = 10        # Connection pool size
timeout = 5000              # Connection timeout in milliseconds
```

#### Static Files Configuration
```toml
[static_files]
directory = "public"        # Static files directory
url_prefix = "/static"      # URL prefix for static files
cache_enabled = true        # Enable caching headers
cache_max_age = 86400       # Cache max-age in seconds
```

#### CORS Configuration
```toml
[cors]
enabled = false             # Enable CORS
allowed_origins = ["*"]     # Allowed origins
allowed_methods = ["GET", "POST"]  # Allowed HTTP methods
allowed_headers = ["Content-Type"] # Allowed headers
```

#### Logging Configuration
```toml
[logging]
level = "info"              # Log level: debug, info, warn, error
file = "logs/app.log"       # Optional log file path
```

#### Upload Configuration
```toml
[uploads]
directory = "uploads"       # Upload directory
max_file_size = 10485760    # Max file size in bytes (10MB)
max_files = 5               # Max files per upload
allowed_extensions = []     # Allowed file extensions (empty = all)
blocked_extensions = ["exe", "bat", "sh", "cmd"]  # Blocked extensions
create_directories = true   # Auto-create upload directories
```

#### Custom Configuration
```toml
[custom]
api_key = "your-api-key"
smtp_host = "smtp.gmail.com"
smtp_port = "587"
from_email = "noreply@example.com"
feature_flag = "true"
```

## Configuration Files

### File Locations and Loading Order

1. **Base Configuration**: `config.toml`
2. **Environment-Specific**: `config.{environment}.toml`
3. **Environment Variables**: `RUSTF_*` prefixed variables
4. **CLI Arguments**: `--config` flag

### Example config.toml

```toml
# Base configuration for all environments
environment = "development"

[server]
host = "127.0.0.1"
port = 8000
timeout = 30

[views]
directory = "views"
default_layout = "layouts/default"
cache_enabled = false
extension = "html"

[session]
secret = "dev-secret-change-in-production"
timeout = 3600
cookie_name = "rustf_session"

[database]
url = "postgresql://localhost/myapp_dev"
max_connections = 5

[static_files]
directory = "public"
url_prefix = "/static"

[uploads]
directory = "uploads"
max_file_size = 10485760

[custom]
api_endpoint = "http://localhost:3000/api"
feature_x_enabled = "false"
```

### Environment-Specific Overrides

#### config.prod.toml
```toml
# Production-specific overrides
environment = "production"

[server]
host = "0.0.0.0"
port = 80
ssl_enabled = true
ssl_cert = "/etc/ssl/certs/app.crt"
ssl_key = "/etc/ssl/private/app.key"
max_connections = 2000

[views]
cache_enabled = true

[session]
secret = "${RUSTF_SESSION_SECRET}"  # Read from environment
secure = true

[database]
url = "${DATABASE_URL}"  # Read from environment
max_connections = 20

[logging]
level = "warn"
file = "/var/log/rustf/app.log"

[custom]
api_endpoint = "https://api.production.com"
feature_x_enabled = "true"
```

## Environment Variables

All configuration values can be overridden using environment variables with the `RUSTF_` prefix:

### Server Settings
```bash
RUSTF_ENV=production              # Set environment
RUSTF_HOST=0.0.0.0               # Server host
RUSTF_PORT=3000                  # Server port
RUSTF_TIMEOUT=60                 # Request timeout
RUSTF_SSL_ENABLED=true           # Enable SSL
RUSTF_SSL_CERT=/path/to/cert    # SSL certificate
RUSTF_SSL_KEY=/path/to/key      # SSL private key
RUSTF_MAX_CONNECTIONS=5000       # Max connections
```

### Database Settings
```bash
DATABASE_URL=postgresql://user:pass@host/db  # Database URL (standard)
RUSTF_DATABASE_URL=postgresql://...          # Alternative prefix
RUSTF_DB_MAX_CONNECTIONS=20                  # Pool size
RUSTF_DB_TIMEOUT=10000                       # Timeout in ms
```

### Session Settings
```bash
RUSTF_SESSION_SECRET=very-secret-key         # Session secret
RUSTF_SESSION_TIMEOUT=7200                   # Session timeout
RUSTF_SESSION_COOKIE_NAME=my_session         # Cookie name
RUSTF_SESSION_SECURE=true                    # Secure cookies
RUSTF_SESSION_HTTP_ONLY=true                 # HttpOnly cookies
```

### View Settings
```bash
RUSTF_VIEWS_DIR=/app/templates               # Views directory
RUSTF_DEFAULT_LAYOUT=layouts/main            # Default layout
RUSTF_VIEW_CACHE=true                        # Enable caching
RUSTF_TEMPLATE_STORAGE=embedded              # Storage method
```

### Custom Settings
```bash
# Custom settings are accessed with "custom." prefix in CONF
export RUSTF_CUSTOM_API_KEY="secret-key"
export RUSTF_CUSTOM_FEATURE_FLAG="true"

# In code:
let api_key = CONF::get_string("custom.api_key");
```

## Loading Configuration

### Application Startup

Configuration is automatically initialized during application startup:

```rust
use rustf::prelude::*;

#[rustf::auto_discover]
#[tokio::main]
async fn main() -> Result<()> {
    // Configuration is loaded and CONF is initialized automatically
    let app = RustF::with_args()?;  // Loads config with CLI support
    
    // Alternative loading methods:
    // let app = RustF::new();                    // Default config
    // let app = RustF::from_file("config.toml")?; // Specific file
    // let app = RustF::from_env()?;              // Environment only
    // let app = RustF::with_config(my_config);   // Programmatic
    
    app.start().await
}
```

### CLI Arguments

The framework supports configuration via command-line arguments:

```bash
# Use default configuration loading
./myapp

# Specify custom config file
./myapp --config /path/to/config.toml
./myapp -c config.prod.toml

# Override views directory (if using filesystem storage)
./myapp --views /path/to/templates
```

### Programmatic Configuration

```rust
use rustf::prelude::*;
use rustf::config::AppConfig;

fn main() -> Result<()> {
    // Create configuration programmatically
    let mut config = AppConfig::default();
    config.server.port = 3000;
    config.server.host = "0.0.0.0".to_string();
    config.database.url = Some("postgresql://localhost/mydb".to_string());
    config.custom.insert("api_key".to_string(), "secret".to_string());
    
    // Use the configuration
    let app = RustF::with_config(config);
    app.start().await
}
```

## Environment Detection

RustF automatically detects the environment from these sources (in order):

1. `RUSTF_ENV` environment variable
2. `RAILS_ENV` environment variable (Rails compatibility)
3. `NODE_ENV` environment variable (Node.js compatibility)
4. Default: `development`

```rust
pub enum Environment {
    Development,  // Default for local development
    Production,   // Live production environment
}
```

### Environment-Specific Behavior

```rust
// Check environment in code
let env = CONF::env();  // Returns "development", "production", etc.

if CONF::is_production() {
    // Production-specific code
    enable_caching();
    disable_debug_endpoints();
}

if CONF::is_development() {
    // Development-specific code
    enable_hot_reload();
    show_detailed_errors();
}
```

## Configuration Validation

RustF validates configuration during startup:

### Automatic Validation

- **Required in Production**:
  - Session secret must not be default value
  - SSL certificate/key paths must exist if SSL is enabled
  - Database URL is recommended (warns if missing)

- **Path Validation**:
  - Views directory should exist (warns if missing)
  - Static files directory must exist
  - Upload directory is created if missing

- **Type Validation**:
  - Port must be valid (1-65535)
  - Timeouts must be positive
  - File sizes must be reasonable

### Custom Validation

```rust
use rustf::prelude::*;

app.on("config.loaded", |ctx| Box::pin(async move {
    // Validate custom configuration
    if !CONF::has("custom.api_key") {
        return Err(Error::internal("API key is required in configuration"));
    }
    
    let api_key = CONF::get_string("custom.api_key").unwrap();
    if api_key.len() < 32 {
        return Err(Error::internal("API key must be at least 32 characters"));
    }
    
    Ok(())
}))
```

## Best Practices

### 1. Use Environment Variables for Secrets

```toml
# config.production.toml
[session]
secret = "${RUSTF_SESSION_SECRET}"  # Never hardcode production secrets

[database]
url = "${DATABASE_URL}"              # Use environment variable

[custom]
api_key = "${API_KEY}"               # External service credentials
```

### 2. Environment-Specific Files

```
config.toml          # Base configuration
config.dev.toml      # Development overrides
config.prod.toml     # Production settings
```

### 3. Custom Settings Organization

```toml
[custom]
# Group related settings with prefixes
smtp_host = "smtp.gmail.com"
smtp_port = "587"
smtp_user = "user@gmail.com"
smtp_password = "${SMTP_PASSWORD}"

redis_url = "redis://localhost:6379"
redis_prefix = "myapp:"

feature_new_ui = "false"
feature_beta_api = "true"
```

### 4. Access Patterns

```rust
// ✅ Good: Use CONF for all configuration access (available in prelude)
use rustf::prelude::*;

let port = CONF::get_int("server.port").unwrap_or(8000);
let db_url = CONF::get_string("database.url");

// ❌ Bad: Don't try to access config through Context
// ctx.config() // This method no longer exists!

// ✅ Good: Check existence before accessing
if CONF::has("custom.smtp_host") {
    setup_email_service();
}

// ✅ Good: Use typed getters for safety
let enabled = CONF::get_bool("custom.feature_enabled").unwrap_or(false);
let timeout = CONF::get_int("server.timeout").unwrap_or(30);

// ✅ Good: Use defaults for optional settings
let cache_size = CONF::get_or("custom.cache_size", 100);
```

### 5. Testing Configuration

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rustf::config::AppConfig;
    
    #[test]
    fn test_configuration_loading() {
        let config = AppConfig::from_file("config.test.toml").unwrap();
        
        // Initialize CONF for testing
        CONF::init(config).unwrap();
        
        // Test configuration values
        assert_eq!(CONF::get_int("server.port"), Some(8080));
        assert_eq!(CONF::env(), Some("testing".to_string()));
        assert!(CONF::has("database.url"));
    }
}
```

## Migration from Context Config

If you're migrating from an older version of RustF that had `ctx.config()`:

### Before (Old Way)
```rust
async fn handler(ctx: Context) -> Result<Response> {
    let upload_dir = ctx.config().uploads.directory.clone();
    let max_size = ctx.config().uploads.max_file_size;
    // ...
}
```

### After (New Way)
```rust
use rustf::prelude::*;  // CONF is included in the prelude

async fn handler(ctx: Context) -> Result<Response> {
    let upload_dir = CONF::get_string("uploads.directory").unwrap_or_else(|| "uploads".to_string());
    let max_size = CONF::get_int("uploads.max_file_size").unwrap_or(10485760);
    // ...
}
```

## Configuration Examples

### Minimal Configuration

```toml
# config.toml - Minimal configuration using defaults
[server]
port = 3000

[database]
url = "postgresql://localhost/myapp"
```

### Full Production Configuration

```toml
# config.prod.toml
environment = "production"

[server]
host = "0.0.0.0"
port = 443
timeout = 60
ssl_enabled = true
ssl_cert = "/etc/letsencrypt/live/example.com/fullchain.pem"
ssl_key = "/etc/letsencrypt/live/example.com/privkey.pem"
max_connections = 5000

[views]
directory = "/app/views"
default_layout = "layouts/default"
cache_enabled = true
storage = "embedded"  # Use embedded templates in production

[session]
secret = "${SESSION_SECRET}"  # From environment
timeout = 86400  # 24 hours
cookie_name = "app_session"
secure = true
http_only = true

[session.storage]
type = "redis"
url = "${REDIS_URL}"
prefix = "session:"
pool_size = 10

[database]
url = "${DATABASE_URL}"
max_connections = 50
timeout = 10000

[static_files]
directory = "/app/public"
url_prefix = "/static"
cache_enabled = true
cache_max_age = 2592000  # 30 days

[cors]
enabled = true
allowed_origins = ["https://example.com", "https://app.example.com"]
allowed_methods = ["GET", "POST", "PUT", "DELETE", "OPTIONS"]
allowed_headers = ["Content-Type", "Authorization", "X-Requested-With"]

[logging]
level = "info"
file = "/var/log/app/production.log"

[uploads]
directory = "/app/storage/uploads"
max_file_size = 52428800  # 50MB
max_files = 10
allowed_extensions = ["jpg", "jpeg", "png", "gif", "pdf", "doc", "docx"]
create_directories = true

[custom]
cdn_url = "https://cdn.example.com"
api_endpoint = "https://api.example.com/v1"
smtp_host = "smtp.sendgrid.net"
smtp_port = "587"
smtp_user = "apikey"
smtp_password = "${SENDGRID_API_KEY}"
from_email = "noreply@example.com"
support_email = "support@example.com"
google_analytics_id = "UA-XXXXXXXXX-X"
stripe_public_key = "${STRIPE_PUBLIC_KEY}"
stripe_secret_key = "${STRIPE_SECRET_KEY}"
redis_url = "${REDIS_URL}"
elasticsearch_url = "${ELASTICSEARCH_URL}"
feature_new_dashboard = "true"
feature_beta_api = "false"
maintenance_mode = "false"
rate_limit_requests = "1000"
rate_limit_window = "3600"
```

## Troubleshooting

### Configuration Not Loading

```rust
// Check if CONF is initialized
if !CONF::is_initialized() {
    panic!("Configuration not initialized!");
}

// Check what environment is loaded
let env = CONF::env().unwrap_or_else(|| "unknown".to_string());
println!("Running in {} environment", env);

// Debug configuration values
if let Some(config) = CONF::all() {
    println!("Server port: {}", config.server.port);
    println!("Database URL: {:?}", config.database.url);
}
```

### Environment Variables Not Working

```bash
# Make sure to export variables
export RUSTF_PORT=3000
export DATABASE_URL="postgresql://localhost/mydb"

# Or set them when running
RUSTF_ENV=production DATABASE_URL="..." cargo run
```

### Custom Values Not Accessible

```rust
// Custom values must be in [custom] section
// config.toml:
// [custom]
// my_value = "test"

// Access with "custom." prefix:
let value = CONF::get_string("custom.my_value");  // ✅ Correct
let value = CONF::get_string("my_value");         // ❌ Wrong
```

## Summary

RustF's configuration system provides:

✅ **Single Access Point** - Global CONF for uniform configuration access  
✅ **Dot Notation** - Clean path-based access to nested values  
✅ **Multiple Sources** - Files, environment variables, and code  
✅ **Environment Support** - Development and production configs  
✅ **Type Safety** - Typed getters for different value types  
✅ **Validation** - Automatic validation for production requirements  
✅ **Extensibility** - Custom settings for application-specific config  
✅ **AI-Friendly** - One way to do things reduces confusion  

The configuration system follows the principle of "convention over configuration" while still providing the flexibility needed for complex applications. By using the global CONF accessor exclusively, the codebase remains consistent and easy to understand for both humans and AI assistants.