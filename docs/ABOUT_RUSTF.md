# RustF Framework Core Principles & Conventions

**Complete documentation based on current framework implementation**

## Overview

RustF is an AI-friendly MVC web framework for Rust inspired by Total.js v4. It emphasizes convention over configuration, auto-discovery patterns, and comprehensive documentation designed for both human developers and AI coding assistants. The framework provides a complete development stack including CLI tools, database utilities, and schema-driven model generation.

### Core Philosophy

- **Convention Over Configuration** - Sensible defaults reduce boilerplate
- **AI-Friendly Development** - Predictable patterns and comprehensive documentation
- **Total.js Inspiration** - Familiar patterns for web developers
- **Type Safety** - Compile-time validation throughout the stack
- **Auto-Discovery** - Automatic component registration at build time
- **Schema-Driven** - YAML schemas drive model generation and database tools

### Key Features

✅ **Auto-Discovery System** - Automatic registration of controllers, models, middleware  
✅ **Convention-Based Architecture** - Predictable project structure and patterns  
✅ **Comprehensive CLI Tools** - Project scaffolding, schema management, database tools  
✅ **Multiple View Engines** - Tera templates, filesystem, and embedded views  
✅ **Advanced Session Management** - Memory, Redis, and database storage backends  
✅ **Built-in Security Features** - CORS, CSRF protection, secure headers  
✅ **Database Tools** - Multi-database query builder with schema management  
✅ **Performance Optimized** - Request pooling, caching, and memory safety  

## Project Structure & Conventions

### Standard Project Layout

```
your_project/
├── src/
│   ├── main.rs              # Application entry point with auto-discovery
│   ├── controllers/         # HTTP request handlers
│   │   ├── home.rs          # Home page controller
│   │   ├── auth.rs          # Authentication controller  
│   │   ├── users.rs         # User management controller
│   │   └── api/             # API controllers (namespaced)
│   │       ├── users.rs     # User API endpoints
│   │       └── posts.rs     # Post API endpoints
│   ├── models/              # Database models (schema-generated)
│   │   ├── users.rs         # User model wrapper
│   │   ├── posts.rs         # Post model wrapper
│   │   └── base/            # Auto-generated base models (DO NOT EDIT)
│   │       ├── users_base.rs
│   │       └── posts_base.rs
│   ├── modules/             # Business logic modules
│   │   ├── user_service.rs  # User business logic
│   │   └── email_service.rs # Email functionality
│   ├── middleware/          # Custom middleware
│   │   └── auth.rs          # Authentication middleware
│   ├── _controllers.rs      # Auto-generated (IDE support only)
│   ├── _models.rs           # Auto-generated (IDE support only)
│   └── _modules.rs          # Auto-generated (IDE support only)
├── views/                   # Template files
│   ├── layouts/
│   │   └── application.html # Default layout
│   ├── home/
│   │   ├── index.html       # Home page template
│   │   └── about.html       # About page template
│   └── auth/
│       └── login.html       # Login form template
├── schemas/                 # YAML schema definitions
│   ├── users.yaml           # User table schema
│   ├── posts.yaml           # Posts table schema
│   └── _meta.yaml           # Database metadata
├── public/                  # Static files
│   ├── css/
│   ├── js/
│   └── images/
├── uploads/                 # File uploads directory
├── config.toml              # Base configuration
├── config.production.toml   # Production overrides
└── Cargo.toml               # Rust project configuration
```

### File Naming Conventions

| Component | Convention | Example |
|-----------|------------|---------|
| **Controllers** | `snake_case.rs` | `users.rs`, `auth.rs` |
| **Models** | `snake_case.rs` | `users.rs`, `blog_posts.rs` |
| **Modules** | `snake_case.rs` | `user_service.rs`, `email_service.rs` |
| **Middleware** | `snake_case.rs` | `auth.rs`, `rate_limit.rs` |
| **Templates** | `snake_case.html` | `index.html`, `user_profile.html` |
| **Schemas** | `snake_case.yaml` | `users.yaml`, `blog_posts.yaml` |

### Directory Conventions

