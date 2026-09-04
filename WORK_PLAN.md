# Work Plan - RustF Framework Development

**Last Updated**: 2026-09-04
**Current Sprint**: Release Candidate 1 Polishing
**Branch**: `perf-sweep-and-skill` (pending merge to `main`)

---

## Pending (2026-09-04)

- [ ] **Streaming for 1.0 — reviewed, uncommitted.** Branch `feat/streaming-body`. Three Codex passes; the third returned **ship** with zero findings. Report: `docs/reviews/2026-09-04-codex-streaming-review.md`. Awaiting Morle's diff review before commit.
- [ ] **Context API cleanup — implemented, uncommitted.** Plan and outcome: `docs/plans/2026-09-04-context-api-cleanup-plan.md`. Removed the 16 type-first aliases, untyped `param()`/`query()`, `ctx.text()`, `ctx.cancel()`, `ctx.csrf()`, `body_form_data_cloned()`, `RequestData`, and 11 dead `Response::` constructors; added the `*_as::<T>` family; made `FormData` multi-value; replaced `body_form_typed`'s JSON bridge with a real form deserializer. 694 tests green, scaffold smoke test passes. Awaiting Morle's diff review.
- [ ] **TODO — WebSocket support (Total.js `SOCKET` model).** Plan: `docs/plans/2026-09-04-websocket-plan.md`. Additive; target 1.1.0 unless 1.0 must carry it. Open decisions: 1.0 vs 1.1, room layer in v1, per-route flag syntax.
- [ ] **TODO — resurrect or delete the dead test directories.** `rustf/tests/{integration,security,performance,unit}/` hold nine `.rs` files with no `main.rs`, so cargo has never compiled them. They still use the pre-rc `async fn(mut ctx: Context) -> Result<Response>` signature. Either port them to the current API and wire them into the harness, or delete them — right now they read as coverage that does not exist.
- [ ] **TODO — `rustf-cli new project`'s `schemas/sessions.yaml` does not parse.** `schema generate models` fails on the scaffold's own shipped schema: `unknown field 'description'`. Found during the 2026-09-04 scaffold smoke test. Either add `description` to the field schema or drop it from the template.

---

## Completed Tasks ✅

### Session 2026-04-24 (Perf Sweep + Claude Skill)

Branch `perf-sweep-and-skill`. 11 commits. Plan document: `PERF_AND_SKILL_PLAN.md`. Measurement log: `bench-results.md`. Every commit reviewed by user before landing.

**Runtime perf wins (Chunk 2):**
- [x] **Ship gzip compression middleware** — closes the #1 item in `Page_Load_Perf_Analysis.md`. Opt-in via `RustF::with_compression()`. 9 KB HTML → 643 B (93 % shrink) at default level. Commit `a911f68`.
- [x] **Hand-rolled HTTP date formatter, DRY + 2.5× faster** — collapsed three duplicates (app.rs, cache/response.rs, security/static_files.rs) into `utils::http_date`. `cache/response.rs` was a literal `format!("{:?}", ...)` placeholder, so this also fixes a latent bug in cached-response `Last-Modified`. 287 ns → 114 ns per format. Commit `4ccf0ee`.
- [x] **Cached cookie parse on Request** — once_cell-backed HashMap; session + flash + CSRF middleware now share one parse instead of three. Commit `61d1741`.
- [x] **Deferred Task 2** (repository/session pass-by-ref into renderer) with measured rationale. Direct `Map::from_iter` only recovers 13 % of the clone cost; the other 87 % is inside the engine's `RenderContext` which would need an Arc/lifetime refactor unsuitable for RC1. `benches/view_render.rs` left in-tree as measurement infrastructure for the future cleanup. Commit `383b119`.
- [x] **Verified Task 3 already done**. `app.rs:try_read_static_file` already opens the file once and calls `.metadata()` on the fd before reading. No change needed.

**Framework additions (Chunk 3):**
- [x] **`MethodOverrideMiddleware`** — upgrades POST + `_method=PUT|PATCH|DELETE` to the real method so HTML forms from the CRUD scaffolder work out of the box (Rails/Express convention). Opt-in via `RustF::with_method_override()`. 7 unit tests. Commit `ded4a5f`.

