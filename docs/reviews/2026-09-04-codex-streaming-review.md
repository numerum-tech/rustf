Verdict: ship with fixes. The streaming-body core is coherent, but I would not freeze the 1.0 API until the header authority and SSE encoding issues are fixed. The biggest risk is that callers can still create contradictory framing headers around streams, and the SSE encoder treats bare carriage returns as data instead of line breaks, allowing user-controlled fields to forge extra SSE lines. Static files and HEAD semantics also need explicit decisions before this is called final.

## Blocker

None found.

## High

### User headers can override or contradict the body framing

- Location: `rustf/src/http/response.rs:612`, `rustf/src/http/response.rs:644`, `rustf/src/http/response.rs:656`
- What is wrong: `Response::into_hyper` only inserts a declared stream `Content-Length` when no `content-length` header is already present, then blindly appends all user/framework headers. For an unsized `Body::Stream`, an existing `Content-Length` is also preserved. Because `Response.headers` is public and `with_header` accepts `Content-Length` / `Transfer-Encoding`, callers and middleware can produce `Body::from_stream(...)` with `Content-Length: 123`, `Body::from_sized_stream(..., 5)` with `Content-Length: 123`, or a stream with both `Content-Length` and `Transfer-Encoding`.
- Why it matters: this breaks HTTP framing. An unsized stream with a stale `Content-Length` can hang clients or be treated as truncated; a mismatched sized stream can abort the connection; `Content-Length` plus `Transfer-Encoding` is explicitly contradictory in HTTP/1.1 and dangerous around intermediaries.
- Suggested fix: make `into_hyper` authoritative for framing. Remove all existing `Content-Length` and `Transfer-Encoding` response headers before body conversion, then set `Content-Length` only from the actual full body length or declared stream size. Never preserve `Transfer-Encoding`; let hyper choose chunked for HTTP/1.1 unsized bodies and omit it for HTTP/2. Consider adding typed header setters or rejecting hop-by-hop/framing headers in `add_header`.
- Verified by: reading `declared_len` handling and the later append loop. Existing tests cover only happy paths where no stale framing headers are present.

### Bare `\r` in SSE fields can forge additional SSE lines

- Location: `rustf/src/http/sse.rs:129`, `rustf/src/http/sse.rs:140`, `rustf/src/http/sse.rs:150`, `rustf/src/http/sse.rs:164`
- What is wrong: the encoder splits comments, ids, and event names with `str::lines()`, and data with `split_inclusive('\n')`. Neither treats a standalone carriage return as a line break in the same way the SSE wire parser does. For example, data like `ok\rretry: 0` is emitted as `data: ok\rretry: 0\n\n`, which a compliant event-stream parser can interpret as a `data:` line followed by a forged `retry:` line.
- Why it matters: `SseEvent` is likely to carry user-controlled data. The code and changelog explicitly claim that multi-line values cannot smuggle a second field, but bare CR violates that claim. This is not HTTP response splitting because it is in the body, but it is SSE field injection.
- Suggested fix: normalize all SSE field values by splitting on any SSE line ending: `\r\n`, bare `\n`, and bare `\r`. Emit each fragment with the intended field prefix. Strip NUL from `id` as now; decide whether to strip or reject NUL in `event` too. Add tests for bare `\r` in `data`, `id`, `event`, and `comment`.
- Verified by: reading the encoder branches and comparing them to the existing tests, which cover `\n` and `\r\n` but not bare `\r`.

### `HEAD` requests can send a full streaming body

- Location: `rustf/src/app.rs:1048`, `rustf/src/app.rs:1069`, `rustf/src/http/response.rs:612`
- What is wrong: `handle_request_with_peer` does not special-case `HEAD`, static files are served before route matching for any method whose path matches a static prefix, and `Response::into_hyper` has no request method context to suppress bodies. A `HEAD /public/big.bin` therefore follows the same path as `GET /public/big.bin` and can stream the file body.
- Why it matters: `HEAD` responses must include the headers that a `GET` would have sent, but no body. For large streamed static files this wastes disk and network resources and can confuse clients. It also weakens the reliability of `Content-Length` as metadata for clients that probe before downloading.
- Suggested fix: carry the method into response finalization or handle `HEAD` in `handle_request_with_peer` after the response is built: preserve status and headers, replace the body with `Body::empty()`, and ensure the `Content-Length` still describes the would-be `GET` body. Also decide whether `HEAD` should match `GET` routes or only static files.
- Verified by: reading the request path through static serving and response conversion. I did not run a live `HEAD` probe because this sandbox denies binding local sockets.

