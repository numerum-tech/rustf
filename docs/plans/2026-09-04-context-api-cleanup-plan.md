# Context API cleanup before 1.0.0 — plan

**Status:** DONE — implemented 2026-09-04, awaiting Morle's diff review before
commit. Started after the streaming review closed clean
(`docs/reviews/2026-09-04-codex-streaming-review.md`, third pass: ship).
**Branch:** `feat/streaming-body` (current work) or `refactor/context-api` after merge.
**Author context:** Claude with Morle, 2026-09-04. Every commit reviewed by user before landing.

## Goal

One obvious name per request-data accessor on `Context`, chosen for what a
coding agent types at `ctx.` and sees in autocomplete. Remove every second
spelling and every entry point that leads to hand-written
`.unwrap_or("1").parse::<i32>().unwrap_or(1)` chains. Make the obvious
path work for floats, dates, ids and decimals, not only `str`/`int`/`bool`.
All of this is breaking versus `1.0.0-rc2`; it must land before `1.0.0`
final, after which it becomes a 2.0 change.

## Decisions already taken (2026-09-04)

- **Source-first wins.** `query_int`, `param_str`, `body_bool_or` stay.
  Rationale: an agent types the *source* it wants (`ctx.param`, `ctx.query`,
  `ctx.body`) and autocomplete lists the typed and defaulted variants under
  that prefix. Type-first (`int_param`) needs a convention the agent does
  not have, and every framework agents are trained on is source-first
  (Express `req.params`, Total.js `controller.query`, Django `request.GET`,
  Symfony `query->getInt()`). Evidence in-repo: 15 `param_int` calls in CLI
  templates and sample-app, zero `int_param`, zero hand-rolled `.parse()`.
- **Untyped `ctx.param()` / `ctx.query()` go.** They are the top
  autocomplete hit and the root of the unwrap chain; 78 doc sites teach that
  pattern today. Raw access stays through the public fields
  `ctx.req.params` / `ctx.req.query`. `ctx.header()` stays (genuinely
  optional string, no typed family competing).
- **Generic `_as` family** for every other type instead of one method per
  type: `query_as::<T: FromStr>`, `query_as_or`, same for `param_` and
  `body_`. Covers `f64`, `i64`, `Uuid`, `Decimal`, `NaiveDate`,
  `DateTime<Utc>`, `IpAddr`. Error message names the expected type.
- **`body_form_typed` must parse non-string fields.** Today it builds a JSON
  object of strings, so `price: f64` fails. Switch to a form-aware
  deserializer.
- Remove: `cancel()` (no-op Total.js artifact), `body_form_data_cloned()`
  (callers can `.clone()`). See the second-round decisions below for
  `plain()` vs `text()`.

## Decisions taken 2026-09-04 (second round, with Morle)

- **Keep `ctx.plain()`, remove `ctx.text()`.** They were byte-identical
  (`text/plain; charset=utf-8`). `plain` is the Total.js spelling, which is the
  tie-breaker for a framework that models itself on Total.js. 5 call sites, all
  in `rustf/tests`.
- **`RequestData` is deleted.** Zero callers anywhere: only `ctx.request_data()`,
  one book line, and its own tests.
- **`Request::csrf()` is deleted.** It wrapped `generate_csrf(None)` in
  `.unwrap_or_default()`, so with no session it silently returns `""` and every
  form renders an empty token. Only its own tests call it;
  `ctx.generate_csrf() -> Result<String>` is the documented path.
- **`FormData` becomes multi-value.** It wrapped `HashMap<String, String>` and
  silently dropped extra values for checkboxes and `tags[]`. It now wraps
  `HashMap<String, FormValue>`: `get()` returns `Option<&str>` (first value,
  so most call sites are unchanged in spirit), `get_all()` returns `Vec<&str>`.
  The `Deref<Target = HashMap<String, String>>` impl goes away. 13 call sites.
  Pre-1.0 is the only window; after 1.0 this is a 2.0 change.