**Bug fixes:**
- [x] **Guard MemoryStorage cleanup against missing Tokio runtime** — `storage.rs:124` unconditionally called `tokio::spawn`, which panicked in benches and any sync constructor path. Defensive `Handle::try_current` guard. Unblocked `benches/context.rs` and `benches/session.rs`. Commit `be19fc5`.

**Developer experience (Chunk 3):**
- [x] **Populated sample-app with a runnable example**. `home` controller (3 routes), `home_service` module (business logic demonstrating the layering rule), `timing` dual-phase middleware (`X-Response-Time` header), layout + 2 views + minimal CSS. Before: `sample-app/src/controllers/` was empty — a fresh clone had nothing runnable. Commit `15ad894`.
- [x] **`rustf-cli new crud <name>` scaffolder** — emits 8 files (controller + module + model stub + 4 views + test) all wired to the `Base Model → Model → Module → Controller` layering rule. Controller never imports `crate::models::*`. In-memory Vec model stub so the scaffold works out of the box; file-top comment explains swap-in of schema-generated base. Smoke-tested end-to-end: create → show → update (POST + `_method=PUT`) → delete. Commit `fc6ab58`.

**AI collaboration (Chunk 4):**
- [x] **`rustf` Claude Code skill** — 410 lines, every claim cited at file:line. Covers: layering rule (headline), handler signature (overrides drifted README), `routes!` macro with `{id}` params, middleware traits + `#[async_trait]`, Context API cheat sheet, base/wrapper model pattern, kebab-case worker names, template-without-extension, built-in-middleware imports, config merging, auto-discovery. Twelve common mistakes checklist. Shipped both in this repo (`.claude/skills/rustf/SKILL.md`) and bundled into the CLI project template so every new project gets it. Commit `38850a5`.

**Verification (Chunk 5):**
- [x] **End-to-end `request_lifecycle` bench** — full `hyper::Request<Body>` → `handle_request` → `Response`. Bare JSON: 1.39 µs. With compression middleware configured: 1.55 µs (160 ns skip-path overhead — cheap). Commit `242339e`.
- [x] **Documentation**: `PERF_AND_SKILL_PLAN.md` (plan), `bench-results.md` (baselines + per-task deltas).

**Numbers (dev machine):**
- All 414 existing lib tests + 7 new method-override tests + 8 new http_date tests + 1 new cookie-cache test pass.
- HTTP date formatting: 2.5× faster.
- End-to-end request: ~1.4 µs bare.
- Compression: 93 % shrink on 9 KB+ HTML.

### Session 2025-12-10 (Release Readiness)

- [x] **Fix Initial Configuration Test**
  - Fixed failing `test_conf_initialization_and_access` in `rustf/src/configuration.rs`
  - Refactored test to use `toml::from_str` for proper section flattening and cleaner initialization
- [x] **Fix Sample App Compilation & Tests**
  - Addressed 200+ compilation warnings in `sample-app`
  - Fixed missing dependencies (`toml`) in `Cargo.toml`
  - Fixed missing imports in `email_service.rs` tests
  - Suppressed dead code warnings in generated model code
- [x] **Documentation**
  - Created `CHANGELOG.md` for v1.0.0-rc1
  - Updated `WORK_PLAN.md` status
- [x] **Fix Deployment Scripts**
  - Removed invalid `multilingual = false` from `book/book.toml` to fix GitHub deployment job

### Session 2025-11-05 (Log Cleanup)

- [x] **Database Layer Log Cleanup** (~1.5 hours)
  - Removed 44 debug logs from database adapters and type converters
  - MySQL/PostgreSQL/SQLite adapters: SQL query and parameter logging (18 logs)
  - Type converters: column extraction and conversion logging (26 logs)
  - Eliminated ~11 debug logs per database query
  - Kept all warning and error logs for production diagnostics
  - Built and tested successfully
  - **Commit**: `f347595`

