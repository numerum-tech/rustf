# WebSocket support — assessment and implementation plan

**Status:** TODO — not started. Assessment done 2026-09-04.
**Blocked on:** Codex sign-off of the streaming fixes, then the Context API
cleanup (`2026-09-04-context-api-cleanup-plan.md`) if Morle wants that first.
Independent of both technically; ordering is a release-scope choice.
**Target:** 1.1.0 unless Morle decides 1.0.0 must say "WebSocket" on the tin.
Everything here is additive (new route kind, new `Context` method, new
module), so it does not need to precede the 1.0 freeze.
**Author context:** Claude with Morle. Every commit reviewed before landing.

---

## 1. Assessment — where things stand

### Nothing exists yet

- No WebSocket code in `rustf/`. No `tungstenite` / `tokio-tungstenite` /
  `fastwebsockets` in either `Cargo.lock`.
- `README.md:412` lists WebSocket under **Future Enhancements** (honest).
- `docs/ABOUT_MODULES.md:81` / `book/src/advanced/modules.md:82` cite
  "WebSocket Managers" as a module example — aspirational, nothing behind it.
- `rustf-cli serve --websocket` (`rustf-cli/src/commands/serve.rs`) is a flag
  for the CLI's own MCP server and only reserves a port; there is no WS
  transport behind it either. Not reusable for apps and not a dependency.

### What is already in place and helps

| Piece | State | Why it matters |
|---|---|---|
| `rustf/src/http/server.rs:205` | `AutoBuilder::serve_connection_with_upgrades(io, service)` | Connection-level upgrade plumbing already on. `hyper::upgrade::on(req)` will resolve. |
| `Request` (`http/request.rs`) | Has private fields (`body_bytes`, `peer_addr`, …) | An `upgrade: Option<hyper::upgrade::OnUpgrade>` field is non-breaking. |
| `Response` | Private `advertised_len` since the streaming work; `into_hyper` owns framing | A `101 Switching Protocols` is headers-only; no body/framing surprises. |
| Middleware chain (`app.rs:~1160`) | Inbound (session, rate limit, CORS, …) runs before the handler | Auth / session / rate-limit checks happen on the handshake request before any upgrade. |
| `Route { method, path, handler, xhr_only, before }` | `xhr_only` is already a per-route flag | A `websocket: bool` flag follows the same pattern; `routes![XHR ...]` shows the macro arm precedent. |
| Graceful shutdown (`server.rs`) | `GracefulShutdown::watch(conn)` | Needs extending: upgraded sockets leave hyper's control (see §4). |

### What blocks a naive implementation

1. **`Request::from_hyper_with_connection` drops the upgrade handle.** It
   copies method/uri/headers, then `req.into_body().collect()`. The
   `OnUpgrade` extension dies with the hyper request. Fix: call
   `hyper::upgrade::on(&mut req)` **before** consuming the body, only when
   the request carries `Connection: upgrade` + `Upgrade: websocket`, and
   store it in the new private field. Upgrade requests have no body, so the
   existing `has_body` short-circuit already skips the read.
2. **Handler must return before the upgrade can happen.** hyper performs the
   upgrade only after the `101` response has been written. So the handler
   cannot "get a socket"; it registers a callback and returns. The framework
   builds the 101, spawns a task that awaits `OnUpgrade`, wraps the IO, and
   runs the callback. This is hyper's documented pattern.
3. **Routing.** `routes![GET "/x" => h]` stringifies any ident as the method,
   so `SOCKET "/chat" => h` would register method `"SOCKET"` and never match
   a `GET` handshake. Needs a dedicated macro arm that produces a `GET` route
   with `websocket = true`, and a router check that a non-upgrade `GET` to a
   socket route gets `426 Upgrade Required` rather than running the handler.
4. **HTTP/2.** `AutoBuilder` negotiates h1/h2. WebSocket over h2
   (RFC 8441 extended CONNECT) is not supported by hyper's server. Browsers
   open WebSocket over HTTP/1.1 anyway; document "HTTP/1.1 only".
5. **Logging/timing middleware** measure handler duration, which ends at the
   101. Connection lifetime is not observed by outbound middleware. Note in
   docs; optional `on_close` hook later.

### Dependency choice

