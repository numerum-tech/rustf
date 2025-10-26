# RustF Middleware System Documentation

## Overview

RustF provides a comprehensive dual-phase middleware system that separates request processing from response modification. This architecture provides clear separation of concerns and predictable execution order, making it highly suitable for AI-assisted development.

## Core Architecture

### Dual-Phase Processing

The middleware system operates in two distinct phases:

1. **Inbound Phase** - Processes incoming requests before they reach controllers
2. **Outbound Phase** - Modifies responses after controllers have executed

This separation allows middleware to cleanly handle both request validation/transformation and response enhancement without complex state management.

## Core Components

### InboundMiddleware Trait

Processes requests before they reach controllers:

```rust
use async_trait::async_trait;

#[async_trait]
pub trait InboundMiddleware: Send + Sync + 'static {
    async fn process_request(&self, ctx: &mut Context) -> Result<InboundAction>;
    
    fn name(&self) -> &'static str { "unnamed" }
    fn priority(&self) -> i32 { 0 }
    fn should_run(&self, ctx: &Context) -> bool { true }
}
```

**Key Methods:**
- `process_request()` - Main processing logic for incoming requests
- `name()` - Identifies the middleware for debugging/logging
- `priority()` - Execution order (lower numbers run first, -1000 to 1000)
- `should_run()` - Conditional execution based on request context

### InboundAction Enum

Inbound middleware returns one of three actions:

```rust
pub enum InboundAction {
    Continue,           // Continue to next middleware
    Stop,               // Stop chain and return response set on context
    Capture,           // Continue but process response later
}
```

- `Continue` - Pass request to next middleware without capturing response
- `Stop` - Stop processing and return the response that was set on context using response helpers
- `Capture` - Continue processing but ensure outbound phase runs

### OutboundMiddleware Trait

Modifies responses after controllers execute:

```rust
use async_trait::async_trait;

#[async_trait]
pub trait OutboundMiddleware: Send + Sync + 'static {
    async fn process_response(&self, ctx: &mut Context) -> Result<()>;
}
```

**Important Change:** The outbound middleware now receives `&mut Context` instead of separate context and response parameters. The response is accessed and modified through `ctx.response` field.

### DualPhaseMiddleware Trait

For middleware that needs both phases:

```rust
pub trait DualPhaseMiddleware: InboundMiddleware + OutboundMiddleware {}
```

Any type implementing both traits automatically implements `DualPhaseMiddleware`.

## ⚠️ Critical: Async Middleware Requirements

**IMPORTANT**: All middleware in RustF MUST be async to prevent application hangs:

1. **Always use `#[async_trait]`** - Required for all middleware trait implementations
2. **Never use `block_on`** - Using `futures::executor::block_on` will cause the application to hang, especially with database storage
3. **All I/O operations must be async** - Session operations, database queries, and network calls must use async/await

### Why This Matters
The middleware system was redesigned to be fully async to fix a critical issue where applications would hang when using database-backed session storage. The previous implementation used `block_on` which blocked the async runtime, causing deadlocks with connection pools and async I/O operations.

## Writing Middleware

### Simple Inbound Middleware

```rust
use rustf::middleware::{InboundMiddleware, InboundAction};
use rustf::context::Context;
use rustf::error::Result;
use async_trait::async_trait;

pub struct AuthMiddleware {
    required_role: String,
}

#[async_trait]
impl InboundMiddleware for AuthMiddleware {
    async fn process_request(&self, ctx: &mut Context) -> Result<InboundAction> {
        // Check authentication
        if let Some(user) = ctx.session_get::<User>("user") {
            if user.role == self.required_role {
                return Ok(InboundAction::Continue);
            }
        }
        
        // Not authorized - use context helpers to set response
        ctx.throw403(Some("Unauthorized"))?;
        Ok(InboundAction::Stop)
    }
    
    fn name(&self) -> &'static str {
        "auth"
    }
    
    fn priority(&self) -> i32 {
        -500  // Run early (lower numbers execute first)
    }
}
```

### Simple Outbound Middleware

