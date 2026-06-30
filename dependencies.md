# RustF Dependencies

This document lists the direct dependencies declared by the RustF framework
workspace crates and what they are used for in the codebase.

Scope:

- `rustf` - core framework library
- `rustf-cli` - CLI and MCP server
- `rustf-schema` - schema parsing, validation, and code generation
- `rustf-macros` - procedural macros

Notes:

- This covers direct dependencies from the crate manifests, not every transitive crate.
- `dev-dependencies` are listed separately at the end.
- Optional dependencies are marked explicitly.

## Workspace Internal Crates

| Dependency | Used by | Purpose |
| --- | --- | --- |
| `rustf` | `rustf-cli` | Lets the CLI scaffold projects, share framework types, and reuse framework logic. |
| `rustf-schema` | `rustf`, `rustf-cli` | Schema model, validation, and code generation support. It is optional in `rustf` behind the `schema` feature and always enabled in `rustf-cli`. |
| `rustf-macros` | `rustf` | Auto-discovery and embed-related procedural macros re-exported by the framework. |

## `rustf` Core Framework

### Async runtime and HTTP stack

| Dependency | Purpose |
| --- | --- |
| `tokio` | Primary async runtime for the server, request handling, background tasks, and async IO. |
| `hyper` | Low-level HTTP protocol server implementation. |
| `hyper-util` | Tokio integration and server utilities layered on top of Hyper. |
| `http-body-util` | HTTP body helpers used in request/response handling and tests. |
| `bytes` | Efficient byte buffers for HTTP and body processing. |
| `futures` | Async combinators and future utilities beyond `std`. |
| `async-trait` | Async trait methods for extensibility points like storage and middleware-style abstractions. |
| `num_cpus` | CPU count detection for runtime or pool sizing. |

### Serialization, configuration, and data formats

| Dependency | Purpose |
| --- | --- |
| `serde` | Core serialization and deserialization for config, session data, models, and public types. |
| `serde_json` | JSON encoding for API responses and generic JSON handling. |
| `simd-json` | Faster JSON parsing in hot paths such as Redis session decoding and some internal parsing. |
| `toml` | Parsing framework configuration files. |
| `serde-toml-merge` | Layering and merging TOML configuration sources. |

### Routing, parsing, and general utilities

| Dependency | Purpose |
| --- | --- |
| `url` | URL parsing and manipulation. |
| `regex` | Route, validation, and text pattern matching. |
| `once_cell` | Lazy static initialization without macros. |
| `lazy_static` | Legacy/static global initialization used in some shared registries and helpers. |
| `indexmap` | Ordered maps where deterministic iteration order matters. |
| `anyhow` | Flexible error propagation in framework internals and CLI-facing helpers. |
| `thiserror` | Typed error definitions for the framework error surface. |
| `log` | Logging facade used across the framework. |

### Sessions, security, and state

| Dependency | Purpose |
| --- | --- |
| `dashmap` | Concurrent in-memory maps for session storage, caches, translations, and rate limiting. |
| `rand` | Randomness for session IDs, nonces, and other security-sensitive tokens. |
| `uuid` | UUID support for models, config, and database-related values. |
| `base64` | Base64 encoding for CSP nonces and session/security helpers. |
| `percent-encoding` | URL and path-safe encoding helpers. |
| `urlencoding` | Helper-level URL encode/decode functions exposed through framework utilities. |
| `sha2` | Modern hashing for security-sensitive operations. |
| `sha1` | Compatibility hashing for protocols or legacy integrations. |
| `md-5` | MD5 hashing for compatibility cases where weak hashing is still required by external behavior. |
| `flate2` | Gzip compression middleware and related benchmarking. |
| `chrono` | Timestamps in logs, error payloads, sessions, and database conversion logic. |

### Database and schema support

| Dependency | Purpose |
| --- | --- |
| `sqlx` (optional) | Async database access layer for PostgreSQL, MySQL, and SQLite support. Enabled by `database` and the `db-*` features. |
| `rust_decimal` (optional) | DECIMAL/NUMERIC handling in database types and generated models. Enabled through `decimal`. |
| `ipnetwork` | PostgreSQL `inet` and CIDR type conversion support. |
| `rustf-schema` (optional) | Re-exported schema definitions and validation/codegen support behind the `schema` feature. This feature is no longer part of `rustf`'s default feature set. |

### Embedding and code generation hooks

| Dependency | Purpose |
| --- | --- |
| `rust-embed` (optional) | Embeds views and static assets into the binary when `embedded-views` is enabled. |
| `rustf-macros` | Provides the auto-discovery and embed helper macros used by applications. |
| `clap` (optional) | Powers the framework's optional built-in CLI support module. |

### Redis integration

| Dependency | Purpose |
| --- | --- |
| `redis` (optional) | Redis client used by the built-in shared session storage backend. |
| `deadpool-redis` (optional) | Connection pooling for Redis-backed session storage. |

## `rustf-cli`

### CLI, logging, and runtime

