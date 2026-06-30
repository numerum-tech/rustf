# rustf

[![crates.io](https://img.shields.io/crates/v/rustf.svg)](https://crates.io/crates/rustf)
[![docs.rs](https://img.shields.io/docsrs/rustf)](https://docs.rs/rustf)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

A convention-based MVC web framework for Rust, inspired by [Total.js](https://www.totaljs.com/) v4.
Designed to be equally intuitive for human developers and AI coding assistants:
auto-discovery, predictable patterns, a Total.js-style template engine, sessions,
and a multi-database model layer.

> **Status:** `1.0.0-rc1` — release candidate. APIs are close to stable; feedback welcome.

📖 **[Documentation](https://numerum-tech.github.io/rustf/)** · [Repository](https://github.com/numerum-tech/rustf)

## Quick start

```toml
[dependencies]
rustf = "1.0.0-rc1"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

```rust
use rustf::prelude::*;

#[tokio::main]
async fn main() -> rustf::Result<()> {
    let app = RustF::new()
        .controllers(auto_controllers!())
        .middleware_from(auto_middleware!());
    app.start().await
}
```

```rust
// src/controllers/home.rs
use rustf::prelude::*;

pub fn install() -> Vec<Route> {
    routes![ GET "/" => index ]
}

async fn index(ctx: &mut Context) -> rustf::Result<()> {
    ctx.json(json!({ "status": "ok", "framework": "RustF" }))
}
```

## Highlights

- **Auto-discovery** — controllers, models, middleware, workers, events discovered at build time (no `mod.rs` wiring).
- **Total.js-style views** — `@{...}` template engine with conditionals, loops, sections, layouts, form helpers (`@{text}`/`@{textarea}`/`@{checkbox}`/`@{radio}`/…), and meta helpers (`@{title}`/`@{description}`/`@{meta}`).
- **Model layer** — query builder + `BaseModel` over PostgreSQL, MySQL/MariaDB and SQLite (via `sqlx`).
- **Sessions** — in-memory by default; Redis-backed for multi-instance/clustered deployments.
- **Security built-in** — CSRF, security headers, path-traversal protection, context-aware escaping.

## Cargo features (opt-out)

All database drivers and Redis are **on by default**. For a leaner build, disable
defaults and enable only what you need:

```toml
rustf = { version = "1.0.0-rc1", default-features = false, features = [
    "embedded-views",
    "db-postgres",   # pulls in `database` + `decimal` automatically
] }
```

| Feature | Default | Enables |
|---------|:-------:|---------|
| `db-postgres` / `db-mysql` / `db-sqlite` | ✅ | per-driver SQL support (each implies `database`) |
| `database` | ✅ | driverless SQL core + `decimal` |
| `redis` | ✅ | Redis-backed (cross-instance) session storage |
| `embedded-views` | ✅ | compile templates into the binary |
| `schema` | — | YAML schema validation + codegen |
| `cli` | — | `clap`-based argument parsing |

TOML config loading, the auto-discovery macros, and UUID support are core and always compiled.

See the [Installation guide](https://numerum-tech.github.io/rustf/getting-started/installation.html#cargo-features) for the full table.

## License

Licensed under either of [Apache License 2.0](../LICENSE-APACHE) or
[MIT license](../LICENSE-MIT) at your option.
