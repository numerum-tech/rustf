# WebSocket

RustF speaks WebSocket through one route keyword and one `Context` method, in
the shape Total.js gave the idea: **one room per URL**, configured once, shared
by everyone who connects to it.

```rust
use rustf::prelude::*;

pub fn install() -> Vec<Route> {
    routes![
        GET    "/chat"        => page,     // the page
        SOCKET "/chat/{room}" => chat,     // the socket
    ]
}

async fn chat(ctx: &mut Context) -> rustf::Result<()> {
    ctx.socket(|room| {
        room.on_open(|client| async move {
            client.send_json(&json!({ "type": "welcome", "id": client.id() }))
        });
        room.on_message(|client, message| async move {
            client.room().send_except(message, client.id())
        });
        room.on_close(|client, _code| async move {
            client.room().send_json(&json!({ "type": "left", "id": client.id() }))
        });
    })
}
```

Generate that, plus a page and a browser client, with:

```bash
rustf-cli new socket --name chat
```

## The `SOCKET` route keyword

`SOCKET "/chat/{room}" => chat` registers a **GET** route — the opening
handshake *is* a GET — and marks it as a socket. What changes is the dispatch:

| Request | Result |
|---|---|
| `GET /chat/general` with `Upgrade: websocket` | handshake, then `101` |
| `GET /chat/general` without it | `426 Upgrade Required`, handler never runs |
| Handshake with a version other than 13 | `426`, advertising `Sec-WebSocket-Version: 13` |
| Handshake with a missing or malformed `Sec-WebSocket-Key` | `400` |
| Handshake from a disallowed origin | `403` |

Route parameters, query strings, middleware and the `before:` hook all work
exactly as on any other route, because until the `101` this *is* an ordinary
HTTP request.

### Per-route flags

Flags follow the handler, mirroring Total.js's
`ROUTE('SOCKET /chat/', fn, ['json'], 1024)`:

```rust
routes![
    SOCKET "/chat"  => chat,                          // defaults
    SOCKET "/feed"  => feed, [json],                  // payload must be JSON
    SOCKET "/bulk"  => bulk, [json, max = 1 << 20],   // and a 1 MiB cap
    SOCKET "/raw"   => raw,  [text],                  // explicit default
]
```

- `json` — a message whose payload is not valid JSON closes the connection with
  `1003 Unsupported Data`; your handler never sees it. **Both** frame types are
  checked, so the flag cannot be sidestepped with a different opcode — and by
  the same token a binary frame carrying valid JSON is accepted and arrives as
  `Message::Binary`. Use `msg.parse::<T>()`, which reads either.
- `text` — the default: messages arrive as sent.
- `max = <bytes>` — overrides `[websocket] max_message_size` for this route.
  Anything larger closes with `1009 Message Too Big`.

## Authorization

Total.js writes `'+SOCKET'` for "authorized only". RustF has no new syntax for
this, because the handshake is a normal request and the existing tools already
apply:

```rust
async fn chat(ctx: &mut Context) -> rustf::Result<()> {
    if ctx.session().map(|s| s.is_authenticated()) != Some(true) {
        return ctx.throw401(Some("Sign in first"));
    }

    ctx.socket(|room| { /* ... */ })
}
```

Or, for a whole controller, the `before:` hook:

```rust
routes![
    before: require_login,
    SOCKET "/chat/{room}" => chat,
]
```

Either way the check runs **before** any upgrade: the client gets a `401` and
no socket is created.

## Rooms

`ctx.socket(configure)` looks up the room for the *matched path* — `/chat/general`,
not `/chat/{room}` — creating it if this is the first client. `configure` runs
once per room, exactly like the Total.js action. A second client on the same URL
joins the room the first one configured.

Once per *room*, though, not once per process. With `autodestroy` on — the
default — a room disappears when its last client leaves, so the next connection
to that URL creates and configures a new one. A configure closure that panics
ends its room the same way. Keep external side effects out of it.

One more detail: the room is published to `SOCKET` *while* it is being
configured, which is what lets a configure closure call `SOCKET::room`. A
registry lookup can therefore briefly see a room whose handlers are not
installed yet, though joining always waits.

### `SocketRoom`

