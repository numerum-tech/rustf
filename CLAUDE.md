# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

RustF is a convention-based MVC web framework for Rust, inspired by Total.js v4. It's designed to be equally intuitive for human developers and AI coding assistants, with auto-discovery, predictable patterns, and comprehensive security features.

This is a **Rust workspace** with four primary crates:
- `rustf/` - Core MVC framework library
- `rustf-cli/` - CLI tool for project analysis and code generation
- `rustf-macros/` - Procedural macros for auto-discovery
- `rustf-schema/` - YAML-based schema and code generation

## Common Commands

### Building & Testing
```bash
# Build all crates in workspace
cargo build

# Build with all features enabled
cargo build --all-features

# Build specific crate
cargo build -p rustf
cargo build -p rustf-cli

# Run all tests
cargo test

# Run tests with logging output
RUST_LOG=debug cargo test -- --nocapture

# Run benchmarks
cargo bench

# Run specific benchmark
cargo bench --bench routing
```

### Code Quality
```bash
# Run clippy (enforced in CI)
cargo clippy

# Format code
cargo fmt

# Check formatting without modifying
cargo fmt -- --check
```

### Working with Examples
```bash
# Run the example application
cd rustf-example
cargo run

# Access at http://127.0.0.1:8000
```

## Architecture Overview

### Memory Safety Pattern
The framework extensively uses `Arc` (Atomic Reference Counting) to achieve memory safety without unsafe code:
- All major components are wrapped in `Arc`: `Arc<ViewEngine>`, `Arc<ModelRegistry>`, `Arc<AppConfig>`
- All shared state is `Send + Sync` for safe concurrent access
- No unsafe code in core framework

### Context-Centric Design
The `Context` struct (rustf/src/context.rs) is the heart of request handling, modeled after Total.js:
- Contains request/response, session, and view engine references
- Provides two data storage mechanisms:
  - `repository: HashMap<String, Value>` - Accessible in views (use for template data)
  - `data: HashMap<String, Box<dyn Any>>` - Only for middleware (use for middleware state)
- Methods like `ctx.redirect()`, `ctx.session_set()`, `ctx.flash_success()` follow Total.js patterns

### Dual-Phase Middleware System
Located in `rustf/src/middleware/`:
- **InboundMiddleware** - Processes requests before handlers, can short-circuit with `InboundAction::Stop`
- **OutboundMiddleware** - Processes responses after handlers
- This two-phase design solves Rust's lifetime challenges while maintaining clean separation

### Auto-Discovery System
Procedural macros in `rustf-macros/src/` provide compile-time auto-discovery:
- `auto_controllers!()` - Scans `src/controllers/*.rs` at build time
- `auto_models!()` - Scans `src/models/*.rs`
- `auto_middleware!()` - Scans `src/middleware/*.rs`
- Generates module declarations and IDE support files (`_controllers.rs`, `_models.rs`)
- **Zero runtime overhead** - all resolution happens at compile time

### High-Performance Routing
Located in `rustf/src/routing/`:
- Trie-based router with radix tree structure (O(log n) complexity)
- Efficiently handles thousands of routes
- Zero-copy parameter extraction
- Routes defined using `routes![]` macro

### Database Architecture
Multi-database support in `rustf/src/database/`:
- Unified adapter pattern for MySQL, PostgreSQL, SQLite
- SQLx integration with compile-time SQL validation
- Global `DB` singleton for convenient access
- Type conversion system in `database/types/converters/`
- Advanced query builder with dialect-specific features in `models/query_builder_modules/dialects/`

## Key Conventions

### File Naming & Structure
- Controllers: `src/controllers/snake_case.rs`
- Models: `src/models/snake_case.rs`
- Middleware: `src/middleware/snake_case.rs`
- Templates: `views/feature_name/snake_case.html`
- Schemas: `schemas/snake_case.yaml`

### Controller Pattern
Every controller must export an `install()` function returning `Vec<Route>`:

```rust
use rustf::prelude::*;

pub fn install() -> Vec<Route> {
    routes![
        GET "/" => index,
        POST "/" => create,
        GET "/:id" => show,
        PUT "/:id" => update,
        DELETE "/:id" => delete,
    ]
}

async fn index(ctx: Context) -> rustf::Result<Response> {
    // Handler implementation
    Ok(Response::json(json!({"status": "ok"}))?)
}
```

### Model Pattern
Models should export a `register()` function:

```rust
pub async fn register(registry: &mut ModelRegistry) {
    // Register model with registry
}
```

