# RustF Session System Documentation

## Overview

RustF provides a comprehensive session management system with support for multiple storage backends, flash messages, and thread-safe concurrent access. The session system is designed to handle both persistent data and temporary flash messages with automatic expiration and cleanup.

> **New in 2025**: Custom session storage can now be implemented via the [Definitions System](./ABOUT_DEFINITIONS.md), providing a simpler, convention-based approach. See [Custom Session Storage via Definitions](#custom-session-storage-via-definitions-system-recommended) for details.

## Core Components

### Session Data Structure

The `Session` struct provides lock-free concurrent access using `DashMap`:

```rust
pub struct Session {
    id: String,
    data: Arc<DashMap<String, Value>>,       // Persistent session data
    flash: Arc<DashMap<String, Value>>,      // Flash messages (consumed when read)
}
```

### Session Storage Backends

RustF supports multiple storage backends through the `SessionStorage` trait:

- **Memory Storage** - Fast in-memory storage with automatic cleanup (implemented)
- **Redis Storage** - Persistent storage with connection pooling (implemented, requires `redis` feature)
- **Database Storage** - Planned but not yet implemented (configuration exists, requires `database` feature)

## Basic Session Usage

### Accessing Session in Controllers

Sessions are available through the `Context` object in controllers:

```rust
use rustf::prelude::*;

async fn login(ctx: Context) -> Result<Response> {
    // Set session data
    ctx.session_set("user_id", 123)?;
    ctx.session_set("username", "john_doe")?;

    // Get session data
    let user_id: Option<i32> = ctx.session_get("user_id");
    let username: Option<String> = ctx.session_get("username");

    ctx.json(json!({
        "message": "Login successful",
        "user_id": user_id,
        "username": username
    }))
}

async fn logout(ctx: Context) -> Result<Response> {
    // Clear all session data but keep session active for tracking
    ctx.session_clear();

    // Alternative: completely destroy the session
    // ctx.session_destroy();

    ctx.flash_success("You have been logged out successfully");
    ctx.redirect("/login")
}
```

### Session Data Types

Sessions support any serializable data type:

```rust
// Basic types
ctx.session_set("counter", 42)?;
ctx.session_set("is_admin", true)?;
ctx.session_set("email", "user@example.com")?;

// Complex types
let user_data = json!({
    "id": 123,
    "roles": ["user", "moderator"],
    "preferences": {
        "theme": "dark",
        "language": "en"
    }
});
ctx.session_set("user", user_data)?;

// Custom structs (must implement Serialize/Deserialize)
#[derive(Serialize, Deserialize)]
struct UserProfile {
    id: i32,
    name: String,
    preferences: HashMap<String, String>,
}

let profile = UserProfile { /* ... */ };
ctx.session_set("profile", profile)?;
```

## Flash Messages

Flash messages are temporary messages that are consumed when read, perfect for displaying one-time notifications.

### Setting Flash Messages

```rust
// Convenience methods for common message types
ctx.flash_success("Account created successfully!");
ctx.flash_error("Invalid credentials");
ctx.flash_info("Please verify your email");

// Custom flash messages
ctx.flash_set("warning", "Your session will expire soon")?;
ctx.flash_set("custom_data", json!({"type": "notification", "data": 123}))?;
```

### Reading Flash Messages

Flash messages are automatically consumed when read:

```rust
// Get specific flash message (consumes it)
let error_msg: Option<String> = ctx.flash_get("error");
let success_msg: Option<String> = ctx.flash_get("success");

// Get all flash messages at once (consumes all)
let all_flash: HashMap<String, Value> = ctx.flash_get_all();
```

### Flash Messages in Views

Flash messages are automatically included in view contexts:

```html
<!-- views/layouts/default.html -->
<!DOCTYPE html>
<html>
<head>
    <title>My App</title>
</head>
<body>
    <!-- Flash messages are available in 'flash' variable -->
    @{if flash.success}
        <div class="alert alert-success">@{flash.success}</div>
    @{fi}

    @{if flash.erro}
        <div class="alert alert-error">@{flash.error}</div>
    @{fi}

    @{if flash.info}
        <div class="alert alert-info">@{flash.info}</div>
    @{fi}

</body>
</html>
```

## Session Configuration

### Default Memory Storage

The default session store uses in-memory storage with automatic cleanup:

```rust
// Default configuration (30 minutes timeout, 5 minutes cleanup interval)
let app = RustF::new(); // Uses MemorySessionStorage by default
```

### Custom Memory Storage

```rust
use rustf::session::storage::MemorySessionStorage;
use std::time::Duration;

// Custom timeout settings
let storage = MemorySessionStorage::with_timeout(
    Duration::from_secs(60 * 60),    // 1 hour session timeout
    Duration::from_secs(10 * 60)     // 10 minutes cleanup interval
);

let session_store = SessionStore::with_storage(Arc::new(storage));
```

### Redis Storage

For production environments, use Redis for persistent session storage:

```rust
#[cfg(feature = "redis")]
use rustf::session::redis::RedisSessionStorage;

#[tokio::main]
async fn main() -> Result<()> {
    // Default Redis connection (redis://localhost:6379)
    let redis_storage = RedisSessionStorage::new().await?;

    // Custom Redis configuration
    let redis_storage = RedisSessionStorage::from_url(
        "redis://localhost:6379",
        "myapp:session:",  // Key prefix
        20                 // Pool size
    ).await?;

    let session_store = SessionStore::with_storage(Arc::new(redis_storage));

    // Configure app with Redis sessions
    let app = RustF::new()
        .with_session_store(session_store)
        .controllers(auto_controllers!());

    app.serve(None).await
}
```

## Session Lifecycle Management

### Standard Session Methods

RustF provides industry-standard session lifecycle methods:

| Method | Purpose | Scope | When to Use |
|--------|---------|-------|-------------|
| `clear()` | Remove all data, keep session active | Memory only | Partial logout, data reset |
| `flush()` | Alias for `clear()` | Memory only | Laravel compatibility |
| `destroy()` | Mark for destruction, clear data | Memory only | Local cleanup |
| `regenerate_id()` | New ID, keep data | Memory only | Security after login |

### SessionStore Methods (with Storage)

| Method | Purpose | Scope | When to Use |
|--------|---------|-------|-------------|
| `destroy_session()` | Complete removal | Memory + Storage | Full logout, security breach |
| `regenerate_session_id()` | New ID + storage update | Memory + Storage | Session fixation protection |
| `clear_session()` | Clear data, keep in storage | Memory + Storage | Reset user data |

### Usage Examples

```rust
// Partial logout - clear data but keep session for analytics
ctx.session_clear();

// Complete logout - remove session entirely
// (requires access to SessionStore)
session_store.destroy_session(&session_id).await?;

// Security after login - regenerate ID to prevent fixation
session.regenerate_id();

// Or with storage backend update
if let Some(new_session) = session_store.regenerate_session_id(&old_id).await? {
    // Session ID changed, update any client-side references
    log::info!("New session ID: {}", new_session.id());
}
```

## Session Security

### Session ID Generation

Session IDs are cryptographically secure 32-character alphanumeric strings:

```rust
// Automatically generated for each new session
let session_id = storage.generate_id(); // e.g., "a7b8c9d0e1f2g3h4i5j6k7l8m9n0o1p2"
```

### Session Expiration

Sessions automatically expire based on last access time:

```rust
// Configure session timeout
let storage = MemorySessionStorage::with_timeout(
    Duration::from_secs(30 * 60), // 30 minutes of inactivity
    Duration::from_secs(5 * 60)   // Cleanup every 5 minutes
);
```

### Session Cleanup

Both storage backends handle automatic cleanup:

- **Memory Storage**: Background task removes expired sessions
- **Redis Storage**: Redis TTL automatically expires sessions

```rust
// Manual cleanup (returns number of sessions cleaned)
let cleaned_count = session_store.cleanup_expired().await?;
log::info!("Cleaned up {} expired sessions", cleaned_count);
```

## Custom Session Storage via Definitions System (Recommended)

RustF now provides a modern, convention-based approach to implementing custom session storage through the Definitions System. This is the **recommended method** for adding database or custom storage backends.

### Quick Setup with Definitions

#### Step 1: Create Session Storage Definition

Create `src/definitions/session_storage.rs`:

```rust
use rustf::definitions::Definitions;
use rustf::session::{SessionStorage, SessionData, StorageStats};
use rustf::config::SessionConfig;
use rustf::error::Result;
use async_trait::async_trait;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;

/// Install function called by auto-discovery
/// This MUST be present in every definitions file
pub fn install(defs: &mut Definitions) {
    log::info!("Installing custom session storage from definitions");
    defs.set_session_storage_factory(create_session_storage);
}

/// Factory function that creates our custom session storage
/// This is called by the framework when initializing sessions
fn create_session_storage(
    config: &SessionConfig
) -> Result<Arc<dyn SessionStorage>> {
    // Get database URL from environment or config
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://localhost/myapp".to_string());

    // Create your custom storage
    let storage = PostgresSessionStorage::new(&database_url)?;
    Ok(Arc::new(storage))
}

// Your SessionStorage implementation (see detailed example below)
pub struct PostgresSessionStorage {
    pool: PgPool,
}

// ... implement SessionStorage trait (see complete example in previous section)
```

#### Step 2: Use Auto-Discovery

In your `main.rs`:

```rust
#[rustf::auto_discover]
#[tokio::main]
async fn main() -> rustf::Result<()> {
    let app = RustF::new()
        .definitions_from(auto_definitions!())  // Automatically finds session_storage.rs
        .controllers(auto_controllers!());

    // Your custom session storage is now active!
    app.start().await
}
```

That's it! Your custom session storage is automatically discovered and integrated.

**Important**: The `install()` function is mandatory - it's how the auto-discovery system knows what to register.

### Benefits of the Definitions Approach

1. **Zero Configuration** - Just create the file in the right place
2. **Auto-Discovery** - No manual registration needed
3. **Convention-Based** - Follow the pattern, it just works
4. **Type-Safe** - Compile-time checking of your implementation
5. **Testable** - Easy to unit test in isolation

### Complete Example: PostgreSQL Storage via Definitions

Here's a complete, production-ready implementation using the definitions system:

```rust
// src/definitions/session_storage.rs
use rustf::definitions::Definitions;
use rustf::session::{SessionStorage, SessionData, StorageStats};
use rustf::config::SessionConfig;
use rustf::error::Result;
use async_trait::async_trait;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::sync::Arc;
use std::time::Duration;
use std::collections::HashMap;
use serde_json;

/// Install function called by auto-discovery - REQUIRED!
pub fn install(defs: &mut Definitions) {
    log::info!("Installing PostgreSQL session storage from definitions");
    defs.set_session_storage_factory(create_session_storage);
}

/// Factory function for session storage
fn create_session_storage(
    config: &SessionConfig
) -> Result<Arc<dyn SessionStorage>> {
    // Configuration can come from multiple sources
    let database_url = std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("SESSION_DATABASE_URL"))
        .unwrap_or_else(|_| "postgresql://localhost/myapp".to_string());

    // Create storage with configuration
    let storage = PostgresSessionStorage::new(
        &database_url,
        config.idle_timeout.as_secs(),
        config.cookie_name.clone()
    )?;

    Ok(Arc::new(storage))
}

pub struct PostgresSessionStorage {
    pool: PgPool,
    default_ttl_seconds: u64,
    table_name: String,
}

impl PostgresSessionStorage {
    pub fn new(
        database_url: &str,
        default_ttl_seconds: u64,
        cookie_name: String
    ) -> Result<Self> {
        // Create connection pool synchronously for the factory
        let pool = futures::executor::block_on(async {
            PgPoolOptions::new()
                .max_connections(5)
                .connect(database_url)
                .await
        }).map_err(|e| rustf::error::Error::internal(
            format!("Failed to connect to database: {}", e)
        ))?;

        // Create sessions table
        futures::executor::block_on(async {
            sqlx::query(&format!(
                "CREATE TABLE IF NOT EXISTS sessions_{} (
                    id VARCHAR(64) PRIMARY KEY,
                    data JSONB NOT NULL,
                    expires_at TIMESTAMP NOT NULL,
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    ip_address INET,
                    user_agent TEXT
                )",
                sanitize_table_name(&cookie_name)
            ))
            .execute(&pool)
            .await
        }).map_err(|e| rustf::error::Error::internal(
            format!("Failed to create sessions table: {}", e)
        ))?;

        // Create indexes for performance
        futures::executor::block_on(async {
            sqlx::query(&format!(
                "CREATE INDEX IF NOT EXISTS idx_sessions_{}_expires
                 ON sessions_{} (expires_at)",
                sanitize_table_name(&cookie_name),
                sanitize_table_name(&cookie_name)
            ))
            .execute(&pool)
            .await
        }).ok(); // Index creation failure is non-fatal

        Ok(Self {
            pool,
            default_ttl_seconds,
            table_name: format!("sessions_{}", sanitize_table_name(&cookie_name)),
        })
    }
}

#[async_trait]
impl SessionStorage for PostgresSessionStorage {
    async fn get(&self, session_id: &str) -> Result<Option<SessionData>> {
        let query = format!(
            "SELECT data FROM {}
             WHERE id = $1 AND expires_at > NOW()",
            self.table_name
        );

        let row = sqlx::query(&query)
            .bind(session_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| rustf::error::Error::internal(
                format!("Failed to get session: {}", e)
            ))?;

        match row {
            Some(row) => {
                let data: serde_json::Value = row.try_get("data")
                    .map_err(|e| rustf::error::Error::internal(
                        format!("Failed to deserialize session: {}", e)
                    ))?;

                let mut session_data: SessionData = serde_json::from_value(data)
                    .map_err(|e| rustf::error::Error::internal(
                        format!("Invalid session data format: {}", e)
                    ))?;

                // Update last accessed time
                session_data.touch();

                // Update in database (fire and forget for performance)
                let update_query = format!(
                    "UPDATE {} SET updated_at = NOW() WHERE id = $1",
                    self.table_name
                );

                let _ = sqlx::query(&update_query)
                    .bind(session_id)
                    .execute(&self.pool)
                    .await;

                Ok(Some(session_data))
            }
            None => Ok(None)
        }
    }

    async fn set(
        &self,
        session_id: &str,
        data: &SessionData,
        ttl: Duration
    ) -> Result<()> {
        let expires_at = chrono::Utc::now() +
            chrono::Duration::seconds(ttl.as_secs() as i64);

        let json_data = serde_json::to_value(data)
            .map_err(|e| rustf::error::Error::internal(
                format!("Failed to serialize session: {}", e)
            ))?;

        let query = format!(
            "INSERT INTO {} (id, data, expires_at, updated_at)
             VALUES ($1, $2, $3, NOW())
             ON CONFLICT (id) DO UPDATE
             SET data = $2, expires_at = $3, updated_at = NOW()",
            self.table_name
        );

        sqlx::query(&query)
            .bind(session_id)
            .bind(json_data)
            .bind(expires_at)
            .execute(&self.pool)
            .await
            .map_err(|e| rustf::error::Error::internal(
                format!("Failed to save session: {}", e)
            ))?;

        Ok(())
    }

    async fn delete(&self, session_id: &str) -> Result<()> {
        let query = format!(
            "DELETE FROM {} WHERE id = $1",
            self.table_name
        );

        sqlx::query(&query)
            .bind(session_id)
            .execute(&self.pool)
            .await
            .map_err(|e| rustf::error::Error::internal(
                format!("Failed to delete session: {}", e)
            ))?;

        Ok(())
    }

    async fn exists(&self, session_id: &str) -> Result<bool> {
        let query = format!(
            "SELECT 1 FROM {}
             WHERE id = $1 AND expires_at > NOW()
             LIMIT 1",
            self.table_name
        );

        let exists = sqlx::query(&query)
            .bind(session_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| rustf::error::Error::internal(
                format!("Failed to check session existence: {}", e)
            ))?
            .is_some();

        Ok(exists)
    }

    async fn cleanup_expired(&self) -> Result<usize> {
        let query = format!(
            "DELETE FROM {} WHERE expires_at <= NOW()",
            self.table_name
        );

        let result = sqlx::query(&query)
            .execute(&self.pool)
            .await
            .map_err(|e| rustf::error::Error::internal(
                format!("Failed to cleanup sessions: {}", e)
            ))?;

        Ok(result.rows_affected() as usize)
    }

    fn backend_name(&self) -> &'static str {
        "postgresql-definitions"
    }

    async fn stats(&self) -> Result<StorageStats> {
        let total_query = format!(
            "SELECT COUNT(*) as count FROM {}",
            self.table_name
        );

        let active_query = format!(
            "SELECT COUNT(*) as count FROM {} WHERE expires_at > NOW()",
            self.table_name
        );

        let total: i64 = sqlx::query(&total_query)
            .fetch_one(&self.pool)
            .await
            .and_then(|row| row.try_get("count"))
            .unwrap_or(0);

        let active: i64 = sqlx::query(&active_query)
            .fetch_one(&self.pool)
            .await
            .and_then(|row| row.try_get("count"))
            .unwrap_or(0);

        Ok(StorageStats {
            total_sessions: total as usize,
            active_sessions: active as usize,
            expired_sessions: (total - active) as usize,
            backend_metrics: HashMap::new(),
        })
    }
}

fn sanitize_table_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect::<String>()
        .to_lowercase()
}
```

**Key Points:**
- The `install(defs: &mut Definitions)` function is mandatory for auto-discovery
- The factory function (`create_session_storage`) should be private since it's registered via `install()`
- The factory receives `&SessionConfig` from `rustf::config::SessionConfig`, not the session manager's config

### How It Works

1. **Auto-Discovery**: The `auto_definitions!()` macro finds `session_storage.rs` with an `install()` function
2. **Registration**: The `install()` function is called and registers your factory via `defs.set_session_storage_factory()`
3. **Factory Pattern**: When sessions are initialized, RustF calls your registered factory function
4. **Integration**: Your storage replaces the default memory storage automatically
5. **Configuration**: The factory receives `SessionConfig` for customization

### When to Use Definitions vs Manual Integration

**Use Definitions (Recommended) When:**
- You want the simplest setup
- Following conventions is acceptable
- You're building a standard application
- You want auto-discovery benefits

**Use Manual Integration When:**
- You need complex initialization logic
- You want full control over the storage lifecycle
- You're building a library or framework
- You have special requirements that don't fit the convention

For more details on the Definitions System, see [ABOUT_DEFINITIONS.md](./ABOUT_DEFINITIONS.md).

## Advanced Usage

### Session Management Methods

RustF provides comprehensive session management methods:

```rust
async fn session_management(ctx: Context) -> Result<Response> {
    let session = &ctx.session;

    // Basic session operations
    session.set("user_id", 123)?;
    let user_id: Option<i32> = session.get("user_id");
    let removed_value = session.remove("temp_data");

    // Session lifecycle management
    session.clear();        // Clear all data but keep session active
    session.flush();        // Alias for clear() (Laravel compatibility)
    session.destroy();      // Mark session for destruction

    // Session information
    let session_id = session.id();
    let is_empty = session.is_empty();
    let data_count = session.data_count();
    let flash_count = session.flash_count();

    ctx.json(json!({
        "session_id": session_id,
        "is_empty": is_empty,
        "data_entries": data_count,
        "flash_messages": flash_count
    }))
}

// Security: Session ID regeneration for fixation protection
async fn regenerate_session(ctx: Context) -> Result<Response> {
    // Get mutable reference to session
    let session = &mut ctx.session;

    // Regenerate session ID while keeping all data
    session.regenerate_id();

    ctx.flash_info("Session ID regenerated for security");
    ctx.json(json!({"new_session_id": session.id()}))
}

// Context convenience methods
async fn context_session_methods(ctx: Context) -> Result<Response> {
    // Convenience methods available on Context
    ctx.session_set("key", "value")?;
    let value: Option<String> = ctx.session_get("key");
    ctx.session_clear();     // Clear all session data
    ctx.session_flush();     // Alias for clear
    ctx.session_destroy();   // Mark for destruction

    ctx.json(json!({"status": "ok"}))
}
```

### Session Store Operations

Perform operations on the session store itself:

```rust
use rustf::session::{SessionStore, SessionData};

async fn session_store_management(session_store: &SessionStore) -> Result<()> {
    let session_id = "user_123_session";

    // Basic session store operations
    let exists = session_store.exists(session_id).await?;
    let session_opt = session_store.get(session_id).await?;

    // Complete session destruction (removes from storage)
    session_store.destroy_session(session_id).await?;

    // Regenerate session ID with storage backend updates
    if let Some(new_session) = session_store.regenerate_session_id(session_id).await? {
        log::info!("Session regenerated: {}", new_session.id());
    }

    // Clear session data but keep it in storage
    session_store.clear_session(session_id).await?;

    // Storage statistics and monitoring
    let stats = session_store.stats().await?;
    log::info!("Total sessions: {}, Active: {}",
               stats.total_sessions,
               stats.active_sessions);

    let backend = session_store.backend_name();
    log::info!("Using {} storage backend", backend);

    Ok(())
}
```

### Custom Storage Backend (Manual Approach)

> **Note**: The Definitions System approach (described above) is now the recommended way to implement custom session storage. The manual approach below is still supported for advanced use cases requiring full control.

#### Understanding SessionData Structure

Before implementing custom storage, understand the `SessionData` structure:

```rust
use serde::{Serialize, Deserialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    /// Session data as JSON object
    pub data: Value,
    /// Flash messages as JSON object
    pub flash: Value,
    /// Security fingerprint containing IP and user agent info
    pub fingerprint: Option<SessionFingerprint>,
    /// Session creation timestamp (Unix seconds)
    pub created_at: u64,
    /// Last accessed timestamp (Unix seconds)
    pub last_accessed: u64,
    /// Absolute timeout timestamp (Unix seconds)
    pub absolute_timeout: u64,
    /// Current privilege level (for security escalation tracking)
    pub privilege_level: u32,
}

impl SessionData {
    /// Update last accessed time to current timestamp
    pub fn touch(&mut self) {
        self.last_accessed = unix_timestamp();
    }

    /// Check if session has expired based on dual timeout
    pub fn is_expired(&self, idle_timeout_secs: u64) -> bool {
        let now = unix_timestamp();

        // Check absolute timeout
        if now > self.absolute_timeout {
            return true;
        }

        // Check idle timeout
        (now - self.last_accessed) > idle_timeout_secs
    }
}
```

#### Understanding SessionFingerprint Structure

The `SessionFingerprint` structure captures client information for session security validation:

```rust
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionFingerprint {
    /// IP prefix for soft validation (first 3 octets for IPv4, first 3 segments for IPv6)
    pub ip_prefix: String,
    /// Hashed user agent for privacy (stored as u64 hash, not plaintext)
    pub user_agent_hash: u64,
    /// Original creation IP address for audit logging
    pub created_ip: String,
    /// Creation timestamp (Unix seconds)
    pub created_at: u64,
}

impl SessionFingerprint {
    /// Create fingerprint from request (called automatically by session middleware)
    pub fn from_request(request: &Request) -> Self {
        let created_at = unix_timestamp();
        let created_ip = request.client_ip();  // Supports X-Forwarded-For and X-Real-IP
        let ip_prefix = Self::extract_ip_prefix(&created_ip);
        let user_agent_hash = Self::hash_user_agent(request.user_agent());

        Self {
            ip_prefix,
            user_agent_hash,
            created_ip,
            created_at,
        }
    }

    /// Validate fingerprint against request based on FingerprintMode
    pub fn validate(&self, request: &Request, mode: FingerprintMode) -> bool {
        // Implementation varies based on mode (see FingerprintMode below)
    }
}
```

**Important Notes:**
- IP address and user agent are **automatically captured** when a session is created
- User agent is stored as a hash for privacy protection
- The `created_ip` field stores the full original IP for audit purposes
- The `ip_prefix` is used for soft validation (allows some IP mobility)

#### FingerprintMode Enum

Controls how strictly sessions are bound to client characteristics:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FingerprintMode {
    /// No fingerprint validation - sessions work from any IP/browser
    Disabled,

    /// Soft validation (default) - validates IP prefix + user agent hash
    /// Allows mobility within same subnet (e.g., WiFi to cellular on same network)
    Soft,

    /// Strict validation - exact IP and user agent must match
    /// Maximum security but may cause issues with mobile users or dynamic IPs
    Strict,
}

impl Default for FingerprintMode {
    fn default() -> Self {
        Self::Soft  // Balanced security and usability
    }
}
```

**Validation Behavior:**
- **Disabled**: No validation, session works from anywhere
- **Soft**: IP must match first 3 octets (IPv4) or first 3 segments (IPv6), and user agent hash must match
- **Strict**: Exact IP address and user agent hash must match

**Accessing Fingerprint Data in Controllers:**

```rust
async fn show_session_info(ctx: Context) -> Result<Response> {
    if let Some(session) = ctx.session() {
        // Get the fingerprint if it exists
        if let Some(fingerprint) = session.fingerprint() {
            // Access captured client information
            let created_ip = &fingerprint.created_ip;
            let created_at = fingerprint.created_at;
            let ip_prefix = &fingerprint.ip_prefix;

            return ctx.json(json!({
                "session_id": session.id(),
                "created_from_ip": created_ip,
                "created_at": created_at,
                "ip_prefix": ip_prefix,
                // Note: user_agent_hash is a u64, not human-readable
                "user_agent_hash": fingerprint.user_agent_hash,
            }));
        }
    }

    ctx.json(json!({"error": "No session found"}))
}
```

#### Complete Database Storage Implementation with SQLx

Here's a complete working example using SQLx for PostgreSQL:

```rust
use async_trait::async_trait;
use rustf::session::{SessionStorage, SessionData, StorageStats};
use sqlx::{PgPool, Row};
use std::time::Duration;
use serde_json;

pub struct DatabaseSessionStorage {
    pool: PgPool,
    table_name: String,
}

impl DatabaseSessionStorage {
    pub async fn new(database_url: &str) -> Result<Self> {
        // Create connection pool
        let pool = PgPool::connect(database_url).await?;

        // Create sessions table if not exists
        sqlx::query(&format!(r#"
            CREATE TABLE IF NOT EXISTS sessions (
                id VARCHAR(64) PRIMARY KEY,
                data JSONB NOT NULL,
                expires_at TIMESTAMP NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
        "#)).execute(&pool).await?;

        // Create index for cleanup
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_sessions_expires ON sessions(expires_at)")
            .execute(&pool).await?;

        Ok(Self {
            pool,
            table_name: "sessions".to_string(),
        })
    }
}

#[async_trait]
impl SessionStorage for DatabaseSessionStorage {
    async fn get(&self, session_id: &str) -> Result<Option<SessionData>> {
        let query = format!(
            "SELECT data FROM {} WHERE id = $1 AND expires_at > NOW()",
            self.table_name
        );

        let row = sqlx::query(&query)
            .bind(session_id)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(row) => {
                let data: serde_json::Value = row.get("data");
                let mut session_data: SessionData = serde_json::from_value(data)?;

                // Update last accessed time
                session_data.touch();

                // Update in database
                let update_query = format!(
                    "UPDATE {} SET updated_at = NOW() WHERE id = $1",
                    self.table_name
                );
                sqlx::query(&update_query)
                    .bind(session_id)
                    .execute(&self.pool)
                    .await?;

                Ok(Some(session_data))
            }
            None => Ok(None),
        }
    }

    async fn set(&self, session_id: &str, data: &SessionData, ttl: Duration) -> Result<()> {
        let expires_at = chrono::Utc::now() + chrono::Duration::seconds(ttl.as_secs() as i64);
        let json_data = serde_json::to_value(data)?;

        let query = format!(r#"
            INSERT INTO {} (id, data, expires_at, updated_at)
            VALUES ($1, $2, $3, NOW())
            ON CONFLICT (id) DO UPDATE
            SET data = $2, expires_at = $3, updated_at = NOW()
        "#, self.table_name);

        sqlx::query(&query)
            .bind(session_id)
            .bind(json_data)
            .bind(expires_at)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn delete(&self, session_id: &str) -> Result<()> {
        let query = format!("DELETE FROM {} WHERE id = $1", self.table_name);
        sqlx::query(&query)
            .bind(session_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn exists(&self, session_id: &str) -> Result<bool> {
        let query = format!(
            "SELECT 1 FROM {} WHERE id = $1 AND expires_at > NOW()",
            self.table_name
        );

        let exists = sqlx::query(&query)
            .bind(session_id)
            .fetch_optional(&self.pool)
            .await?
            .is_some();

        Ok(exists)
    }

    async fn cleanup_expired(&self) -> Result<usize> {
        let query = format!("DELETE FROM {} WHERE expires_at <= NOW()", self.table_name);
        let result = sqlx::query(&query)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() as usize)
    }

    fn backend_name(&self) -> &'static str {
        "database"
    }

    async fn stats(&self) -> Result<StorageStats> {
        let total_query = format!("SELECT COUNT(*) as count FROM {}", self.table_name);
        let active_query = format!(
            "SELECT COUNT(*) as count FROM {} WHERE expires_at > NOW()",
            self.table_name
        );

        let total: i64 = sqlx::query(&total_query)
            .fetch_one(&self.pool)
            .await?
            .get("count");

        let active: i64 = sqlx::query(&active_query)
            .fetch_one(&self.pool)
            .await?
            .get("count");

        Ok(StorageStats {
            total_sessions: total as usize,
            active_sessions: active as usize,
            expired_sessions: (total - active) as usize,
            backend_metrics: HashMap::new(),
        })
    }
}

// Integration (see "How Session Storage Integration Works" section for details)
// You'll use this storage with SessionMiddleware::with_storage()
```

## Complete Working Example: Putting It All Together

Here's a complete, working example that shows exactly how to integrate custom database storage:

```rust
// main.rs
use rustf::prelude::*;
use rustf::middleware::builtin::session::SessionMiddleware;
use rustf::session::{SessionStorage, SessionData, StorageStats, manager::SessionConfig};
use rustf::config::AppConfig;
use async_trait::async_trait;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
use serde_json;

// Your custom database storage implementation
pub struct DatabaseSessionStorage {
    pool: PgPool,
}

impl DatabaseSessionStorage {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPool::connect(database_url).await?;

        // Create sessions table
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS sessions (
                id VARCHAR(64) PRIMARY KEY,
                data JSONB NOT NULL,
                expires_at TIMESTAMP NOT NULL
            )"
        ).execute(&pool).await?;

        Ok(Self { pool })
    }
}

#[async_trait]
impl SessionStorage for DatabaseSessionStorage {
    async fn get(&self, session_id: &str) -> Result<Option<SessionData>> {
        let row = sqlx::query!(
            "SELECT data FROM sessions WHERE id = $1 AND expires_at > NOW()",
            session_id
        )
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let mut data: SessionData = serde_json::from_value(row.data)?;
                data.touch();
                Ok(Some(data))
            }
            None => Ok(None)
        }
    }

    async fn set(&self, session_id: &str, data: &SessionData, ttl: Duration) -> Result<()> {
        let expires_at = chrono::Utc::now() + chrono::Duration::seconds(ttl.as_secs() as i64);
        let json_data = serde_json::to_value(data)?;

        sqlx::query!(
            "INSERT INTO sessions (id, data, expires_at)
             VALUES ($1, $2, $3)
             ON CONFLICT (id) DO UPDATE
             SET data = $2, expires_at = $3",
            session_id,
            json_data,
            expires_at
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete(&self, session_id: &str) -> Result<()> {
        sqlx::query!("DELETE FROM sessions WHERE id = $1", session_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn exists(&self, session_id: &str) -> Result<bool> {
        let exists = sqlx::query!(
            "SELECT 1 as exists FROM sessions WHERE id = $1 AND expires_at > NOW()",
            session_id
        )
        .fetch_optional(&self.pool)
        .await?
        .is_some();

        Ok(exists)
    }

    async fn cleanup_expired(&self) -> Result<usize> {
        let result = sqlx::query!("DELETE FROM sessions WHERE expires_at <= NOW()")
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() as usize)
    }

    fn backend_name(&self) -> &'static str {
        "postgresql"
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    // Step 1: Load config but disable auto session
    let mut config = AppConfig::from_file("config.toml")?;
    config.session.enabled = false;  // IMPORTANT: Disable auto-creation

    // Step 2: Create RustF app without auto session
    let app = RustF::with_config(config.clone());

    // Step 3: Create your custom database storage
    let db_storage = Arc::new(
        DatabaseSessionStorage::new(&config.database.url).await?
    );

    // Step 4: Configure session settings
    let session_config = SessionConfig {
        cookie_name: config.session.cookie_name,
        idle_timeout: Duration::from_secs(config.session.idle_timeout),
        absolute_timeout: Duration::from_secs(config.session.absolute_timeout),
        exempt_routes: config.session.exempt_routes,
        same_site: parse_same_site(&config.session.same_site),
        enabled: true,
        ..Default::default()
    };

    // Step 5: Create session middleware with your storage
    let session_middleware = SessionMiddleware::with_storage(
        db_storage,
        session_config
    );

    // Step 6: Register everything and start
    let app = app
        .middleware_from(|registry| {
            // Register your custom session middleware
            registry.register_dual("session", session_middleware);

            // Add other middleware as needed
            registry.register_dual("cors", CorsMiddleware::new());
        })
        .controllers(auto_controllers!())
        .models(auto_models!());

    println!("🚀 Server starting with PostgreSQL session storage");
    app.start().await
}

fn parse_same_site(value: &str) -> rustf::session::SameSite {
    match value.to_lowercase().as_str() {
        "strict" => rustf::session::SameSite::Strict,
        "lax" => rustf::session::SameSite::Lax,
        "none" => rustf::session::SameSite::None,
        _ => rustf::session::SameSite::Lax,
    }
}

## Session Data Serialization

Sessions use JSON serialization internally via `serde_json`:

```rust
// These types work automatically
ctx.session_set("string", "Hello")?;
ctx.session_set("number", 42)?;
ctx.session_set("boolean", true)?;
ctx.session_set("array", vec![1, 2, 3])?;
ctx.session_set("object", json!({"key": "value"}))?;

// Custom types need Serialize/Deserialize
#[derive(Serialize, Deserialize)]
struct CustomData {
    id: i32,
    name: String,
}

let custom = CustomData { id: 1, name: "test".to_string() };
ctx.session_set("custom", custom)?;

let retrieved: Option<CustomData> = ctx.session_get("custom");
```

## Error Handling

Session operations return `Result<T>` for proper error handling:

```rust
async fn safe_session_usage(ctx: Context) -> Result<Response> {
    // Handle serialization errors
    match ctx.session_set("user_id", 123) {
        Ok(()) => log::info!("Session data saved"),
        Err(e) => log::error!("Failed to save session: {}", e),
    }

    // Handle deserialization
    match ctx.session_get::<i32>("user_id") {
        Some(id) => log::info!("User ID: {}", id),
        None => log::info!("No user ID in session"),
    }

    // Flash message errors are typically ignored
    let _ = ctx.flash_set("message", "Hello");

    ctx.json(json!({"status": "ok"}))
}
```

## Storage Backend Comparison

### Memory Storage (Implemented)

**Pros:**
- Very fast (in-memory access)
- No external dependencies
- Automatic cleanup with background tasks

**Cons:**
- Sessions lost on server restart
- Not suitable for multi-server deployments
- Memory usage grows with active sessions

**Best for:** Development, single-server deployments, temporary sessions

### Redis Storage (Implemented)

**Pros:**
- Persistent across server restarts
- Supports multiple server instances
- Automatic TTL expiration
- High performance with connection pooling

**Cons:**
- Requires Redis server
- Network latency for session access
- Additional infrastructure complexity

**Best for:** Production deployments, multi-server setups, persistent sessions

### Database Storage (User Implementation Required)

**Configuration structure exists but implementation is intentionally delegated to users:**

```toml
# config.toml (configuration structure exists)
[session.storage]
type = "database"
table = "sessions"
connection_url = "postgresql://localhost/myapp"
cleanup_interval = 300
```

**Important:** When you configure database storage, the framework will return an error with instructions:
```
Database session storage must be implemented by the application.
Please implement the SessionStorage trait for your database backend.
See the documentation for examples using SQLx or other database libraries.
```

This is by design - database storage should be implemented by users to:
- Support any database backend (PostgreSQL, MySQL, SQLite, etc.)
- Allow custom table schemas and optimization
- Enable integration with your existing database infrastructure
- Provide flexibility in storage strategies (JSON, normalized, etc.)

## How Session Storage Integration Works

### Architecture Overview

Session storage in RustF is integrated through middleware, not directly through the application builder:

```
RustF → SessionMiddleware → SessionManager → SessionStorage (Your Implementation)
         ↑
    Auto-created from config.toml if session.enabled = true
```

### Default Behavior

When you create a RustF application, it automatically creates session middleware if `session.enabled = true` in your config:

```rust
// This happens automatically in RustF::with_config() (app.rs line 64)
if let Some(session_middleware) = create_session_middleware(&config) {
    middleware.register_dual("session", session_middleware);
}
```

The default session middleware uses `MemorySessionStorage`. To use custom storage, you must override this behavior.

## Integrating Custom Storage

### Method 1: Override Default Session Middleware

**Step 1:** Disable automatic session middleware in config:

```toml
# config.toml
[session]
enabled = false  # Disable auto-creation of session middleware
```

**Step 2:** Create and register your custom session middleware:

```rust
use rustf::prelude::*;
use rustf::middleware::builtin::session::SessionMiddleware;
use rustf::session::manager::SessionConfig;
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // Load config with sessions disabled
    let mut config = AppConfig::from_file("config.toml")?;
    config.session.enabled = false;  // Ensure auto-session is disabled

    // Create RustF without auto session middleware
    let app = RustF::with_config(config.clone());

    // Create your custom storage
    let custom_storage = Arc::new(
        DatabaseSessionStorage::new("postgresql://localhost/myapp").await?
    );

    // Create session configuration
    let session_config = SessionConfig {
        cookie_name: config.session.cookie_name.clone(),
        idle_timeout: Duration::from_secs(config.session.idle_timeout),
        absolute_timeout: Duration::from_secs(config.session.absolute_timeout),
        exempt_routes: config.session.exempt_routes.clone(),
        enabled: true,  // Re-enable for our custom middleware
        ..Default::default()
    };

    // Create session middleware with custom storage
    let session_middleware = SessionMiddleware::with_storage(
        custom_storage,
        session_config
    );

    // Register the custom session middleware
    let app = app.middleware_from(|registry| {
        registry.register_dual("session", session_middleware);
    })
    .controllers(auto_controllers!());

    app.start().await
}
```

### Method 2: Custom Session Manager

Create a custom SessionManager and use it in middleware:

```rust
use rustf::session::manager::{SessionManager, SessionConfig};
use rustf::middleware::builtin::session::SessionMiddleware;

#[tokio::main]
async fn main() -> Result<()> {
    // Disable auto session
    let mut config = AppConfig::default();
    config.session.enabled = false;

    let app = RustF::with_config(config);

    // Create custom storage
    let storage = Arc::new(MyCustomStorage::new().await?);

    // Create custom manager
    let session_manager = SessionManager::new(storage, SessionConfig::default());

    // Create middleware with custom manager
    let session_middleware = SessionMiddleware::with_manager(session_manager);

    // Register it
    let app = app.middleware_from(|registry| {
        registry.register_dual("session", session_middleware);
    })
    .controllers(auto_controllers!());

    app.start().await
}
```

### Method 3: Factory Pattern for Multiple Environments

Create a factory that handles different storage backends based on configuration:

```rust
use rustf::session::factory::SessionStorageFactory;
use rustf::config::SessionStorageConfig;

pub struct MyAppSessionFactory;

impl MyAppSessionFactory {
    pub async fn create_middleware(
        app_config: &AppConfig
    ) -> Result<SessionMiddleware> {
        // Create storage based on config
        let storage = match &app_config.session.storage {
            SessionStorageConfig::Memory { .. } => {
                // Use built-in factory for memory
                SessionStorageFactory::create_storage(&app_config.session.storage).await?
            }
            SessionStorageConfig::Redis { .. } => {
                // Use built-in factory for Redis
                SessionStorageFactory::create_storage(&app_config.session.storage).await?
            }
            SessionStorageConfig::Database { connection_url, .. } => {
                // Use your custom database storage
                Arc::new(DatabaseSessionStorage::new(connection_url).await?)
            }
        };

        // Convert config
        let session_config: SessionConfig = app_config.session.clone().into();

        // Create middleware
        Ok(SessionMiddleware::with_storage(storage, session_config))
    }
}

// Usage in main.rs
#[tokio::main]
async fn main() -> Result<()> {
    let mut config = AppConfig::from_file("config.toml")?;
    config.session.enabled = false;  // Disable auto-creation

    let app = RustF::with_config(config.clone());

    // Create custom middleware based on config
    let session_middleware = MyAppSessionFactory::create_middleware(&config).await?;

    let app = app.middleware_from(|registry| {
        registry.register_dual("session", session_middleware);
    })
    .controllers(auto_controllers!());

    app.start().await
}
```

## Important Notes

### What DOESN'T Work

```rust
// ❌ WRONG - These methods don't exist:
app.with_session_store(session_store)  // No such method
SessionStore::with_storage(storage)    // SessionStore is internal to SessionManager
```

### What DOES Work

```rust
// ✅ CORRECT - Use SessionMiddleware methods:
SessionMiddleware::with_storage(storage, config)  // Custom storage
SessionMiddleware::with_manager(manager)          // Custom manager
SessionMiddleware::new(config)                    // Default memory storage
```

### Key Integration Points

1. **SessionMiddleware** - The main integration point for custom storage
2. **SessionManager** - Manages session lifecycle and storage interaction
3. **SessionStorage trait** - Your custom implementation goes here
4. **config.session.enabled** - Must be `false` to prevent auto-creation

## Monitoring and Statistics

Currently implemented storage backends provide statistics for monitoring:

```rust
// Access statistics through your storage implementation
async fn session_monitoring(storage: &Arc<dyn SessionStorage>) -> Result<()> {
    let stats = storage.stats().await?;

    println!("Session Statistics:");
    println!("  Total sessions: {}", stats.total_sessions);
    println!("  Active sessions: {}", stats.active_sessions);
    println!("  Expired sessions: {}", stats.expired_sessions);
    println!("  Backend: {}", session_store.backend_name());

    // Backend-specific metrics
    for (key, value) in &stats.backend_metrics {
        println!("  {}: {}", key, value);
    }

    Ok(())
}
```

### Memory Storage Metrics

- `total_data_entries`: Total data entries across all sessions
- `total_flash_entries`: Total flash messages across all sessions
- `oldest_session_age_secs`: Age of oldest session in seconds
- `session_timeout_secs`: Session timeout configuration
- `cleanup_interval_secs`: Cleanup interval configuration

### Redis Storage Metrics

- `redis_pattern`: Key pattern used for sessions
- `scan_method`: Scanning method used (non-blocking)
- `redis_memory_used`: Current Redis memory usage
- `redis_memory_peak`: Peak Redis memory usage

## Best Practices

### 1. Keep Session Data Small
Store only essential data in sessions to minimize memory/storage usage:

```rust
// Good: Store minimal user info
ctx.session_set("user_id", 123)?;
ctx.session_set("role", "admin")?;

// Less ideal: Store large objects
ctx.session_set("full_user_profile", large_user_object)?;
```

### 2. Use Flash Messages for UI Feedback
Flash messages are perfect for one-time user notifications:

```rust
async fn create_user(ctx: Context) -> Result<Response> {
    match create_user_in_db(&user_data).await {
        Ok(_) => {
            ctx.flash_success("User created successfully!");
            ctx.redirect("/users")
        }
        Err(e) => {
            ctx.flash_error(&format!("Failed to create user: {}", e));
            ctx.redirect("/users/new")
        }
    }
}
```

### 3. Handle Session Expiration Gracefully
Check for required session data and handle missing sessions:

```rust
async fn protected_route(ctx: Context) -> Result<Response> {
    match ctx.session_get::<i32>("user_id") {
        Some(user_id) => {
            // User is logged in, continue
            handle_authenticated_request(ctx, user_id).await
        }
        None => {
            ctx.flash_info("Please log in to continue");
            ctx.redirect("/login")
        }
    }
}
```

### 4. Use Appropriate Storage Backend
- **Development**: Memory storage is fine
- **Single server production**: Memory storage with sufficient RAM
- **Multi-server production**: Redis storage for session sharing

### 5. Configure Appropriate Timeouts
Balance security and user experience:

```rust
// Short timeout for sensitive data
let secure_storage = MemorySessionStorage::with_timeout(
    Duration::from_secs(15 * 60), // 15 minutes
    Duration::from_secs(2 * 60)   // 2 minutes cleanup
);

// Longer timeout for general use
let general_storage = MemorySessionStorage::with_timeout(
    Duration::from_secs(2 * 60 * 60), // 2 hours
    Duration::from_secs(10 * 60)      // 10 minutes cleanup
);
```

### 6. Use Appropriate Session Lifecycle Methods
Choose the right method for different scenarios:

```rust
// User logout - clear data but keep session for analytics
async fn logout(ctx: Context) -> Result<Response> {
    ctx.session_clear();  // Keep session ID for tracking
    ctx.flash_success("You have been logged out");
    ctx.redirect("/login")
}

// Security incident - completely destroy session
async fn security_logout(ctx: Context, session_store: &SessionStore) -> Result<Response> {
    let session_id = ctx.session.id();
    session_store.destroy_session(session_id).await?;  // Complete removal
    ctx.redirect("/login")
}

// After login - regenerate ID to prevent session fixation
async fn after_login(ctx: Context) -> Result<Response> {
    ctx.session.regenerate_id();  // Security best practice
    ctx.session_set("user_id", user.id)?;
    ctx.redirect("/dashboard")
}
```

## Integration with Application

Sessions are automatically available in all controllers and middleware:

```rust
use rustf::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let app = RustF::new()
        .controllers(auto_controllers!())
        .middleware_from(auto_middleware!());

    // Sessions work automatically - no additional configuration needed
    app.serve(None).await
}
```

The session system integrates seamlessly with the RustF context system, providing a consistent API across controllers, middleware, and views for managing user state and temporary messages.
