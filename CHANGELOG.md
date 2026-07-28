# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0-rc2] - 2026-07-28

### Changed
- **`Context` repository is held as a `serde_json::Value` object** instead of a
  `HashMap<String, Value>`, so `ctx.view(...)` hands the template engine a
  borrow of it rather than rebuilding and deep-cloning a `Value` on every
  render. Public API is unchanged (`repository_set` / `repository_get` /
  `repository_clear` behave as before).
- Documented the model-vs-repository split on `Context::view` and
  `Context::repository_set`, and in the bundled `rustf` skill and the CLI
  controller templates: the `data` argument carries the page's single subject
  (read as `@{M.field}`), everything else goes through the repository (read as
  `@{R.key}`).

### Fixed
- **Redis-backed session tests no longer fail where no Redis server is running.**
  The five `session::redis` unit tests and the session middleware integration
  test hard-required a server on `127.0.0.1:6379` and panicked without one, so
  CI's `cargo test --all-features` job for the `rustf` crate failed on every run
  (they were never `#[ignore]`d, despite the CI comment and the rc1 notes saying
  so). They now probe for a reachable server — via the timeout-bounded `PING`
  that `RedisSessionStorage::from_url` already performs — and skip with a
  message when there is none. Set `RUSTF_TEST_REDIS=1` to turn an unreachable
  server back into a failure, so a CI job that provisions Redis cannot lose the
  coverage silently.
- **UTF-8 char-boundary panic in the Total.js expression parser.**
  `find_operator_at_level`, `find_ternary_operator` and `find_ternary_colon`
  returned a `char` index that callers used as a byte offset, so any multi-byte
  character before an operator or ternary panicked with `byte index N is not a
  char boundary`. Offsets are now tracked via `char_indices()` and operators are
  compared char-wise.

## [1.0.0-rc1] - 2026-07-15

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

- **BREAKING — auto-discovery layout.** `#[auto_discover]` no longer emits
  IDE-only `src/_controllers.rs` / `_models.rs` / etc. It now generates
  framework-owned modules under `src/.rustf/<dir>_gen.rs` and a thin,
  user-editable wrapper `src/<dir>/mod.rs` that re-exports them. Discovery now
  also covers `src/middleware/`, `src/events/`, and `src/definitions/`.

  **Migration for existing projects:**
  1. Delete the legacy `src/_*.rs` files (the macro removes them automatically
     on the next build; safe to `git rm` them).
  2. If you do **not** already have a hand-written `src/<dir>/mod.rs`, nothing
     else is needed — the macro scaffolds the wrapper for you.
  3. If you **do** have a hand-written `src/<dir>/mod.rs`, the build now fails
     with a `compile_error!` unless the wrapper keeps the generated import and
     re-export intact. Add these two lines (custom code alongside is allowed):
     ```rust
     #[path = "../.rustf/<dir>_gen.rs"]
     mod generated;
     pub use generated::*;
     ```
  4. Every `.rs` file in `middleware/`, `events/`, and `definitions/` must
     export the expected registration entry point (`install(registry)`,
     `install(emitter)`, `install(defs)` respectively); move helper-only files
     elsewhere or they will fail to compile.
  5. Add `.rustf/` to your `.gitignore` — the generated `src/.rustf/` directory
     is framework-owned and should not be committed. (New projects scaffolded by
     `rustf-cli` already include this.)
- **Template `||` / `&&`**: now return the operand value (JS / Total.js value-fallback semantics), so `@{M.a || 'default'}` renders the default; conditions still evaluate via truthiness.
- **SQL drivers & Redis** are now optional (opt-out) Cargo features instead of mandatory dependencies; default builds are unchanged.
- **HTTP server migrated to hyper 1.x** (hyper-util `auto` server + graceful shutdown, `http-body-util`); off the EOL-track hyper 0.14.
- Bumped `thiserror` to 2.0; updated transitive `slab` off a yanked version.
- Refactored `auto_modules!` macro to specified declaration-only behavior.
- Simplified environment configuration from 4 to 2 environments.
- Optimized database adapters (MySQL, PostgreSQL, SQLite) by removing excessive debug logging.

### Fixed

- File-response helpers (`Response::file_download{,_from}`, `file_inline{,_from}`
  and their `Context` wrappers) now read files with async `tokio::fs` instead of
  blocking `std::fs::read`, so serving files no longer stalls a runtime worker
  thread. These methods are now `async` — add `.await` at call sites.

- **CLI DB export parity.** `rustf-cli db` data export now returns real rows for
  **all three** backends. PostgreSQL previously returned `"[]"` and MySQL
  returned `"[]"`/`""` placeholders regardless of the data; both now serialize
  actual rows to JSON or CSV, matching SQLite. Added the `bigdecimal` sqlx
  feature so `NUMERIC`/`DECIMAL` columns export their real value instead of
  null. CSV rendering is shared across backends (`rows_to_csv`).
- **CLI DB introspection parity.** `db describe` now reports foreign-key
  constraints for MySQL and PostgreSQL (both previously returned an empty
  `constraints` list), matching SQLite. Relationship metadata is no longer
  dropped by backend.
- `@{break}` / `@{continue}` inside `@{if}` now correctly affect the enclosing loop (were silently swallowed).
- `@{root}` reads `views.default_root` and strips the trailing `/` (previously always empty).
- Redis `never-type-fallback` future-incompatibility.
- Invalid `total.js` crate keyword → `totaljs`; version drift across crates aligned to `1.0.0-rc1`.
- `rustf-schema` codegen: HTML-escaped output, missing CRUD methods (empty template vars), `.length`/`{{else if}}` handlebars gaps, and a circular-dependency detector that ignored cycles.
- Cleared all build warnings across `rustf` and `rustf-cli` (removed ~2.5k lines of dead code in the CLI).
- Middleware template dual-phase architecture implementation.
- Various test failures in configuration and core modules.
- Compilation warnings in sample application.

### Removed

- **BREAKING — `DB::execute_raw`.** Removed the MySQL-only `DB::execute_raw`,
  which existed only behind `#[cfg(feature = "db-mysql")]`, returned the
  MySQL-specific `sqlx::mysql::MySqlQueryResult`, and errored `"not yet
  implemented"` for PostgreSQL/SQLite — a cross-database parity gap at a
  framework-level API. It was a redundant, injection-prone (no params) duplicate
  of already cross-dialect methods. **Migration:** use
  `DB::execute_with_params(sql, vec![])` for writes/DDL (returns rows affected),
  `DB::fetch_all_with_params(sql, params)` for row-returning queries, or
  `DB::execute_insert_returning(...)` for inserts that need the new id.
- Removed automatic `MODULE::init()` calls in favor of explicit initialization.
- Removed deprecated CLI flags (`--module-type`).
