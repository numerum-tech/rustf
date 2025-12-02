# RustF Worker System

## Table of Contents

- [Introduction](#introduction)
- [Quick Start](#quick-start)
- [Core Concepts](#core-concepts)
- [Worker Registration](#worker-registration)
- [Executing Workers](#executing-workers)
- [Worker Context](#worker-context)
- [Advanced Features](#advanced-features)
- [Real-World Examples](#real-world-examples)
- [Best Practices](#best-practices)
- [API Reference](#api-reference)
- [Troubleshooting](#troubleshooting)

---

## Introduction

The RustF Worker system provides a lightweight, Total.js-inspired approach to background task execution. Unlike traditional worker queues or job processors, RustF workers are **on-demand tasks** that execute asynchronously and automatically stop when complete.

### Why Workers?

Workers are ideal for:

- **Background Processing**: Long-running tasks that shouldn't block HTTP responses
- **Async Operations**: Email sending, file uploads, report generation
- **Scheduled Tasks**: Cleanup, maintenance, data synchronization
- **Decoupled Logic**: Separating business logic from HTTP request handling
- **Concurrent Execution**: Running multiple independent tasks simultaneously

### Key Features

- **🌍 Global API**: Access workers from anywhere using `WORKER::`
- **⚡ On-Demand**: Workers execute when called, no persistent processes
- **⏱️ Timeout Support**: Automatic cancellation after specified duration
- **📨 Message Streaming**: Real-time communication between worker and caller
- **📊 Statistics**: Automatic tracking of runs, errors, and execution times
- **🔄 Concurrent**: Multiple workers can run simultaneously
- **🎯 Type-Safe**: Full Rust type safety with async/await

### Design Philosophy

RustF workers follow Total.js conventions:

- **Registration-based**: Define workers once, execute many times
- **Context-rich**: Workers receive a `WorkerContext` with configuration and utilities
- **Fire-and-forget or await**: Choose between async execution or waiting for results
- **Stateless**: Each execution is independent with isolated state

---

## Quick Start

### 1. Create Worker Directory

Create the required `src/workers/` directory:

```bash
mkdir -p src/workers
```

### 2. Create Your First Worker

Create a worker file in `src/workers/`:

```rust
// src/workers/email.rs
use rustf::prelude::*;
use rustf::workers::WORKER;
use std::time::Duration;

pub async fn install() -> Result<()> {
    WORKER::register("send-email", |ctx| async move {
        ctx.info("Sending email...");

        // Simulate email sending
        tokio::time::sleep(Duration::from_secs(2)).await;

        ctx.info("Email sent successfully!");
        Ok(())
    }).await?;

    Ok(())
}
```

### 3. Enable Auto-Discovery

Use `#[rustf::auto_discover]` in your main function:

```rust
// src/main.rs
use rustf::prelude::*;

#[rustf::auto_discover]  // ← Automatically discovers workers in src/workers/
#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let app = RustF::new()
        .auto_load()  // ← Loads all discovered workers
        .run("127.0.0.1:3000")
        .await?;

    Ok(())
}
```

### 4. Execute the Worker

Call the worker from any controller:

```rust
use rustf::workers::WORKER;

async fn send_welcome_email(ctx: &mut Context) -> Result<()> {
    // Execute worker and wait for completion
    WORKER::run("send-email", None).await?;

    ctx.json(json!({
        "status": "Email sent"
    }))
}
```

**That's it!** The framework automatically discovers and registers all workers in `src/workers/`.

---

## Core Concepts

### Worker Lifecycle

```
┌─────────────┐
│ Registration│  ← Define worker once during startup
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   Idle      │  ← Worker definition exists, waiting to be called
└──────┬──────┘
       │
       ▼ WORKER::call()
┌─────────────┐
│  Running    │  ← Worker executing asynchronously
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Completed   │  ← Worker finishes, stats recorded
└─────────────┘
```

**Important**: RustF workers are **not persistent processes**. Each execution:
1. Starts fresh with new context
2. Runs to completion
3. Automatically stops and cleans up
4. Records statistics

### WorkerContext

Every worker receives a `WorkerContext` providing:

- **Identity**: Worker name and run ID
- **Configuration**: Access to app config
- **Payload**: Input data passed during execution
- **Logging**: Structured logging with worker identification
- **State**: Per-execution state management
- **Messaging**: Emit messages to caller
- **Environment**: Development vs. production detection
- **Utilities**: Sleep, timing, etc.

```rust
WORKER::register("example", |ctx| async move {
    ctx.info(&format!("Worker: {}, Run: {}",
        ctx.worker_name(),
        ctx.run_id()
    ));

    if ctx.is_development() {
        ctx.debug("Running in development mode");
    }

    // Access payload
    if let Some(data) = ctx.payload() {
        ctx.info(&format!("Received: {}", data));
    }

    Ok(())
}).await?;
```

### WorkerHandle

When calling a worker with `WORKER::call()`, you receive a `WorkerHandle`:

```rust
let mut handle = WORKER::call("worker-name", None, None).await?;

// Get worker info
let run_id = handle.id();
let name = handle.worker_name();

// Receive messages from worker
while let Some(message) = handle.recv().await {
    println!("Worker sent: {}", message);
}

// Wait for completion
handle.await_result().await?;

// Or cancel if needed
handle.cancel().await?;
```

### Worker Statistics

The system automatically tracks statistics for each worker:

```rust
#[derive(Debug, Clone, Serialize)]
pub struct WorkerStats {
    pub runs: u64,              // Total executions
    pub errors: u64,            // Failed executions
    pub last_run_ms: Option<u64>, // Duration of last run
    pub total_runtime_ms: u64,  // Cumulative runtime
}

// Access statistics
let stats = WORKER::stats("send-email").await;
if let Some(stats) = stats {
    println!("Runs: {}, Errors: {}, Avg: {}ms",
        stats.runs,
        stats.errors,
        stats.total_runtime_ms / stats.runs
    );
}
```

---

## Worker Registration

### Directory Structure (Required)

**All workers MUST be placed in the `src/workers/` directory.** The framework uses compile-time auto-discovery to find and register workers.

```
your-app/
├── src/
│   ├── workers/           # ⚠️ Required directory
│   │   ├── email.rs       # Worker file
│   │   ├── cleanup.rs     # Worker file
│   │   └── reports.rs     # Worker file
│   ├── controllers/
│   ├── models/
│   └── main.rs
└── Cargo.toml
```

### Worker File Structure

Each worker file in `src/workers/` must have an `install()` function:

```rust
// src/workers/email.rs
use rustf::prelude::*;
use rustf::workers::WORKER;
use std::time::Duration;

/// Install email worker - this function is called automatically by the framework
pub async fn install() -> Result<()> {
    WORKER::register("send-email", |ctx| async move {
        let payload = ctx.payload()
            .ok_or_else(|| Error::validation("Email worker requires payload"))?;

        let to = payload["to"].as_str()
            .ok_or_else(|| Error::validation("Missing 'to' field"))?;

        ctx.info(&format!("Sending email to: {}", to));

        // Email sending logic...
        tokio::time::sleep(Duration::from_secs(1)).await;

        ctx.info("Email sent successfully");
        Ok(())
    }).await?;

    Ok(())
}
```

### Auto-Discovery Methods

RustF provides three ways to enable worker auto-discovery:

#### Method 1: `#[rustf::auto_discover]` (Recommended)

The simplest approach - automatically discovers all workers:

```rust
// src/main.rs
use rustf::prelude::*;

#[rustf::auto_discover]  // ← Discovers src/workers/ automatically
#[tokio::main]
async fn main() -> Result<()> {
    let app = RustF::with_args()?.auto_load();
    app.start().await
}
```

#### Method 2: `.auto_load()` (Explicit)

Manually enable auto-loading with full control:

```rust
// src/main.rs
use rustf::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let app = RustF::new()
        .with_workers()
        .auto_load();  // ← Loads workers from src/workers/

    app.run("127.0.0.1:3000").await
}
```

#### Method 3: `auto_workers!()` Macro (Advanced)

Direct macro usage for custom setups:

```rust
// src/main.rs
use rustf::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let app = RustF::new()
        .with_workers()
        .workers_from(auto_workers!())  // ← Scans src/workers/ at compile time
        .run("127.0.0.1:3000")
        .await?;

    Ok(())
}
```

### Multiple Workers Per File

You can register multiple workers in a single file:

```rust
// src/workers/notifications.rs
use rustf::prelude::*;
use rustf::workers::WORKER;

pub async fn install() -> Result<()> {
    // Email notification worker
    WORKER::register("notify-email", |ctx| async move {
        ctx.info("Sending email notification");
        // Implementation...
        Ok(())
    }).await?;

    // SMS notification worker
    WORKER::register("notify-sms", |ctx| async move {
        ctx.info("Sending SMS notification");
        // Implementation...
        Ok(())
    }).await?;

    // Push notification worker
    WORKER::register("notify-push", |ctx| async move {
        ctx.info("Sending push notification");
        // Implementation...
        Ok(())
    }).await?;

    Ok(())
}
```

### Nested Directory Organization

Workers can be organized in subdirectories:

```
src/workers/
├── notifications/
│   ├── email.rs
│   ├── sms.rs
│   └── push.rs
├── reports/
│   ├── daily.rs
│   └── monthly.rs
└── maintenance/
    ├── cleanup.rs
    └── backup.rs
```

The framework automatically discovers workers in subdirectories up to 3 levels deep.

### Skipping Files

Files are automatically skipped if they:
- Are named `mod.rs`
- End with `.inc.rs`
- Start with underscore (`_helper.rs`)

```rust
// src/workers/mod.rs - ✅ Skipped automatically
// src/workers/helpers.inc.rs - ✅ Skipped automatically
// src/workers/_internal.rs - ✅ Skipped automatically
// src/workers/email.rs - ❌ Discovered and loaded
```

### Worker Registration Best Practices

1. **Use `src/workers/` directory**: This is mandatory - workers outside this directory won't be discovered
2. **One `install()` per file**: Each worker file must export a public async `install()` function
3. **Descriptive file names**: Use clear names like `email.rs`, `cleanup.rs`, `reports.rs`
4. **Descriptive worker names**: Use kebab-case for worker names (`send-email`, `generate-report`)
5. **One responsibility per worker**: Each worker should do one thing well
6. **Validate payloads early**: Always validate input data at the start of the worker
7. **Handle errors properly**: Use `Result<()>` and return meaningful errors

```rust
// ✅ Good: Clear name, validated input, error handling
WORKER::register("send-welcome-email", |ctx| async move {
    let payload = ctx.payload()
        .ok_or_else(|| Error::validation("Payload required"))?;

    let email = payload["email"].as_str()
        .ok_or_else(|| Error::validation("Email address required"))?;

    // Validate email format
    if !email.contains('@') {
        return Err(Error::validation("Invalid email format"));
    }

    send_email(email).await?;
    Ok(())
}).await?;

// ❌ Bad: Generic name, no validation, panics
WORKER::register("worker1", |ctx| async move {
    let email = ctx.payload().unwrap()["email"].as_str().unwrap();
    send_email(email).await.unwrap();
    Ok(())
}).await?;
```

---

## Executing Workers

### Method 1: Call and Await (Most Common)

Execute a worker and wait for completion:

```rust
// Simple execution
WORKER::run("send-email", None).await?;

// With payload
let payload = json!({
    "to": "user@example.com",
    "subject": "Welcome!"
});
WORKER::run("send-email", Some(payload)).await?;
```

### Method 2: Call with Handle (Advanced Control)

Get a handle for message streaming and control:

```rust
let mut handle = WORKER::call("worker-name", None, Some(payload)).await?;

// Receive messages while worker runs
while let Some(message) = handle.recv().await {
    println!("Progress: {}", message);
}

// Wait for final result
handle.await_result().await?;
```

### Method 3: Fire and Forget

Start a worker without waiting:

```rust
tokio::spawn(async move {
    let _ = WORKER::run("cleanup", None).await;
});
```

### Timeout Support

Set a maximum execution time:

```rust
use std::time::Duration;

// Worker will be cancelled if it exceeds 5 seconds
let handle = WORKER::call(
    "slow-worker",
    Some(Duration::from_secs(5)),  // Timeout
    Some(payload)
).await?;

match handle.await_result().await {
    Ok(_) => println!("Completed successfully"),
    Err(e) => println!("Timeout or error: {}", e),
}
```

### Passing Payloads

Workers accept JSON payloads for input data:

```rust
// Calling code
let payload = json!({
    "user_id": 123,
    "action": "process_upload",
    "file_path": "/tmp/upload.pdf"
});
WORKER::run("process-file", Some(payload)).await?;

// Worker code
WORKER::register("process-file", |ctx| async move {
    let payload = ctx.payload().ok_or_else(||
        Error::validation("Payload required")
    )?;

    let user_id = payload["user_id"].as_i64().unwrap();
    let file_path = payload["file_path"].as_str().unwrap();

    ctx.info(&format!("Processing file for user {}", user_id));
    // Process file...

    Ok(())
}).await?;
```

### Concurrent Execution

Run multiple workers simultaneously:

```rust
use futures::future::join_all;

async fn process_batch(ctx: &mut Context) -> Result<()> {
    let items = vec![
        json!({"id": 1}),
        json!({"id": 2}),
        json!({"id": 3}),
    ];

    // Launch all workers concurrently
    let futures: Vec<_> = items.into_iter()
        .map(|item| WORKER::call("process-item", None, Some(item)))
        .collect();

    let handles = join_all(futures).await;

    // Wait for all to complete
    for handle in handles {
        handle?.await_result().await?;
    }

    ctx.json(json!({"status": "All items processed"}))
}
```

---

## Worker Context

The `WorkerContext` provides rich functionality for worker execution.

### Identity and Metadata

```rust
WORKER::register("example", |ctx| async move {
    // Unique identifier for this execution
    let run_id = ctx.run_id(); // "550e8400-e29b-41d4-a716-446655440000"

    // Worker definition name
    let name = ctx.worker_name(); // "example"

    ctx.info(&format!("Execution {} of {}", run_id, name));
    Ok(())
}).await?;
```

### Logging

Workers have structured logging with automatic identification:

```rust
WORKER::register("logger-demo", |ctx| async move {
    ctx.info("Informational message");
    ctx.warn("Warning message");
    ctx.error("Error message");
    ctx.debug("Debug message (only in development)");

    // Generic logging with custom level
    ctx.log(log::Level::Info, "Custom log");

    Ok(())
}).await?;

// Output:
// [Worker:logger-demo run:550e8400...] Informational message
// [Worker:logger-demo run:550e8400...] Warning message
```

### Environment Detection

```rust
WORKER::register("env-aware", |ctx| async move {
    if ctx.is_development() {
        ctx.info("Running in development - verbose logging enabled");
        // Use test APIs, skip external services, etc.
    }

    if ctx.is_production() {
        ctx.info("Running in production - using live services");
        // Use production APIs, send real emails, etc.
    }

    Ok(())
}).await?;
```

Environment is determined from `RUSTF_ENV` or `NODE_ENV` environment variables.

### Configuration Access

Access application configuration from workers:

```rust
WORKER::register("config-example", |ctx| async move {
    let config = ctx.config();

    // Use configuration values
    let base_url = config.get::<String>("api.base_url")
        .unwrap_or_else(|| "http://localhost".to_string());

    ctx.info(&format!("Using API: {}", base_url));
    Ok(())
}).await?;
```

### State Management

Each worker execution has isolated state:

```rust
WORKER::register("stateful", |ctx| async move {
    // Set state values
    ctx.set_state("progress", json!(0)).await?;
    ctx.set_state("items_processed", json!([])).await?;

    for i in 1..=5 {
        // Update state
        ctx.set_state("progress", json!(i * 20)).await?;

        // Read state
        if let Some(progress) = ctx.get_state("progress").await {
            ctx.info(&format!("Progress: {}%", progress));
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    // Clear all state
    ctx.clear_state().await?;
    Ok(())
}).await?;
```

**Note**: State is per-execution. Different runs of the same worker have independent state.

### Message Emission

Send real-time messages to the caller:

```rust
// Worker code
WORKER::register("progress-reporter", |ctx| async move {
    for i in 1..=10 {
        // Emit progress updates
        ctx.emit(json!({
            "progress": i * 10,
            "message": format!("Processing item {}/10", i)
        }))?;

        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    ctx.emit(json!({"status": "complete"}))?;
    Ok(())
}).await?;

// Calling code
let mut handle = WORKER::call("progress-reporter", None, None).await?;

// Receive progress updates
while let Some(message) = handle.recv().await {
    let progress = message["progress"].as_i64().unwrap_or(0);
    println!("Progress: {}%", progress);
}

handle.await_result().await?;
```

### Utilities

```rust
WORKER::register("utilities", |ctx| async move {
    // Sleep/delay
    ctx.sleep(Duration::from_secs(1)).await;

    // Access payload
    if let Some(payload) = ctx.payload() {
        ctx.info(&format!("Received: {}", payload));
    }

    Ok(())
}).await?;
```

---

## Advanced Features

### Cancellation

Cancel running workers using the handle:

```rust
async fn cancelable_operation(ctx: &mut Context) -> Result<()> {
    // Start long-running worker
    let handle = WORKER::call("slow-worker", None, None).await?;
    let run_id = handle.id().to_string();

    // Start cancellation timer
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(3)).await;

        // Cancel after 3 seconds
        if let Err(e) = WORKER::cancel(&run_id).await {
            eprintln!("Failed to cancel: {}", e);
        }
    });

    // Try to await result
    match handle.await_result().await {
        Ok(_) => ctx.json(json!({"status": "completed"})),
        Err(_) => ctx.json(json!({"status": "cancelled"})),
    }
}
```

### Listing Workers

Query registered and running workers:

```rust
async fn worker_dashboard(ctx: &mut Context) -> Result<()> {
    // List all registered worker definitions
    let definitions = WORKER::definitions().await?;

    // List all currently running worker run IDs
    let running = WORKER::running().await?;

    // List runs for a specific worker
    let email_runs = WORKER::running_for("send-email").await?;

    ctx.json(json!({
        "definitions": definitions,
        "running_count": running.len(),
        "email_worker_runs": email_runs.len()
    }))
}
```

### Statistics and Monitoring

Track worker performance:

```rust
async fn worker_stats(ctx: &mut Context) -> Result<()> {
    let definitions = WORKER::definitions().await?;
    let mut stats_map = serde_json::Map::new();

    for name in definitions {
        if let Some(stats) = WORKER::stats(&name).await {
            let avg_ms = if stats.runs > 0 {
                stats.total_runtime_ms / stats.runs
            } else {
                0
            };

            stats_map.insert(name, json!({
                "total_runs": stats.runs,
                "errors": stats.errors,
                "success_rate": if stats.runs > 0 {
                    (stats.runs - stats.errors) as f64 / stats.runs as f64 * 100.0
                } else {
                    0.0
                },
                "average_duration_ms": avg_ms,
                "last_run_ms": stats.last_run_ms
            }));
        }
    }

    ctx.json(json!(stats_map))
}
```

### Graceful Shutdown

Workers automatically shutdown when the application stops:

```rust
#[tokio::main]
async fn main() -> Result<()> {
    let app = RustF::new()
        .with_workers()
        .workers_from(|_manager| async move {
            WORKER::register("long-task", |ctx| async move {
                ctx.info("Long task started");
                tokio::time::sleep(Duration::from_secs(60)).await;
                ctx.info("Long task completed");
                Ok(())
            }).await?;
            Ok(())
        })
        .run("127.0.0.1:3000")
        .await?;

    // When app shuts down (Ctrl+C, etc.):
    // 1. All running workers are cancelled
    // 2. No new workers can be started
    // 3. Resources are cleaned up

    Ok(())
}
```

### Error Handling Patterns

```rust
WORKER::register("robust-worker", |ctx| async move {
    // Validate input early
    let payload = ctx.payload()
        .ok_or_else(|| Error::validation("Payload required"))?;

    let user_id = payload["user_id"].as_i64()
        .ok_or_else(|| Error::validation("user_id is required"))?;

    // Use Result propagation
    let user = fetch_user(user_id).await?;

    // Handle specific errors
    match process_user(&user).await {
        Ok(_) => {
            ctx.info("User processed successfully");
            Ok(())
        }
        Err(e) if e.is_validation() => {
            ctx.warn(&format!("Validation failed: {}", e));
            Err(e)
        }
        Err(e) => {
            ctx.error(&format!("Processing failed: {}", e));
            Err(e)
        }
    }
}).await?;
```

---

## Real-World Examples

### Example 1: Email Sending Worker

```rust
use rustf::prelude::*;
use rustf::workers::WORKER;

pub async fn install() -> Result<()> {
    WORKER::register("send-email", |ctx| async move {
        let payload = ctx.payload()
            .ok_or_else(|| Error::validation("Email payload required"))?;

        let to = payload["to"].as_str()
            .ok_or_else(|| Error::validation("'to' address required"))?;
        let subject = payload["subject"].as_str()
            .unwrap_or("No Subject");
        let body = payload["body"].as_str()
            .unwrap_or("");

        ctx.info(&format!("Sending email to: {}", to));

        // Simulate email sending
        if ctx.is_development() {
            ctx.info(&format!("📧 DEV MODE - Email to {} with subject: {}", to, subject));
            tokio::time::sleep(Duration::from_millis(100)).await;
        } else {
            // Production: use real SMTP service
            // send_smtp_email(to, subject, body).await?;
            tokio::time::sleep(Duration::from_secs(2)).await;
        }

        ctx.emit(json!({
            "status": "sent",
            "to": to,
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))?;

        ctx.info("✅ Email sent successfully");
        Ok(())
    }).await?;

    Ok(())
}

// Usage from controller
async fn register_user(ctx: &mut Context) -> Result<()> {
    let body = ctx.full_body();
    let email = body["email"].as_str().unwrap();

    // Create user...

    // Send welcome email in background
    WORKER::run("send-email", Some(json!({
        "to": email,
        "subject": "Welcome to Our App!",
        "body": "Thanks for registering..."
    }))).await?;

    ctx.json(json!({"status": "User registered"}))
}
```

### Example 2: File Processing Worker

```rust
WORKER::register("process-upload", |ctx| async move {
    let payload = ctx.payload()
        .ok_or_else(|| Error::validation("Payload required"))?;

    let file_path = payload["file_path"].as_str()
        .ok_or_else(|| Error::validation("file_path required"))?;

    ctx.info(&format!("Processing file: {}", file_path));

    // Step 1: Validate file
    ctx.emit(json!({"step": "validate", "progress": 25}))?;
    validate_file(file_path).await?;
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Step 2: Process file
    ctx.emit(json!({"step": "process", "progress": 50}))?;
    let result = process_file(file_path).await?;
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Step 3: Generate thumbnail
    ctx.emit(json!({"step": "thumbnail", "progress": 75}))?;
    generate_thumbnail(file_path).await?;
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Step 4: Save to database
    ctx.emit(json!({"step": "save", "progress": 100}))?;
    save_to_database(&result).await?;

    ctx.info("✅ File processing complete");
    Ok(())
}).await?;

// Usage with progress tracking
async fn upload_file(ctx: &mut Context) -> Result<()> {
    let file_path = "/tmp/uploads/file.pdf";

    let mut handle = WORKER::call(
        "process-upload",
        Some(Duration::from_secs(30)),
        Some(json!({"file_path": file_path}))
    ).await?;

    // Stream progress to client
    while let Some(progress) = handle.recv().await {
        log::info!("Progress: {:?}", progress);
        // Could send SSE to client here
    }

    handle.await_result().await?;
    ctx.json(json!({"status": "Processing complete"}))
}
```

### Example 3: Cleanup Worker

```rust
WORKER::register("cleanup-temp-files", |ctx| async move {
    ctx.info("🧹 Starting cleanup task");

    let temp_dir = "/tmp/app_uploads";
    let cutoff = chrono::Utc::now() - chrono::Duration::hours(24);

    let mut removed_count = 0;

    // Scan directory
    let entries = tokio::fs::read_dir(temp_dir).await?;
    // Process each file...

    ctx.emit(json!({
        "files_removed": removed_count,
        "directory": temp_dir
    }))?;

    ctx.info(&format!("✅ Cleanup complete: {} files removed", removed_count));
    Ok(())
}).await?;

// Schedule via cron or event system
app.on("ready", |ctx| Box::pin(async move {
    // Run cleanup every hour
    tokio::spawn(async {
        loop {
            tokio::time::sleep(Duration::from_secs(3600)).await;
            let _ = WORKER::run("cleanup-temp-files", None).await;
        }
    });
    Ok(())
}))
```

### Example 4: Report Generation Worker

```rust
WORKER::register("generate-report", |ctx| async move {
    let payload = ctx.payload()
        .ok_or_else(|| Error::validation("Report parameters required"))?;

    let report_type = payload["type"].as_str().unwrap_or("monthly");
    let user_id = payload["user_id"].as_i64().unwrap();

    ctx.info(&format!("Generating {} report for user {}", report_type, user_id));

    // Fetch data
    ctx.emit(json!({"stage": "fetching_data", "progress": 10}))?;
    let data = fetch_report_data(user_id, report_type).await?;

    // Process data
    ctx.emit(json!({"stage": "processing", "progress": 40}))?;
    let processed = process_report_data(&data).await?;

    // Generate PDF
    ctx.emit(json!({"stage": "generating_pdf", "progress": 70}))?;
    let pdf_path = generate_pdf_report(&processed).await?;

    // Upload to storage
    ctx.emit(json!({"stage": "uploading", "progress": 90}))?;
    let url = upload_to_storage(&pdf_path).await?;

    ctx.emit(json!({
        "stage": "complete",
        "progress": 100,
        "url": url
    }))?;

    ctx.info(&format!("✅ Report generated: {}", url));
    Ok(())
}).await?;
```

### Example 5: Batch Processing Worker

```rust
WORKER::register("batch-processor", |ctx| async move {
    let payload = ctx.payload()
        .ok_or_else(|| Error::validation("Batch items required"))?;

    let items = payload["items"].as_array()
        .ok_or_else(|| Error::validation("items must be an array"))?;

    let total = items.len();
    ctx.info(&format!("Processing {} items", total));

    for (index, item) in items.iter().enumerate() {
        let progress = ((index + 1) as f64 / total as f64 * 100.0) as u64;

        ctx.emit(json!({
            "current": index + 1,
            "total": total,
            "progress": progress,
            "item": item
        }))?;

        // Process item
        process_item(item).await?;

        // Small delay to avoid overwhelming external APIs
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    ctx.info(&format!("✅ Processed {} items", total));
    Ok(())
}).await?;
```

---

## Best Practices

### When to Use Workers

**✅ Good Use Cases:**

- **Long-running tasks**: Anything taking >100ms that blocks the response
- **External API calls**: Third-party services, webhooks, notifications
- **File operations**: Uploads, conversions, compression
- **Email/SMS sending**: Any messaging operations
- **Report generation**: PDFs, exports, analytics
- **Data synchronization**: Batch updates, imports
- **Cleanup tasks**: Maintenance, garbage collection

**❌ Not Suitable For:**

- **Quick database queries**: Just use models directly
- **Simple calculations**: Pure functions don't need workers
- **Request validation**: Should happen synchronously
- **Response formatting**: Part of controller logic
- **Configuration loading**: Do at startup, not in workers

### Error Handling

```rust
// ✅ Good: Specific error types, proper propagation
WORKER::register("good-errors", |ctx| async move {
    let payload = ctx.payload()
        .ok_or_else(|| Error::validation("Payload required"))?;

    match external_api_call().await {
        Ok(result) => {
            ctx.info("API call succeeded");
            Ok(())
        }
        Err(e) if e.is_timeout() => {
            ctx.warn("API timeout, will retry later");
            Err(Error::timeout("External API timeout"))
        }
        Err(e) => {
            ctx.error(&format!("Unexpected error: {}", e));
            Err(e)
        }
    }
}).await?;

// ❌ Bad: Swallowing errors, panics
WORKER::register("bad-errors", |ctx| async move {
    let data = ctx.payload().unwrap(); // Panic!
    external_api_call().await.ok(); // Error ignored!
    Ok(())
}).await?;
```

### Resource Management

```rust
// ✅ Good: Cleanup resources, bounded operations
WORKER::register("resource-safe", |ctx| async move {
    let temp_file = create_temp_file().await?;

    let result = process_file(&temp_file).await;

    // Always cleanup
    tokio::fs::remove_file(&temp_file).await?;

    result
}).await?;

// ❌ Bad: Resource leaks, unbounded operations
WORKER::register("resource-leak", |ctx| async move {
    let file = create_temp_file().await?;
    process_file(&file).await?;
    // File never cleaned up!

    // Unbounded loop - could run forever
    loop {
        process_next().await?;
    }
}).await?;
```

### Payload Validation

```rust
// ✅ Good: Validate early, clear errors
WORKER::register("validated", |ctx| async move {
    let payload = ctx.payload()
        .ok_or_else(|| Error::validation("Payload required"))?;

    // Validate all required fields upfront
    let email = payload["email"].as_str()
        .ok_or_else(|| Error::validation("email is required"))?;

    if !email.contains('@') {
        return Err(Error::validation("Invalid email format"));
    }

    let age = payload["age"].as_i64()
        .ok_or_else(|| Error::validation("age must be a number"))?;

    if age < 0 || age > 150 {
        return Err(Error::validation("Invalid age range"));
    }

    // Proceed with validated data
    process_user(email, age).await
}).await?;
```

### Logging and Observability

```rust
WORKER::register("observable", |ctx| async move {
    let start = std::time::Instant::now();

    ctx.info("Worker started");

    // Log major steps
    ctx.info("Step 1: Fetching data");
    let data = fetch_data().await?;

    ctx.info(&format!("Step 2: Processing {} records", data.len()));
    let result = process_data(&data).await?;

    ctx.info("Step 3: Saving results");
    save_results(&result).await?;

    let duration = start.elapsed();
    ctx.info(&format!("✅ Completed in {:.2}s", duration.as_secs_f64()));

    // Emit metrics
    ctx.emit(json!({
        "duration_ms": duration.as_millis(),
        "records_processed": data.len()
    }))?;

    Ok(())
}).await?;
```

### Testing Workers

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_email_worker() {
        // Setup
        let app = RustF::new().with_workers();

        // Register test worker
        WORKER::register("test-email", |ctx| async move {
            let payload = ctx.payload().unwrap();
            assert!(payload["to"].as_str().is_some());
            Ok(())
        }).await.unwrap();

        // Execute
        let result = WORKER::run("test-email", Some(json!({
            "to": "test@example.com"
        }))).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_worker_timeout() {
        WORKER::register("slow", |ctx| async move {
            tokio::time::sleep(Duration::from_secs(10)).await;
            Ok(())
        }).await.unwrap();

        let handle = WORKER::call(
            "slow",
            Some(Duration::from_secs(1)),
            None
        ).await.unwrap();

        let result = handle.await_result().await;
        assert!(result.is_err()); // Should timeout
    }
}
```

### Performance Considerations

1. **Use timeouts**: Always set reasonable timeouts for external operations
2. **Batch operations**: Process multiple items in one worker run when possible
3. **Avoid blocking**: Use async I/O, don't block the thread
4. **Limit concurrency**: Don't spawn thousands of workers simultaneously
5. **Monitor statistics**: Track execution times and error rates

```rust
// ✅ Good: Bounded concurrency
async fn process_many_items(items: Vec<Value>) -> Result<()> {
    use futures::stream::{self, StreamExt};

    stream::iter(items)
        .map(|item| async move {
            WORKER::run("process-item", Some(item)).await
        })
        .buffer_unordered(10) // Max 10 concurrent workers
        .collect::<Vec<_>>()
        .await;

    Ok(())
}

// ❌ Bad: Unbounded concurrency
async fn process_many_items_bad(items: Vec<Value>) -> Result<()> {
    for item in items {
        tokio::spawn(async move {
            WORKER::run("process-item", Some(item)).await
        });
    }
    Ok(())
}
```

---

## API Reference

### WORKER Global API

```rust
pub struct WORKER;

impl WORKER {
    /// Register a worker definition
    pub async fn register<F, Fut>(
        name: impl Into<String>,
        handler: F
    ) -> Result<()>
    where
        F: Fn(WorkerContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static;

    /// Call a worker and get a handle
    pub async fn call(
        worker_name: &str,
        timeout: Option<Duration>,
        payload: Option<Value>
    ) -> Result<WorkerHandle>;

    /// Run a worker and await completion
    pub async fn run(
        worker_name: &str,
        payload: Option<Value>
    ) -> Result<()>;

    /// Cancel a running worker
    pub async fn cancel(run_id: &str) -> Result<()>;

    /// List registered worker names
    pub async fn definitions() -> Result<Vec<String>>;

    /// List currently running worker IDs
    pub async fn running() -> Result<Vec<String>>;

    /// List running instances of a specific worker
    pub async fn running_for(worker_name: &str) -> Result<Vec<String>>;

    /// Get statistics for a worker
    pub async fn stats(worker_name: &str) -> Option<WorkerStats>;

    /// Shutdown all running workers
    pub async fn shutdown() -> Result<()>;
}
```

### WorkerContext

```rust
pub struct WorkerContext {
    // Identity
    pub fn worker_name(&self) -> &str;
    pub fn run_id(&self) -> &str;

    // Configuration
    pub fn config(&self) -> &Arc<AppConfig>;

    // Payload
    pub fn payload(&self) -> Option<&Value>;

    // Environment
    pub fn is_development(&self) -> bool;
    pub fn is_production(&self) -> bool;

    // Logging
    pub fn log(&self, level: log::Level, message: &str);
    pub fn info(&self, message: &str);
    pub fn warn(&self, message: &str);
    pub fn error(&self, message: &str);
    pub fn debug(&self, message: &str);

    // State
    pub async fn set_state(
        &self,
        key: impl Into<String>,
        value: Value
    ) -> Result<()>;
    pub async fn get_state(&self, key: &str) -> Option<Value>;
    pub async fn clear_state(&self) -> Result<()>;

    // Messaging
    pub fn emit(&self, message: Value) -> Result<()>;

    // Utilities
    pub async fn sleep(&self, duration: Duration);
}
```

### WorkerHandle

```rust
pub struct WorkerHandle {
    /// Get the unique run ID
    pub fn id(&self) -> &str;

    /// Get the worker name
    pub fn worker_name(&self) -> &str;

    /// Cancel this worker execution
    pub async fn cancel(&self) -> Result<()>;

    /// Receive next message from worker
    pub async fn recv(&mut self) -> Option<Value>;

    /// Wait for worker to complete
    pub async fn await_result(self) -> Result<()>;
}
```

### WorkerStats

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerStats {
    /// Total number of executions
    pub runs: u64,

    /// Number of failed executions
    pub errors: u64,

    /// Duration of last execution in milliseconds
    pub last_run_ms: Option<u64>,

    /// Total cumulative runtime in milliseconds
    pub total_runtime_ms: u64,
}
```

---

## Troubleshooting

### Worker Not Found

**Error**: `Worker 'my-worker' not registered`

**Solutions**:

1. **Ensure worker file is in `src/workers/` directory**:
```
src/
└── workers/
    └── my_worker.rs  ← Must be here!
```

2. **Check that the file has an `install()` function**:
```rust
// src/workers/my_worker.rs
pub async fn install() -> Result<()> {
    WORKER::register("my-worker", |ctx| async move {
        // Worker implementation
        Ok(())
    }).await?;
    Ok(())
}
```

3. **Verify auto-discovery is enabled**:
```rust
// src/main.rs
#[rustf::auto_discover]  // ← Required!
#[tokio::main]
async fn main() -> Result<()> {
    let app = RustF::new().auto_load();
    app.start().await
}
```

4. **Check that file isn't being skipped** (shouldn't be `mod.rs`, `_*.rs`, or `*.inc.rs`)

### Worker Manager Not Initialized

**Error**: `Worker manager not initialised`

**Solution**: Use `.auto_load()` or explicitly enable workers:

```rust
// Recommended: Use auto_load
let app = RustF::new()
    .auto_load()  // Automatically enables workers
    .run("127.0.0.1:3000")
    .await?;

// Or explicitly:
let app = RustF::new()
    .with_workers()
    .workers_from(auto_workers!())
    .run("127.0.0.1:3000")
    .await?;
```

### Worker Timeout

**Error**: `Worker 'slow-task' timed out after 5s`

**Solutions**:

1. Increase timeout:
```rust
WORKER::call("slow-task", Some(Duration::from_secs(30)), payload).await?;
```

2. Optimize worker to run faster:
```rust
// Break work into smaller chunks
// Use async I/O instead of blocking
// Process in parallel where possible
```

3. Remove timeout (not recommended for production):
```rust
WORKER::call("slow-task", None, payload).await?;
```

### Messages Not Received

**Problem**: `handle.recv()` returns `None` immediately.

**Solution**: Worker must emit messages:

```rust
// Worker must explicitly emit
WORKER::register("messenger", |ctx| async move {
    ctx.emit(json!({"progress": 50}))?; // Emit message
    Ok(())
}).await?;

// Caller receives messages
let mut handle = WORKER::call("messenger", None, None).await?;
while let Some(msg) = handle.recv().await {
    println!("{}", msg);
}
```

### Payload Access Issues

**Problem**: `ctx.payload()` returns `None`.

**Solution**: Pass payload when calling:

```rust
// Pass payload
WORKER::run("my-worker", Some(json!({"key": "value"}))).await?;

// Access in worker
WORKER::register("my-worker", |ctx| async move {
    if let Some(payload) = ctx.payload() {
        // Use payload
    }
    Ok(())
}).await?;
```

### Worker Hangs Forever

**Problem**: Worker never completes.

**Solutions**:

1. Add timeout:
```rust
WORKER::call("worker", Some(Duration::from_secs(30)), payload).await?;
```

2. Debug with logging:
```rust
WORKER::register("debug", |ctx| async move {
    ctx.info("Step 1");
    step1().await?;
    ctx.info("Step 2");
    step2().await?;
    ctx.info("Complete");
    Ok(())
}).await?;
```

3. Check for infinite loops or deadlocks in worker code.

### High Memory Usage

**Problem**: Memory grows when running many workers.

**Solutions**:

1. Limit concurrent workers:
```rust
use futures::stream::{self, StreamExt};

stream::iter(items)
    .map(|item| WORKER::run("process", Some(item)))
    .buffer_unordered(10) // Max 10 concurrent
    .collect::<Vec<_>>()
    .await;
```

2. Clean up resources in workers:
```rust
WORKER::register("cleanup", |ctx| async move {
    let resource = allocate().await?;
    let result = process(&resource).await;
    drop(resource); // Explicit cleanup
    result
}).await?;
```

3. Use streaming for large data:
```rust
// Process in chunks instead of loading everything
for chunk in data.chunks(1000) {
    process_chunk(chunk).await?;
}
```

### Statistics Not Updating

**Problem**: `WORKER::stats()` returns outdated values.

**Solution**: Statistics update after worker completion. Ensure:

1. Worker completes successfully or fails (not cancelled mid-execution)
2. Await worker completion before checking stats
3. Stats are per-definition, not per-run

```rust
// Execute worker
WORKER::run("task", None).await?;

// Now stats are updated
let stats = WORKER::stats("task").await;
```

---

## Summary

The RustF Worker system provides:

- **Simple Registration**: `WORKER::register()` with async handlers
- **Flexible Execution**: Call and await, fire-and-forget, or stream messages
- **Rich Context**: Logging, state, configuration, and messaging
- **Monitoring**: Built-in statistics and running worker tracking
- **Robust**: Timeout support, cancellation, and graceful shutdown

**Quick Reference**:

```rust
// Enable
app.with_workers()

// Register
WORKER::register("name", |ctx| async move { Ok(()) }).await?

// Execute
WORKER::run("name", Some(payload)).await?

// Monitor
WORKER::stats("name").await
WORKER::running().await?
```

For more examples, see the [rustf-example](https://github.com/your-repo/rustf/tree/main/rustf-example) project.

---

**Documentation Version**: 1.0
**Last Updated**: 2025-01-22
**RustF Version**: 0.1.0