| Option | Pros | Cons |
|---|---|---|
| **`tokio-tungstenite`** (recommended) | De-facto standard; hyper-1 compatible via `WebSocketStream::from_raw_socket(TokioIo::new(upgraded), Role::Server, cfg)`; `tungstenite::handshake::derive_accept_key` gives the `Sec-WebSocket-Accept` value, so no extra sha1/base64 deps; permessage-deflate optional; mature close-handshake and control-frame handling. | Pulls `tungstenite` + its deps (small). |
| `hyper-tungstenite` | Does handshake + upgrade in ~3 calls. | Thin layer over the above; adds a dep for ~40 lines we can own. Skip. |
| `fastwebsockets` | Faster, smaller frames overhead. | Lower-level API, less ergonomic for the agent-facing wrapper, smaller ecosystem. Not worth it for a framework whose bottleneck is app code. |

Feature flag `websocket`, **on by default** (same policy as `redis`); opt-out
with `--no-default-features`. Handshake bits (`derive_accept_key`) come from
`tungstenite`, re-exported through `tokio-tungstenite`.

---

## 2. API design (Total.js-shaped, agent-first)

Design rule from the Context API work: the agent types the *source*
(`ctx.`) and autocomplete must show the whole path. So: one route keyword,
one `ctx.` entry point, one socket type with obvious verbs.

### Routing

```rust
pub fn install() -> Vec<Route> {
    routes![
        GET    "/chat"        => page,     // renders the page
        SOCKET "/chat/{room}" => chat,     // GET + Upgrade: websocket
    ]
}
```

`SOCKET` expands to `Route::websocket(path, handler)` → `method = "GET"`,
`websocket = true`. A plain `GET` without upgrade headers on that route
returns `426 Upgrade Required` (`Upgrade: websocket`), never runs the
handler. Auto-discovery unchanged.

### Total.js reference model (what we mirror)

```js
ROUTE('+SOCKET /chat/{room}/', function() {          // '+' = authorized only
    var self = this;                                  // the socket controller: one per matched URL
    self.autodestroy();                               // destroy when the last client leaves
    self.on('open',    function(client) { client.send({ hello: client.user.name }); });
    self.on('message', function(client, message) { self.send(message, client); });  // broadcast, exclude sender
    self.on('close',   function(client) { ... });
    self.on('error',   function(err, client) { ... });
}, ['json'], 1024);                                   // flags: json|text|raw; max message length (KiB)
```

Total.js facts that shape the Rust API:

- The action runs **once per unique URL** when the first client connects; the
  controller it configures *is* the room. Later clients on the same URL join it.
  Broadcast is `self.send(msg, blacklist)`; membership is `self.connections`
  (`id → client`), `self.online`, `self.find(id)`, `self.destroy()`.
- `client` carries the handshake context: `client.id`, `client.query`,
  `client.params`, `client.headers`, `client.ip`, `client.session`,
  `client.user`, `client.close(code, reason)`, `client.send(msg)`.
- `['json']` flag auto-parses/serialises messages; `'text'` / `'raw'` do not.
- `+SOCKET` / `-SOCKET` encode the authorization requirement in the route.
- `F.connections['/chat/room1/']` reaches a controller from anywhere
  (HTTP handler, worker, scheduler) to push into it.

### Rust mapping