## Medium

### Static file serving opens the path before proving it is contained and regular

- Location: `rustf/src/app.rs:1413`, `rustf/src/app.rs:1419`, `rustf/src/app.rs:1428`
- What is wrong: `try_read_static_file` opens `path` first, then canonicalizes it and checks containment/regular-file metadata. A symlink escape is not read because the code later returns `None`, but the file outside the static root has already been opened. Special files under the static tree are also opened before `metadata.is_file()` rejects them.
- Why it matters: opening before validation is unnecessary exposure. On Unix, opening FIFOs/devices can block or consume scarce blocking-pool threads; opening an escaped target before rejecting it is not a data leak here, but it is still the wrong security boundary for static serving.
- Suggested fix: validate with `tokio::fs::canonicalize` and `tokio::fs::metadata` before opening, or use platform-specific safe-open techniques (`openat`/`O_NOFOLLOW`) if you want to eliminate TOCTOU completely. At minimum, reject non-regular files before constructing the streaming body, and add a FIFO/special-file regression test on Unix.
- Verified by: reading `try_read_static_file`; the symlink escape test only asserts no response is returned, not that the escaped file is never opened.

### Private file helpers do synchronous filesystem work inside async methods

- Location: `rustf/src/http/response.rs:391`, `rustf/src/http/response.rs:392`, `rustf/src/http/response.rs:398`, `rustf/src/http/response.rs:406`
- What is wrong: `resolve_contained_file` uses `Path::canonicalize()` and `std::fs::metadata()` synchronously. The async file helpers call this before awaiting `tokio::fs::File::open`.
- Why it matters: these calls can block a Tokio worker on slow filesystems, network mounts, overloaded disks, or maliciously expensive path lookups. Streaming is specifically for long-lived/high-concurrency responses, so blocking setup work should be avoided.
- Suggested fix: make `resolve_contained_file` async and use `tokio::fs::canonicalize` / `tokio::fs::metadata`, or wrap the sync path resolution in `tokio::task::spawn_blocking`. Keep the same containment and regular-file checks.
- Verified by: reading the private file helper path from `file_download_stream_from` / `file_inline_stream_from` into `resolve_contained_file`.

### `sse_with_keep_alive` emits a keep-alive immediately, not after an idle interval

- Location: `rustf/src/http/sse.rs:194`, `rustf/src/http/sse.rs:234`, `rustf/src/http/sse.rs:317`
- What is wrong: `tokio::time::interval(interval)` ticks immediately. The implementation returns `SseEvent::comment("")` on that first tick whenever the source stream is pending, and the unit test locks that behavior in.
- Why it matters: the public docs say keep-alives are injected when the feed is idle for the interval. An immediate comment is usually harmless, but it is observable wire behavior, can surprise tests/clients expecting the first item to be a real event, and becomes hard to change after 1.0.
- Suggested fix: use `tokio::time::interval_at(tokio::time::Instant::now() + interval, interval)` or consume/reset the first tick during construction. Update docs/tests to define the first keep-alive as after one idle interval.
- Verified by: reading the timer construction and the `keep_alive_emits_comments_only_while_idle` test asserting an immediate comment.

### Sized streams trust the declared byte count without enforcement

- Location: `rustf/src/http/body.rs:75`, `rustf/src/http/body.rs:130`, `rustf/src/http/response.rs:644`
- What is wrong: `Body::from_sized_stream` and `BodyStream::with_size` expose a public way to set `Content-Length`, but no wrapper counts emitted bytes and errors when the stream yields fewer or more bytes. File streaming also declares the metadata length once and does not bound reads to that length if the file grows.
- Why it matters: a wrong length becomes a protocol error visible to clients as a hang, truncation, or connection abort. The docs warn callers, but the framework can avoid the worst class of bugs by enforcing its own invariant.
- Suggested fix: wrap sized streams in a counting adapter. Stop after exactly `size` bytes, error if a chunk would exceed the remaining count, and error on EOF before the declared count. For `Body::from_file`, consider `take(size)` semantics so file growth cannot emit more than the declared length.
- Verified by: reading `with_size`, `from_sized_stream`, `from_file`, and `into_hyper`. The test suite verifies correct sized files, not undersized/oversized streams.