- **`src/controllers/`** - HTTP request handlers following MVC pattern
- **`src/models/`** - Database models with base/wrapper pattern
- **`src/modules/`** - Business logic separate from HTTP concerns
- **`src/middleware/`** - Request/response processing middleware
- **`views/`** - Template files organized by feature
- **`schemas/`** - YAML schema files for database-driven development
- **`public/`** - Static assets served directly by web server
- **`uploads/`** - User-uploaded files (configurable location)

## Auto-Discovery System

### How Auto-Discovery Works

RustF uses procedural macros to automatically discover and register components at compile time:

1. **Build-Time Scanning** - Macros scan filesystem during compilation
2. **Code Generation** - Generates module declarations and registration code
3. **IDE Support** - Creates `_*.rs` files for IDE autocomplete (not compiled)
4. **No Manual Registration** - Components are automatically included

### Enabling Auto-Discovery

```toml
# Cargo.toml
[dependencies]
rustf = { version = "0.1", features = ["auto-discovery"] }
```

### Application Setup with Auto-Discovery

```rust
// src/main.rs
use rustf::prelude::*;

// Auto-discovery uses build-time macros to scan and include components:
// 1. Scans src/controllers/*.rs, src/models/*.rs, src/modules/*.rs at compile time
// 2. Generates module declarations and registration code
// 3. Creates IDE support files (_controllers.rs, _models.rs, etc.)
// Note: Uses procedural macros, not attributes

#[tokio::main]
async fn main() -> rustf::Result<()> {
    env_logger::init();
    
    let app = RustF::new()
        .controllers(auto_controllers!())  // Auto-discover all controllers
        .models(auto_models!())           // Auto-register all models
        .middleware_from(auto_middleware!()); // Auto-register middleware
    
    app.start().await
}
```

### Auto-Discovery Macros

| Macro | Purpose | Returns |
|-------|---------|---------|
| `auto_controllers!()` | Discovers controllers | `Vec<Route>` |
| `auto_models!()` | Registers models | `Fn(&mut ModelRegistry)` |
| `auto_middleware!()` | Registers middleware | `Fn(&mut MiddlewareRegistry)` |
| `auto_modules!()` | Registers modules | `Fn(&mut SharedRegistry)` |

### Component Requirements for Auto-Discovery

Each component type must follow specific patterns:

**Controllers:**
```rust
// Must have: pub fn install() -> Vec<Route>
pub fn install() -> Vec<Route> {
    routes![
        GET "/users" => list_users,
        POST "/users" => create_user,
    ]
}
```

**Models:**
```rust
// Must have: pub fn register(registry: &mut ModelRegistry)
pub fn register(registry: &mut ModelRegistry) {
    registry.register("users", || Box::new(Users::new()));
}
```

**Middleware:**
```rust
// Must have: pub fn install(registry: &mut MiddlewareRegistry)
pub fn install(registry: &mut MiddlewareRegistry) {
    registry.register("auth", AuthMiddleware::new());
}
```

## Configuration System

### Configuration Hierarchy

RustF uses a layered configuration system:

1. **Default values** - Framework defaults
2. **Base config file** - `config.toml`
3. **Environment-specific config** - `config.{env}.toml`
4. **Environment variables** - `RUSTF_*` prefixed variables
5. **Security defaults** - Applied in production environments

### Environment Detection

```rust
// Environment is detected in this order:
// 1. RUSTF_ENV environment variable
// 2. RAILS_ENV environment variable (Rails compatibility)
// 3. NODE_ENV environment variable (Node.js compatibility)
// 4. Defaults to "development"

let env = AppConfig::detect_environment();
```

### Configuration Loading

```rust
// Automatic loading with environment detection
let app = RustF::load()?; // Uses config.toml + environment overrides

// Manual configuration
let app = RustF::from_file("config.toml")?;
let app = RustF::from_env()?; // Environment variables only
let app = RustF::new(); // Framework defaults only
```

### Complete Configuration Example

