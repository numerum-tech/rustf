# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0-rc1] - 2026-06-22

### Added
- **View engine — form helpers**: `@{text}`, `@{textarea}`, `@{password}`, `@{hidden}`, `@{checkbox}`, `@{radio}` — auto-bind to the model field and render the element with an attribute object (`{ class: 'x', required: true }`).
- **View engine — meta helpers**: `@{title('...')}` and `@{description('...')}` (deferred meta data, carried view → layout); read back via `@{title}` / `@{description}`.
- **View engine — `@{mobile}`**: User-Agent based mobile-device boolean; request `@{url}` / `@{hostname}` are now populated from the request.
- **Cargo features**: per-driver SQL features `db-postgres` / `db-mysql` / `db-sqlite` and a `redis` feature — all on by default (opt-out via `--no-default-features`); `decimal` is implied by `database`.
- **Packaging**: per-crate READMEs and `readme` / `documentation` / `homepage` metadata for crates.io.
- **Module System**: New explicit developer control for module registration with `ModuleRegistry`.
- **CLI Improvements**: Simplified module generation with `rustf-cli`, added `--shared` flag.
- **Middleware**: Dual-phase middleware architecture (Inbound/Outbound).
- **Workers**: New `rustf-cli new worker` command and simplified worker templates.
- **Performance**: Significant log cleanup in database and view layers (removed ~46 debug logs).
- **Core**: Framework prelude added to utility modules.

### Changed
- **Template `||` / `&&`**: now return the operand value (JS / Total.js value-fallback semantics), so `@{M.a || 'default'}` renders the default; conditions still evaluate via truthiness.
- **SQL drivers & Redis** are now optional (opt-out) Cargo features instead of mandatory dependencies; default builds are unchanged.
- Bumped `thiserror` to 2.0; updated transitive `slab` off a yanked version.
- Refactored `auto_modules!` macro to specified declaration-only behavior.
- Simplified environment configuration from 4 to 2 environments.
- Optimized database adapters (MySQL, PostgreSQL, SQLite) by removing excessive debug logging.

### Fixed
- `@{break}` / `@{continue}` inside `@{if}` now correctly affect the enclosing loop (were silently swallowed).
- `@{root}` reads `views.default_root` and strips the trailing `/` (previously always empty).
- Redis `never-type-fallback` future-incompatibility; redis-server integration tests gated behind `#[ignore]`.
- Invalid `total.js` crate keyword → `totaljs`; version drift across crates aligned to `1.0.0-rc1`.
- `rustf-schema` codegen: HTML-escaped output, missing CRUD methods (empty template vars), `.length`/`{{else if}}` handlebars gaps, and a circular-dependency detector that ignored cycles.
- Cleared all build warnings across `rustf` and `rustf-cli` (removed ~2.5k lines of dead code in the CLI).
- Middleware template dual-phase architecture implementation.
- Various test failures in configuration and core modules.
- Compilation warnings in sample application.

### Removed
- Removed automatic `MODULE::init()` calls in favor of explicit initialization.
- Removed deprecated CLI flags (`--module-type`).