- [x] **View Template System Log Cleanup** (~30 min)
  - Removed 2 debug logs from Total.js engine
  - Layout application logging (2 logs per page render)
  - Reduced log noise for template rendering
  - Built and tested successfully
  - **Commit**: `f347595` (combined with database layer)

### Session 2025-10-30 (Module System Refactoring)

- [x] **Refactor module system for explicit developer control** (~2 hours)
  - Problem: Framework was forcing SharedModule trait on ALL modules
  - Solution: Implemented named module registration with developer control
  - Created ModuleRegistry with DashMap for thread-safe access
  - Changed auto_modules!() macro to declaration-only
  - Removed framework's automatic MODULE::init() call
  - Kept MODULE::shutdown_all() for graceful shutdown
  - Updated sample-app with comprehensive test examples
  - **Commit**: `3d7e413`

- [x] **Simplify CLI module generation** (30 min)
  - Removed --module-type flag (overcomplicated)
  - Added --shared boolean flag (simpler intent)
  - Default: generates utility modules (no flag needed)
  - With --shared: generates SharedModule services
  - Updated CLI dispatcher and generation functions
  - **Commit**: `3d7e413`

- [x] **Add framework prelude to utility module template** (20 min)
  - User requirement: utility modules should include framework prelude
  - Added `use rustf::prelude::*;` to utility module section
  - Rebuilt CLI tool to embed updated template
  - Tested both utility and service module generation
  - Verified sample-app compiles with all module types
  - **Commit**: `3d7e413`

### Session 2025-10-29 (Previous Session - 3 hours)

- [x] **Fix broken middleware template**
  - Replaced single-phase with dual-phase architecture
  - Added required `#[async_trait]` macro
  - Fixed Context API usage
  - **Commit**: `9cab76e`

- [x] **Improve middleware template to showcase dual-phase pattern**
  - Shows realistic pattern with inbound and outbound phases
  - Added `#[derive(Clone)]` for dual-phase
  - **Commit**: `2f9018c`

- [x] **Simplify environment configuration**
  - Reduced from 4 to 2 environments
  - Made CLI project-folder-centric
  - **Commit**: `9cab76e`

- [x] **Add worker generation support**
  - Implemented `rustf-cli new worker` command
  - **Commit**: `2d9f45d`

- [x] **Simplify worker template**
  - Removed predefined types
  - Reduced from 280 to 40 lines
  - **Commit**: `ed34a0f`

---

## Current Sprint Status

### ✅ COMPLETED: Framework Log Cleanup (Phase 1)
**Goal**: Remove unnecessary debug logs causing performance overhead and log verbosity  
**Status**: ✅ PHASE 1 COMPLETE - Database and view layers cleaned  
**Duration**: 1 session (current)

**Key Achievements**:
- [x] Database layer: Removed 44 debug logs (adapters + type converters)
- [x] View template system: Removed 2 debug logs
- [x] Total: 46 logs removed, 151 lines deleted
- [x] Built and tested successfully (zero compilation errors)
- [x] User reviewed and approved all changes before commit
- [x] Commit created with comprehensive documentation

**Performance Impact**:
- Database operations: Eliminated ~11 logs per query
- Template rendering: Eliminated 2 logs per page render
- Reduced string formatting overhead
- Cleaner logs when running with `RUST_LOG=info`

**Remaining Components** (Optional for future sessions):
- Configuration & Startup: 7 logs
- Events System: 2 logs
- Query Builder: ~10 logs
- Miscellaneous: ~12 logs

---

## Next Sprint Options

### Option A: Continue Log Cleanup (Estimate: 1-2 hours)

**Component 3: Configuration & Startup** (7 logs)
- [ ] Review `rustf/src/config.rs` - 2 logs
- [ ] Review `rustf/src/configuration.rs` - 2 logs
- [ ] Review `rustf/src/app.rs` - 3 logs

**Component 4: Events System** (2 logs)
- [ ] Review `rustf/src/events.rs` - 2 logs

**Component 5: Query Builder** (~10 logs)
- [ ] Review `rustf/src/models/query_builder.rs`
- [ ] Review dialect-specific files