```toml
# config.toml (base configuration)
environment = "development"

[server]
host = "127.0.0.1"
port = 8000
timeout = 30
max_connections = 1000
ssl_enabled = false

[views]
directory = "views"
default_layout = "layouts/default"
cache_enabled = false
extension = "html"
engine = "tera"  # "tera", "filesystem", "embedded", "auto"

[session]
secret = "your-secret-key-here"
timeout = 3600
cookie_name = "rustf_session"
secure = false
http_only = true

[session.storage]
type = "memory"
cleanup_interval = 300

# Redis session storage (requires "redis" feature)
# [session.storage]
# type = "redis"
# url = "redis://localhost:6379"
# prefix = "rustf:session:"
# pool_size = 10

[static_files]
directory = "public"
url_prefix = "/static"
cache_enabled = true
cache_max_age = 86400

[database]
url = "postgresql://user:pass@localhost/dbname"
max_connections = 10
timeout = 30

[cors]
enabled = false
allowed_origins = ["*"]
allowed_methods = ["GET", "POST"]
allowed_headers = ["Content-Type"]

[logging]
level = "info"
# file = "app.log"  # Optional log file

[uploads]
directory = "uploads"
max_file_size = 10485760  # 10MB
max_files = 5
blocked_extensions = ["exe", "bat", "sh", "cmd"]
create_directories = true

[custom]
# Custom application-specific settings
app_name = "My RustF App"
api_version = "v1"
```

### Environment Variables

All configuration can be overridden with environment variables:

```bash
# Server configuration
export RUSTF_HOST=0.0.0.0
export RUSTF_PORT=3000
export RUSTF_SSL_ENABLED=true
export RUSTF_SSL_CERT=/path/to/cert.pem
export RUSTF_SSL_KEY=/path/to/key.pem

# View engine configuration
export RUSTF_VIEW_ENGINE=tera
export RUSTF_VIEWS_DIR=templates
export RUSTF_VIEW_CACHE=true

# Session configuration
export RUSTF_SESSION_SECRET=your-production-secret
export RUSTF_SESSION_TIMEOUT=7200

# Database configuration
export DATABASE_URL=postgresql://user:pass@localhost/production_db
export RUSTF_DB_MAX_CONNECTIONS=20

# Environment
export RUSTF_ENV=production
```

## Development Patterns & Conventions

### MVC Architecture Pattern

```rust
// Controller (HTTP layer) - handles requests/responses
pub async fn create_user(ctx: Context) -> Result<Response> {
    let form_data = ctx.body_form()?;
    
    // Delegate business logic to modules
    let user_service = crate::modules::user_service::UserService::new();
    let user = user_service.create_user(form_data).await?;
    
    ctx.json(json!({"user": user, "success": true}))
}

// Module (Business layer) - contains business logic
impl UserService {
    pub async fn create_user(&self, data: FormData) -> Result<User> {
        // Validation, business rules, etc.
        let user_data = self.validate_user_data(data)?;
        
        // Use models for data persistence
        Users::create(user_data).await
    }
}

// Model (Data layer) - handles database operations
impl Users {
    pub async fn create(data: UserData) -> Result<User> {
        // Database operations
        let query = Users::query()
            .insert(data)
            .returning_all();
        DB::execute(query).await
    }
}
```

### Controller Conventions

```rust
// controllers/users.rs
use rustf::prelude::*;

pub fn install() -> Vec<Route> {
    routes![
        // RESTful conventions
        GET    "/users"        => index,     // List all users
        GET    "/users/new"    => new_form,  // Show creation form
        POST   "/users"        => create,    // Create user
        GET    "/users/{id}"    => show,      // Show specific user
        GET    "/users/{id}/edit" => edit_form, // Show edit form
        PUT    "/users/{id}"    => update,    // Update user
        DELETE "/users/{id}"    => destroy,   // Delete user
    ]
}

// Handler naming conventions
async fn index(ctx: Context) -> Result<Response> {
    // List/index handlers return collections
    let users = Users::all().await?;
    ctx.view("/users/index", json!({"users": users}))
}

async fn show(ctx: Context) -> Result<Response> {
    // Show handlers return single resources
    let id = ctx.param("id").unwrap_or("0");
    let user = Users::find(id.parse()?).await?;
    ctx.view("/users/show", json!({"user": user}))
}

async fn create(ctx: Context) -> Result<Response> {
    // Create handlers process forms and redirect
    let form_data = ctx.body_form()?;
    match Users::create(form_data).await {
        Ok(user) => {
            ctx.flash_success("User created successfully!");
            ctx.redirect(&format!("/users/{}", user.id))
        }
        Err(e) => {
            ctx.flash_error(&format!("Failed to create user: {}", e));
            ctx.redirect("/users/new")
        }
    }
}
```

