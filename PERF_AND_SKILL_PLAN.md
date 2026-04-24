# RustF Perf Sweep + Claude Skill — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Branch:** `perf-sweep-and-skill`
**Created:** 2026-04-24
**Author context:** Claude (with user oversight — every commit requires explicit user diff review).

**Goal:** Land gzip compression (already in-flight), three non-breaking runtime-perf wins, two dev-speed wins, a single reusable Claude skill for RustF, and end-to-end benches proving the deltas.

**Architecture:** Each task is a small PR-sized commit preceded by a failing test or bench. Skill ships both as `.claude/skills/rustf/SKILL.md` in this repo and bundled into the `rustf-cli new project` template so every future RustF project gets it.

**Tech Stack:** Rust 2021, tokio, hyper, flate2 (new), criterion for benches, serde_json + simd-json already integrated.

---

## Scope boundaries

### In scope (RC1-safe, non-breaking)
1. Commit the already-implemented compression middleware.
2. Repository + session pass-by-ref into the renderer (context.rs → views/*).
3. Static-file single-I/O (merge metadata + read).
4. HTTP date formatter + parser cached with 1s granularity.
5. Cookie parse cached on Request.
6. Populate `sample-app/` with a working minimal controller + middleware + views (today those dirs are empty — a fresh clone has nothing runnable).
7. `rustf-cli new crud <model>` generator — one command produces controller + wrapper model stub + 4 views + integration test.
8. `.claude/skills/rustf/SKILL.md` in this repo + template copy for new projects.
9. `benches/request_lifecycle.rs` — end-to-end bench to quantify actual wins.

### Explicit "not doing" (deferred, post-RC1)
- Response `headers: Vec<(String, String)>` → `HashMap` — public type change.
- True streaming request/response bodies — large refactor.
- Rkyv session storage — separate plan per `HYBRID_SERIALIZATION_PLAN.md`.
- Removing `embedded-views`/`auto-discovery` from `rustf` default features.
- Skipping empty-HashMap for parameterless routes — low impact.
- ETag on rendered view responses — useful but bigger design question.
- `rustf-cli new project --database <driver>` flag — real but modest (~30-60s first build); not worth the CLI surface area alone.
- Auto-discovery scan mtime cache in `rustf-macros` — framework-side, not end-user dev-speed; deferred.
- Hot-reload dev server (`rustf-cli dev`) — biggest iteration-speed win but deserves its own focused plan; deferred.
- Dev-mode rich error pages — overlaps with `rustf/src/error/pages.rs`, needs scoping; deferred.

---

## Granularity note

The writing-plans skill recommends 2-5 min steps. Many steps below touch 2-4 files and run 10-20 min because the framework's layers are tightly coupled (e.g. touching `ViewEngine::render_rich` requires engine.rs + renderer.rs + context.rs in lockstep). This is honest — splitting further would fragment diffs the user needs to review atomically.

---

## Chunk 1: Baseline + Compression

### Task 0: Establish baseline

- [ ] **Step 0.1** — run all existing benches, save outputs to `bench-results.md` at repo root:
  ```bash
  cargo bench --bench routing --bench middleware --bench context --bench session --bench minifier --bench configuration 2>&1 | tee bench-results.md
  ```
- [ ] **Step 0.2** — build sample-app once, record cold-build time: `cd sample-app && time cargo build 2>&1 | tail -5`. Log to `bench-results.md`.
- [ ] **Step 0.3** — touch `sample-app/src/main.rs` and time incremental rebuild: `touch src/main.rs && time cargo build 2>&1 | tail -5`. Log.
- [ ] **Step 0.4** — commit `bench-results.md` (first commit on this branch, no code changes).

### Task 1: Land gzip compression middleware

Already implemented on this branch (uncommitted). Review, test, bench, commit.

**Files (already present):**
- Created: `rustf/src/middleware/builtin/compression.rs`
- Modified: `rustf/src/app.rs` (`with_compression()` helper)
- Modified: `rustf/src/middleware/builtin/mod.rs` (re-export)
- Modified: `rustf/Cargo.toml` (`flate2 = "1.0"`)

- [ ] **Step 1.1** — read the full diff: `git diff HEAD`.
- [ ] **Step 1.2** — run `cargo test -p rustf --lib middleware::builtin::compression::tests`.
- [ ] **Step 1.3** — run `cargo clippy --workspace -- -D warnings` to confirm no regressions.
- [ ] **Step 1.4** — write `rustf/benches/compression.rs`: compress 1KB, 4KB, 64KB, 256KB HTML payloads; record throughput and compression ratio.
- [ ] **Step 1.5** — run new bench, log to `bench-results.md`.
- [ ] **Step 1.6** — USER APPROVAL: show the diff + proposed commit message. Wait for explicit "ok to commit".
- [ ] **Step 1.7** — on approval: `git add` the 4 existing files + the new bench, commit.

---

## Chunk 2: Runtime Perf Sweep (A)

### Task 2: Pass repository + session by reference into the renderer

**Problem:** [rustf/src/context.rs:272](rustf/src/context.rs#L272) converts the repository `HashMap<String, Value>` to a `serde_json::Value` via `to_value(&self.repository)` on every `ctx.view()`. Same for session at :278. Cloning the entire tree for every render.

**Fix:** Change the `ViewEngine::render_rich` signature to accept `&HashMap<String, Value>` and `Option<&Session>`. Renderer iterates without copying.

**Files:**
- Modify: `rustf/src/context.rs:268-310`
- Modify: `rustf/src/views/mod.rs` (trait signature)
- Modify: `rustf/src/views/totaljs/engine.rs`
- Modify: `rustf/src/views/totaljs/renderer.rs`
- Add: `rustf/benches/view_render.rs`

- [ ] **Step 2.1** — write `benches/view_render.rs`: 100-key repository with a small template, drive `ctx.view()` in a tight loop. Run baseline, log.
- [ ] **Step 2.2** — update `ViewEngine` trait in `views/mod.rs`: change `render_rich(&self, template: &str, repository: Value, session: Value, layout: &str)` → `render_rich(&self, template: &str, repository: &HashMap<String, Value>, session: Option<&Session>, layout: &str)`. Keep old signature as a deprecated default method if downstream crates depend on it.
- [ ] **Step 2.3** — update `TotalJsEngine::render_rich` impl to take the new refs.
- [ ] **Step 2.4** — update renderer (`totaljs/renderer.rs`) to borrow values from the repository HashMap rather than owning them. Watch for places that currently `.into_iter()` — switch to `.iter()`.
- [ ] **Step 2.5** — update `ctx.view()` in `context.rs` to pass `&self.repository` and the session Arc directly.
- [ ] **Step 2.6** — run `cargo test -p rustf` (especially `view_api_test.rs` and the `totaljs/tests/` suite).
- [ ] **Step 2.7** — re-run `view_render` bench, log delta.
- [ ] **Step 2.8** — USER APPROVAL → commit.

### Task 3: Static-file single-I/O

**Problem:** [rustf/src/app.rs:1195-1267](rustf/src/app.rs#L1195-L1267) — `tokio::fs::metadata()` then `tokio::fs::read()`. Two opens, two syscalls per served file.

**Fix:** `File::open()` → `handle.metadata().await?` → conditional short-circuit → `handle.read_to_end().await?`. Single open.

**Files:**
- Modify: `rustf/src/app.rs:1195-1267`
- Add: `rustf/tests/static_files_test.rs`
- Add or extend: `rustf/benches/static_files.rs`

- [ ] **Step 3.1** — write integration test `rustf/tests/static_files_test.rs` that creates a temp static file, serves it through `RustF`, asserts content + Last-Modified + ETag. Run; confirm PASS on current `main`.
- [ ] **Step 3.2** — write micro-bench `rustf/benches/static_files.rs` hitting the served path; run baseline.
- [ ] **Step 3.3** — refactor `serve_static_file` in `app.rs`: single `File::open`, then `.metadata()` on the handle, conditional-request check, then `read_to_end`.
- [ ] **Step 3.4** — run test from Step 3.1; must still PASS.
- [ ] **Step 3.5** — run bench; log delta.
- [ ] **Step 3.6** — USER APPROVAL → commit.

### Task 4: HTTP date cached with 1s granularity

**Problem:**
- Formatting: [rustf/src/app.rs:1326](rustf/src/app.rs#L1326) formats current date every request.
- Parsing: `parse_http_date()` at app.rs:1280-1312 tries three chrono formats sequentially on every `If-Modified-Since`.

**Fix:**
- Add `rustf/src/utils/http_date.rs` with a `current_http_date() -> Arc<str>` backed by `AtomicU64` (seconds) + `RwLock<Arc<str>>` refreshed when the second rolls. This is the standard hyper/actix approach.
- Reorder the parser to try the most common format first + cache per-request which format matched.

**Files:**
- Add: `rustf/src/utils/http_date.rs`
- Modify: `rustf/src/utils/mod.rs` (re-export)
- Modify: `rustf/src/app.rs:1280-1312, 1326`

- [ ] **Step 4.1** — create `utils/http_date.rs` with the cached formatter + a unit test (two calls within same second return the same `Arc<str>`; after sleeping past a second boundary, new value returned).
- [ ] **Step 4.2** — run the unit test; PASS.
- [ ] **Step 4.3** — replace `app.rs:1326` format call with `crate::utils::http_date::current()`.
- [ ] **Step 4.4** — reorder `parse_http_date`: RFC 1123 first (most common), RFC 850 second, asctime third.
- [ ] **Step 4.5** — run `cargo test -p rustf`; confirm static-files tests from Task 3 still pass.
- [ ] **Step 4.6** — re-run static-files bench from Task 3; log delta.
- [ ] **Step 4.7** — USER APPROVAL → commit.

### Task 5: Cookie parse cached on Request

**Problem:** [rustf/src/http/request.rs:498](rustf/src/http/request.rs#L498) — `parse_cookies(header)` runs inside `ctx.cookie(name)`. Session middleware, flash reads, CSRF middleware each call this — 3+ parses per typical request.

**Fix:** Add `cookies_cache: OnceCell<HashMap<String, String>>` to `Request`. Populate lazily on first `cookie()` call.

**Files:**
- Modify: `rustf/src/http/request.rs` (struct + `cookie()` method)
- Modify: `rustf/src/context.rs` if `cookie()` lives there instead (check both)

- [ ] **Step 5.1** — grep for `parse_cookies(` to enumerate callsites. Confirm single fix point.
- [ ] **Step 5.2** — add `cookies_cache: once_cell::unsync::OnceCell<HashMap<String, String>>` field to `Request`. Check once_cell is already a dep (likely yes — used elsewhere).
- [ ] **Step 5.3** — rewrite the cookie getter to `get_or_init` on the cache.
- [ ] **Step 5.4** — add a unit test asserting two calls to `request.cookie(...)` with distinct names both return values and that `parse_cookies` was only invoked once (use an instrumented test helper or reason by construction).
- [ ] **Step 5.5** — run `cargo test -p rustf`.
- [ ] **Step 5.6** — USER APPROVAL → commit.

---

## Chunk 3: End-User Dev-Speed (B)

### Task 6: Populate `sample-app/` with a runnable example

**Problem:** [sample-app/src/controllers/](sample-app/src/controllers/), `src/middleware/`, and `views/` are empty. A fresh clone of the repo has nothing runnable — `cargo run -p sample-app` boots but serves 404 everywhere. A developer learning the framework by reading code has only the CLI-emitted templates to go by, and those drift from reality (see `rustf_critical_gotchas.md`).

**Fix:** Drop in a minimal, working `home` controller with 2 routes, a timing middleware, and 2 views. All code copy-verified against real framework APIs (not the drifted docs).

**Files to create:**
- `sample-app/src/controllers/home.rs` — `GET /` → view; `GET /api/status` → JSON.
- `sample-app/src/middleware/timing.rs` — dual-phase example adding `X-Response-Time` header.
- `sample-app/views/layouts/default.html` — minimal layout.
- `sample-app/views/home/index.html` — landing page.
- `sample-app/views/home/about.html` — verifies layout + repository data flow.
- `sample-app/public/css/app.css` — tiny stylesheet so the page isn't bare.

- [ ] **Step 6.1** — write `sample-app/src/controllers/home.rs`. Handler signature exactly `async fn(ctx: &mut Context) -> rustf::Result<()>`. Both routes tested manually.
- [ ] **Step 6.2** — write `sample-app/src/middleware/timing.rs` with `#[async_trait]`, `InboundAction::Capture`, outbound adds `X-Response-Time`. Verify it compiles with `cargo build -p sample-app`.
- [ ] **Step 6.3** — write the two views + layout. Use `@{title}` and `@{R.*}` syntax. Verify renders via `cargo run -p sample-app` and curl: `curl -i http://127.0.0.1:8000/ | head -20` → 200 + `X-Response-Time` header present + HTML body contains expected content.
- [ ] **Step 6.4** — tiny `app.css` with ~20 lines (nav, card, success color). Not a designer's stylesheet — just "not naked".
- [ ] **Step 6.5** — runtime test: boot sample-app, hit `/`, `/about`, `/api/status`, and a 404. All correct. Record in `bench-results.md` under "sample-app manual test".
- [ ] **Step 6.6** — USER APPROVAL → commit.

### Task 7: `rustf-cli new crud <model>` generator

**Problem:** `rustf-cli new controller <name>` emits a bare skeleton. For a typical CRUD feature a developer then manually writes: 7 RESTful routes, matching handlers, a wrapper model that delegates to the generated base, 4 views (index/show/new/edit), an integration test. 30-45 minutes of repetitive boilerplate.

**Fix:** One command, one model name, full CRUD scaffold wired to existing conventions.

```bash
rustf-cli new crud posts
# Emits (enforces the Base Model → Model → Module → Controller layering):
#   src/controllers/posts.rs       (7 routes; handlers call modules::posts_service)
#   src/modules/posts_service.rs   (business logic; only file that touches Posts model)
#   src/models/posts.rs            (wrapper with include!() + pub fn register + find_by_*)
#   views/posts/index.html         (list)
#   views/posts/show.html          (detail)
#   views/posts/new.html           (create form)
#   views/posts/edit.html          (update form)
#   tests/posts_test.rs            (integration test for each route)
```
**Layering rule enforced in the templates:** the controller imports `crate::modules::posts_service` and does NOT import the model directly. All `Posts::query()`, `Posts::get_by_id()`, `Posts::create()` calls live in `posts_service.rs`.

The model's `base/posts.inc.rs` still comes from the YAML schema flow (`schemas/posts.yaml` → `rustf-cli schema generate models`), not from this command — this generator doesn't touch the DB, only scaffolds the HTTP layer.

**Files:**
- Create: `rustf-cli/templates/components/crud_controller.rs.template`
- Create: `rustf-cli/templates/components/crud_module.rs.template`  ← business logic layer
- Create: `rustf-cli/templates/components/crud_model.rs.template`
- Create: `rustf-cli/templates/components/crud_test.rs.template`
- Create: `rustf-cli/templates/views/crud/index.html.template`
- Create: `rustf-cli/templates/views/crud/show.html.template`
- Create: `rustf-cli/templates/views/crud/new.html.template`
- Create: `rustf-cli/templates/views/crud/edit.html.template`
- Modify: `rustf-cli/src/commands/new_cmd.rs` (add `Crud` variant to the `new` enum)
- Modify: `rustf-cli/src/commands/new_component.rs` (add `generate_crud(name: &str)` wiring)

- [ ] **Step 7.1** — read `new_component.rs` to understand the existing controller-generation path; match its style exactly.
- [ ] **Step 7.2** — write the 3 `.rs.template` files. Handler signatures verified against `routing/mod.rs:11-13`. Controller uses the flash-redirect pattern on create/update/delete success (per user's web-dev rule: no URL params for inter-view messages).
- [ ] **Step 7.3** — write the 4 `.html.template` files. Minimal markup, use `@{R.items}` / `@{R.item}` for repository access.
- [ ] **Step 7.4** — add `Crud { name }` to the CLI arg enum; wire to `generate_crud()`.
- [ ] **Step 7.5** — implement `generate_crud(name)`: pluralize the name for URL paths (simple `s` append for now; unusual plurals are a user problem), singularize for variable names where needed.
- [ ] **Step 7.6** — integration test: `cargo run -p rustf-cli -- new crud posts` in `/tmp/rustf-crud-test/` (with a minimal scaffolded project); verify all 7 files appear; `cargo build` in that project succeeds.
- [ ] **Step 7.7** — run the generated integration test: `cargo test -p <tmp-project>` (at minimum compiles; full pass needs DB, acceptable to stop at "compiles").
- [ ] **Step 7.8** — USER APPROVAL → commit.

---

## Chunk 4: Claude Skill (C)

### Task 8: RustF skill in this repo

**Goal:** A single `.claude/skills/rustf/SKILL.md` that fires when an agent opens this repo (or any RustF project) and overrides the README's drifted examples with the code-verified ones.

**File:**
- Create: `.claude/skills/rustf/SKILL.md`

**Frontmatter (exact — agents match on the `description`):**
```yaml
---
name: rustf
description: Use when writing or modifying Rust code in a RustF MVC framework project — enforces correct handler signatures, middleware dual-phase patterns, module registration, route macros, and template conventions. Trigger when editing any file under src/controllers/, src/middleware/, src/models/, src/modules/, src/workers/, src/events/ or when modifying Cargo.toml / config.toml in a project that has a rustf dependency.
---
```

**Content outline (target ≤500 lines, every example drawn from real code — cite file:line):**

1. **When to use** (1 paragraph).
2. **Canonical handler signature** with `rustf/src/routing/mod.rs:11-13` citation. Show one correct + three wrong forms the README+docs accidentally suggest.
3. **The `routes![]` macro** — verbatim syntax from `rustf/src/routing/mod.rs:60`; the XHR method variant.
4. **Middleware traits** — `InboundMiddleware` / `OutboundMiddleware` with `#[async_trait]`, `InboundAction::{Continue,Stop,Capture}` semantics, priority (lower = earlier).
5. **Layering discipline (CRITICAL — enforced by the skill, not by the compiler).**
   Mandatory direction: `Base Model → Model → Module → Controller`. Controllers are thin: routing, flash, redirects, JSON/view responses. All business logic and all model access happens through modules. A controller importing `crate::models::*` is a smell. If two controllers need the same query or validation, promote it to a module.
   - `src/controllers/<feature>.rs` — handlers + routes only. Imports `crate::modules::<feature>_service`.
   - `src/modules/<feature>_service.rs` — business logic, validation, cross-cutting rules. Uses `Models::query()` + external crates. DRY: one function, many callers.
   - `src/models/<name>.rs` — wrapper around generated base; holds model-specific query helpers, NOT business logic.
   - `src/models/base/<name>.inc.rs` — generated, never touch.
   Two module flavors (decision rule: *does the module need state?*):
   - **Plain module — DEFAULT.** Stateless `pub fn` on a unit struct (like `sample-app/src/modules/simple_util.rs`). Import and call directly, no registration. Use for validation, formatting, stateless business rules.
   - **Registered SharedModule service — ONLY if state is required.** For stateful/configurable components (email with SMTP config, payment providers with API keys, caches). `impl SharedModule`, registered via `MODULE::register("name", Service::new(...))` in `main.rs`. Multiple named instances allowed (`email-primary`, `email-backup`). See `sample-app/src/modules/email_service.rs`.
   If in doubt, start stateless. Promote to SharedModule only when you reach for instance fields.
6. **Context API — top 30 methods** grouped: request-read, response-write, session, flash, repository (view data) vs ctx.set/get (middleware data), throw4xx/5xx.
7. **Models — base/wrapper pattern.** Show a working wrapper from sample-app, the `include!()` indirection, the mandatory `pub fn register(registry: &mut rustf::models::ModelRegistry)`. Emphasize: *model-specific* helpers go here (e.g. `find_by_email`), *business logic* does NOT.
8. **Workers** — `pub async fn install()`, `WORKER::register("kebab-name", |ctx| async { ... })`, kebab-case rule.
9. **Template names have no extension** — short + bold.
10. **Built-in middleware imports** — `use rustf::middleware::builtin::*;` (not in prelude).
11. **Config layering** — `config.toml` + `config.dev.toml` auto-merged.
12. **Common mistakes checklist** — 15 items, one line each. Includes "controller calls Model::query directly" as a blocker item.
13. **Authoritative source files** — pointer list (rustf/src/routing/mod.rs, middleware/traits.rs, context.rs, app.rs, macros/lib.rs, sample-app/src/*).

- [ ] **Step 8.1** — create `.claude/skills/rustf/` directory.
- [ ] **Step 8.2** — write SKILL.md per outline. Every code block must be copy-pastable from real code (verified by re-reading the cited line).
- [ ] **Step 8.3** — manual QA: open the sample-app in a new Claude Code session in a throwaway dir, confirm the skill surfaces when asked to "add a new controller". Use the user's own Claude Code session as the test harness; report result.
- [ ] **Step 8.4** — USER REVIEW of the full SKILL.md.
- [ ] **Step 8.5** — commit.

### Task 9: Bundle the skill into the CLI project template

Every project scaffolded with `rustf-cli new project` should start with the skill.

**Files:**
- Create: `rustf-cli/templates/project/.claude/skills/rustf/SKILL.md` (copy of the one above)
- Modify: `rustf-cli/src/commands/new.rs` or the template emit code — ensure `.claude/` directory + nested files are included in the output tree

- [ ] **Step 9.1** — `cp .claude/skills/rustf/SKILL.md rustf-cli/templates/project/.claude/skills/rustf/SKILL.md`.
- [ ] **Step 9.2** — find how non-root template files are enumerated (likely a `walk_templates` or embedded-resources list). Add the new `.claude/` path.
- [ ] **Step 9.3** — test: `cargo run -p rustf-cli -- new project /tmp/rustf-test-skill --database none`; verify `/tmp/rustf-test-skill/.claude/skills/rustf/SKILL.md` exists and matches.
- [ ] **Step 9.4** — USER APPROVAL → commit.

---

## Chunk 5: End-to-end verification + wrap-up

### Task 10: Request lifecycle bench

**File:** `rustf/benches/request_lifecycle.rs` (new).

Measures the full path: Request (mock) → Context::new → middleware chain (incl. compression enabled) → JSON handler → Response serialization. This is the single bench that validates the whole sweep.

- [ ] **Step 10.1** — write the bench. Use a minimal `RustF::new().with_compression().controllers(...)` app, construct a `Request` manually, call the private dispatch path (may need a `#[cfg(feature = "bench")]` entry point — add one if needed).
- [ ] **Step 10.2** — run against the tip of `main` (before this branch): `git stash && git checkout main && cargo bench --bench request_lifecycle`. Log.
- [ ] **Step 10.3** — switch back, run on current branch tip: log delta.
- [ ] **Step 10.4** — write up results in `bench-results.md` — real numbers, no rounding up.
- [ ] **Step 10.5** — USER APPROVAL → commit.

### Task 11: Wrap-up

- [ ] **Step 11.1** — update `WORK_PLAN.md`: new session entry "Session 2026-04-24 — Perf Sweep + Claude Skill" with actual measured deltas (no predictions, no hype).
- [ ] **Step 11.2** — append summary to `.wolf/memory.md` per OpenWolf protocol.
- [ ] **Step 11.3** — propose PR title + body targeting `main`. USER APPROVAL.
- [ ] **Step 11.4** — on approval, push branch and open PR via `gh pr create`.

---

## Review checkpoints

Per user rule, **every commit on this branch requires user diff review**. The plan above encodes that as explicit "USER APPROVAL" steps before each commit.

If any task reveals that a proposed fix either (a) doesn't measurably improve things, or (b) breaks an invariant, STOP and flag it — don't commit "just because the plan said so". Update this plan file first.

## Rollback strategy

Each commit is independent. If any step causes regressions detected downstream, `git revert` that specific commit; the rest of the sweep stays.