**Component 6: Miscellaneous** (~12 logs)
- [ ] Review session, routing, schema, workers

### Option B: Module System Documentation (Estimate: 1-2 hours)

1. **Update Documentation for Module System**
   - [ ] Create `docs/MODULE_REGISTRATION_GUIDE.md` with new pattern
   - [ ] Update `docs/ABOUT_MODULES.md` 
   - [ ] Create migration guide for projects using old system
   - [ ] Add examples to README.md

2. **Create Example Application**
   - [ ] Generate new project with CLI
   - [ ] Add multiple controllers (auth, dashboard, API)
   - [ ] Generate and configure workers (email, cleanup)
   - [ ] Generate and configure middleware (auth, logging)
   - [ ] Test auto-discovery with new module system
   - [ ] Document any integration issues

### Option C: Testing & Integration (Estimate: 2-3 hours)

1. **Test Module System Integration**
   - [ ] Verify MODULE::shutdown_all() works correctly
   - [ ] Test with multiple module instances
   - [ ] Test error handling for duplicate registration
   - [ ] Test with async module initialization
   - [ ] Performance test with many modules

2. **Verify Auto-Discovery**
   - [ ] Test auto_modules!() declaration discovery
   - [ ] Test auto_controllers!() with new module pattern
   - [ ] Test auto_middleware!() integration
   - [ ] Verify no compilation errors

3. **Test Log Cleanup Impact**
   - [ ] Run sample-app and verify reduced log verbosity
   - [ ] Benchmark database query performance
   - [ ] Compare before/after log output

---

## High Priority Tasks (Across All Sprints)

### Documentation & Examples
1. **Module System Documentation** (from previous sprint)
   - [ ] Create comprehensive registration guide
   - [ ] Document breaking changes and migration path
   - [ ] Add code examples for common patterns

2. **Log Configuration Guide** (new)
   - [ ] Document when to use debug vs info vs warn
   - [ ] Explain performance impact of logging levels
   - [ ] Provide best practices for framework users

### Code Quality
3. **Add More CLI Generators** (Estimate: 3-4 hours)
   - [ ] `rustf-cli new model --name User --from-schema schemas/user.yaml`
   - [ ] `rustf-cli new view --name home/index --layout default`
   - [ ] `rustf-cli new definition --name custom`

4. **Improve Template Comments** (Estimate: 1 hour)
   - [ ] Review all templates for clarity
   - [ ] Ensure comments explain WHY, not just WHAT
   - [ ] Add cross-references to documentation

---

## Backlog / Future Ideas

### Features Under Consideration
- Database migration generator
- API documentation generator (OpenAPI)
- GraphQL schema generator
- WebSocket handler generator
- Admin panel generator
- Authentication scaffolding
- Deployment configuration generator

### Performance Optimization
- Benchmark template caching after log removal
- Profile database operations with reduced logging
- Test memory usage improvements
- Consider trace logging for deep debugging scenarios
- **Threshold-gated render offload (multicore tail latency).** `ctx.view` /
  the Total.js engine render is synchronous CPU work with no `.await`, so it
  occupies its tokio worker thread for the full render duration. Fine on the
  ~99.5% template-cache-hit fast path (µs-scale), but a very large template or
  data payload can pin a worker and raise tail latency for co-scheduled
  requests. Fix: offload the render via `spawn_blocking` **only above a size
  threshold** (e.g. rendered output / data > ~256 KB) so the cheap fast path
  stays inline and avoids the spawn/clone overhead. Do NOT blanket-wrap — it
  would tax the common case. Behavior is documented on `Context::view`.
  Low priority (not a defect, a tuning trade-off). Discovered 2026-07-01
  alongside the file-response blocking-I/O fix (commit `661bcf4`), which was
  the higher-impact sibling issue.

### Research Needed
- Async module initialization patterns
- Module dependency injection
- Hot-reload for modules
- Module versioning strategy
- Thread pool configuration for workers

---

## Blockers & Issues

**None** - Session completed successfully with all changes committed.

---