### Error Handling Conventions

```rust
// Use Result<Response> consistently
async fn handler(ctx: Context) -> Result<Response> {
    // Validate input
    let user_id = ctx.param("id")
        .ok_or_else(|| Error::bad_request("User ID required"))?
        .parse::<i64>()
        .map_err(|_| Error::bad_request("Invalid user ID"))?;
    
    // Business logic with error propagation
    let user = Users::find(user_id).await
        .map_err(|_| Error::not_found("User not found"))?;
    
    // Success response
    ctx.json(json!({"user": user}))
}

// HTTP error helper methods
ctx.throw400(Some("Invalid input"))      // Bad Request
ctx.throw401(Some("Login required"))     // Unauthorized  
ctx.throw403(Some("Access denied"))      // Forbidden
ctx.throw404(Some("Not found"))          // Not Found
ctx.throw500(Some("Server error"))       // Internal Error
```

### Response Patterns

```rust
// View responses (HTML)
ctx.view("/users/profile", json!({
    "user": user,
    "title": "User Profile"
}))

// JSON responses (APIs)
ctx.json(json!({
    "success": true,
    "data": users,
    "meta": {"count": users.len()}
}))

// Redirects with flash messages
ctx.flash_success("Operation completed!");
ctx.redirect("/users")

// File responses
ctx.file_download("uploads/document.pdf", Some("document.pdf"))

// Text responses
ctx.text("Plain text response")
```

### Validation Conventions

```rust
// Input validation pattern
async fn create_user(ctx: Context) -> Result<Response> {
    let form_data = ctx.body_form()?;
    
    // Validate required fields
    let email = form_data.get("email")
        .filter(|e| !e.is_empty() && e.contains('@'))
        .ok_or_else(|| Error::bad_request("Valid email required"))?;
    
    let password = form_data.get("password")
        .filter(|p| p.len() >= 8)
        .ok_or_else(|| Error::bad_request("Password must be at least 8 characters"))?;
    
    // Delegate to business logic
    let user_service = UserService::new();
    let user = user_service.create_user(email, password).await?;
    
    ctx.json(json!({"user": user, "success": true}))
}
```

## Security Conventions

### Built-in Security Features

```rust
// Automatic security in production
if config.environment.is_production() {
    // Force secure session cookies
    config.session.secure = true;
    config.session.http_only = true;
    
    // Warn about SSL
    if !config.server.ssl_enabled {
        eprintln!("WARNING: SSL not enabled in production");
    }
}

// CORS protection
ctx.cors_header("Access-Control-Allow-Origin", "https://myapp.com");

// CSRF protection (middleware)
ctx.verify_csrf_token()?;
```

### Input Sanitization

```rust
// Always validate and sanitize input
let user_input = ctx.query("search")
    .map(|s| s.trim())
    .filter(|s| !s.is_empty())
    .ok_or_else(|| Error::bad_request("Search query required"))?;

// Prevent directory traversal
let filename = ctx.param("filename").unwrap_or("");
if filename.contains("..") || filename.contains('/') {
    return ctx.throw403(Some("Invalid filename"));
}

// SQL injection prevention (automatic with query builder)
let users = Users::query()
    .where_eq("email", email)  // Automatically parameterized
    .get().await?;
```

### Authentication Patterns