### SSE cannot represent fallible event sources directly

- Location: `rustf/src/http/response.rs:366`, `rustf/src/context.rs:749`
- What is wrong: `Response::sse` and `ctx.sse` accept `Stream<Item = SseEvent>` only. A producer whose event source can fail has no way to return `Err` into the body stream for the mid-stream error logging path; it must encode an application-level error event or end silently.
- Why it matters: this is an API-freeze concern. `BodyStream` already models `Result<Bytes>`, but the higher-level SSE API hides transport/source errors exactly where long-lived streams are most likely to hit them.
- Suggested fix: before 1.0, consider accepting `Stream<Item = rustf::Result<SseEvent>>` for `sse` and offering a convenience wrapper for infallible streams, or add a second clearly named fallible SSE constructor.
- Verified by: reading the SSE constructor bounds and the `events.map(|event| Ok(event.to_bytes()))` conversion.

## Low

### Range requests are still ignored for streamed static/media files

- Location: `rustf/src/app.rs:1069`, `rustf/src/app.rs:1400`
- What is wrong: static serving extracts only `If-None-Match` and `If-Modified-Since`. `Range` is not parsed, and large media files above 1 MiB are now streamed only as full `200 OK` responses.
- Why it matters: this is acceptable for many static assets, but poor for video/audio and resumable downloads. The docs say images, fonts, archives, and media stream above the threshold; media clients commonly rely on `Range`.
- Suggested fix: either document that static `Range` is unsupported and avoid implying media-grade serving, or implement single-range `206 Partial Content` with `Accept-Ranges: bytes` and correct `Content-Range`/`Content-Length`.
- Verified by: searching for `Range` handling and reading the static-file request path.

### `Body::len_hint` truncates declared sizes on 32-bit targets

- Location: `rustf/src/http/body.rs:162`, `rustf/src/http/body.rs:165`
- What is wrong: `BodyStream` stores size as `u64`, but `Body::len_hint()` and `Response::body_size()` return `Option<usize>` via `n as usize`.
- Why it matters: on 32-bit targets, a stream larger than `usize::MAX` reports a wrapped/truncated size to middleware. The framework may not target 32-bit seriously, but this is a public API and the cast is silent.
- Suggested fix: return `Option<u64>` for stream/body size before the 1.0 freeze, or use `usize::try_from(n).ok()` to avoid lying.
- Verified by: reading the size types and cast.

### Test helpers rely on process-global configuration and skip important protocol cases

- Location: `rustf/tests/streaming_body_test.rs:119`, `rustf/tests/streaming_helpers_test.rs:5`, `rustf/tests/streaming_helpers_test.rs:227`
- What is wrong: the new socket tests put many behaviors into one test because `RustF::new()` is process-global. They cover the happy-path wire format, but not HEAD, HTTP/2, stale `Content-Length`, stale `Transfer-Encoding`, undersized/oversized sized streams, bare-CR SSE injection, zero-length streamed files, or client disconnect cancellation.
- Why it matters: when one large integration test fails early, later checks are not run. More importantly, the highest-risk protocol edge cases are not covered.
- Suggested fix: add focused unit tests around `into_hyper` header normalization once fixed, direct `SseEvent::to_bytes` tests for CR, and at least one end-to-end HEAD/static test. Consider a test-only configuration reset or constructor so integration tests can be split without fighting the singleton.
- Verified by: reading the new tests and running `cd rustf && cargo test --all-features`; the run was blocked by sandbox networking at `tests/server_hosting_ephemeral_test.rs` with `Failed to bind 127.0.0.1:0: Operation not permitted`, after 493 unit tests and several integration tests had passed.

### Duplicate streaming convention block in the skill doc

- Location: `docs/RUSTF_SKILL.md:302`, `docs/RUSTF_SKILL.md:313`
- What is wrong: the same streaming convention paragraph appears twice back-to-back.
- Why it matters: low functional risk, but this is generated guidance for future agents and duplicate text invites drift.
- Suggested fix: delete the second copy.
- Verified by: reading the docs diff and the line-numbered file.

