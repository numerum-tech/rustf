# RustF Feature Spec: Library Hosting And Programmatic Server Control

## Summary

Add a library-oriented hosting API to `rustf` so applications can embed the framework inside another Rust host process, especially:

- a desktop shell such as Tauri
- a larger server orchestrator
- test harnesses and integration runners

The feature must preserve the current server-first model of RustF while adding explicit programmatic lifecycle control.

## Motivation

RustF already supports:

- SSR rendering
- embedded views and static assets
- crate-based consumption
- graceful shutdown on OS signals

What is still missing for desktop-shell and embedded-host use cases is a clean way to:

- start RustF without relying on process-level signal handling
- bind to a caller-provided listener or ephemeral port
- obtain the effective bound address
- shut the server down programmatically
- run RustF as a managed subsystem inside another application

This is needed for use cases such as:

1. Tauri desktop app starts a local RustF SSR server and opens a webview against it.
2. A server wrapper starts multiple internal RustF apps.
3. Integration tests need deterministic startup and shutdown without sending SIGINT/SIGTERM.

## Goals

- Expose a public hosting API suitable for embedding RustF as a library.
- Support programmatic startup and shutdown.
- Support binding to `127.0.0.1:0` and retrieving the actual bound address.
- Support caller-provided `TcpListener`.
- Preserve the current `app.start().await` developer experience.
- Keep embedded views/assets working unchanged.

## Non-Goals

- Replace the existing `start()` API.
- Add desktop-specific logic to the framework.
- Change the template engine or SSR model.
- Add a browser/webview abstraction.
- Introduce multi-process orchestration.

## Primary Use Cases

### 1. Tauri Desktop Host

The desktop app:

- constructs a RustF app
- starts it on localhost using an ephemeral port
- gets back the effective URL
- points the webview to that URL
- stops RustF when the desktop app exits

### 2. Embedded Single-Binary Server Product

A product binary:

- creates RustF programmatically
- injects its own config
- starts and stops the server from a supervisor layer

### 3. Integration Tests

Tests:

- launch RustF in-process
- wait until the listener is active
- run HTTP assertions
- stop the app cleanly

## Proposed Public API

### New Types

```rust
pub struct ServerHandle {
    // Opaque public type
}

pub struct RunningServer {
    pub local_addr: std::net::SocketAddr,
    pub handle: ServerHandle,
}
```

### New RustF Methods

```rust
impl RustF {
    pub async fn serve_with_handle(self, addr: &str) -> Result<RunningServer>;

    pub async fn serve_on_listener(
        self,
        listener: tokio::net::TcpListener,
    ) -> Result<RunningServer>;
}
```

### New ServerHandle Methods

```rust
impl ServerHandle {
    pub fn local_addr(&self) -> std::net::SocketAddr;

    pub async fn shutdown(self) -> Result<()>;
}
```

## Behavioral Requirements

### Startup

- `serve_with_handle("127.0.0.1:0")` must bind successfully to an ephemeral port.
- The returned value must expose the actual `local_addr`.
- The server must begin accepting connections before the future resolves.

### Shutdown

- `shutdown()` must stop new accepts.
- Existing in-flight requests must be drained gracefully.
- Existing RustF cleanup must still run:
  - shutdown event
  - workers shutdown
  - shared module shutdown
  - DB shutdown

### Signal Handling

Current behavior:

- `start()` / `serve(None)` may keep signal-based shutdown behavior.

New behavior:

- embedded hosting APIs must not require OS signals
- signal handling should be optional or bypassed in hosted mode

### Listener Ownership

For `serve_on_listener(listener)`:

- the caller provides the already-bound listener
- RustF must serve on it without rebinding
- RustF must return the listener address via `local_addr`

## Compatibility Requirements

- Existing `RustF::new()`, `RustF::with_args()`, `RustF::start()` must continue to work unchanged.
- Existing CLI-generated apps must not require any code changes.
- Embedded views via `embedded-views` must continue to work unchanged.
- Filesystem-based views must continue to work unchanged.

## Suggested Internal Design

### Current Problem

Today, server lifecycle is tightly coupled to:

- internal listener creation
- signal registration
- blocking `serve()` flow

### Proposed Refactor

Split the current server startup into 3 layers:

1. `build_server_runtime(app, listener, shutdown_source)`
   - internal assembly
   - no signal assumptions

2. `serve_on_listener(...)`
   - library-hosted path
   - caller-managed lifecycle

3. existing `serve()` / `start()`
   - convenience wrapper
   - creates listener
   - installs signal handling

### Shutdown Model

Recommended implementation:

- internal `tokio::sync::oneshot` or `tokio::sync::watch`
- one sender held by `ServerHandle`
- server accept loop listens to shutdown receiver

This lets desktop and tests trigger shutdown directly.

## Tauri-Oriented Example

```rust
use rustf::prelude::*;

#[rustf::auto_discover]
async fn start_embedded() -> rustf::Result<()> {
    let app = RustF::with_config(load_config()).auto_load();
    let running = app.serve_with_handle("127.0.0.1:0").await?;

    let url = format!("http://{}", running.local_addr);
    println!("Open webview at {}", url);

    // host keeps running.handle and calls shutdown later
    Ok(())
}
```

## Testing Requirements

Add tests covering:

1. `serve_with_handle("127.0.0.1:0")` returns a real bound address.
2. HTTP requests succeed against the returned address.
3. `shutdown()` stops the server cleanly.
4. cleanup hooks still execute during programmatic shutdown.
5. `serve_on_listener()` works with a caller-provided listener.
6. existing `start()` behavior still works.

## Acceptance Criteria

The feature is complete when:

- a RustF app can be started in-process without using OS signals
- the host can retrieve the effective local address
- the host can stop the server programmatically
- Tauri-style local hosting is possible without patching generated apps heavily
- current server-mode apps remain backward compatible

## Recommended Rollout

### Phase 1

- add internal lifecycle abstraction
- add `serve_with_handle`
- add `shutdown()`

### Phase 2

- add `serve_on_listener`
- add tests for ephemeral port and shutdown semantics

### Phase 3

- document desktop-host integration
- update `rustf-cli` scaffold docs with hosted-mode examples

## Open Questions

1. Should `ServerHandle::shutdown()` consume `self` or borrow `&self`?
2. Should RustF expose a non-blocking `spawn()` helper in addition to `serve_with_handle()`?
3. Should hosted mode disable signal handlers automatically, or should this be explicit in API naming?
4. Should there also be a `local_url()` helper for convenience?

## Recommendation

Implement this as a focused hosting feature, not as a Tauri feature.

That keeps RustF:

- server-first
- SSR-first
- reusable in desktop and non-desktop hosts
- compatible with the current framework model