```rust
// Session-based authentication
async fn login(ctx: Context) -> Result<Response> {
    let credentials = ctx.body_form()?;
    
    if let Some(user) = authenticate_user(credentials).await? {
        ctx.session_set("user_id", user.id)?;
        ctx.flash_success("Login successful!");
        ctx.redirect("/dashboard")
    } else {
        ctx.flash_error("Invalid credentials");
        ctx.redirect("/login")
    }
}

// Middleware protection
pub struct AuthMiddleware;

impl Middleware for AuthMiddleware {
    async fn handle(&self, ctx: &mut Context, next: Next) -> MiddlewareResult {
        if ctx.session_get::<i64>("user_id").is_some() {
            next.call(ctx).await // User is authenticated
        } else {
            MiddlewareResult::Stop(ctx.redirect("/login"))
        }
    }
}
```

## Database & Schema-Driven Development

### Schema Definition

```yaml
# schemas/users.yaml
table: users
description: "User accounts and profiles"

columns:
  id:
    type: bigserial
    primary_key: true
    description: "Unique user identifier"
    
  email:
    type: varchar(255)
    nullable: false
    unique: true
    description: "User email address"
    
  password_hash:
    type: varchar(255)
    nullable: false
    description: "Hashed password"
    
  name:
    type: varchar(100)
    nullable: false
    description: "User's full name"
    
  is_active:
    type: boolean
    default: true
    nullable: false
    description: "Account status"
    
  created_at:
    type: timestamp
    default: "CURRENT_TIMESTAMP"
    nullable: false
    
  updated_at:
    type: timestamp
    default: "CURRENT_TIMESTAMP"
    nullable: false
    
  status:
    type: enum
    enum_values: ["ACTIVE", "INACTIVE", "SUSPENDED"]
    nullable: false
    default: "ACTIVE"
    description: "User account status"
    postgres_type_name: "user_status"  # PostgreSQL enum type name

indexes:
  - name: idx_users_email
    columns: [email]
    unique: true
```

### Generated Model Usage

```rust
// Wrapper model (safe to edit)
use crate::models::base::users_base::UsersBase;

pub struct Users {
    base: UsersBase,
}

impl Users {
    // Business logic methods (safe to add)
    pub fn new() -> Self {
        Self { base: UsersBase::new() }
    }
    
    pub async fn find_by_email(email: &str) -> Result<Option<Self>> {
        Users::query()
            .where_eq(UsersBase::Types::email, email)
            .first()
            .await
    }
    
    pub async fn active_users() -> Result<Vec<Self>> {
        Users::query()
            .where_eq(UsersBase::Types::is_active, true)
            .order_by(UsersBase::Types::created_at, OrderDirection::Desc)
            .get()
            .await
    }
}

// Auto-generated base model (DO NOT EDIT)
// Located in src/models/base/users_base.rs
```

### Model-Scoped Query Builder

```rust
// Type-safe query building
let users = Users::query()
    .where_eq("is_active", true)
    .where_in("role", vec!["admin", "moderator"])
    .where_not_null("verified_at")
    .order_by("created_at", OrderDirection::Desc)
    .limit(10)
    .get()
    .await?;

// Static convenience methods
let user = Users::find(123).await?;
let admins = Users::where_eq("role", "admin").await?;
let total = Users::count().await?;

// Raw SQL when needed
let result = DB::query("SELECT COUNT(*) as total FROM users WHERE is_active = $1")
    .bind(true)
    .fetch_one()
    .await?;
```

### Working with Enums

RustF provides sophisticated enum handling, especially for PostgreSQL databases:

```rust
// Enum constants are available in the model
impl Users {
    // Constants module with enum values (without type suffix)
    pub mod status {
        pub const ACTIVE: &'static str = "ACTIVE";
        pub const INACTIVE: &'static str = "INACTIVE";
        pub const SUSPENDED: &'static str = "SUSPENDED";
    }
}

// Field-specific enum converter methods (for query builders)
// PostgreSQL: Appends type cast (::type_name)
// MySQL/SQLite: Pass-through (no type cast needed)
let typed_value = Users::as_status_enum("ACTIVE");
// Returns: "ACTIVE::user_status" for PostgreSQL
// Returns: "ACTIVE" for MySQL/SQLite

// Using enums in queries
let active_users = Users::query()
    .where_eq("status", Users::as_status_enum("ACTIVE"))
    .get()
    .await?;

// Setters automatically handle PostgreSQL type casting
let mut user = Users::find(1).await?.unwrap();
user.set_status(Some("INACTIVE"));  // Automatically becomes "INACTIVE::user_status"
user.update(&pool).await?;

// Builder pattern also handles type casting
let new_user = Users::builder()
    .email("user@example.com")
    .name("John Doe")
    .status("ACTIVE")  // Automatically typed for PostgreSQL
    .save(&pool)
    .await?;
```