## Nit

### Keep-alive docs are slightly stronger than the implementation

- Location: `CHANGELOG.md:25`, `book/src/api-reference/context.md:260`, `rustf/src/http/sse.rs:235`
- What is wrong: docs say comments are injected when the feed is idle / only while nothing else is being sent. The first Tokio interval tick means an idle stream gets an immediate comment, before one interval has elapsed.
- Why it matters: this is mostly a documentation mismatch if the immediate tick is intentional.
- Suggested fix: either change the implementation as recommended above, or explicitly document the immediate initial comment.
- Verified by: comparing docs to the `keep_alive` implementation and its unit test.

## Checked And Found Correct

- `Body::Full` preserves the buffered default, and common `with_body(vec/string/&str/Bytes)` call sites continue to compile through `Into<Body>`.
- `Response` no longer derives `Clone`, which is appropriate for one-shot streams.
- `BodyStream` requires `Send + 'static`, and `into_hyper` uses `UnsyncBoxBody`, which matches the stream being `Send` but not necessarily `Sync`.
- File streaming keeps the `tokio::fs::File` inside the stream state; when hyper drops the body on disconnect, the stream and file handle are dropped.
- `file_chunks` reads asynchronously and holds only one 64 KiB buffer per poll.
- `open_sized` rejects directories and other non-regular files for the private streamed file helpers before constructing the response.
- Path containment for private file helpers canonicalizes the base and candidate path and rejects absolute paths or relative traversal outside the base.
- Content-Disposition filename handling strips CR/LF/NUL/quotes/backslashes from the fallback and percent-encodes the RFC 5987 filename.
- Static file cache validators (`ETag`, `Last-Modified`, `304`) are still computed before choosing buffered versus streamed bodies.
- Static symlink escape returns no response, and there is an existing Unix test for that behavior.
- Compression middleware now skips streamed bodies instead of trying to read or rewrite them, and updates/removes representation headers correctly for buffered gzip responses.
- `ctx.update_response` preserves non-content headers when replacing a response with stream/SSE/file helpers, so middleware headers such as `Set-Cookie` are not inherently lost.
- A body stream yielding `Err` is logged via `inspect_err`, and the existing socket test checks that an aborted chunked stream does not get a terminating zero chunk.
- SSE sets `Content-Type: text/event-stream`, `Cache-Control: no-cache`, and `X-Accel-Buffering: no`.
- SSE multi-line `\n` and `\r\n` data encoding is covered by unit tests, and JSON SSE data uses `serde_json::to_string`.
- Embedded static assets remain buffered; the streaming threshold applies to filesystem static files only.

---

## Resolution (Claude, 2026-09-04, same day)

Every finding above was addressed in the working tree; `cargo test --all-features` (44 test binaries incl. doctests) and `RUSTFLAGS=-D warnings cargo build` are green, `sample-app` compiles.