- **The `Response::` content-type and status constructors are deleted.** They
  are not the twins of `ctx.*` they look like — `Response::` is the
  framework-internal builder for paths with no `Context` (`app.rs` 404/304/
  static/error, `ctx.*` delegation). No user-facing path takes a user-built
  `Response`: the handler signature is `Result<()>` and `InboundAction::Stop`
  carries no payload. But `Response` reaches the prelude through
  `pub use crate::*`, so every dead constructor shows up in agent autocomplete
  and invites the stale `Ok(Response::json(..))` handler shape.
  Deleted (zero callers outside `response.rs`'s own tests): `json`, `html`,
  `text`, `success`, `bad_request`, `unauthorized`, `conflict`, `no_content`,
  `not_implemented`, `not_found_with_message`, `internal_server_error`.
  Kept (real internal callers): `new`, `ok`, `not_found`, `internal_error`,
  `forbidden`, `not_modified`, `advertise_content_length`, `redirect*`,
  `file_*`, `binary`, `stream`, `sse*`, `with_header`, `add_header`,
  `with_body`, `with_stream`, `body_size`, `into_hyper`.
  Note this also kills the `internal_error` / `internal_server_error` pair.

## Scope

### Remove from `Context` (`rustf/src/context.rs`)

- [ ] 16 type-first aliases: `str_query`, `int_query`, `bool_query`,
      `str_query_or`, `int_query_or`, `bool_query_or`, `str_param`,
      `int_param`, `str_param_or`, `int_param_or`, `str_body`, `int_body`,
      `bool_body`, `str_body_or`, `int_body_or`, `bool_body_or`.
- [ ] Untyped `param(&self, key) -> Option<&str>` and
      `query(&self, key) -> Option<&str>`.
- [ ] `text()` (keep `plain()`), `cancel()`, `body_form_data_cloned()`.

### Add to `Context`

- [ ] `query_as<T: FromStr>(&self, key) -> Result<T>` and
      `query_as_or<T: FromStr>(&self, key, default: T) -> T`.
- [ ] `param_as` / `param_as_or` (same shape).
- [ ] `body_as` / `body_as_or` (same shape, `&mut self` like `body_str`).
- [ ] Empty value == missing, matching `query_str`.
- [ ] Error text: `Query parameter 'start' must be a valid NaiveDate`
      (type name from `std::any::type_name`, module path stripped).
- [ ] One doc line on `body_form()` and on the `Request` pub fields pointing
      to the typed accessors, so hover text reinforces autocomplete.

### `body_form_typed<T>` (`rustf/src/context.rs:~1003`)

- [ ] Replace the JSON-of-strings bridge with a form deserializer that
      parses primitives from strings. Candidates: `serde_urlencoded`
      (already in `Cargo.lock` transitively; flat, repeated keys → `Vec`
      via `serde_html_form` if needed) or `serde_html_form`. Must keep
      `FormValue::Multiple` → `Vec<T>` working.
- [ ] Tests: struct with `f64`, `i64`, `bool`, `Option<String>`,
      `Vec<String>`, `NaiveDate`; missing optional field; bad number → clear
      error.

### `RequestData` — per open decision

- [ ] Delete the type, `ctx.request_data()`, and the book mention
      (`book/src/api-reference/context.md:385`), **or** rename methods to
      source-first and add `_as`.

### Docs to rewrite (typed forms, no `unwrap_or(..).parse()`)

Counts from `git grep` on 2026-09-04:

- [ ] `ctx.query(` — 35 sites; `ctx.param(` — 43 sites. Files:
      `book/src/api-reference/context.md`, `book/src/api-reference/routing.md`,
      `book/src/api-reference/utilities.md`, `book/src/examples/real-world-app.md`,
      `book/src/examples/rest-api.md`, `book/src/getting-started/hello-world.md`,
      `book/src/guides/controllers.md`, `docs/ABOUT_CONTROLLERS.md`, plus any
      `docs/ABOUT_*.md` hit by the grep.
- [ ] Type-first mentions: `int_param` ×4 (`book/src/examples/*.md`),
      `str_body`/`int_body` ×12 (`docs/ABOUT_CONTROLLERS.md`,
      `book/src/guides/controllers.md`).
- [ ] `ctx.plain` ×3 (`docs/RUSTF_SKILL.md`, `.claude/skills/rustf/SKILL.md`,
      `rustf-cli/templates/project/claude_skills/rustf/SKILL.md`).
- [ ] `book/src/api-reference/context.md`: drop the "type-first alias"
      paragraph; add `_as` family and the float/date example.
- [ ] Skill (3 copies): add rule "never `.parse()` a query/param/body value
      by hand; use `*_int` / `*_bool` / `*_as::<T>` / `*_or`"; replace
      `ctx.param_int("id")? as i64` with `ctx.param_as::<i64>("id")?`.
- [ ] CLI templates (`rustf-cli/templates/**`) and `sample-app/src`: 9 sites
      of `ctx.param(` / `ctx.query(`; 1 `ctx.plain(`.

### Code sites to migrate

- [ ] `rustf/src` — 1 `ctx.query(` outside `context.rs`.
- [ ] `rustf/tests` — 6 `ctx.query(`.
- [ ] `rustf/src/http/request_data.rs` — if kept, 11 internal alias calls.

### Release bookkeeping

- [ ] CHANGELOG `[Unreleased]` → `### Removed` (aliases, untyped accessors,
      `plain`, `cancel`, `body_form_data_cloned`, maybe `RequestData`) and
      `### Added` (`_as` family, real form deserialization). Mark BREAKING.
- [ ] `docs/RUSTF_SKILL.md` §"common mistakes" gains the parse rule.
- [ ] `.wolf/cerebrum.md` decision log entry; `.wolf/anatomy.md` if
      `request_data.rs` is deleted.

## Order of work

1. Add the `_as` family + tests (additive, nothing breaks).
2. Fix `body_form_typed` + tests (additive).
3. Migrate code sites (framework, tests, templates, sample-app) to typed forms.
4. Remove aliases, untyped accessors, `plain`, `cancel`, `body_form_data_cloned`,
   `RequestData` per decision. Build must be clean with `-D warnings`.
5. Rewrite docs and skill copies.
6. CHANGELOG, cerebrum, anatomy.
7. `cargo test --all-features` in `rustf`, `cargo check` in `sample-app`,
   `rustf-cli new crud` smoke test to prove templates still compile.

## Acceptance

- `git grep -E "ctx\.(query|param)\("` returns 0 hits in code and docs.
- `git grep -E "\.(str|int|bool)_(query|param|body)"` returns 0 hits.
- `git grep -E "unwrap_or\(.*\)\.parse"` returns 0 hits in docs and templates.
- A handler can do `let start: NaiveDate = ctx.query_as("start")?;` and
  `#[derive(Deserialize)] struct F { price: f64 }` via `body_form_typed`.
- Suite green, sample-app compiles, CRUD scaffold compiles.

## Risks

- Anyone on rc2 who used `ctx.param()` breaks. Acceptable pre-1.0; the
  CHANGELOG entry must show the one-line replacement for each removal.
- `body_form_typed` deserializer swap changes error messages; tests pin the
  new ones.
- Doc rewrite is the bulk of the effort (~90 sites); do it with a script
  and review the diff, not by hand.

## Outcome (2026-09-04)

Implemented in full. Verification actually run:

- `rustf`: 694 tests pass under `RUSTFLAGS="-D warnings"` (512 lib + 182 across
  33 integration binaries), including the two socket-binding streaming ones.
- `cargo check` clean on `rustf-cli`, `rustf-schema`, `rustf-macros`,
  `sample-app`.
- `cargo fmt --check` clean on `rustf`. It was already dirty before this work
  on `rustf-cli/build.rs`, `rustf-cli/src/analysis/handlers.rs`,
  `rustf-macros/src/lib.rs` and `rustf-schema/src/codegen/sqlx.rs` — files this
  change never touched.
- New `rustf/src/http/form_de.rs` draws zero clippy warnings.
- **Scaffold smoke test**: generated a fresh project with
  `rustf-cli new project`, added a schema, ran `schema generate models`, then
  `new crud --name widgets`; the result compiles with `SQLX_OFFLINE=true` and
  the generated service uses `&FormData` with `get_str` / `get_as::<T>`.

The form deserializer is hand-written (`rustf/src/http/form_de.rs`) rather than
delegating to `serde_urlencoded`. `serde_urlencoded` is already in the lock file
and the local cargo cache, but it has no sequence support, so
`FormValue::Multiple -> Vec<T>` would have regressed. `serde_html_form` handles
that but is not vendored. Deserializing straight from the parsed `FormValue`
map also gives exact error messages (`Field 'price' must be a valid f64 (got
'abc')`) instead of serde's `invalid type: string`.

### Deviations from the plan as written

- **`ctx.plain()` kept, `ctx.text()` removed** (the plan originally had it the
  other way). `plain` is the Total.js spelling.
- **`Response`'s dead constructors were removed too.** Not in the original
  scope; added after Morle asked why the `ctx.*` / `Response::*` twins exist at
  all. See the second-round decisions above.
- **`FormData` went multi-value now** rather than being deferred past 1.0.
- **`RequestData` deleted** (the plan leaned this way; confirmed by zero
  callers).

### Acceptance, as measured

- `git grep -E "ctx\.(query|param)\("` — 0 hits in live code and docs. Two
  prose mentions survive in `book/src/api-reference/context.md`, both saying
  the method no longer exists.
- `git grep -E "\.(str|int|bool)_(query|param|body)"` — 0 hits.
- `git grep -E "unwrap_or\(.*\)\.parse"` — 1 hit, deliberate: the
  "❌ Approach 1: Manual (verbose, error-prone)" counter-example in
  `docs/ABOUT_CONTROLLERS.md` / `book/src/guides/controllers.md` that the
  typed-helper section is contrasted against.

### Left undone, deliberately

`rustf/tests/{integration,security,performance,unit}/` are four directories of
`.rs` files with no `main.rs`, so cargo has never compiled them. They still use
the pre-rc `async fn(mut ctx: Context) -> Result<Response>` handler signature
and contain `ctx.query(` calls. Rewriting them to the current API and wiring
them into the test harness is its own task; half-migrating dead files would
only hide the rot. Tracked in `WORK_PLAN.md`.
