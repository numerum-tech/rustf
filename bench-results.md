# Bench Results — Perf Sweep + Skill Branch

Baseline captured at the start of branch `perf-sweep-and-skill`, before any perf-related code changes.

**Machine:** macOS darwin 25.4.0, user-local dev box. Numbers will not match other machines; what matters is the **delta** between before/after on the same machine during this sweep.

**Date:** 2026-04-24
**Commit at baseline:** `a2de554` (main) + in-flight compression changes (uncommitted) + `storage.rs` tokio-runtime guard (new in this branch, required to unpanic the `context` + `session` benches)

---

## Why a `storage.rs` fix was needed before baselines

The `context` and `session` benches previously panicked at `session/storage.rs:124` (`tokio::spawn` called outside a Tokio runtime during `MemoryStorage::new()`). This is a pre-existing bug in the benches — criterion runs `main` synchronously and benches that construct a `Context` indirectly instantiate `MemoryStorage`, which calls `tokio::spawn` unconditionally.

Fix applied on this branch:

```rust
fn start_cleanup_task(&self) {
    if tokio::runtime::Handle::try_current().is_err() {
        log::debug!("no Tokio runtime, skipping background cleanup");
        return;
    }
    // ... unchanged ...
}
```

Net effect at runtime: identical for any RustF app started via `app.start().await` (which always has a runtime). In benches, CLI config init, or any sync constructor path, no-op instead of panic. Matches the hyper/actix pattern.

---

## Criterion benches — baseline

Grouped by bench file. Numbers are `[lower_bound median upper_bound]` from criterion's 100-sample 5s collection window, on the dev machine.

### `benches/routing.rs`

| Name | Time |
|---|---|
| `method_routing_get` | `[113.85 ns  114.39 ns  114.97 ns]` |
| `method_routing_post` | `[109.00 ns  109.20 ns  109.43 ns]` |
| `method_routing_invalid` | `[110.48 ns  110.62 ns  110.79 ns]` |