| Severity | Finding | Resolution |
|---|---|---|
| High | User headers can override/contradict framing | `into_hyper` is now the single framing authority: `Transfer-Encoding` always dropped (warn log), `Content-Length` dropped whenever the body knows its length (buffered, sized stream) and replaced by the true value; kept only for an empty buffered body (HEAD / 304). 4 unit tests in `response.rs`. |
| High | Bare `\r` forges SSE lines | `field_lines()` splits every field on `\r\n`, `\n` and bare `\r`; NUL stripped from `id` and `event`. Tests cover CR in `data`/`id`/`event`/`comment`. |
| High | HEAD streams a full body | `handle_request_with_peer` now wraps `dispatch_request`: for HEAD it records `len_hint()` as `Content-Length` and replaces the body with `Body::empty()`. HEAD with no HEAD route falls back to the GET route. Wire-tested for a >1 MiB static file and a GET route. |
| Medium | Static file opened before validation | `try_read_static_file` canonicalises, checks containment and `metadata().is_file()` via `tokio::fs` *before* `File::open`; fd metadata still drives ETag/size. |
| Medium | Sync fs calls in async helpers | `resolve_contained_file` is async on `tokio::fs::{canonicalize, metadata}`; all four callers await it. |
| Medium | Keep-alive comment fires immediately | `interval_at(now + interval, interval)`; first comment after one full idle interval. Unit test asserts no comment before the interval; docs/CHANGELOG updated. |
| Medium | Declared size not enforced | `BodyStream::into_enforced_stream()` (used by `into_hyper`): truncates an over-producing source to the declared length (warn log) and errors when the source ends short. 3 unit tests. |
| Medium | SSE cannot carry fallible sources | Sealed `SseItem` trait implemented for `SseEvent` and `rustf::Result<SseEvent>`; `Response::sse*`, `ctx.sse*` and `keep_alive` are generic over it. Keep-alive comments are wrapped in the source's item type. |
| Low | Range requests unsupported | Documented as unsupported on `STATIC_STREAM_THRESHOLD`, in CHANGELOG and the book; media wording removed. Not implemented for 1.0. |
| Low | `len_hint` truncates on 32-bit | `usize::try_from(n).ok()` — reports `None` instead of a wrapped value. |
| Low | Test gaps | Added: HEAD static + HEAD route fallback, zero-length streamed file, `into_hyper` header normalisation ×4, size enforcement ×3, CR injection, keep-alive-after-interval, `Result<SseEvent>` through keep-alive. Not added: HTTP/2 wire test, client-disconnect cancellation test. |
| Low | Duplicate skill paragraph | Removed in `docs/RUSTF_SKILL.md` and `.claude/skills/rustf/SKILL.md`. |
| Nit | Keep-alive docs stronger than code | Code now matches the docs. |

## Re-review (Codex, second pass)

Verdict: ship with fixes. Most original findings are fixed in code, and the library unit suite is green, but the framing/HEAD fix still leaves real protocol holes: arbitrary or duplicate `Content-Length` can survive on empty `Body::Full` responses, and the HEAD wrapper can preserve stale lengths for unsized streams or synthesize `Content-Length: 0` for 204/304. I would fix those before freezing 1.0.