| Method | What it does |
|---|---|
| `send(msg)` | to everyone |
| `send_except(msg, id)` | to everyone but one — Total.js `self.send(msg, sender)` |
| `send_to(id, msg)` | to one client |
| `send_json(&v)` / `send_json_except(&v, id)` | the same, serialised as JSON |
| `online()` / `is_empty()` | membership count |
| `client_ids()` / `find(id)` | membership |
| `autodestroy(bool)` | destroy when the last client leaves (**on** by default) |
| `destroy()` | close everyone with `1001` and unregister |
| `path()` | the URL this room serves |

Sends never block. Each client has a bounded outbound queue; a client that has
stopped reading is disconnected with `1008 Policy Violation` rather than
allowed to stall the broadcast.

### `SocketClient`

`on_close` and `on_error` are **best effort and bounded at five seconds**: they
run while the connection is being torn down, so one that takes longer is
cancelled at its next await point and the fact logged. Cleanup that must
complete belongs in a `WORKER`.

`id()`, `room()`, `params()` / `param(name)`, `query()` / `query_param(name)`,
`headers()` / `header(name)`, `ip()`, `session()`, `send()`, `send_text()`,
`send_json()`, `close(code, reason)`.

The handshake data is a snapshot taken before the upgrade — the `Request` is
long gone by the time the connection is live.

### Reaching a room from elsewhere

`SOCKET` is RustF's `F.connections[url]`. Use it from an HTTP handler, a
worker, a scheduled job — anywhere:

```rust
if let Some(room) = SOCKET::room("/chat/general") {
    room.send_json(&json!({ "type": "announcement", "text": text }))?;
}
```

`SOCKET::rooms()`, `SOCKET::paths()` and `SOCKET::online()` report what is
live.

## The raw socket

When you want the connection and not the room — a protocol bridge, a one-way
feed — take it directly:

```rust
async fn echo(ctx: &mut Context) -> rustf::Result<()> {
    ctx.websocket(|mut socket| async move {
        while let Some(message) = socket.recv().await {
            socket.send(message?).await?;
        }
        Ok(())
    })
}
```

`recv()` yields only text and binary messages: pings are answered
automatically and a close frame ends the loop. `split()` gives independent
sending and receiving halves for a handler that pushes and reads at once — but
see the note on keepalive below.

## Three rules for handlers

1. **Call `ctx.socket` / `ctx.websocket` last.** It writes the `101`. Anything
   that replaces `ctx.res` afterwards is written to a connection already
   speaking WebSocket; RustF logs it and discards the upgrade rather than
   leaving it hanging.
2. **A rejected handshake is an ordinary response.** `Ok(())` comes back either
   way; check `ctx.is_upgrading()` if the distinction matters.
3. **Errors after the upgrade cannot reach the client.** There is no HTTP
   response left to put them in, so they are logged.

## Configuration

```toml
[websocket]
max_message_size    = 1048576   # 1 MiB. tungstenite's own default is 64 MiB
max_frame_size      = 262144    # 256 KiB
ping_interval       = 30        # seconds; 0 disables the keepalive
allowed_origins     = []        # empty = same-origin only; ["*"] disables the check
accept_subprotocols = []        # e.g. ["json"]; case-sensitive; first match is echoed
require_origin      = false     # refuse a handshake that carries no Origin at all
max_connections     = 0         # cap on concurrent sockets; 0 = no limit
```

`max_connections` is worth setting on anything public. A handshake is cheap and
a connection is long-lived, so without a cap one client can hold open as many
tasks and outbound queues as it likes; over the limit a handshake gets `503`.

## Security

**Cross-Site WebSocket Hijacking is the one to know about.** The handshake is
a plain GET, so a browser attaches the visitor's cookies to it — and CSRF
tokens do not apply, because there is no form. The `Origin` check is the whole
defence:

- Default: **same origin only**. Scheme, host and port must match the `Host`
  the request was sent to.
- `allowed_origins` *extends* that default, so naming a partner origin does not
  lock out your own pages.
- `Origin: null` — sandboxed iframes, `file://` pages — is never treated as
  same-origin. List it explicitly if you really mean it.
- A request with **no** `Origin` is allowed by default: that is normally a
  non-browser client, which attaches no cookies of its own. It is a default,
  not a guarantee — a reverse proxy can strip `Origin` while still forwarding
  `Cookie`. Set `require_origin = true` on an endpoint that is only ever opened
  by a browser.
- `allowed_origins = ["*"]` turns the check off. Only do that for an endpoint
  that carries no ambient authority.