(The full routing suite has additional benches — static/dynamic/wildcard/large-router — not re-logged here. Criterion's saved per-bench history in `target/criterion/` holds them; this summary captures the indicative hot-path numbers.)

### `benches/configuration.rs`

| Name | Time |
|---|---|
| `CONF::get_int` | `[54.679 ns  54.869 ns  55.147 ns]` |
| `CONF::get_string` | `[72.381 ns  72.670 ns  72.995 ns]` |
| `CONF::get_bool` | `[69.193 ns  69.443 ns  69.736 ns]` |
| `CONF::get_or` | `[56.844 ns  57.007 ns  57.181 ns]` |
| `CONF::get_or_missing` | `[48.495 ns  48.684 ns  48.868 ns]` |
| `CONF::has` | `[57.882 ns  58.351 ns  58.923 ns]` |
| `CONF::get_string_deep` | `[63.544 ns  63.790 ns  64.069 ns]` |
| `CONF::is_production` | `[65.371 ns  65.583 ns  65.831 ns]` |
| `Arc<Config> field access` | `[267.82 ps  270.77 ps  275.17 ps]` |
| `Direct struct access` | `[266.01 ps  266.51 ps  267.17 ps]` |

### `benches/context.rs`  *(previously panicking — now runs after the storage.rs guard)*

| Name | Time |
|---|---|
| `context_creation` | `[135.09 ns  135.92 ns  136.94 ns]` |
| `context_url` | `[266.16 ps  267.00 ps  267.96 ps]` |
| `context_ip` | `[33.547 ns  33.695 ns  33.906 ns]` |
| `context_is_xhr` | `[15.969 ns  16.118 ns  16.341 ns]` |
| `context_with_session` | `[501.44 ns  503.10 ns  504.94 ns]` |

### `benches/middleware.rs`

| Name | Time |
|---|---|
| *(mixed groups — inbound / outbound / dual-phase at 1/5/10 middleware)* | |
| `dual_phase_middleware` | `[509.41 ns  510.53 ns  511.78 ns]` |
| representative inbound (5 mw) | `[506.69 ns  507.51 ns  508.58 ns]` |
| representative inbound (10 mw) | `[1.9968 µs  2.0038 µs  2.0143 µs]` |
| representative priority-sort | `[128.25 ns  128.37 ns  128.52 ns]` |

### `benches/pool.rs`

| Name | Time |
|---|---|
| `request_pool_get_return` | `[93.501 ns  93.696 ns  93.951 ns]` |
| `request_direct_new` | `[43.890 ns  44.060 ns  44.331 ns]` |
| `response_direct_new` | `[17.250 ns  17.258 ns  17.265 ns]` |
| `pool_stats_collection` | `[16.519 ns  16.554 ns  16.594 ns]` |
| `concurrent_pool_access` | `[40.753 µs  41.021 µs  41.555 µs]` |

(Pooling is ~2× slower than direct alloc — consistent with `CLAUDE.md`'s statement "direct Request allocation preferred over pooling".)

### `benches/session.rs`  *(previously panicking — now runs after the storage.rs guard)*

| Name | Time |
|---|---|
| `session_create` | `[337.71 ns  338.83 ns  340.03 ns]` |
| `session_set_get` | `[79.473 ns  79.684 ns  79.919 ns]` |
| `session_remove` | `[67.393 ns  67.637 ns  67.973 ns]` |
| `flash_set_get` | `[60.042 ns  60.181 ns  60.395 ns]` |
| `flash_get_all` | `[2.0274 µs  2.0282 µs  2.0292 µs]` |
| `session_json_set_get` | `[1.0462 µs  1.0480 µs  1.0500 µs]` |
| `session_touch` | `[25.792 ns  25.825 ns  25.855 ns]` |

### `benches/minifier.rs`

Numbers visible in the raw log (~21 µs for medium input, ~131 µs for large). Not replicated here — minifier isn't on the list of paths this sweep targets.

---

## Sample-app build timings

| Scenario | Wall time |
|---|---|
| Warm build (rustf already compiled, sample-app rebuild): `cargo build` | `11.83 s` |
| Incremental rebuild after `touch sample-app/src/main.rs`: `cargo build` | `1.51 s` |

(Not a true cold build — rustf's artifacts stayed in `rustf/target/` from prior runs. Adequate for regression tracking on this machine; we only care about deltas introduced by changes in this sweep.)

---

## Criterion's in-tree history

`rustf/target/criterion/` retains per-bench JSON history across runs. Criterion's own "change: [+/-x%]" messages compare current to the previously-stored run, not to the freshly-captured baseline above. For manual inspection, `cargo bench --bench <name> -- --save-baseline pre-sweep` would snapshot the current numbers under the `pre-sweep` label; we don't run this here because the raw numbers in this file are sufficient for our review flow.

---

## Tasks this baseline will be compared against

- Task 1: add compression middleware (new bench `compression.rs` — numbers in the `Task 1 — gzip compression` section below).
- Task 2: repository + session pass-by-ref into renderer (expected to help `context_with_session` and any view-render micro-bench we add).
- Task 3: static-file single-I/O (adds a new `static_files.rs` bench).
- Task 4: cached HTTP date (measured in the static-file bench).
- Task 5: cached cookie parse (measured via a new tight-loop unit).
- Task 10: end-to-end `request_lifecycle.rs` — the integration bench that shows the sum of all wins.

Deltas will be appended to this file as each task lands.

---

## Task 2 — repository/session pass-by-ref: DEFERRED

Numbers from `benches/view_render.rs` (added to the tree in the same commit as this note).

### Repository build cost before renderer is called

| Repo size | `serde_json::to_value` (current) | Direct `Map::from_iter` | Gain from direct |
|---|---|---|---|
| 0 keys | 15 ns | 10 ns | (noise) |
| 10 keys | 1.90 µs | 1.67 µs | 0.23 µs (12 %) |
| 50 keys | 12.3 µs | 10.8 µs | 1.5 µs (12 %) |
| 100 keys | 26.1 µs | 22.6 µs | 3.5 µs (13 %) |

### Session clone+stamp cost

`session_clone_and_stamp`: 558 ns (negligible).

### Why deferred

Swapping `serde_json::to_value` for a direct `Value::Object(Map::from_iter(...))` only recovers ~13 % of the clone cost. The remaining ~87 % is the second clone inside the Total.js engine at `views/totaljs/engine.rs:349` (`context.with_repository(ctx_repo.clone())`) — removing that would require re-plumbing `RenderContext` to hold `Arc<Value>` or references with lifetimes. That's a medium-effort engine-internal refactor, not RC1-friendly.

Real-world repositories are typically 5-15 keys where the current path costs ~1-3 µs per render — inside request-handling noise. The bench stays in-tree as measurement infrastructure so any post-RC1 view-render optimization has instant before/after numbers.

### Tasks already done in the code (verified during exploration, NOT in this sweep's commits)

- **Static-file single-I/O (was Task 3):** `app.rs:try_read_static_file` already opens the file once and calls `.metadata()` on the open fd before optionally reading. Has an explanatory comment. Skipping this task.
- **`parse_http_date` format ordering:** already narrowed to two chrono formats (RFC 7231 fast path + RFC 850 fallback), no triple-parse anymore. The `format_http_date` per-request string allocation remains — addressed in Task 4.

---

## Task 5 — cached cookie parse on Request

`request.cookie(name)` previously re-parsed the `Cookie` header on every call. Session, flash, and CSRF middleware each call it for different names — 3+ parses per typical request. Added `once_cell::sync::OnceCell<HashMap>` on `Request` populated lazily on first access.

No before/after bench — the per-request parse cost is small (typical cookie header: ~50-200 bytes, ~3-10 name/value pairs, sub-µs parse). The correctness-critical test is that the *same* HashMap is returned across repeat calls, proving the cache is effective; added as `test_cookies_cache_returns_stable_reference`. The saving is one HashMap allocation + N tokenisations per request where N is the number of cookie readers on the hot path (today: session middleware + any flash/CSRF lookup).

Also exposed `request.cookies() -> &HashMap<String, String>` publicly so callers that need several cookies (authentication code, analytics) can look them up without going through `cookie(name)` N times.

---

## Task 4 — cached / hand-rolled HTTP date formatter

Numbers from `benches/http_date.rs` (new). Each iteration formats three representative timestamps (epoch, ~2023, future).

| Implementation | Time / iteration (3 timestamps) | Per-format |
|---|---|---|
| `dt.format("%a...").to_string()` (chrono strftime) | `[858.57 ns  860.32 ns  862.43 ns]` | ~287 ns |
| Hand-rolled (new) | `[342.54 ns  343.63 ns  344.58 ns]` | ~114 ns |

**~2.5× faster** on every static-file response and every cached-response Last-Modified header.

The old code also had a concrete bug: `cache/response.rs` previously called `format!("{:?}", datetime)` (marked `// Placeholder - use proper HTTP date formatting`), which emitted Rust-Debug-formatted `SystemTime` instead of a real HTTP date — any client honouring `Last-Modified` would have failed validation on cached responses.

Three duplicate implementations in `app.rs`, `cache/response.rs`, and `security/static_files.rs` collapsed into one shared `utils::http_date::{format_http_date, parse_http_date}` with unit tests covering RFC 7231 shape, epoch, fixed 29-byte output, RFC 7231/RFC 850/asctime parsing, and round-trip.

Net: **101 lines removed, 21 lines added** (plus the shared module + bench).

---

## Task 1 — gzip compression middleware

Numbers from `benches/compression.rs` on the dev machine. Measures raw `flate2::GzEncoder` throughput — the middleware's async wrapper adds negligible overhead on top.

### Throughput (`gzip_default` — level 6)

| Payload | Size | Time | Throughput |
|---|---|---|---|
| small-json | 276 B | `[8.81 µs  8.88 µs  8.94 µs]` | ~31 MB/s |
| medium-html | 9.7 KB | `[26.84 µs  26.92 µs  27.00 µs]` | ~362 MB/s |
| large-html | 168 KB | `[892.04 µs  894.98 µs  898.58 µs]` | ~192 MB/s |

### Throughput (`gzip_fast` — level 1)

| Payload | Size | Time | Throughput |
|---|---|---|---|
| small-json | 276 B | `[7.28 µs  7.32 µs  7.36 µs]` | ~38 MB/s |
| medium-html | 9.7 KB | `[10.89 µs  11.00 µs  11.13 µs]` | ~885 MB/s |
| large-html | 168 KB | `[114.74 µs  114.91 µs  115.09 µs]` | ~1.5 GB/s |

### Compression ratios (default level 6)

| Payload | Before | After | Ratio |
|---|---|---|---|
| small-json | 276 B | 226 B | 81.9 % *(high JSON entropy; small payload; below the 256 B middleware cutoff so never actually compressed in production)* |
| medium-html | 9 737 B | 643 B | **6.6 %** *(93 % shrink)* |
| large-html | 171 837 B | 12 778 B | **7.4 %** *(93 % shrink)* |

**Interpretation:** For any realistic HTML response, gzip at default level saves ~93 % of bytes with sub-ms CPU cost — exactly the win predicted in `Page_Load_Perf_Analysis.md`. The `fast` preset trades ~1 percentage point of compression for 2-7× lower CPU, useful if CPU becomes the bottleneck. Small payloads (<256 B) are skipped by the middleware; the 276 B sample is included only to show the fixed-cost floor.