| Finding | Status | File:line evidence |
|---|---|---|
| User headers can override or contradict body framing | PARTIALLY FIXED | `Transfer-Encoding` is dropped at `rustf/src/http/response.rs:678-684`, stream `Content-Length` is inserted from `declared_len` at `rustf/src/http/response.rs:672-675`, and caller `Content-Length` is dropped unless the body is empty at `rustf/src/http/response.rs:647-648` and `rustf/src/http/response.rs:685-688`. Remaining issue: every caller-supplied `Content-Length` is still appended for any empty `Body::Full` at `rustf/src/http/response.rs:690-695`, so duplicate or wrong lengths can still reach the wire outside the intended HEAD/304 case. |
| Bare `\r` in SSE fields can forge additional SSE lines | FIXED | `SseEvent::to_bytes` sends comments/id/event/data through `field_lines` at `rustf/src/http/sse.rs:141-189`; `field_lines` splits on LF, CRLF, and bare CR at `rustf/src/http/sse.rs:204-232`; NUL is stripped from id/event at `rustf/src/http/sse.rs:157-168` via `push_without_nul` at `rustf/src/http/sse.rs:235-241`. Tests cover bare CR injection and NUL at `rustf/src/http/sse.rs:403-433`. |
| HEAD requests can send a full streaming body | PARTIALLY FIXED | Body suppression is implemented at `rustf/src/app.rs:1070-1087`, and HEAD falls back to GET routes at `rustf/src/app.rs:1242-1251`. Middleware still runs for routed HEAD requests because `dispatch_request` builds `Context` and executes the middleware chain at `rustf/src/app.rs:1124-1128` before the HEAD body is discarded. Remaining issues: for an unsized stream, `len_hint()` is `None`, so stale `Content-Length` headers are not removed before the body becomes empty at `rustf/src/app.rs:1077-1086`; for 204/304 HEAD responses, the wrapper can synthesize `Content-Length: 0` from an empty body at `rustf/src/app.rs:1078-1084`. |
| Static file serving opens before proving containment/regular file | FIXED | `try_read_static_file` canonicalizes and checks containment before open at `rustf/src/app.rs:1457-1468`, verifies regular-file metadata before open at `rustf/src/app.rs:1469-1474`, and opens only after those checks at `rustf/src/app.rs:1476-1480`. |
| Private file helpers do synchronous filesystem work inside async methods | FIXED | `resolve_contained_file` is async at `rustf/src/http/response.rs:403`; it uses `tokio::fs::canonicalize` and `tokio::fs::metadata` at `rustf/src/http/response.rs:404-424`; callers await it at `rustf/src/http/response.rs:202`, `rustf/src/http/response.rs:244`, and `rustf/src/http/response.rs:272`. |
| `sse_with_keep_alive` emits immediately instead of after an idle interval | FIXED | `keep_alive` now uses `tokio::time::interval_at(now + interval, interval)` at `rustf/src/http/sse.rs:292-303`, and the paused-time test asserts no comment before the interval at `rustf/src/http/sse.rs:447-472`. |
| Declared stream sizes are not enforced | FIXED | `into_hyper` consumes streams through `into_enforced_stream` at `rustf/src/http/response.rs:654-665`; `BodyStream::into_enforced_stream` delegates sized streams to `enforce_size` at `rustf/src/http/body.rs:98-111`; `enforce_size` truncates overshoot at `rustf/src/http/body.rs:140-156` and errors on short EOF at `rustf/src/http/body.rs:162-174`. Tests cover exact, oversize, and short streams at `rustf/src/http/body.rs:435-498`. |
| SSE cannot represent fallible event sources directly | FIXED | `Response::sse` accepts any stream whose item implements `SseItem` at `rustf/src/http/response.rs:369-382`; `SseItem` is sealed and implemented for `SseEvent` and `Result<SseEvent>` at `rustf/src/http/sse.rs:243-281`; context helpers use the same bound at `rustf/src/context.rs:751-777`. |
| Range requests unsupported for streamed static/media files | FIXED | Not implemented, but now explicitly documented: `STATIC_STREAM_THRESHOLD` says Range is unsupported at `rustf/src/app.rs:27-30`, CHANGELOG says the static server still does not support Range at `CHANGELOG.md:33-37`, and the book says the same at `book/src/api-reference/context.md:266-267`. |
| `Body::len_hint` truncates declared sizes on 32-bit targets | FIXED | Stream length conversion uses `usize::try_from(n).ok()` at `rustf/src/http/body.rs:248-254`. |
| Test gaps and flaky test logic | PARTIALLY FIXED | Added unit tests for framing normalization at `rustf/src/http/response.rs:736-808`, size enforcement at `rustf/src/http/body.rs:435-498`, SSE CR/NUL and keep-alive timing at `rustf/src/http/sse.rs:403-496`, and socket tests for zero-length files plus HEAD static/route fallback at `rustf/tests/streaming_helpers_test.rs:381-407`. Still not covered here: HTTP/2 wire behavior and client-disconnect cancellation; I also could not run socket integration tests in this sandbox. |
| Duplicate streaming convention block in skill docs | FIXED | `docs/RUSTF_SKILL.md` has only one streaming convention block at `docs/RUSTF_SKILL.md:302-311`; the CLI skill template also has one at `rustf-cli/templates/project/claude_skills/rustf/SKILL.md:302-311`. |
| Keep-alive docs stronger than implementation | FIXED | Code says first comment after one interval at `rustf/src/http/sse.rs:283-303`; CHANGELOG says the first comment is one full interval after stream start at `CHANGELOG.md:27-30`; the book says the same at `book/src/api-reference/context.md:261-264`. |

### New Findings

High - Empty-body `Content-Length` preservation is too broad. `into_hyper` keeps caller `Content-Length` for every empty `Body::Full` (`rustf/src/http/response.rs:647-648`, `rustf/src/http/response.rs:685-695`), not just HEAD/304. A normal `200 OK` empty response with `Content-Length: 999`, or an empty response with two different `Content-Length` headers, can still reach hyper with invalid framing. Suggested fix: do not preserve caller `Content-Length` based only on body emptiness. Either pass explicit response semantics into finalization (`HEAD` flag, allowed 304 representation length), or add a dedicated internal marker/header path for HEAD/304 and collapse to at most one validated `Content-Length`.

