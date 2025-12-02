# RustF Event System Guide

🎯 **Total.js-Inspired Event-Driven Architecture**

This guide covers RustF's comprehensive event system, designed to provide Total.js-style `ON('ready', function())` patterns for application lifecycle management and extensible startup code execution.

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [Application Lifecycle Events](#application-lifecycle-events)
- [Event Registration](#event-registration)  
- [Parallel Execution](#parallel-execution)
- [Performance Configuration](#performance-configuration)
- [Built-in Event Handlers](#built-in-event-handlers)
- [Custom Event Handlers](#custom-event-handlers)
- [Event Context](#event-context)
- [Priority System](#priority-system)
- [Auto-Discovery](#auto-discovery)
- [Common Patterns](#common-patterns)
- [Performance Considerations](#performance-considerations)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)

## Overview

RustF's event system enables developers to hook into application lifecycle events using a familiar, Total.js-inspired pattern. This system provides:

### Key Features

✅ **Total.js-Style Syntax** - Familiar `ON('ready', handler)` pattern  
✅ **Lifecycle Events** - Built-in events for all major application stages  
✅ **Priority-Based Execution** - Control handler execution order  
✅ **Parallel Execution** - Concurrent handler execution within priority groups for optimal performance  
✅ **Built-in Handlers** - Ready-to-use handlers for common tasks  
✅ **Event Context** - Rich context with app config, environment, and utilities  
✅ **Auto-Discovery** - Automatic event handler registration  
✅ **Type Safety** - Full compile-time type checking  
✅ **Async Native** - Built for async/await from the ground up  
✅ **Performance Optimized** - Fast-path execution, configurable timeouts, and error isolation  

### Event-Driven Benefits

- **Decoupled Code**: Event handlers don't need to know about each other
- **Extensible**: Easy to add new functionality without modifying core code
- **Testable**: Individual event handlers can be tested in isolation
- **Organized**: Clean separation between initialization logic
- **Flexible**: Conditional execution based on environment or configuration

## Quick Start

### Basic Event Registration

```rust
use rustf::prelude::*;

#[tokio::main]
async fn main() -> rustf::Result<()> {
    let app = RustF::new()
        // Register event handlers using familiar Total.js syntax
        .on("ready", |ctx| Box::pin(async move {
            println!("🚀 Application ready in {} mode!", ctx.env());
            Ok(())
        }))
        
        .on("startup", |ctx| Box::pin(async move {
            println!("⚡ Startup tasks executing...");
            // Database seeding, directory creation, etc.
            Ok(())
        }))
        
        .controllers(auto_controllers!());
    
    app.start().await
}
```

### Using Built-in Handlers

```rust
use rustf::prelude::*;

let app = RustF::new()
    // Built-in handlers for common tasks
    .on("startup", builtin::directory_setup(&["uploads", "temp", "logs"]))
    .on("startup", builtin::cleanup_manager("temp/", Duration::from_secs(3600)))
    .on("config.loaded", builtin::configuration_validator)
    .on("ready", builtin::health_check)
    
    .controllers(auto_controllers!());
```

## Application Lifecycle Events

RustF emits events automatically during application startup, providing hooks for every stage of initialization:

### Core Lifecycle Events

| Event | When Emitted | Use Case |
|-------|--------------|----------|
| `config.loaded` | Configuration loaded and validated | Config validation, environment checks |
| `database.ready` | Database connection established | Database seeding, migration checks |
| `modules.ready` | Shared modules initialized | Module-dependent initialization |
| `middleware.ready` | Middleware chain configured | Middleware-dependent setup |
| `routes.ready` | Routes registered | Route-dependent initialization |
| `startup` | Before server starts listening | Final startup tasks |
| `ready` | Framework fully initialized | Application ready notifications |

### Event Execution Order

```
1. config.loaded    ← Configuration loaded
2. database.ready   ← Database connected  
3. modules.ready    ← Shared modules initialized
4. middleware.ready ← Middleware configured
5. routes.ready     ← Routes registered
6. startup          ← Pre-server startup tasks
7. ready            ← Application fully ready
   ↓
   Server starts listening
```

### Custom Events

You can also emit custom events from within handlers:

```rust
.on("ready", |ctx| Box::pin(async move {
    println!("Application ready!");
    
    // Emit a custom event
    ctx.emit("custom.initialization", Some(json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "environment": ctx.env()
    }))).await?;
    
    Ok(())
}))

.on("custom.initialization", |ctx| Box::pin(async move {
    if let Some(data) = &ctx.data {
        println!("Custom event data: {}", data);
    }
    Ok(())
}))
```

## Event Registration

### Registration Methods

#### `.on(event, handler)` - Basic Registration

```rust
app.on("ready", |ctx| Box::pin(async move {
    println!("Application ready!");
    Ok(())
}))
```

#### `.on_priority(event, priority, handler)` - Priority-Based

```rust
app.on_priority("startup", -100, |ctx| Box::pin(async move {
    println!("High priority task (runs first)");
    Ok(())
}))

.on_priority("startup", 100, |ctx| Box::pin(async move {
    println!("Low priority task (runs last)");
    Ok(())
}))
```

#### `.events_from(register_fn)` - Bulk Registration

```rust
app.events_from(|emitter| {
    emitter.on("ready", startup_logger);
    emitter.on("database.ready", database_seeder);
    emitter.once("ready", one_time_setup);
})
```

#### `.once(event, handler)` - One-Time Handlers

```rust
// This handler will only execute once, even if the event is emitted multiple times
emitter.once("ready", |ctx| Box::pin(async move {
    println!("This runs only once!");
    Ok(())
}))
```

## Parallel Execution

RustF's event system features high-performance parallel execution of handlers within the same priority group, while maintaining strict priority ordering between groups.

### How Parallel Execution Works

```rust
// Multiple handlers at the same priority execute in parallel
app.on("ready", |_ctx| Box::pin(async move {
    println!("Handler 1 starting...");
    tokio::time::sleep(Duration::from_millis(100)).await; // Simulated work
    println!("Handler 1 done");
    Ok(())
}))

.on("ready", |_ctx| Box::pin(async move {
    println!("Handler 2 starting...");
    tokio::time::sleep(Duration::from_millis(100)).await; // Simulated work  
    println!("Handler 2 done");
    Ok(())
}))

.on("ready", |_ctx| Box::pin(async move {
    println!("Handler 3 starting...");
    tokio::time::sleep(Duration::from_millis(100)).await; // Simulated work
    println!("Handler 3 done");
    Ok(())
}));

// All 3 handlers execute concurrently!
// Total execution time: ~100ms (instead of ~300ms sequential)
```

### Priority Groups and Parallel Execution

```rust
app
    // Priority -100: Infrastructure handlers (execute first, in parallel)
    .on_priority("startup", -100, database_init_handler)    // Runs in parallel
    .on_priority("startup", -100, cache_init_handler)       // with this one
    .on_priority("startup", -100, logging_init_handler)     // and this one
    
    // Wait for all priority -100 handlers to complete...
    
    // Priority 0: Application handlers (execute second, in parallel)  
    .on("startup", user_service_init)     // Runs in parallel
    .on("startup", email_service_init)    // with this one
    .on("startup", file_service_init)     // and this one
    
    // Wait for all priority 0 handlers to complete...
    
    // Priority 100: Cleanup handlers (execute last, in parallel)
    .on_priority("startup", 100, temp_cleanup)         // Runs in parallel
    .on_priority("startup", 100, session_cleanup)      // with this one
```

### Performance Benefits

**Example Performance Improvement:**
- 4 handlers, each taking 100ms
- **Sequential execution**: 400ms total
- **Parallel execution**: 100ms total (**4x faster!**)

## Performance Configuration

Configure event system performance settings for optimal behavior:

### Basic Configuration

```rust
use rustf::events::EventEmitterConfig;
use std::time::Duration;

let app = RustF::new()
    .event_config(EventEmitterConfig::parallel() // Enable parallel execution
        .with_timeout(Duration::from_secs(30))    // Handler timeout protection
        .with_debug_logging(false)                // Disable debug logging for performance
        .with_max_concurrent(8))                  // Limit concurrent handlers
```

### Configuration Presets

```rust
// High-performance configuration (production)
let config = EventEmitterConfig::parallel()
    .with_timeout(Duration::from_secs(60))
    .with_debug_logging(false);

// Debug-friendly configuration (development)
let config = EventEmitterConfig::sequential()
    .with_timeout(Duration::from_secs(10))
    .with_debug_logging(true);

let app = RustF::new().event_config(config);
```

### Configuration Options

| Option | Description | Default | Recommendation |
|--------|-------------|---------|----------------|
| `parallel_execution` | Enable parallel handler execution | `true` | `true` for production, `false` for debugging |
| `handler_timeout` | Maximum handler execution time | 30 seconds | 30-60s for production, 10s for development |
| `debug_logging` | Enable detailed execution logging | `cfg!(debug_assertions)` | `false` for production (performance) |
| `max_concurrent_handlers` | Limit concurrent handlers per priority | `0` (unlimited) | 4-12 based on workload |

## Built-in Event Handlers

RustF provides ready-to-use event handlers for common startup tasks:

### Database Seeder

Automatically runs SQL seed files from a directory:

```rust
.on("database.ready", builtin::database_seeder("seeds/"))
```

**Features:**
- Executes SQL files in alphabetical order
- Only runs in development by default
- Handles multiple database types
- Continues on individual file failures

### Directory Setup

Ensures required directories exist with proper permissions:

```rust
.on("startup", builtin::directory_setup(&[
    "uploads", "temp", "logs", "cache", "sessions"
]))
```

**Features:**
- Creates missing directories
- Sets appropriate permissions (755 on Unix)
- Logs creation status
- Safe for repeated execution

### Cleanup Manager

Removes old temporary files and performs cleanup:

```rust
.on("startup", builtin::cleanup_manager("temp/", Duration::from_secs(86400)))
```

**Features:**
- Removes files older than specified duration
- Handles both files and directories
- Logs cleanup statistics
- Safe error handling

### Configuration Validator

Validates critical configuration settings:

```rust
.on("config.loaded", builtin::configuration_validator)
```

**Features:**
- Validates production-specific settings
- Checks for default/insecure values
- Environment-specific validation rules
- Fails fast on critical issues

### Health Check

Performs basic application health checks:

```rust
.on("ready", builtin::health_check)
```

**Features:**
- Database connectivity check
- System resource validation
- Environment-specific checks
- Detailed health reporting

### Environment Check

Validates the application is running in the expected environment:

```rust
.on("startup", builtin::environment_check("production"))
```

**Features:**
- Environment validation
- Fails in production if mismatch detected
- Configurable expected environment
- Detailed error reporting

## Custom Event Handlers

### Creating Custom Handlers

```rust
// Simple handler function
async fn database_migration_check(ctx: EventContext) -> rustf::Result<()> {
    if ctx.is_production() {
        println!("Checking database migrations in production...");
        // Check migration status
        run_migration_check().await?;
    }
    Ok(())
}

// Register the handler
app.on("database.ready", |ctx| Box::pin(database_migration_check(ctx)))
```

### Handler Functions vs Closures

```rust
// Function-based handler (reusable)
async fn reusable_handler(ctx: EventContext) -> rustf::Result<()> {
    println!("Handler executed for: {}", ctx.event);
    Ok(())
}

app.on("ready", |ctx| Box::pin(reusable_handler(ctx)))

// Closure-based handler (inline)
app.on("startup", |ctx| Box::pin(async move {
    println!("Inline handler for {}", ctx.event);
    // Access closure variables
    let config = &ctx.config;
    Ok(())
}))
```

### Conditional Handlers

```rust
app.on("ready", |ctx| Box::pin(async move {
    match ctx.env() {
        "development" => {
            println!("Development setup");
            setup_dev_data().await?;
        }
        "production" => {
            println!("Production validation");
            validate_production_config(&ctx.config)?;
        }
        _ => {
            println!("Default environment setup");
        }
    }
    Ok(())
}))
```

## Event Context

The `EventContext` provides rich information and utilities for event handlers:

### Context Properties

```rust
async fn handler_example(ctx: EventContext) -> rustf::Result<()> {
    // Event information
    println!("Event: {}", ctx.event);
    println!("Environment: {}", ctx.env());
    
    // Environment checks
    if ctx.is_development() {
        println!("Running in development mode");
    }
    
    if ctx.is_production() {
        println!("Running in production mode");
    }
    
    // Configuration access
    let server_port = ctx.config.server.port;
    let database_url = &ctx.config.database.url;
    
    // Event data (if provided)
    if let Some(data) = &ctx.data {
        println!("Event data: {}", data);
    }
    
    // Emit other events
    ctx.emit("custom.event", Some(json!({"from": ctx.event}))).await?;
    
    Ok(())
}
```

### Available Context Methods

| Method | Description | Example |
|--------|-------------|---------|
| `ctx.event` | Current event name | `"ready"` |
| `ctx.env()` | Current environment | `"development"` |
| `ctx.is_development()` | Development check | `true` |
| `ctx.is_production()` | Production check | `false` |
| `ctx.config` | Application config | `ctx.config.server.port` |
| `ctx.data` | Event data | `json!({"key": "value"})` |
| `ctx.emit(event, data)` | Emit another event | Custom event emission |

## Priority System

Control the execution order of event handlers using priorities:

### Priority Values

- **-200 to -100**: Infrastructure (database, core systems)
- **-99 to -50**: Security and authentication
- **-49 to -1**: Business logic prerequisites  
- **0**: Default priority (recommended for most handlers)
- **1 to 99**: Business logic
- **100+**: Post-processing and cleanup

### Priority Examples

```rust
app
    // Critical infrastructure (runs first)
    .on_priority("startup", -100, builtin::directory_setup(&["logs"]))
    
    // Security setup
    .on_priority("startup", -50, security_initialization)
    
    // Default priority (most handlers)
    .on("startup", application_setup)
    
    // Cleanup tasks (runs last)
    .on_priority("startup", 100, final_cleanup)
```

### Execution Flow

```
Priority -100: Infrastructure setup
Priority -50:  Security initialization  
Priority 0:    Application setup (default)
Priority 100:  Final cleanup
```

## Auto-Discovery

RustF can automatically discover and register event handlers from your codebase:

### File Structure

```
src/
  events/           ← Event handler modules
    database.rs     ← Database-related events
    filesystem.rs   ← File system events
    security.rs     ← Security events
    custom.rs       ← Custom application events
```

### Event Handler Module

```rust
// src/events/database.rs
use rustf::events::{EventEmitter, EventContext};

pub fn install(emitter: &mut EventEmitter) {
    emitter.on("database.ready", seed_development_data);
    emitter.on("ready", database_health_check);
}

async fn seed_development_data(ctx: EventContext) -> rustf::Result<()> {
    if ctx.is_development() {
        println!("Seeding development data...");
        // Database seeding logic
    }
    Ok(())
}

async fn database_health_check(ctx: EventContext) -> rustf::Result<()> {
    println!("Running database health check...");
    // Health check logic
    Ok(())
}
```

### Auto-Discovery Registration

```rust
// main.rs
use rustf::prelude::*;

let app = RustF::new()
    .events_from(auto_events!())  // Auto-discovers src/events/*.rs
    .controllers(auto_controllers!());
```

**Note**: Auto-discovery requires the `auto-discovery` feature to be enabled.

## Common Patterns

### Database Initialization Pattern

```rust
app.on("database.ready", |ctx| Box::pin(async move {
    if ctx.is_development() {
        // Seed development data
        println!("Seeding development database...");
        seed_database().await?;
    } else if ctx.is_production() {
        // Validate production database
        println!("Validating production database...");
        validate_database_schema().await?;
    }
    Ok(())
}))
```

### Multi-Environment Setup

```rust
app.on("ready", |ctx| Box::pin(async move {
    match ctx.env() {
        "development" => {
            setup_debug_tools().await?;
            enable_hot_reload().await?;
        }
        "staging" => {
            setup_staging_environment().await?;
            enable_performance_monitoring().await?;
        }
        "production" => {
            validate_security_settings(&ctx.config)?;
            enable_production_monitoring().await?;
        }
        env => {
            log::warn!("Unknown environment: {}", env);
        }
    }
    Ok(())
}))
```

### Dependency Chain Pattern

```rust
app
    // Step 1: Initialize core services
    .on_priority("startup", -100, |ctx| Box::pin(async move {
        initialize_core_services().await?;
        ctx.emit("services.ready", None).await?;
        Ok(())
    }))
    
    // Step 2: Setup dependent services
    .on("services.ready", |ctx| Box::pin(async move {
        setup_dependent_services().await?;
        ctx.emit("dependencies.ready", None).await?;
        Ok(())
    }))
    
    // Step 3: Final application setup
    .on("dependencies.ready", |ctx| Box::pin(async move {
        finalize_application_setup().await?;
        Ok(())
    }))
```

### Error Recovery Pattern

```rust
app.on("database.ready", |ctx| Box::pin(async move {
    match connect_to_database().await {
        Ok(()) => {
            println!("✅ Database connected");
            ctx.emit("database.connected", None).await?;
        }
        Err(e) => {
            log::error!("❌ Database connection failed: {}", e);
            
            if ctx.is_production() {
                // Fail fast in production
                return Err(e);
            } else {
                // Try fallback in development
                setup_fallback_database().await?;
                ctx.emit("database.fallback", None).await?;
            }
        }
    }
    Ok(())
}))
```

## Best Practices

### 1. Use Descriptive Event Names

```rust
// Good: Clear and descriptive
.on("database.schema.validated", handler)
.on("security.certificates.loaded", handler)
.on("cache.warmed", handler)

// Avoid: Generic or unclear
.on("done", handler)
.on("init", handler)
.on("setup", handler)
```

### 2. Leverage Priority System

```rust
// Good: Logical priority ordering
.on_priority("startup", -100, create_directories)    // Infrastructure
.on_priority("startup", -50, load_certificates)      // Security
.on_priority("startup", 0, initialize_services)      // Business logic
.on_priority("startup", 100, warm_caches)           // Optimization
```

### 3. Handle Errors Gracefully

```rust
.on("ready", |ctx| Box::pin(async move {
    match risky_operation().await {
        Ok(()) => {
            log::info!("✅ Operation completed successfully");
        }
        Err(e) => {
            log::error!("❌ Operation failed: {}", e);
            
            // Don't fail the entire application for non-critical errors
            if is_critical_error(&e) {
                return Err(e);
            }
            
            // Try fallback or continue
            attempt_fallback().await.ok();
        }
    }
    Ok(())
}))
```

### 4. Use Environment-Specific Logic

```rust
.on("ready", |ctx| Box::pin(async move {
    if ctx.is_development() {
        setup_development_tools().await?;
        seed_test_data().await?;
    }
    
    if ctx.is_production() {
        validate_production_config(&ctx.config)?;
        setup_monitoring().await?;
    }
    
    Ok(())
}))
```

### 5. Emit Custom Events for Extension Points

```rust
.on("ready", |ctx| Box::pin(async move {
    initialize_core_application().await?;
    
    // Emit custom event for plugins/extensions
    ctx.emit("application.plugins.load", None).await?;
    
    finalize_initialization().await?;
    Ok(())
}))

// Extensions can hook into the custom event
.on("application.plugins.load", |ctx| Box::pin(async move {
    load_custom_plugins().await?;
    Ok(())
}))
```

### 6. Document Event Contracts

```rust
/// Emitted when the user authentication system is fully initialized
/// 
/// Context Data: None
/// Prerequisites: database.ready, security.certificates.loaded
/// Guarantees: User authentication is available for requests
.on("auth.system.ready", auth_system_handler)
```

## Troubleshooting

### Common Issues

#### 1. Handlers Not Executing

**Problem**: Event handlers don't seem to run.

**Solutions**:
- Verify event names match exactly (case-sensitive)
- Check that events are being emitted by the framework
- Ensure handler registration happens before `app.start()`

```rust
// ❌ Incorrect event name
.on("readdy", handler)  // Typo

// ✅ Correct event name  
.on("ready", handler)
```

#### 2. Handler Execution Order Issues

**Problem**: Handlers run in unexpected order.

**Solutions**:
- Use priority system to control execution order
- Check priority values (lower = earlier execution)
- Avoid depending on registration order

```rust
// ❌ Relying on registration order
.on("startup", handler_a)  // Might run second
.on("startup", handler_b)  // Might run first

// ✅ Using explicit priorities
.on_priority("startup", 10, handler_a)  // Runs first
.on_priority("startup", 20, handler_b)  // Runs second
```

#### 3. Async Handler Issues

**Problem**: Async operations not completing or compiler errors.

**Solutions**:
- Always wrap handlers with `Box::pin(async move { ... })`
- Ensure all async operations are awaited
- Return `rustf::Result<()>` from handlers

```rust
// ❌ Missing Box::pin
.on("ready", |ctx| async move {  // Compiler error
    Ok(())
})

// ✅ Proper async handler
.on("ready", |ctx| Box::pin(async move {
    some_async_operation().await?;
    Ok(())
}))
```

#### 4. Context Data Not Available

**Problem**: `ctx.data` is always `None`.

**Solutions**:
- Check that events are emitted with data
- Verify JSON serialization of event data
- Use built-in events with expected data structure

```rust
// Emitting event with data
ctx.emit("custom.event", Some(json!({
    "message": "Hello",
    "timestamp": chrono::Utc::now()
}))).await?;

// Receiving event data
.on("custom.event", |ctx| Box::pin(async move {
    if let Some(data) = &ctx.data {
        println!("Received: {}", data);
    }
    Ok(())
}))
```

#### 5. Environment Detection Issues

**Problem**: Environment methods return unexpected values.

**Solutions**:
- Set environment variables properly
- Check supported environment variable names
- Use explicit environment checks

```bash
# Set environment (choose one)
export NODE_ENV=production
export RUST_ENV=production  
export APP_ENV=production
```

```rust
// Debug environment detection
.on("ready", |ctx| Box::pin(async move {
    println!("Environment: {}", ctx.env());
    println!("Is development: {}", ctx.is_development());
    println!("Is production: {}", ctx.is_production());
    Ok(())
}))
```

### Debugging Events

Enable debug logging to see event execution:

```rust
// In main.rs
env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

// Events will log execution details:
// [DEBUG] Registered event handler for 'ready' with priority 0 (id: 1)
// [INFO]  Emitting event: 'ready'
// [DEBUG] Executing handler 1 for 'ready' (priority: 0)
// [INFO]  Event 'ready' executed 1 handler(s)
```

## Performance Considerations

### Parallel vs Sequential Execution

RustF automatically optimizes event handler execution for maximum performance:

**Parallel Execution (Default)**:
- Handlers within the same priority group execute concurrently
- Significantly faster for I/O bound tasks (database calls, file operations)
- Optimal for production environments
- Example: 4 handlers × 100ms each = **100ms total** (4x speedup)

**Sequential Execution (Debug Mode)**:
- Handlers execute one after another  
- Easier to debug and trace execution
- Recommended for development and testing
- Example: 4 handlers × 100ms each = **400ms total**

### Performance Optimization Tips

#### 1. **Enable Parallel Execution in Production**
```rust
let app = RustF::new()
    .event_config(EventEmitterConfig::parallel()
        .with_debug_logging(false));  // Disable debug logging for performance
```

#### 2. **Use Priority Groups Strategically**
```rust
// ✅ Good: Group related handlers by priority
.on_priority("startup", -100, database_init)     // Infrastructure first
.on_priority("startup", -100, cache_init)        // (parallel with above)
.on_priority("startup", 0, service_init)         // Services second
.on_priority("startup", 0, api_init)             // (parallel with above)

// ❌ Avoid: Mixed priorities force sequential execution
.on_priority("startup", -100, database_init)
.on_priority("startup", 0, service_init)         // Must wait for database_init
.on_priority("startup", -99, cache_init)         // Must wait for service_init
```

#### 3. **Configure Timeouts Appropriately**
```rust
// Production: Longer timeouts for complex operations
.event_config(EventEmitterConfig::parallel()
    .with_timeout(Duration::from_secs(60)))

// Development: Shorter timeouts to catch hung handlers
.event_config(EventEmitterConfig::sequential()
    .with_timeout(Duration::from_secs(10)))
```

#### 4. **Optimize Handler Design**
```rust
// ✅ Good: Fast, focused handlers
.on("ready", |ctx| Box::pin(async move {
    log::info!("Application ready in {} mode", ctx.env());
    Ok(())
}))

// ✅ Good: Async I/O operations benefit from parallel execution
.on("startup", |_ctx| Box::pin(async move {
    database::migrate().await?;  // Can run in parallel with other I/O
    Ok(())
}))

// ❌ Avoid: CPU-intensive work that doesn't benefit from parallelism
.on("startup", |_ctx| Box::pin(async move {
    // Heavy CPU work - consider moving to background task
    expensive_computation();
    Ok(())
}))
```

### Performance Monitoring

#### Runtime Performance Metrics

Enable debug logging to monitor handler performance:

```rust
let app = RustF::new()
    .event_config(EventEmitterConfig::parallel()
        .with_debug_logging(true));  // Enable in development

// Console output:
// [INFO]  Emitting event: 'startup' (parallel: true)
// [DEBUG] Executing 3 handler(s) for 'startup' at priority -100 (parallel: true)
// [DEBUG] Handler 1 completed in 145ms
// [DEBUG] Handler 2 completed in 203ms 
// [DEBUG] Event 'startup' executed 3 handler(s) with 0 error(s)
```

#### Production Monitoring

```rust
.on("ready", |ctx| Box::pin(async move {
    let start = std::time::Instant::now();
    
    // Your handler logic here
    initialize_services().await?;
    
    let duration = start.elapsed();
    if duration > Duration::from_millis(1000) {
        log::warn!("Slow handler execution: {}ms", duration.as_millis());
    }
    
    Ok(())
}))
```

### Performance Best Practices

1. **Keep Handlers Fast**: Event handlers block application startup
2. **Use Parallel Execution**: Default configuration optimizes for production
3. **Group by Priority**: Related handlers at same priority execute concurrently  
4. **Set Appropriate Timeouts**: Prevent runaway handlers from hanging startup
5. **Monitor in Production**: Track handler execution times
6. **Defer Heavy Work**: Move non-essential tasks to background jobs
7. **Disable Debug Logging**: Reduces overhead in production

### Performance Anti-Patterns

```rust
// ❌ Bad: Synchronous blocking operations
.on("startup", |_ctx| Box::pin(async move {
    std::thread::sleep(Duration::from_secs(5));  // Blocks executor!
    Ok(())
}))

// ✅ Good: Async operations
.on("startup", |_ctx| Box::pin(async move {
    tokio::time::sleep(Duration::from_secs(5)).await;  // Non-blocking
    Ok(())
}))

// ❌ Bad: Sequential chains that could be parallel
.on_priority("startup", 1, handler_a)
.on_priority("startup", 2, handler_b)  // Must wait for handler_a
.on_priority("startup", 3, handler_c)  // Must wait for handler_b

// ✅ Good: Parallel execution when possible
.on("startup", handler_a)  // All execute in parallel
.on("startup", handler_b)  
.on("startup", handler_c)
```

---

## Summary

RustF's event system provides a powerful, Total.js-inspired approach to application lifecycle management with advanced parallel execution capabilities:

✅ **Familiar Syntax** - Total.js-style `ON('ready', handler)` patterns  
✅ **Complete Lifecycle** - Events for every stage of application startup  
✅ **Parallel Execution** - Concurrent handler execution with 4x+ performance improvements  
✅ **Built-in Handlers** - Ready-to-use handlers for common tasks  
✅ **Priority Control** - Fine-grained execution order management with parallel optimization  
✅ **Performance Optimized** - Fast-path execution, timeout protection, and error isolation  
✅ **Type Safety** - Full compile-time checking with excellent error messages  
✅ **Auto-Discovery** - Automatic handler registration from your codebase  
✅ **Rich Context** - Comprehensive context with config, environment, and utilities  
✅ **Production Ready** - Configurable performance settings and comprehensive monitoring  

## Related Topics

- [API Reference: Context](../api-reference/context.md) - Context API documentation
- [Examples](../examples/README.md) - Practical examples and tutorials
- [Workers](workers.md) - Background job processing
- [Modules](modules.md) - Shared business logic modules