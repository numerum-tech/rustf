# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Generic typed accessors on `Context`**: `query_as::<T>` / `param_as::<T>` /
  `body_as::<T>` and their `_or` variants, for any `T: FromStr`. These cover the
  types the `_int` / `_bool` accessors do not — `f64`, `i64`, `u32`, `Uuid`,
  `Decimal`, `NaiveDate`, `DateTime<Utc>`, `IpAddr` — so no call site needs a
  hand-written `.parse()` chain. The error names the parameter and the expected
  type: `Query parameter 'start' must be a valid NaiveDate`.
- **`FormData` typed getters**: `get_as::<T>`, `get_as_or::<T>`, `get_all_as::<T>`,
  alongside the existing `get_str` / `get_int` / `get_bool`.
- **`FormData` and `FormValue` are exported** from the crate root and the
  prelude, so a service can take `&FormData` without reaching into
  `rustf::http`.
- **Streaming response bodies.** `Response.body` is now a `Body` enum —
  `Body::Full(Vec<u8>)` (buffered, the default) or `Body::Stream(BodyStream)`
  (chunks produced on demand). A stream that declares its size is sent with
  `Content-Length`; an open-ended one falls back to chunked transfer encoding.
  `with_body` accepts `impl Into<Body>`, so every existing `with_body(vec)`,
  `ctx.json()` and `ctx.view()` call site compiles unchanged.
- **Streamed file responses**: `ctx.file_download_stream[_from]` and
  `ctx.file_inline_stream[_from]` (and the matching `Response::` constructors)
  serve a file in 64 KiB chunks, so peak memory per request is flat regardless
  of file size. Same path-containment rules as the buffered variants.
- **Server-Sent Events**: `ctx.sse(events)` / `Response::sse(events)` take a
  `Stream<Item = SseEvent>` and emit the `text/event-stream` wire format with
  `Cache-Control: no-cache` and `X-Accel-Buffering: no`. `SseEvent` builds one
  event (`data`, `id`, `event`, `retry`, or a comment; `SseEvent::json` for
  serialised payloads). The stream may yield `SseEvent` or
  `rustf::Result<SseEvent>` (sealed trait `SseItem`), so a fallible source
  can abort the feed with an `Err`. `ctx.sse_with_keep_alive(events, interval)`
  injects a comment line once per interval while the feed is idle (the first
  one a full interval after the stream starts) so proxies do not drop the
  connection. Field values are split on `\r\n`, `\n` and bare `\r` into one
  field line each, so an attacker-controlled `id`, `event` or `data` cannot
  forge another field whatever line ending it uses.
- **Static files above 1 MiB are streamed** (`STATIC_STREAM_THRESHOLD`).
  Smaller assets stay buffered so gzip and other body-inspecting middleware
  still apply to stylesheets and scripts. ETag / Last-Modified / 304 handling
  is unchanged. `Range` requests are still not supported by the static
  server; streaming changes the memory profile, not the protocol surface.
- **`HEAD` requests.** A `HEAD` with no matching `HEAD` route runs the `GET`
  handler; the response keeps the headers (including the `Content-Length`
  the body would have had) and the body is discarded before anything is
  sent. Static files behave the same way. No length is invented for an
  open-ended stream, and none is added to a `1xx`, `204` or `304`.
- **Declared stream sizes are enforced.** A `Body::from_sized_stream` (or
  streamed file) that produces more than its declared length is truncated to
  it and logged; one that ends short yields an error so the connection is
  dropped instead of leaving the client waiting on a `Content-Length` that
  will never be satisfied.
- `Body`, `BodyStream` and `SseEvent` are exported from the prelude.
- `tokio-stream` is now a dependency, so `ReceiverStream::new(rx)` is the
  idiomatic way to feed an mpsc channel into `ctx.sse`.

### Changed
- **BREAKING — `FormData` preserves multi-value fields.** It wrapped
  `HashMap<String, String>`, so a repeated field (a checkbox group, `tags[]`)
  silently kept only one value. It now wraps `HashMap<String, FormValue>`:
  `get(key)` returns `Option<&str>` (the first value) and the new
  `get_all(key)` returns every value. The
  `Deref<Target = HashMap<String, String>>` impl is gone; use `get`, `get_all`,
  `get_value`, `contains_key`, `len`, `is_empty`, `keys`, `iter` or
  `into_inner()`. A function that took `&HashMap<String, String>` from
  `ctx.body_form()` now takes `&FormData`.
- **BREAKING — `ctx.body_form_typed::<T>()` parses non-string fields.** It used
  to build a JSON object of strings and hand it to `serde_json`, so a struct
  field typed `f64` or `bool` failed with `invalid type: string`. A form-aware
  deserializer now parses primitives out of their string form, maps a repeated
  field onto a `Vec<T>` field, and treats an empty value as `None` for an
  `Option<T>` field. Error messages name the field and the expected type.

- **BREAKING — `Response` is no longer `Clone`.** A streaming body is a
  one-shot source of bytes. Nothing in the framework cloned a response
  (`ctx.res` moves via `take_response()`); app code that did must clone the
  pieces it needs instead.