High - HEAD length handling is wrong for unsized streams and 204/304. The HEAD wrapper only removes existing `Content-Length` when `response.body.len_hint()` is `Some` (`rustf/src/app.rs:1077-1085`), then replaces the body with empty at `rustf/src/app.rs:1086`; an unsized stream with a stale length header becomes an empty body where `into_hyper` preserves that stale header. The same code can add `Content-Length: 0` to HEAD responses whose GET result is 204 or 304, even though 204 must not carry Content-Length and 304 may only carry the selected representation length. Suggested fix: in the HEAD path always remove `Content-Length` first; add it back only when the original response status permits it and the original body length is known. For 204, never add it. For 304, preserve/add only a validator-derived representation length, not `Body::empty()` length.

Low - `enforce_size(0)` polls and yields one empty chunk for an over-producing source. With `declared == 0`, `enforce_size` still polls the inner stream at `rustf/src/http/body.rs:139`, truncates the first non-empty chunk to zero at `rustf/src/http/body.rs:140-150`, and yields `Ok(Bytes::new())` at `rustf/src/http/body.rs:156`. This probably does not put payload bytes on the wire, but it is unnecessary and untested. Suggested fix: if `declared == 0`, return an empty stream immediately without polling the source.

### Verified

- Read current implementations for `Body`/`BodyStream`/`enforce_size`, `SseEvent`/`field_lines`/`SseItem`/`KeepAlive`, `Response::into_hyper`, `Context` streaming/SSE helpers, app dispatch/HEAD/static file serving, compression-relevant body access, CHANGELOG, book context docs, `docs/RUSTF_SKILL.md`, and the CLI skill template.
- Read current tests in `rustf/src/http/{body,sse,response}.rs` and `rustf/tests/streaming_helpers_test.rs`.
- Executed `cd rustf` equivalent via workdir and ran `cargo test --all-features --lib`: 502 tests passed.
- Did not execute socket integration tests or HTTP/2 wire tests in this sandbox; the previous full integration run failed on local socket binding (`127.0.0.1:0` operation not permitted), so those remain unverified here.

---

## Resolution of the second pass (Claude, 2026-09-04)

| Finding | Resolution |
|---|---|
| High — empty-body `Content-Length` preservation too broad | Heuristic removed. `into_hyper` now drops **every** caller `Content-Length`. The HEAD/304 length travels in a new private `Response.advertised_len` field, set via `Response::advertise_content_length(len)`, and is emitted only when the body is empty. Tests: duplicate/wrong caller lengths on an empty 200 produce no header; advertised length produces exactly one; advertised length is ignored when a body is present. Consequence: `Response` can no longer be built as a struct literal (noted as BREAKING in CHANGELOG; deliberate, pre-1.0). |
| High — HEAD length wrong for unsized streams / 204 / 304 | HEAD path now always strips `Content-Length`, empties the body, and advertises a length only when the original body length was known **and** the status is not `1xx`, `204` or `304`. Wire-tested: HEAD on an unsized stream sends no `Content-Length: 0`; HEAD on a 204 sends no `Content-Length`; HEAD on a sized static file and on a buffered route still advertise the GET length. |
| Low — `enforce_size(0)` polls the source | Early return of an empty stream when the declared size is 0; the source is never polled. Unit test asserts the source's poll flag stays unset. |

`cargo test --all-features` (44 binaries), `RUSTFLAGS=-D warnings cargo build`, `cargo fmt --check` on touched files, clippy on new code, and `sample-app` `cargo check` are green after these changes.

## Third pass (Codex)

Verdict: ship. The three second-pass findings are fixed in the current working tree; I found no new issue introduced by these fixes.

### Status