### Middleware Pattern
Implement either `InboundMiddleware` or `OutboundMiddleware`:

```rust
pub struct MyMiddleware;

impl InboundMiddleware for MyMiddleware {
    fn process_request(&self, ctx: &mut Context) -> Result<InboundAction> {
        // Process request
        Ok(InboundAction::Continue)
    }
}
```

### Error Handling
- Use the custom `rustf::Result<T>` type alias (defined in `rustf/src/error.rs`)
- The framework enforces `#![warn(clippy::...)]` lints in non-test code
- Return errors using `?` operator for clean propagation

## Global Singletons

The framework provides several global singletons via `once_cell`:
- `CONF` - Global configuration (initialized from `config.toml`)
- `DB` - Database registry
- `APP` - Application instance
- `MAIN` - Main repository
- `VIEW` - View engine
- `WORKER` - Worker manager

Access via `CONF.get()`, `DB.get()`, etc.

## Configuration System

Configuration files: `config.toml`, `config.prod.toml`

Key sections:
```toml
[server]
host = "127.0.0.1"
port = 8000

[views]
engine = "total-js"
storage = "filesystem"  # or "embedded"
directory = "views"
default_layout = "layouts/default"

[static]
prefix = "/public"
directory = "public"

[session]
storage = "memory"  # or "redis", "database"
timeout = 3600

[database]
# Multi-database configuration
```

## Security Features

Built into `rustf/src/security/`:
- **Path Traversal Protection** (`static_files.rs`) - Canonicalization, depth limiting
- **XSS Prevention** (`validation.rs`) - Context-aware escaping for HTML/JS/CSS
- **CSRF Protection** (`csrf.rs`) - Token generation and validation
- **Security Headers** (`headers.rs`) - CSP, HSTS, X-Frame-Options, etc.
- **Rate Limiting** - Sliding window algorithm
- **Input Validation** - Comprehensive sanitization framework

## Template System

Located in `rustf/src/views/`:
- Default engine is Total.js template engine (always available)
- Two storage modes:
  - **Filesystem** - Templates in `views/` directory (default)
  - **Embedded** - Templates compiled into binary (feature: `embedded-views`)
- Template organization:
  - Layouts: `views/layouts/default.html`
  - Feature views: `views/auth/login.html`
  - Partials: `views/shared/header.html`

## CLI Tool (rustf-cli)

Located in `rustf-cli/src/`:
- **AST-based code analysis** using `syn` crate
- **MCP server** for AI agent integration (JSON-RPC 2.0)
- **Real-time file watching** with impact analysis
- **Route analysis** with conflict detection
- **Database introspection** and schema management
- **Code generation** with Handlebars templates

### Configuration Loading (Project-Centric)
The CLI tool is **project-folder-centric**, not environment-variable-centric:
- Always operates on `-P <path>` or current directory
- Automatically loads `config.toml` from project folder
- **Automatically merges `config.dev.toml`** if present (development overlay)
- No need to set `RUSTF_ENV` - each project has its own configs
- Supports multiple projects on same host without environment variable conflicts
- `DATABASE_URL` environment variable still overrides if needed

This ensures CLI database operations always use development settings, preventing accidental production database access.

Key commands:
```bash
rustf-cli analyze         # Analyze project structure
rustf-cli db introspect   # Database schema analysis
rustf-cli new controller  # Generate new controller
rustf-cli new middleware  # Generate new middleware (dual-phase pattern)
rustf-cli new worker      # Generate new worker (generic async task)
rustf-cli new module      # Generate new module/service
rustf-cli new event       # Generate new event handler
rustf-cli schema validate # Validate YAML schemas
rustf-cli watch          # Real-time project monitoring

# CLI automatically loads config.dev.toml for database operations
rustf-cli db list-tables  # Uses development database from config.dev.toml
```

## Schema-Driven Development (rustf-schema)

Located in `rustf-schema/src/`:
- YAML-based schema definitions
- Schema validation with relationship checking
- Code generation using Handlebars templates
- Generates SQLx models with proper type mappings

Schema location: `schemas/*.yaml`

## Feature Flags

Main `rustf` crate features:
```toml
default = ["config", "embedded-views", "auto-discovery", "schema", "decimal", "uuid"]
```

- `config` - TOML configuration support
- `embedded-views` - Compile templates into binary
- `auto-discovery` - Procedural macro support
- `schema` - Schema validation and codegen
- `decimal` - Rust Decimal type support
- `uuid` - UUID type support