- **BREAKING — `Response::body_size()` returns `Option<usize>`.** An
  open-ended stream has no length; returning `0` would have silently misled
  middleware that gates on size. `Body::len_hint()` is the same query on the
  body itself.
- **`ctx.stream(body, ..)` / `Response::stream(body, ..)` now stream.** The
  first argument is `impl Into<Body>`; a `Vec<u8>` still works and is sent
  buffered exactly as before, a `Body::from_stream(..)` is sent chunk by
  chunk. Previously this helper buffered everything and set
  `Transfer-Encoding: chunked` next to a `Content-Length` — a protocol
  violation — which it no longer does.
- `Response::into_hyper` returns `UnsyncBoxBody<Bytes, Error>` rather than
  `BoxBody`: a boxed stream is `Send` but not `Sync`, which is all hyper
  requires.
- **`Response::into_hyper` is now the single authority on framing headers.**
  A hand-set `Transfer-Encoding` is always dropped (with a warning), and a
  hand-set `Content-Length` is always dropped, so a stale, duplicated or
  contradictory value can never reach the wire: hyper derives the length of a
  buffered body, a sized stream supplies its declared size, and an empty body
  standing in for a representation (`HEAD`, `304`) carries the value given to
  the new `Response::advertise_content_length(len)`.
- **BREAKING — `Response` gained a private field.** It can no longer be built
  as a struct literal; use `Response::new(status)` / `Response::ok()` and the
  `with_*` builders. The three public fields (`status`, `headers`, `body`)
  are unchanged. Done now, before 1.0, so later fields are non-breaking.
- `Body::len_hint()` / `Response::body_size()` return `None` rather than a
  wrapped value when a declared stream size does not fit `usize`.
- `Response::file_*` helpers resolve and validate paths with `tokio::fs`
  instead of blocking `std::fs` calls inside async code.
- Compression middleware passes streaming bodies through uncompressed; their
  bytes do not exist yet when outbound middleware runs. Streaming gzip needs a
  flush-per-chunk encoder and is separate work.
- A stream that yields an `Err` mid-flight is now logged at `error` level.
  The status and headers were already sent, so the client sees a truncated
  body under the original status; this log is the only server-side trace.

### Removed
- **BREAKING — untyped `ctx.param()` and `ctx.query()`.** They returned
  `Option<&str>`, were the first autocomplete hit, and led directly to
  `ctx.query("page").unwrap_or("1").parse::<i32>().unwrap_or(1)`. Use the typed
  accessors (`param_str`, `query_int`, `query_as::<T>`, the `_or` variants).
  The raw maps remain public as `ctx.req.params` and `ctx.req.query`.
- **BREAKING — the 16 type-first accessor aliases**: `str_query`, `int_query`,
  `bool_query`, `str_query_or`, `int_query_or`, `bool_query_or`, `str_param`,
  `int_param`, `str_param_or`, `int_param_or`, `str_body`, `int_body`,
  `bool_body`, `str_body_or`, `int_body_or`, `bool_body_or`. Every one had an
  identical source-first spelling (`query_str`, `param_int`, `body_bool_or`, …);
  the replacement is the same words in the other order.
- **BREAKING — `ctx.text()`.** It was byte-identical to `ctx.plain()`, which is
  the Total.js spelling. Use `ctx.plain()`.
- **BREAKING — `ctx.cancel()`.** A no-op that returned `&Self` and did nothing.
- **BREAKING — `ctx.csrf()` and `Request::csrf()`.** `ctx.csrf()` wrapped
  `generate_csrf(None)` in `unwrap_or_default()`, so with no session it returned
  `""` and forms rendered an empty token that failed every POST with no hint.
  `Request::csrf()` returned a fresh random token on each call that was never
  stored in a session, so it could never validate. Use
  `ctx.generate_csrf(None) -> Result<String>`.
- **BREAKING — `ctx.body_form_data_cloned()`.** Call `.clone()` on
  `ctx.body_form_data()` instead.
- **BREAKING — `RequestData`, `BodyData` and `ctx.request_data()`.** The type
  had no callers, and its methods were the type-first spellings being removed.
- **BREAKING — the unused `Response` constructors**: `json`, `html`, `text`,
  `success`, `bad_request`, `unauthorized`, `conflict`, `no_content`,
  `not_implemented`, `not_found_with_message`, `internal_server_error`. None had
  a caller outside `response.rs`'s own tests: they are leftovers from the old
  `Result<Response>` handler signature. No user-facing path takes a user-built
  `Response` — handlers return `Result<()>` and `InboundAction::Stop` carries no
  payload — but `Response` reaches the prelude, so every dead constructor was
  showing up in autocomplete. The constructors the framework itself uses stay
  (`new`, `ok`, `not_found`, `internal_error`, `forbidden`, `not_modified`,
  `redirect*`, `file_*`, `binary`, `stream`, `sse*`, `with_body`,
  `with_stream`). This also removes the `internal_error` /
  `internal_server_error` pair.
- `Request::body_as_form()`, superseded by `Request::body_as_form_data()`.

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