**Key Features:**
- **Transparent Type Casting**: PostgreSQL enum types are automatically handled
- **Database Agnostic API**: Same code works across PostgreSQL, MySQL, and SQLite
- **Type Safety**: Enum constants prevent typos and provide IDE autocomplete
- **Query Builder Integration**: Converter methods ensure correct typing in queries

### Database Type System

RustF uses a sophisticated type system for database operations:

```rust
// SqlValue enum represents all possible database values
pub enum SqlValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
    Bytes(Vec<u8>),
    Timestamp(DateTime<Utc>),
    Enum(String),  // Enum values with optional type casting
    Json(serde_json::Value),
    Uuid(uuid::Uuid),
}

// Models implement field value getters returning SqlValue
impl Users {
    pub fn get_field_value(&self, field: &str) -> Result<SqlValue> {
        match field {
            "status" => Ok(SqlValue::Enum(self.status.clone())),
            // ... other fields
        }
    }
}
```

## CLI Tools & Development Workflow

### Project Creation

```bash
# Create new RustF project
rustf-cli new my_project

# Create with specific template
rustf-cli new my_api --template api-only

# Create with database support
rustf-cli new my_app --database postgresql
```

### Schema Management

```bash
# Validate schemas
rustf-cli schema validate --path schemas

# Generate models from schemas
rustf-cli schema generate models --schema-path schemas --output src/models

# Watch for schema changes (development)
rustf-cli schema watch --schema-path schemas --output src/models
```

### Database Operations

```bash
# Test database connection
rustf-cli db test-connection

# List all tables
rustf-cli db list-tables

# Describe table structure
rustf-cli db describe --table users

# Generate schema from existing database
rustf-cli db generate-schema --output schemas/generated

# Compare schema with database
rustf-cli db diff-schema --schema-path schemas
```

### Development Commands

```bash
# Analyze project structure
rustf-cli analyze --verbose

# Run development server with hot reload
rustf-cli serve --hot-reload

# Validate controllers
rustf-cli controllers validate

# Generate API documentation
rustf-cli docs generate --format markdown
```

## Testing Conventions

### Unit Testing

```rust
// controllers/users_test.rs
#[cfg(test)]
mod tests {
    use super::*;
    use rustf::prelude::*;
    
    #[tokio::test]
    async fn test_create_user_handler() {
        // Create a mock context for testing
        let mut ctx = Context::test();
        ctx.set_form_data(vec![
            ("email", "test@example.com"),
            ("name", "Test User"),
            ("password", "password123")
        ]);
        
        let response = create_user(ctx).await.unwrap();
        assert_eq!(response.status_code(), 302); // Redirect
    }
    
    #[tokio::test]
    async fn test_user_validation() {
        let mut ctx = Context::test();
        ctx.set_form_data(vec![
            ("email", "invalid-email"),
            ("name", "Test User"),
            ("password", "123") // Too short
        ]);
        
        let response = create_user(ctx).await.unwrap();
        assert_eq!(response.status_code(), 400); // Bad Request
    }
}
```

### Integration Testing (Planned Feature)

> **Note**: Full integration testing helpers are planned for a future release. Currently, you can use standard Rust HTTP testing libraries like `reqwest` or `actix-web-test`.

```rust
// tests/integration_tests.rs (Planned API)
use rustf::prelude::*;
use rustf::test_helpers::*; // Planned feature

#[tokio::test]
async fn test_full_user_workflow() {
    // Planned: TestApp helper for integration testing
    let app = TestApp::new().await; // Coming soon
    
    // Create user
    let response = app.post("/users")
        .form_data([("email", "test@example.com"), ("name", "Test User")])
        .send()
        .await;
    assert_eq!(response.status(), 302);
    
    // Additional integration test helpers are planned
}
```