Also worth knowing: `RateLimitMiddleware` runs on the handshake, so per-IP
handshake limits work unchanged, and `max_connections` caps concurrent sockets
server-wide. A *per-IP* concurrent-connection cap is not built in yet.

A stalled client's outbound queue holds up to 64 messages, counted as messages
rather than bytes — with the default 1 MiB cap that is 64 MiB for one stalled
client in the worst case. Size `max_message_size` for what your routes actually
carry, and `max_connections` for how many such clients you can afford.

## Limits and things that are not here

- **HTTP/1.1 only.** WebSocket over HTTP/2 (RFC 8441 extended CONNECT) is not
  supported by hyper's server. Browsers open WebSocket over HTTP/1.1 anyway.
- **Keepalive pings**: the room layer sends them from each connection's writer
  task, and closes the connection with `1001` when nothing at all comes back
  between two pings — which is how a half-open connection is actually detected,
  since a TCP write to a vanished peer succeeds for a long time. A raw
  `ctx.websocket` socket pings while the handler waits in `recv()`; one that
  only sends, or that has `split()` the socket, gets none.
- **A slow `on_message` blocks that connection's reads.** Handlers are awaited
  inline, in order, so while one runs that client's later messages and its
  close frame are not read. Other clients are unaffected and keepalive pings
  keep flowing, because they come from the connection's separate writer task.
  Move slow work to a `WORKER` if the ordering guarantee is not worth it.
  A handler that never returns is cancelled when the connection ends for any
  reason the writer can see (unanswered pings, a close, shutdown), so it cannot
  hold a connection open forever.
- **Outbound middleware does not see the connection.** Logging and timing
  middleware measure the handler, which ends at the `101`; the minutes or hours
  that follow are not observed.
- **No cross-process broadcast.** `SOCKET::room` reaches sockets on *this*
  instance. Fanning out across a cluster needs a Redis pub/sub backend, which
  is not built yet.
- **No permessage-deflate.** Compression over attacker-controlled input is a
  CRIME-class risk; it is off and not configurable.
- **TLS** is terminated by your reverse proxy, as with plain HTTP.

## Shutdown

On graceful shutdown RustF stops accepting new handshakes (a late one on an
existing keep-alive connection gets `503`), closes every room socket with
`1001 Going Away`, and waits for the connections to finish, bounded by
`[server] shutdown_timeout`. A close waits up to 5 seconds for the peer's half
of the closing handshake before the socket is dropped.

A raw `ctx.websocket` handler owns its own loop and cannot be closed from
outside, so write it to return when `recv()` yields `None`. Anything still
running when the shutdown timeout expires is aborted.

## Total.js → RustF

| Total.js | RustF |
|---|---|
| `ROUTE('SOCKET /chat/{room}/', action, ['json'], 1024)` | `routes![SOCKET "/chat/{room}" => chat, [json, max = 1024]]` |
| `'+SOCKET'` / `'-SOCKET'` | `ctx.throw401(..)` in the handler, or `routes![before: require_login, ...]` |
| `this` inside the action | the `&SocketRoom` passed to `ctx.socket` |
| `self.on('open', fn)` | `room.on_open(\|client\| async move { .. })` |
| `self.on('message', fn)` | `room.on_message(\|client, message\| async move { .. })` |
| `self.on('close', fn)` | `room.on_close(\|client, code\| async move { .. })` |
| `self.on('error', fn)` | `room.on_error(\|client, error\| async move { .. })` |
| `self.send(msg, blacklist)` | `room.send_except(msg, id)` |
| `self.connections` / `self.online` | `room.client_ids()` / `room.online()` |
| `self.find(id)` | `room.find(id)` |
| `self.autodestroy()` | `room.autodestroy(true)` — already the default |
| `self.destroy()` | `room.destroy()` |
| `client.send(msg)` | `client.send(msg)` |
| `client.query` / `client.params` | `client.query()` / `client.params()` |
| `client.user` / `client.session` | `client.session()` |
| `F.connections['/chat/room1/']` | `SOCKET::room("/chat/room1")` |

## Feature flag

WebSocket is behind the `websocket` feature, **on by default**. Building with
`--no-default-features` drops `tokio-tungstenite` and removes `Route::websocket`,
`ctx.socket` and `ctx.websocket` — a `SOCKET` route then fails to compile.