```rust
use rustf::middleware::OutboundMiddleware;
use rustf::context::Context;
use rustf::error::Result;
use async_trait::async_trait;

pub struct CompressionMiddleware;

#[async_trait]
impl OutboundMiddleware for CompressionMiddleware {
    async fn process_response(&self, ctx: &mut Context) -> Result<()> {
        // Access the response through ctx.response
        if let Some(response) = ctx.response.as_mut() {
            // Add compression headers
            response.headers.push((
                "Content-Encoding".to_string(),
                "gzip".to_string()
            ));
            
            // Compress body (simplified)
            // response.body = compress(response.body);
        }
        
        Ok(())
    }
}
```

### Dual-Phase Middleware

```rust
use rustf::middleware::{InboundMiddleware, OutboundMiddleware, InboundAction};
use async_trait::async_trait;
use std::time::Instant;

pub struct TimingMiddleware;

#[async_trait]
impl InboundMiddleware for TimingMiddleware {
    async fn process_request(&self, ctx: &mut Context) -> Result<InboundAction> {
        // Store start time
        ctx.set("request_start", Instant::now());
        
        // Capture response to add timing header
        Ok(InboundAction::Capture)
    }
}

#[async_trait]
impl OutboundMiddleware for TimingMiddleware {
    async fn process_response(&self, ctx: &mut Context) -> Result<()> {
        if let Some(start) = ctx.get::<Instant>("request_start") {
            let duration = start.elapsed();
            
            // Access response through ctx.response
            if let Some(response) = ctx.response.as_mut() {
                response.headers.push((
                    "X-Response-Time".to_string(),
                    format!("{}ms", duration.as_millis())
                ));
            }
        }
        Ok(())
    }
}
```

## Registration

### Basic Registration

There are three ways to register middleware in RustF:

#### Method 1: Auto-Discovery (Recommended for custom middleware)

Place your middleware in `src/middleware/*.rs` with an `install` function:

```rust
// src/middleware/auth.rs
pub fn install(registry: &mut MiddlewareRegistry) {
    registry.register_inbound("auth", AuthMiddleware::new());
}

// In main.rs
let app = RustF::new()
    .middleware_from(auto_middleware!()); // Auto-discovers all middleware
```

#### Method 2: Manual Registration with middleware_from

```rust
let app = RustF::new()
    .middleware_from(|registry| {
        // Register inbound middleware
        registry.register_inbound("auth", AuthMiddleware::new());
        
        // Register outbound middleware  
        registry.register_outbound("compression", CompressionMiddleware);
        
        // Register dual-phase middleware
        registry.register_dual("timing", TimingMiddleware);
    });
```

#### Method 3: Direct Registration (Advanced - requires mutable app)

```rust
// Register inbound middleware
app.middleware.register_inbound("auth", AuthMiddleware::new());

// Register outbound middleware  
app.middleware.register_outbound("compression", CompressionMiddleware);

// Register dual-phase middleware
app.middleware.register_dual("timing", TimingMiddleware);
```

### Setting Middleware Priority

Priority is set by implementing the `priority()` method in your middleware trait implementation. Lower numbers execute first (range: -1000 to 1000).

```rust
use async_trait::async_trait;

#[async_trait]
impl InboundMiddleware for AuthMiddleware {
    async fn process_request(&self, ctx: &mut Context) -> Result<InboundAction> {
        // Your middleware logic here
        Ok(InboundAction::Continue)
    }
    
    fn priority(&self) -> i32 {
        -500  // Runs early in the chain
    }
}

// Example: Multiple middleware with different priorities
#[async_trait]
impl InboundMiddleware for LoggingMiddleware {
    async fn process_request(&self, ctx: &mut Context) -> Result<InboundAction> {
        // Log request
        Ok(InboundAction::Continue)
    }
    
    fn priority(&self) -> i32 {
        -1000  // Highest priority, runs first
    }
}

#[async_trait]
impl InboundMiddleware for RateLimitMiddleware {
    async fn process_request(&self, ctx: &mut Context) -> Result<InboundAction> {
        // Check rate limits
        Ok(InboundAction::Continue)
    }
    
    fn priority(&self) -> i32 {
        -900  // Runs after logging but before auth
    }
}
```