### Current Testing Approach

Until the full test helper suite is available, use standard testing patterns:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_model_operations() {
        // Test database operations directly
        let pool = create_test_pool().await;
        
        let user = Users::builder()
            .email("test@example.com")
            .name("Test User")
            .save(&pool)
            .await
            .expect("Failed to create user");
            
        assert_eq!(user.email(), "test@example.com");
    }
}
```

## Performance & Production Considerations

### Production Configuration

```toml
# config.production.toml
environment = "production"

[server]
host = "0.0.0.0"
port = 80
ssl_enabled = true
ssl_cert = "/path/to/cert.pem"
ssl_key = "/path/to/key.pem"
max_connections = 2000
timeout = 60

[views]
cache_enabled = true
engine = "embedded"  # Embedded views for performance

[session]
secure = true
http_only = true
storage = { type = "redis", url = "redis://localhost:6379" }

[static_files]
cache_enabled = true
cache_max_age = 31536000  # 1 year

[database]
max_connections = 20
timeout = 10
```

### Performance Features

```rust
// Current performance optimizations
use rustf::prelude::*;

// Database connection pooling (automatic via sqlx)
// Connections are automatically pooled and reused

// View template caching (enabled in production)
// Templates are compiled once and cached for performance

// Static file serving with proper cache headers
// Static files are served with appropriate cache-control headers

// Session storage optimization
// Redis backend available for distributed sessions
```

### Planned Performance Features

> **Note**: Advanced performance features are planned for future releases:

```rust
// Request pooling (Planned)
// Will reduce allocations in high-traffic scenarios
let pooled_req = global_request_pool().get(); // Coming soon

// Response caching middleware (Planned) 
// Will cache full responses at the framework level
ctx.cache_response(Duration::from_secs(300)); // Coming soon

// Zero-copy optimizations (Planned)
// Will minimize data copying in hot paths
```

### Memory Safety

```rust
// Framework uses Arc for safe component sharing
pub struct RustF {
    models: Arc<ModelRegistry>,
    views: Arc<ViewEngine>,
    config: Arc<AppConfig>,
    // No unsafe code or raw pointers
}

// Context creation with Arc references
let context = Context::new(
    request,
    session,
    Arc::clone(&self.views),
    Arc::clone(&self.config)
);
```

## Best Practices Summary

### ✅ Do

- Use auto-discovery for component registration
- Follow RESTful routing conventions  
- Separate business logic into modules
- Use schema-driven model generation
- Validate all user input
- Use flash messages for user feedback
- Enable caching in production
- Use environment-specific configuration
- Write comprehensive tests
- Use the CLI tools for development
- Use enum converter methods for type-safe queries (e.g., `Users::as_status_enum()`)
- Let setters and builders handle enum type casting automatically
- Define enum types in YAML schemas with `enum_values` property

### ❌ Don't

- Edit files in `base/` directories (auto-generated)
- Mix business logic with HTTP handling
- Use hardcoded configuration values
- Skip input validation
- Ignore security warnings in production
- Commit sensitive data to repositories
- Use direct SQL when query builder suffices
- Create manual module declarations with auto-discovery
- Ignore the framework conventions
- Use unsafe code (framework is memory-safe)
- Manually append PostgreSQL type casts to enum values (let the framework handle it)
- Hardcode enum values outside of the generated constants

## Framework Integration

The RustF ecosystem consists of multiple coordinated components:

- **rustf** - Core framework library
- **rustf-macros** - Auto-discovery procedural macros
- **rustf-schema** - Schema parsing and code generation
- **rustf-cli** - Command-line development tools

All components work together to provide a cohesive development experience with AI-friendly patterns, comprehensive documentation, and production-ready performance.

## Summary

RustF provides a complete, convention-based web framework that emphasizes developer productivity through auto-discovery, schema-driven development, and comprehensive tooling. The framework's AI-friendly design, Total.js inspiration, and focus on type safety make it ideal for both rapid prototyping and production applications.