| Dependency | Purpose |
| --- | --- |
| `clap` | Command-line argument parsing and subcommand structure. |
| `env_logger` | Logger initialization for human-readable CLI logging. |
| `log` | Logging facade used by CLI modules. |
| `tokio` | Async runtime for database introspection, file watching, and server tasks. |
| `anyhow` | Top-level ergonomic error handling for commands. |
| `thiserror` | Typed command and subsystem errors. |

### Serialization and config

| Dependency | Purpose |
| --- | --- |
| `serde` | Shared serialization across exported analysis output and internal models. |
| `serde_json` | JSON output and MCP payload handling. |
| `serde_yaml` | YAML handling for schema and export flows. |
| `toml` | Reading and generating Cargo/config TOML content. |

### MCP and server interfaces

| Dependency | Purpose |
| --- | --- |
| `jsonrpc-core` | Core JSON-RPC primitives for the MCP-style server implementation. |
| `jsonrpc-http-server` | HTTP transport for the CLI server. |
| `jsonrpc-ws-server` | WebSocket transport for the CLI server. |
| `async-trait` | Async trait ergonomics in command/server abstractions. |

### Project analysis and file system work

| Dependency | Purpose |
| --- | --- |
| `walkdir` | Recursive scanning of project files, templates, and source trees. |
| `notify` | File watching for continuous validation and analysis features. |
| `glob` | Pattern-based file discovery. |
| `syn` | Rust AST parsing for project introspection and analysis. |
| `quote` | Token generation paired with AST-driven analysis/codegen helpers. |
| `proc-macro2` | Shared token types used with `syn` and `quote`. |
| `regex` | Parsing and validation of source snippets and naming rules. |
| `once_cell` | Lazy initialization for caches and registries. |
| `indexmap` | Ordered maps in analysis/export flows. |
| `rayon` | Parallel processing for analysis work. |
| `md5` | File checksum generation in watcher/dependency analysis. |

### Scaffolding, templates, and utilities

| Dependency | Purpose |
| --- | --- |
| `handlebars` | Template rendering for generated project files and components. |
| `rust-embed` | Embeds CLI templates into the binary. |
| `rand` | Random values in generators and scaffolding flows. |
| `dirs` | Locating user-specific directories. |
| `lazy_static` | Global registries and static setup where still used. |
| `uuid` | UUID values in generated code and metadata. |
| `chrono` | Timestamps for generated files, backup metadata, and watcher events. |
| `tempfile` | Temporary directories/files for generation and validation workflows. |

### Database and schema tooling

| Dependency | Purpose |
| --- | --- |
| `sqlx` | Database introspection and generated model guidance for PostgreSQL, MySQL, and SQLite. |
| `url` | Connection string parsing and URL utilities. |
| `rust_decimal` | Generated type support when schema inspection finds decimal fields. |
| `rustf` | Reuses framework code and keeps scaffolding aligned with the framework API. |
| `rustf-schema` | Schema structures plus `codegen` support for generation commands. |

### Platform-specific

| Dependency | Purpose |
| --- | --- |
| `winapi` (Windows only) | Windows-specific process and handle operations. |

## `rustf-schema`

| Dependency | Purpose |
| --- | --- |
| `serde` | Schema type serialization/deserialization. |
| `serde_json` | JSON-based schema data handling and export paths. |
| `serde_yaml` | YAML schema parsing, which is the primary schema authoring format. |
| `thiserror` | Typed schema and validation errors. |
| `once_cell` | Lazy static helpers. |
| `handlebars` (optional) | Template-driven code generation when the `codegen` feature is enabled. |
| `chrono` (optional) | Type mapping support for generated date/time fields. |
| `rust_decimal` (optional) | Type mapping support for generated decimal fields. |
| `tokio` (optional) | Async filesystem access for loading schema directories. |

## `rustf-macros`

| Dependency | Purpose |
| --- | --- |
| `proc-macro2` | Token representation for generated procedural macro output. |
| `quote` | Builds Rust token streams emitted by the macros. |
| `syn` | Parses macro input and constructs syntax fragments. |
| `walkdir` | Scans `src/controllers`, `src/models`, `src/middleware`, and related directories during auto-discovery. |

## Dev Dependencies

These are not part of the shipped runtime surface, but they support tests and benchmarks.

### `rustf`

| Dependency | Purpose |
| --- | --- |
| `tokio-test` | Async test helpers. |
| `tempfile` | Temporary test files and directories. |
| `criterion` | Benchmarks for routing, sessions, compression, views, and related subsystems. |

### `rustf-cli`

| Dependency | Purpose |
| --- | --- |
| `tempfile` | Temporary fixtures in CLI tests. |
| `assert_cmd` | End-to-end CLI command assertions. |
| `predicates` | Structured output assertions in tests. |

### `rustf-schema`

| Dependency | Purpose |
| --- | --- |
| `tokio` | Async test coverage for parser/loading code. |
| `tempfile` | Temporary schema fixtures in tests. |