- FIXED — `Response::into_hyper` framing authority: `Response` has the private `advertised_len` field at `rustf/src/http/response.rs:42-50` and the public `advertise_content_length` builder at `rustf/src/http/response.rs:63-70`. `into_hyper` takes explicit `Content-Length` only from a stream's declared size or from `advertised_len` on an empty `Body::Full` at `rustf/src/http/response.rs:657-662`, inserts that value once before caller headers at `rustf/src/http/response.rs:686-690`, and then drops every caller `Transfer-Encoding` and `Content-Length` at `rustf/src/http/response.rs:692-703`. The unit tests assert unsized streams drop both framing headers at `rustf/src/http/response.rs:751-764`, sized streams replace a wrong length and emit exactly one length at `rustf/src/http/response.rs:768-789`, buffered non-empty bodies drop caller length and rely on body size hint at `rustf/src/http/response.rs:793-806`, empty bodies drop duplicated caller lengths at `rustf/src/http/response.rs:810-827`, advertised empty-body length emits exactly one header at `rustf/src/http/response.rs:831-852`, and `advertised_len` is ignored when a body is present at `rustf/src/http/response.rs:856-866`.
- FIXED — HEAD finalization: `handle_request_with_peer` records the original full length or declared stream size at `rustf/src/app.rs:1077-1081`, always strips caller `Content-Length` at `rustf/src/app.rs:1082-1087`, replaces the body with empty at `rustf/src/app.rs:1088`, and advertises length only when the original length is known and status is not informational, `204`, or `304` at `rustf/src/app.rs:1093-1099`. HEAD-to-GET fallback is still intact at `rustf/src/app.rs:1255-1260`. Routed HEAD requests still run middleware because dispatch executes the middleware chain before returning the response at `rustf/src/app.rs:1137-1142` and outbound middleware at `rustf/src/app.rs:1222-1228`. The helper tests cover a sized static HEAD at `rustf/tests/streaming_helpers_test.rs:393-402`, GET-route fallback at `rustf/tests/streaming_helpers_test.rs:404-412`, an unsized `/csv` stream with no invented `Content-Length: 0` at `rustf/tests/streaming_helpers_test.rs:414-423`, and `/nothing` returning `204` with no `Content-Length` at `rustf/tests/streaming_helpers_test.rs:425-432`.
- FIXED — `enforce_size(0)`: sized streams still route through `into_enforced_stream` at `rustf/src/http/body.rs:107-110`, and `enforce_size` now returns `futures::stream::empty()` before polling the source when `declared == 0` at `rustf/src/http/body.rs:120-124`. The unit test `enforced_stream_with_zero_declared_size_never_polls_the_source` asserts the collected stream is empty and the source poll flag remains unset at `rustf/src/http/body.rs:487-500`.

### New Findings

None. I also grepped for struct-literal fallout with `rg -n "Response\s*\{" rustf rustf-cli sample-app rustf-cli/templates -g '*.rs' -g '*.rs.template' -g '*.tera'`; the hits were type definitions or unrelated response structs such as `rustf/src/http/response.rs:42`, `rustf/src/security/error_handling.rs:180`, and `rustf-cli/src/mcp/interface.rs:10`, with no `rustf::http::Response { .. }` construction in `rustf/`, `rustf-cli/`, `sample-app/`, or `rustf-cli/templates/`.

### Verified

- Read current `rustf/src/http/response.rs`, `rustf/src/app.rs`, `rustf/src/http/body.rs`, `rustf/tests/streaming_helpers_test.rs`, and `CHANGELOG.md`.
- CHANGELOG [Unreleased] accurately describes open-ended streams and SSE/static streaming at `CHANGELOG.md:10-37`, HEAD behavior including no invented length for open-ended streams and none for `1xx`/`204`/`304` at `CHANGELOG.md:38-42`, `into_hyper` as the single framing authority and `advertise_content_length` at `CHANGELOG.md:70-76`, and the deliberate private-field pre-1.0 break at `CHANGELOG.md:77-80`.
- Executed `cargo test --all-features --lib` in `rustf/`: 505 tests passed, 0 failed.
- Did not run socket-binding integration tests or a full `cargo test --all-features`; this pass only executed the requested library test command, so socket-level behavior remains unverified here.

---

## Sign-off (2026-09-04)

Codex third pass: **ship**, all findings FIXED, no new findings, no struct-literal
fallout from the private `Response` field, CHANGELOG accurate.

Codex could not bind sockets in its sandbox, so it ran only
`cargo test --all-features --lib` (505 passed). The socket-level behaviour it
could not verify was run here instead: full `RUSTFLAGS="-D warnings" cargo test
--all-features` over 44 test binaries (unit + doctests + every integration test,
including `streaming_body_test` and `streaming_helpers_test` which bind real
TCP listeners) — all green. `sample-app` `cargo check` clean.

Review closed. Streaming is ready to commit on `feat/streaming-body`.