### Default Security Middleware

RustF provides built-in security middleware that can be enabled with one call:

```rust
let app = RustF::new()
    .with_default_security();  // Enables security headers, input validation, CSP
```

Or configure individually using middleware_from:

```rust
let app = RustF::new()
    .middleware_from(|registry| {
        registry.register_outbound("security_headers", SecurityHeadersMiddleware::new());
        registry.register_inbound("input_validation", InputValidationMiddleware::new());
        registry.register_dual("csp", CspMiddleware::permissive());
        registry.register_inbound("rate_limit", RateLimitMiddleware::new());
    });
```

## Built-in Middleware

**Note**: All built-in middleware properly implement async traits without any blocking operations. They are safe to use with database-backed session storage and other async I/O operations.

### Security Middleware

1. **SecurityHeadersMiddleware** (Outbound)
   - Adds comprehensive security headers (X-Frame-Options, X-Content-Type-Options, etc.)
   - Configurable for development/production environments

2. **InputValidationMiddleware** (Inbound)
   - Validates input for SQL injection, XSS, path traversal
   - Configurable patterns and exclusions

3. **CspMiddleware** (Dual-Phase)
   - Content Security Policy with nonce generation
   - Inbound: generates nonces for inline scripts/styles
   - Outbound: adds CSP headers with nonces

4. **RateLimitMiddleware** (Inbound)
   - IP-based rate limiting with configurable windows
   - DashMap-based for thread-safe operation
   - Automatic cleanup of old entries

### Utility Middleware

1. **LoggingMiddleware** (Dual-Phase)
   - Logs requests with timing information
   - Inbound: logs incoming request
   - Outbound: logs response with duration

2. **CorsMiddleware** (Dual-Phase)
   - Handles CORS preflight and headers
   - Inbound: responds to OPTIONS requests
   - Outbound: adds CORS headers to responses

3. **SessionMiddleware** (Dual-Phase)
   - Session management with cookie handling
   - Inbound: loads session from cookie (async, no blocking)
   - Outbound: saves session to cookie (async, no blocking)
   - **Fixed**: Previously used `block_on` causing hangs with database storage - now fully async

4. **ValidationMiddleware** (Inbound)
   - Form validation with configurable rules
   - CSRF protection integration
   - Automatic error response generation

## Execution Order

### Priority System

Middleware executes based on priority (lower numbers first):

```
-1000: Logging (capture everything)
 -900: Rate limiting (block early)
 -800: Security headers
 -700: CSP 
 -600: Input validation
 -500: CORS
 -400: Authentication
 -300: Session loading
    0: Default priority
 +100: Business logic
 +500: Caching
+1000: Final cleanup
```

### Phase Execution

1. **Inbound Phase** (before controller):
   - Middleware sorted by priority (ascending)
   - Each middleware's `should_run()` checked
   - `process_request()` called sequentially
   - Chain stops if any returns `Stop` (using response set on context)

2. **Controller Execution**:
   - Only if all inbound middleware returned `Continue` or `Capture`

3. **Outbound Phase** (after controller):
   - Only for middleware that returned `Capture` or registered as outbound
   - Executes in reverse order of inbound
   - Each modifies response in place

## Accessing Response in Outbound Phase

With the new architecture, outbound middleware accesses the response through the Context's `response` field:

### Response Access Pattern

```rust
impl OutboundMiddleware for MyMiddleware {
    fn process_response(&self, ctx: &mut Context) -> Result<()> {
        // The response might be None if an error occurred
        if let Some(response) = ctx.response.as_mut() {
            // Modify response headers
            response.headers.push(("X-Custom".to_string(), "value".to_string()));
            
            // Check status
            if response.status.is_server_error() {
                // Handle error responses differently
                response.headers.push(("X-Error".to_string(), "true".to_string()));
            }
            
            // Modify body if needed
            if response.status == StatusCode::OK {
                // Can inspect or modify response.body
            }
        }
        Ok(())
    }
}
```