## Time Tracking

### Session 2025-11-05 (Current - Log Cleanup)
- Initial log analysis and planning: 30 min
- Database adapter cleanup (3 files): 20 min
- Database type converter cleanup (3 files): 40 min
- View template cleanup (1 file): 10 min
- Build and test verification: 10 min
- Git commit and documentation: 20 min
- **Total**: ~2.5 hours

### Session 2025-10-30 (Module System)
- Module system refactoring: 2 hours
- CLI module generation simplification: 30 min
- Framework prelude in templates: 20 min
- Testing and verification: 30 min
- Session documentation: 20 min
- **Total**: ~3.5 hours

### Session 2025-10-29 (Templates)
- Environment configuration: 30 min
- Middleware template fixes: 1 hour
- Middleware template improvements: 45 min
- Worker generation: 45 min
- Worker simplification: 30 min
- **Total**: ~3 hours

### Cumulative Work This Sprint
- **Log Cleanup (Phase 1)**: ~2.5 hours
- **Estimated Remaining (Phase 2)**: ~1-2 hours (if continuing log cleanup)

---

## Success Metrics

### Code Quality ✅
- ✅ All changes compile successfully
- ✅ No functional changes, only log removal
- ✅ All error handling preserved (warn/error logs intact)
- ✅ Framework conventions followed
- ✅ Type safety maintained

### Performance ✅
- ✅ Database operations: ~11 logs per query eliminated
- ✅ Template rendering: 2 logs per render eliminated
- ✅ String formatting overhead removed
- ✅ Log verbosity significantly reduced

### Developer Experience ✅
- ✅ User review before commit (followed critical requirement)
- ✅ Component-by-component approach (systematic)
- ✅ Clear commit message with detailed changes
- ✅ Session documentation comprehensive

### Framework Philosophy ✅
- ✅ Performance optimization focus
- ✅ End-user value prioritized
- ✅ Production diagnostics preserved
- ✅ Implementation details removed

---

## Key Technical Decisions (This Session)

### 1. Component-by-Component Approach
- **Pattern**: Review logs systematically by framework component
- **Why**: Manageable review chunks, clear categorization
- **Impact**: Easier user approval, better tracking

### 2. Keep All Warning/Error Logs
- **Pattern**: Only remove debug/trace, keep warn/error
- **Why**: Production diagnostics are critical
- **Examples Kept**: "Could not extract array type", "Failed to extract value"

### 3. User Review Before Commit (Critical)
- **Pattern**: Present changes → user approval → commit
- **Why**: User explicitly required: "I must review your changes before you commit"
- **Previous Issue**: Earlier session had commits without review

### 4. Single Commit for Related Components
- **Pattern**: Database + Views in one commit
- **Why**: Both are framework performance optimization
- **Commit**: `f347595` - 8 files, 46 logs, 151 lines deleted

---

## Notes for Future Sessions

### Log Cleanup Methodology
- Focus on framework performance, not sample-app
- Debug logs = implementation details (remove)
- Warn/Error logs = production diagnostics (keep)
- Build and test after each component
- User approval before any commits

### Key User Preferences
- Challenge approaches when necessary ✅
- Always test before affirming work is done ✅
- Think from end-user perspective ✅
- Must review changes before commit ✅ (CRITICAL - followed)
- Be realistic in comments ✅
- Work interactively ✅

### Testing Verification
- Cargo build successful (only unrelated warnings)
- No compilation errors introduced
- All error handling preserved
- Git diff shows only log line deletions

### Documentation That Needs Updating
- Consider adding logging guide for framework users
- Document performance impact of logging levels
- Best practices for when to use each log level

### Next Session Recommendations
1. **Option A**: Continue log cleanup (remaining components)
2. **Option B**: Test log cleanup impact with sample-app
3. **Option C**: Focus on module system documentation
4. **Option D**: Add more CLI generators

---

**Session Status**: ✅ Complete - All changes committed, documentation saved  
**Ready for**: Next development session (user choice of direction)  
**Recommended Next Task**: Continue log cleanup OR test impact with sample-app