| Total.js | RustF | Note |
|---|---|---|
| `ROUTE('SOCKET /chat/{room}/', action, ['json'], 1024)` | `routes![SOCKET "/chat/{room}" => chat]` | flags/limits come from `[websocket]` config and per-route builder (`.json()`, `.max_message_size(..)`) |
| `'+SOCKET'` / `'-SOCKET'` | `routes![before: require_auth, SOCKET .. ]` or `ctx.require_auth()?` in the action | RustF already has the `before:` hook and `require_auth`; no new syntax |
| action `function() { this.on('open'/'message'/'close') }` | `async fn chat(ctx: &mut Context) -> Result<()>` that calls `ctx.socket(\|room\| ..)` | see below; runs per handshake, but the `room` it configures is shared per URL like Total.js |
| `self` (socket controller) | `SocketRoom` (per matched path, auto-created on first client) | `send(msg)`, `send_except(msg, client)`, `send_to(id, msg)`, `online()`, `connections()`, `find(id)`, `destroy()`, `autodestroy(bool)` |
| `client` | `SocketClient` | `id()`, `params()`, `query()`, `headers()`, `ip()`, `session()`, `send(msg)`, `send_json(&T)`, `close(code, reason)` |
| `self.on('open', f)` | `room.on_open(\|client\| async move { .. })` | registered once per room (first handshake wins; later registrations ignored with a debug log) — same "runs once per URL" semantics as Total.js |
| `self.on('message', f)` | `room.on_message(\|client, msg\| async move { .. })` | `msg: Message` (`Text`/`Binary`) or `recv_json::<T>()` when the route is `.json()` |
| `self.on('close', f)` | `room.on_close(\|client, code\| async move { .. })` | |
| `self.on('error', f)` | `room.on_error(\|client, err\| async move { .. })` | default: log at `error`, close `1011` |
| `F.connections[url]` | `SOCKET::room("/chat/room1")` global (like `CONF`, `WORKER`) | `Option<Arc<SocketRoom>>`; `SOCKET::rooms()` |
| `['json']` | `Message::json()` / `client.send_json(&T)` / `msg.parse::<T>()` | always available; the route-level `.json()` flag additionally rejects non-JSON text with `1003` |

### Handler (the Total.js action, in Rust)

```rust
pub fn install() -> Vec<Route> {
    routes![
        GET    "/chat"        => page,
        SOCKET "/chat/{room}" => chat,      // GET + Upgrade: websocket
    ]
}

async fn chat(ctx: &mut Context) -> rustf::Result<()> {
    ctx.require_auth()?;                       // '+SOCKET'. Err → 401, no upgrade.
    ctx.socket(|room| {                        // room = SocketRoom for this exact path, shared by all clients on it
        room.autodestroy(true);
        room.on_open(|client| async move {
            let name: String = client.session_get("user_name").unwrap_or_default();
            client.send_json(&json!({ "type": "hello", "name": name })).await
        });
        room.on_message(|client, msg| async move {
            client.room().send_except(msg, client.id()).await   // relay to everyone else
        });
        room.on_close(|client, _code| async move {
            client.room().send_json(&json!({ "type": "left", "id": client.id() })).await
        });
    })
}
```