### When Response Might Be None

The response field will be `None` in rare cases:
- If a panic occurred before the response was set
- If middleware or handler failed to set any response

In practice, the framework ensures a response is always set, even for errors.

### Important Notes

1. **InboundAction::Stop** - Uses the response set on context via helpers like `ctx.json()`, `ctx.throw403()`, etc.
2. **Handler Responses** - Controllers set response via `ctx.json()`, `ctx.view()`, etc.
3. **Error Responses** - Error methods like `ctx.throw404()` also set the response
4. **Middleware Order** - Outbound middleware runs in reverse order of inbound
5. **Context Initialization** - Context now initializes with a default 200 OK response, enabling response helpers everywhere

## Advanced Patterns

### Conditional Middleware

```rust
use async_trait::async_trait;

#[async_trait]
impl InboundMiddleware for ApiAuthMiddleware {
    fn should_run(&self, ctx: &Context) -> bool {
        // Only run for API routes
        ctx.request.uri.starts_with("/api/")
    }
    
    async fn process_request(&self, ctx: &mut Context) -> Result<InboundAction> {
        // API authentication logic
        Ok(InboundAction::Continue)
    }
}
```

### State Sharing Between Phases

```rust
use async_trait::async_trait;

#[async_trait]
impl InboundMiddleware for MetricsMiddleware {
    async fn process_request(&self, ctx: &mut Context) -> Result<InboundAction> {
        // Store request data for metrics
        ctx.set("metrics_start", Instant::now());
        ctx.set("metrics_path", ctx.request.uri.clone());
        
        Ok(InboundAction::Capture)
    }
}

#[async_trait]
impl OutboundMiddleware for MetricsMiddleware {
    async fn process_response(&self, ctx: &mut Context) -> Result<()> {
        if let Some(start) = ctx.get::<Instant>("metrics_start") {
            let path = ctx.get::<String>("metrics_path").unwrap_or_default();
            let duration = start.elapsed();
            
            // Get status from response
            let status = ctx.response.as_ref()
                .map(|r| r.status.as_u16())
                .unwrap_or(500);
            
            // Record metrics
            record_metric(&path, status, duration);
        }
        Ok(())
    }
}
```

### Early Response Pattern

```rust
use async_trait::async_trait;

#[async_trait]
impl InboundMiddleware for CacheMiddleware {
    async fn process_request(&self, ctx: &mut Context) -> Result<InboundAction> {
        let cache_key = generate_cache_key(&ctx.request);
        
        if let Some(cached) = self.cache.get(&cache_key) {
            // Return cached response immediately
            ctx.set_response(cached);
            return Ok(InboundAction::Stop);
        }
        
        // Store key for outbound phase
        ctx.set("cache_key", cache_key);
        Ok(InboundAction::Capture)
    }
}
```