**Note**: Redis session storage is now a built-in feature (no longer behind a feature flag). It's always available regardless of the features enabled.

## Documentation

Comprehensive docs in `docs/`:
- `ABOUT_*.md` - Core concept explanations
- `*_GUIDE.md` - Implementation guides
- `QUERY_BUILDER.md` - Query builder reference
- `CSRF_GUIDE.md` - CSRF protection details
- `PAGINATION_HELPER.md` - Pagination implementation

## Important Implementation Notes

### When Adding New Controllers
1. Create file in `src/controllers/snake_case.rs`
2. Implement `install()` function returning `Vec<Route>`
3. Auto-discovery will pick it up automatically (no manual registration needed)
4. Use `rustf::prelude::*` for common imports

### When Adding New Middleware
1. Create file in `src/middleware/snake_case.rs`
2. Implement `InboundMiddleware` and/or `OutboundMiddleware` trait (dual-phase pattern recommended)
3. Must use `#[async_trait]` macro on trait implementations
4. For dual-phase middleware, add `#[derive(Clone)]` to struct
5. Return `InboundAction::Capture` in inbound phase if you need outbound processing
6. Auto-discovery handles registration
7. Consider priority order for execution sequence

**Key Points**:
- Middleware template generates dual-phase pattern by default (more educational)
- Access request via `ctx.req` (not `ctx.request`)
- Access response via `ctx.res` (not `ctx.response`)
- Use `ctx.set()` and `ctx.get()` for data storage between phases
- Registration uses `register_dual()`, `register_inbound()`, or `register_outbound()`

### When Adding New Workers
1. Create file in `src/workers/snake_case.rs`
2. Implement `pub async fn install() -> Result<()>` function
3. Use `WORKER::register("kebab-case-name", |ctx| async move { ... })` inside install
4. Workers are auto-discovered from `src/workers/` directory
5. Execute with `WORKER::run("kebab-case-name", payload).await`
6. For progress updates, use `WORKER::call()` and receive via `ctx.emit()`

**Key Points**:
- Worker template is intentionally simple and generic (no predefined types)
- Reference `docs/ABOUT_WORKERS.md` for specific patterns (email, batch, cleanup, etc.)
- Worker names use kebab-case for registration (e.g., "send-email")
- File names use snake_case (e.g., `send_email.rs`)
- CLI philosophy: provide structure, not implementation

### When Working with Context
- Use `ctx.repository` for data that views need to access
- Use `ctx.data` for middleware-only state (not accessible in views)
- Always check session existence before accessing session data
- Use flash messages for one-time notifications: `ctx.flash_success()`, `ctx.flash_error()`

### When Working with Database
- Database operations use SQLx with compile-time SQL validation
- Type conversions handled in `database/types/converters/`
- Query builder supports MySQL, PostgreSQL, SQLite dialects
- Use global `DB` singleton for convenient access

### Performance Considerations
- Template caching provides 99.5% hit rate (638k ops/sec)
- Session operations are lock-free using DashMap
- Router uses trie structure for O(log n) matching
- Direct Request allocation preferred over pooling (benchmarks show pooling is 2x slower)

## Testing Strategy

Benchmark suite in `rustf/benches/`:
- `configuration.rs` - Config loading performance
- `context.rs` - Context operations
- `middleware.rs` - Middleware execution
- `pool.rs` - Object pooling (documents why pooling is NOT used)
- `routing.rs` - Route matching
- `session.rs` - Session management

Run specific benchmark: `cargo bench --bench <name>`

## Working with the Codebase

### Adding Features to Core Framework
1. Locate appropriate module in `rustf/src/`
2. Follow existing patterns (Arc wrapping, Result types)
3. Update `lib.rs` exports if adding public API
4. Add tests and benchmarks
5. Update relevant documentation in `docs/`

### Extending CLI Tool
1. Add command in `rustf-cli/src/commands/`
2. Register in `main.rs` command parser
3. Follow existing patterns for file analysis
4. Update CLI help text

### Modifying Auto-Discovery
1. Edit procedural macros in `rustf-macros/src/`
2. Use `walkdir` for filesystem scanning
3. Generate both module code and IDE support files
4. Test with example application

### Schema Changes
1. Edit schema types in `rustf-schema/src/types.rs`
2. Update parser in `parser.rs`
3. Modify validators in `validator.rs`
4. Update Handlebars templates in `codegen/templates/`
