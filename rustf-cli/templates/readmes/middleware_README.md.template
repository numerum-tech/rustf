# Middleware Directory

This directory contains custom middleware components for cross-cutting concerns such as authentication, logging, CORS, rate limiting, and security headers.

## 🤖 AI Agent Quick Reference

**Purpose**: Process requests before handlers and/or responses after handlers  
**File Pattern**: `*.rs` files in this directory are auto-discovered  
**Key Function**: Each middleware file should export `pub fn install(registry: &mut MiddlewareRegistry)`

## RustF Middleware Model

RustF uses a **dual-phase** middleware system:

- **Inbound middleware** runs before the route handler
- **Outbound middleware** runs after the route handler
- **Dual middleware** implements both phases

Use these traits:

- `InboundMiddleware`
- `OutboundMiddleware`
- `InboundAction`
- `MiddlewareRegistry`

## Quick Start

```rust
use async_trait::async_trait;
use rustf::middleware::{InboundAction, InboundMiddleware, MiddlewareRegistry, OutboundMiddleware};
use rustf::prelude::*;

#[derive(Clone)]
pub struct ExampleMiddleware;

#[async_trait]
impl InboundMiddleware for ExampleMiddleware {
    async fn process_request(&self, ctx: &mut Context) -> Result<InboundAction> {
        let path = ctx.path();
        log::info!("Incoming request: {}", path);

        ctx.set("example_start", std::time::Instant::now())?;
        Ok(InboundAction::Capture)
    }

    fn name(&self) -> &'static str {
        "example"
    }

    fn priority(&self) -> i32 {
        0
    }
}

#[async_trait]
impl OutboundMiddleware for ExampleMiddleware {
    async fn process_response(&self, ctx: &mut Context) -> Result<()> {
        let Some(start) = ctx.get::<std::time::Instant>("example_start") else {
            return Ok(());
        };
        let elapsed = start.elapsed().as_millis().to_string();

        if let Some(response) = ctx.res.as_mut() {
            response.add_header("X-Response-Time", &elapsed);
        }

        Ok(())
    }
}

pub fn install(registry: &mut MiddlewareRegistry) {
    registry.register_dual("example", ExampleMiddleware);
}
```

## Common Patterns

### Authentication Gate

```rust
use async_trait::async_trait;
use rustf::middleware::{InboundAction, InboundMiddleware, MiddlewareRegistry};
use rustf::prelude::*;

#[derive(Clone)]
pub struct AuthMiddleware {
    protected_prefixes: Vec<String>,
}

impl AuthMiddleware {
    pub fn new(prefixes: Vec<&str>) -> Self {
        Self {
            protected_prefixes: prefixes.into_iter().map(str::to_string).collect(),
        }
    }
}

#[async_trait]
impl InboundMiddleware for AuthMiddleware {
    async fn process_request(&self, ctx: &mut Context) -> Result<InboundAction> {
        let path = ctx.path();
        let requires_auth = self
            .protected_prefixes
            .iter()
            .any(|prefix| path.starts_with(prefix));

        if requires_auth && ctx.require_auth().is_err() {
            ctx.redirect("/login")?;
            return Ok(InboundAction::Stop);
        }

        Ok(InboundAction::Continue)
    }

    fn name(&self) -> &'static str {
        "auth"
    }

    fn priority(&self) -> i32 {
        -50
    }
}

pub fn install(registry: &mut MiddlewareRegistry) {
    registry.register_inbound(
        "auth",
        AuthMiddleware::new(vec!["/admin", "/dashboard", "/api/private"]),
    );
}
```

### Request Logging

```rust
use async_trait::async_trait;
use rustf::middleware::{InboundAction, InboundMiddleware, MiddlewareRegistry, OutboundMiddleware};
use rustf::prelude::*;

#[derive(Clone)]
pub struct RequestLoggingMiddleware;

#[async_trait]
impl InboundMiddleware for RequestLoggingMiddleware {
    async fn process_request(&self, ctx: &mut Context) -> Result<InboundAction> {
        log::info!("→ {} {}", ctx.req.method, ctx.req.uri);
        Ok(InboundAction::Capture)
    }

    fn name(&self) -> &'static str {
        "request_logging"
    }
}

#[async_trait]
impl OutboundMiddleware for RequestLoggingMiddleware {
    async fn process_response(&self, ctx: &mut Context) -> Result<()> {
        if let Some(response) = ctx.res.as_ref() {
            log::info!("← {} {}", response.status.as_u16(), ctx.path());
        }
        Ok(())
    }
}

pub fn install(registry: &mut MiddlewareRegistry) {
    registry.register_dual("request_logging", RequestLoggingMiddleware);
}
```

## Registration Methods

```rust
registry.register_inbound("auth", middleware);
registry.register_outbound("security_headers", middleware);
registry.register_dual("logging", middleware);
```

## Built-in Middleware

RustF also ships built-in middleware you can enable through `main.rs`:

```rust
let app = RustF::new().auto_load().with_method_override();
```

And configure in `config.toml`:

- `[middleware.cors]`
- `[middleware.rate_limit]`
- `[middleware.csrf]`

Sessions are auto-registered from `[session]` configuration and do not need
manual middleware registration.

## Guidelines

1. Use inbound middleware for access control, request shaping, and preflight checks.
2. Use outbound middleware for headers, compression, cookies, and response metadata.
3. Use `InboundAction::Capture` when you need an outbound phase.
4. Use low priorities for middleware that must run early.
5. Do not log secrets, authorization headers, or raw bodies in production.