## Testing Middleware

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rustf::http::Request;
    use rustf::views::ViewEngine;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_auth_middleware() {
        let middleware = AuthMiddleware::new("admin");
        
        // Create test context
        let request = Request::default();
        let views = Arc::new(ViewEngine::from_directory("views"));
        let mut ctx = Context::new(request, views);
        
        // Test without auth
        let action = middleware.process_request(&mut ctx).await.unwrap();
        assert!(matches!(action, InboundAction::Stop));
        // Check that response was set on context
        assert!(ctx.response.is_some());
        
        // Test with auth
        ctx.session_set("user", User { role: "admin".into() });
        let action = middleware.process_request(&mut ctx).await.unwrap();
        assert!(matches!(action, InboundAction::Continue));
    }
    
    #[tokio::test]
    async fn test_outbound_middleware() {
        let middleware = CompressionMiddleware;
        
        // Create test context with response
        let request = Request::default();
        let views = Arc::new(ViewEngine::from_directory("views"));
        let mut ctx = Context::new(request, views);
        
        // Process response
        middleware.process_response(&mut ctx).await.unwrap();
        
        // Verify headers were added
        if let Some(response) = &ctx.response {
            let has_encoding = response.headers.iter()
                .any(|(k, _)| k == "Content-Encoding");
            assert!(has_encoding);
        }
    }
}
```

## Best Practices

1. **Use appropriate phase**: 
   - Input validation → Inbound
   - Response headers → Outbound
   - Timing/metrics → Dual-phase

2. **Set meaningful priorities**:
   - Security checks: -900 to -500
   - Business logic: -100 to +100
   - Response modification: +500 to +900

3. **Minimize state in middleware**:
   - Use context for request-scoped data
   - Use Arc for shared immutable data

4. **Handle errors gracefully**:
   - Return appropriate HTTP status codes
   - Log errors for debugging

5. **Keep middleware focused**:
   - Single responsibility principle
   - Compose multiple middleware for complex logic

## Migration from Express/Total.js

| Express/Total.js | RustF Equivalent |
|-----------------|------------------|
| `app.use(middleware)` | `registry.register_inbound()` or `registry.register_dual()` |
| `next()` | `Ok(InboundAction::Continue)` |
| `res.send()` in middleware | Set response on context, return `Ok(InboundAction::Stop)` |
| Error middleware | Implement error handling in middleware |
| Route-specific middleware | Use `should_run()` method |
| Middleware priority/order | Implement `priority()` method (lower = earlier) |
| Synchronous middleware | **All middleware MUST be async in RustF** |

## Migration from Old RustF Blocking Middleware

If you have existing RustF middleware using the old blocking pattern:

### Old Pattern (DEPRECATED - Will cause hangs)
```rust
// DON'T DO THIS - Will cause application hangs
impl InboundMiddleware for MyMiddleware {
    fn process_request(&self, ctx: &mut Context) -> Result<InboundAction> {
        // This would use block_on internally for async operations
        let session = futures::executor::block_on(load_session()); // WRONG!
        Ok(InboundAction::Continue)
    }
}
```

### New Async Pattern (REQUIRED)
```rust
use async_trait::async_trait;

#[async_trait]
impl InboundMiddleware for MyMiddleware {
    async fn process_request(&self, ctx: &mut Context) -> Result<InboundAction> {
        // Properly await async operations
        let session = load_session().await?; // CORRECT!
        Ok(InboundAction::Continue)
    }
}
```

### Key Changes
1. Add `#[async_trait]` to all middleware implementations
2. Change `fn` to `async fn` for process methods
3. Replace any `block_on` calls with `.await`
4. Ensure all I/O operations use async versions

## AI Development Guidelines

When developing middleware with AI assistance:

1. **Always use async traits**: 
   - Include `use async_trait::async_trait;` at the top
   - Add `#[async_trait]` before every middleware impl block
   - Use `async fn` for `process_request()` and `process_response()`
   
2. **Never use blocking operations**:
   - **CRITICAL**: Never use `futures::executor::block_on` - it will hang the application
   - All I/O operations must be async (database, file system, network)
   - Session operations are async - use `.await` properly
   
3. **Specify the phase**: Clearly indicate if middleware should be inbound, outbound, or dual-phase

4. **Define the action**: Specify when to Continue, Stop, or Capture

5. **Set priorities**: Indicate relative execution order needs (-1000 to 1000)

6. **Handle errors**: Specify error response formats

7. **Use built-in middleware**: Leverage existing security middleware when applicable

8. **Test thoroughly**: 
   - Use `#[tokio::test]` for async tests
   - Test both phases if dual-phase
   - Verify `.await` is used for all async operations

### Example AI Prompt
```
Create an authentication middleware that:
- Runs in the inbound phase only
- Checks for JWT token in Authorization header
- Uses async database queries to validate user
- Returns 401 if unauthorized
- Has priority -400 (after rate limiting)
- MUST use async/await pattern
```

## Conclusion

The RustF dual-phase middleware system provides a clean, predictable architecture for request/response processing. The separation of inbound and outbound phases eliminates complex state management while maintaining the flexibility needed for sophisticated middleware implementations.