- `ctx.socket(configure)` validates the handshake (`Upgrade`, `Connection`,
  `Sec-WebSocket-Version: 13`, `Sec-WebSocket-Key`, Origin policy), looks
  up or creates the `SocketRoom` for `ctx.req.path()`, runs `configure(room)`
  **only if the room was just created** (Total.js "action runs once per
  URL"), stores the pending upgrade on the `Context`, and sets `ctx.res` to
  the `101` with `Sec-WebSocket-Accept`. Handshake errors → 400/403/426.
- After the handler returns, `execute_route_handler` spawns the upgrade
  task: await `OnUpgrade`, wrap the IO, create `SocketClient` (with a
  snapshot of params/query/headers/ip/session id), add it to the room, fire
  `on_open`, then run the read loop dispatching `on_message`, and fire
  `on_close` on exit. Outbound middleware still runs on the 101.
- Handler `Err` after `ctx.socket(..)` is impossible to observe by the
  client (101 already prepared) — so `ctx.socket` must be the **last** call;
  documented, and a debug assertion catches a handler that replaces
  `ctx.res` afterwards.
- Escape hatch for people who want a raw per-client loop (axum style):
  `ctx.websocket(|socket| async move { .. })` giving a `WebSocket` with
  `recv/send_*/close/split`. Same plumbing, no room. Keep it: it is the
  building block the room runtime is written on, and some agents will look
  for it.

### `rustf::websocket` types

- `SocketRoom` — `Arc`-shared per path; `DashMap<ClientId, SocketClient>`;
  handlers stored as `Arc<dyn Fn(..) -> BoxFuture>`; `send*` fan out over
  bounded mpsc senders; `destroy()` closes everyone with `1001`;
  `autodestroy` removes the room from the registry when the last client
  leaves (default **on**, as in Total.js).
- `SocketClient` — `Clone`, cheap handle: id, `room()`, snapshot of
  handshake data, `send_*`, `close`. Slow reader → bounded queue full →
  close `1008`.
- `Message` — `Text(String)`, `Binary(Bytes)`; `Message::json(&T)`,
  `msg.parse::<T>()`. Control frames never reach user code; `recv` answers
  Ping with Pong.
- `SOCKET` global — `room(path) -> Option<Arc<SocketRoom>>`, `rooms()`,
  `online()` total. Named after the route keyword so autocomplete lines up
  (`SOCKET::room`, like `WORKER::run`).
- `WebSocket` — raw per-connection wrapper (escape hatch, see above).

### Config `[websocket]` (`AppConfig.websocket`, serde defaults)

```toml
[websocket]
max_message_size = 1048576     # 1 MiB; tungstenite's 64 MiB default is a DoS vector
max_frame_size   = 262144
ping_interval    = 30          # seconds; server-side keepalive, 0 disables
allowed_origins  = []          # empty = same-origin only (scheme+host+port must match Host); ["*"] disables the check
accept_subprotocols = []       # e.g. ["json"]; first client-requested match is echoed back
```

---

## 3. Security

- **Cross-Site WebSocket Hijacking.** Browsers send cookies on the
  handshake, so a session-authenticated socket opened from an attacker's
  page inherits the victim's session. CSRF tokens don't apply (no form).
  Defense is the `Origin` check: default same-origin, explicit allowlist
  otherwise. Refuse with 403 before upgrade. Non-browser clients send no
  `Origin`; allow absent Origin (they carry no ambient credentials).
- **Auth before upgrade.** `ctx.require_auth()` / any inbound middleware
  decision happens on the handshake; an `Err` from the handler produces the
  usual error response and no socket is created.
- **Size limits** from config, enforced by tungstenite; oversize → close
  `1009 Message Too Big`.
- **Connection count.** `RateLimitMiddleware` already runs on the handshake
  request, so per-IP handshake limits work unchanged. A per-IP *open
  connection* cap (`max_connections_per_ip`) is a v1.1 item.
- **Slow readers.** Sender uses a bounded mpsc; a client that never reads is
  closed when the buffer fills (`1008 Policy Violation`), never blocks the
  hub.

## 4. Graceful shutdown

Upgraded connections are no longer hyper connections, so
`GracefulShutdown::watch` does not see them. Keep a `TaskTracker` (or
`JoinSet` behind a mutex) in `Server`; on shutdown, broadcast close
`1001 Going Away` to every live socket, then wait for the tracker with the
existing drain timeout. Test it.

---

## 5. Implementation steps (end-user value first)

Each step is a reviewable commit; each has a failing test first.

1. **Handshake plumbing** — `Request.upgrade: Option<OnUpgrade>` taken in
   `from_hyper_with_connection` when `Upgrade: websocket`; `Request::is_websocket_upgrade()`.
   Unit test: header parsing; upgrade handle present only for upgrade requests.
2. **`websocket` module + `Cargo` feature** — `rustf/src/websocket/{mod,socket,message,handshake}.rs`;
   `tokio-tungstenite` dep behind `websocket` feature (default on);
   `WebSocket` wrapper with `recv/send_*/close/split`; `derive_accept_key`
   handshake validation returning `Result<Response>` (101 or 400/403/426).
3. **`Route::websocket` + `SOCKET` macro arm + router 426** — flag on `Route`,
   `routes!(@route SOCKET, ..)`, `execute_route_handler` refuses non-upgrade
   GET on socket routes with 426.
4. **`ctx.websocket(callback)` + spawn** — pending-upgrade slot on `Context`;
   after handler returns, `execute_route_handler` spawns the upgrade task
   and returns the 101. Callback `Err` → log + close 1011.
   **Wire test (own test binary):** echo route with `tokio-tungstenite`
   client as dev-dep: connect, send text, receive echo, close handshake.
   Also: unauthenticated → 401, no upgrade; bad Origin → 403; plain GET → 426.
5. **Config `[websocket]`** — struct + defaults + TOML merge test; size limits
   and ping interval applied to `WebSocketConfig`; ping task per connection.
   Test: oversize message → 1009.
6. **`SocketRoom` + `SocketClient` + `SOCKET` registry + `ctx.socket(..)`** —
   the Total.js layer on top of step 4: room per path, configure-once
   semantics, `on_open/on_message/on_close/on_error`, `send/send_except/send_to`,
   `autodestroy`, `SOCKET::room(path)`. Tests: two clients on
   `/chat/a`, one on `/chat/b`; `send_except` skips sender and never
   crosses rooms; `on_open`/`on_close` fire once per client; second
   handshake does not re-run `configure`; room disappears from `SOCKET::rooms()`
   after last leave; `SOCKET::room("/chat/a").send(..)` from an HTTP handler
   reaches the socket clients.
7. **Graceful shutdown** — tracker + `1001` on shutdown. Test: open socket,
   `handle.shutdown()`, client receives close 1001 within drain timeout.
8. **Prelude exports** — `SocketRoom`, `SocketClient`, `Message`, `CloseCode`,
   `SOCKET`, plus `WebSocket` for the raw path.
9. **CLI** — `rustf-cli new socket <name>` (or `new controller --socket`)
   emitting the `SOCKET` route + echo handler + a minimal `views/<name>/index.html`
   with an `EventSource`-style JS snippet for WebSocket; skill file rule:
   "real-time push one-way → `ctx.sse`; two-way → `SOCKET` route + `ctx.websocket`".
10. **Docs** — `docs/ABOUT_WEBSOCKET.md`, `book/src/advanced/websocket.md`
    (handshake flow, auth-before-upgrade, Origin policy, rooms via `ctx.socket` /
    `SOCKET::room`, Total.js → RustF mapping table, HTTP/1.1-only
    note, no-CSRF note), README feature moved out of "Future", CHANGELOG
    `### Added`. `rustf_feature_gaps` memory updated.
11. **Sample-app** — `/chat/{room}` demo using `ctx.socket` + session user name.

Estimate: 2–3 focused days including tests and docs. Steps 1–4 alone give a
working echo socket and are the first reviewable milestone.

---

## 6. Open decisions for Morle

1. **Ship in 1.0.0 or 1.1.0?** Additive, so 1.1 is safe. Argument for 1.0:
   "production framework" pitch gets asked "WebSocket?" immediately;
   argument against: 1.0 is already carrying the streaming break set and the
   Context API cleanup.
2. **Room layer (`ctx.socket` / `SocketRoom`) in v1?** Recommend yes (step 6):
   it is the Total.js model and what every chat/notification example needs.
   The raw `ctx.websocket` closure ships too, as the building block.
3. **Default feature on?** Recommend yes, consistent with `redis`.
4. **Event registration shape**: closures on the room
   (`room.on_message(|client, msg| async move {..})`, recommended — closest
   to `self.on('message', fn)` and zero ceremony) vs a
   `SocketHandler` trait with `on_open/on_message/on_close` methods
   implemented on a struct (more Rust-typical, but a struct + impl block per
   route is exactly the ceremony Total.js avoids). Closures first; trait as
   sugar later if asked.
5. **Route keyword**: `SOCKET` — settled by Total.js lineage
   (`ROUTE('SOCKET /path')`). Authorization stays on the existing `before:`
   hook / `require_auth()` instead of inventing `+SOCKET` / `-SOCKET`
   prefixes in the macro; mention the Total.js equivalence in docs.
6. **Per-route flags** (`['json']`, max length): via a builder on the route
   (`Route::websocket(..).json().max_message_size(1 << 20)`) or a macro
   suffix (`SOCKET "/chat" => chat, [json]`)? Recommend builder; the macro
   already has enough arms.

## 7. Out of scope (say so in docs)

- WebSocket over HTTP/2 / RFC 8441.
- Cross-process broadcast (Redis pub/sub) — future `SocketRoom` backend.
- permessage-deflate — can be enabled later via tungstenite config, off by
  default (compression + attacker-controlled input = CRIME-class risk).
- TLS termination — same story as HTTP: reverse proxy.

## 8. Acceptance

- `routes![SOCKET "/echo" => echo]` + `ctx.websocket(..)` echo works over a
  real socket from `tokio-tungstenite` client; close handshake clean.
- 401 / 403 / 426 paths verified on the wire; no upgrade task spawned.
- Oversize message closes with 1009; shutdown closes with 1001.
- Room tests green (`send_except`, cross-room isolation, configure-once,
  autodestroy, `SOCKET::room` from HTTP); `cargo test --all-features` green;
  `--no-default-features --features config` still builds (feature gate correct).
- Book + ABOUT doc (with a Total.js → RustF mapping table) + skill rule +
  CLI generator + sample-app room demo.
