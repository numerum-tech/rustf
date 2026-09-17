//! Turning a handler's intent into an actual upgraded connection.
//!
//! hyper only performs the upgrade *after* the `101` response has been
//! written, so a handler cannot be handed a live socket. It registers what to
//! do with one instead: [`Context::websocket`] validates the handshake, writes
//! the `101`, and parks a [`PendingUpgrade`] on the context. The dispatcher
//! picks that up once the handler returns and spawns the task that waits for
//! the upgrade to complete.

use crate::context::Context;
use crate::error::{Error, Result};
use crate::routing::SocketOptions;
use crate::websocket::handshake::{self, HandshakeOptions};
use crate::websocket::socket::WebSocket;
use futures::future::BoxFuture;
use std::time::Duration;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;

/// What to run once the connection has been upgraded.
pub(crate) type SocketTask = Box<dyn FnOnce(WebSocket) -> BoxFuture<'static, Result<()>> + Send>;

/// An upgrade the handler asked for, waiting for the `101` to go out.
pub(crate) struct PendingUpgrade {
    /// hyper's handle; resolves once the `101` has been written.
    pub(crate) on_upgrade: hyper::upgrade::OnUpgrade,
    /// Frame and message limits for the connection.
    pub(crate) config: WebSocketConfig,
    /// Keepalive period, `None` when disabled.
    pub(crate) ping_interval: Option<Duration>,
    /// The route's `[json]` flag, enforced by the read loop.
    pub(crate) json_only: bool,
    /// The request path, for log lines about a connection nobody can see.
    pub(crate) path: String,
    /// The handler's callback.
    pub(crate) task: SocketTask,
    /// Holds this connection's slot against `max_connections` until it ends.
    pub(crate) permit: ConnectionPermit,
    /// The `Sec-WebSocket-Accept` this handshake negotiated.
    ///
    /// Checked against the response that actually goes out, so a `101` whose
    /// upgrade headers were mangled by outbound middleware is not treated as a
    /// successful handshake.
    pub(crate) accept_key: String,
    /// The subprotocol negotiated, if any. Also checked against the final
    /// response: answering a protocol the client never offered makes a
    /// conforming client drop the connection.
    pub(crate) subprotocol: Option<String>,
}

impl std::fmt::Debug for PendingUpgrade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingUpgrade")
            .field("path", &self.path)
            .field("ping_interval", &self.ping_interval)
            .finish_non_exhaustive()
    }
}

impl PendingUpgrade {
    /// Wait for the upgrade, then run the handler's callback on the socket.
    ///
    /// Never returns an error: by the time this runs the `101` has been sent
    /// and there is no longer an HTTP response to put a failure in, so
    /// everything is logged instead.
    pub(crate) async fn run(self) {
        let PendingUpgrade {
            on_upgrade,
            config,
            ping_interval,
            json_only,
            path,
            task,
            // Held for the life of the connection; released when this returns.
            permit: _permit,
            accept_key: _,
            subprotocol: _,
        } = self;

        let upgraded = match on_upgrade.await {
            Ok(upgraded) => upgraded,
            Err(error) => {
                log::warn!("WebSocket upgrade failed for {path}: {error}");
                return;
            }
        };

        let socket =
            WebSocket::from_upgraded(upgraded, config, ping_interval, json_only).await;

        if let Err(error) = task(socket).await {
            log::error!("WebSocket handler for {path} ended with an error: {error}");
        }
    }
}

/// Build the tungstenite config for a route.
///
/// Per-route `[max = ..]` overrides the global `[websocket] max_message_size`;
/// the frame cap always comes from configuration, since a frame limit above
/// the message limit would be meaningless.
pub(crate) fn socket_config(
    settings: &crate::config::WebSocketSettings,
    route: &SocketOptions,
) -> WebSocketConfig {
    let max_message_size = route.max_message_size.unwrap_or(settings.max_message_size);

    WebSocketConfig::default()
        .max_message_size(Some(max_message_size))
        // A frame may never need to be larger than a whole message.
        .max_frame_size(Some(settings.max_frame_size.min(max_message_size)))
}

impl Context {
    /// Accept a WebSocket connection on this route and drive it yourself.
    ///
    /// The raw escape hatch. `callback` receives the live [`WebSocket`] once
    /// the upgrade completes, and owns the connection until it returns.
    ///
    /// ```ignore
    /// async fn echo(ctx: &mut Context) -> rustf::Result<()> {
    ///     ctx.websocket(|mut socket| async move {
    ///         while let Some(message) = socket.recv().await {
    ///             socket.send(message?).await?;
    ///         }
    ///         Ok(())
    ///     })
    /// }
    /// ```
    ///
    /// Three things to know:
    ///
    /// * **Call it last.** It writes the `101` response; anything that
    ///   replaces `ctx.res` afterwards is written to a connection the client
    ///   is already speaking WebSocket over.
    /// * **Rejections are ordinary responses.** A bad handshake, a foreign
    ///   origin or a plain `GET` produce `400` / `403` / `426` and no socket.
    ///   `Ok(())` comes back either way — check [`Context::is_upgrading`] if
    ///   the distinction matters.
    /// * **Errors after this point cannot reach the client.** Once the
    ///   upgrade is under way, a `callback` error is logged, not returned.
    ///
    /// For the Total.js room model — one shared room per URL, with
    /// `on_open`/`on_message`/`on_close` — use
    /// [`Context::socket`](Context::socket) instead.
    pub fn websocket<F, Fut>(&mut self, callback: F) -> Result<()>
    where
        F: FnOnce(WebSocket) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        self.accept_upgrade(Box::new(move |socket| Box::pin(callback(socket))))
    }

    /// Accept a WebSocket connection into the room for this URL.
    ///
    /// The Total.js model. RustF keeps one [`SocketRoom`] per matched path;
    /// `configure` runs for the first client to arrive on that path, and
    /// everyone after joins the room it set up.
    ///
    /// Once per *room*, not once per process: a room with `autodestroy` on
    /// (the default) disappears when its last client leaves, and the next
    /// connection to that URL configures a new one. A panicking closure has the
    /// same effect. Keep external side effects out of it.
    ///
    /// ```ignore
    /// pub fn install() -> Vec<Route> {
    ///     routes![
    ///         GET    "/chat"        => page,
    ///         SOCKET "/chat/{room}" => chat,
    ///     ]
    /// }
    ///
    /// async fn chat(ctx: &mut Context) -> rustf::Result<()> {
    ///     ctx.socket(|room| {
    ///         room.on_open(|client| async move {
    ///             client.send_json(&json!({ "type": "welcome", "id": client.id() }))
    ///         });
    ///         room.on_message(|client, message| async move {
    ///             client.room().send_except(message, client.id())
    ///         });
    ///     })
    /// }
    /// ```
    ///
    /// The same three rules as [`websocket`](Context::websocket) apply: call
    /// it last, rejections come back as ordinary `4xx` responses, and errors
    /// after the upgrade are logged rather than returned.
    ///
    /// Reach the same room from anywhere else — an HTTP handler, a worker, a
    /// scheduled job — with [`SOCKET::room`](crate::websocket::SOCKET::room).
    pub fn socket<F>(&mut self, configure: F) -> Result<()>
    where
        F: FnOnce(&crate::websocket::room::SocketRoom) + Send + 'static,
    {
        // Snapshot what the handler's closures may still need: the request is
        // gone long before the connection is.
        let handshake = crate::websocket::room::Handshake {
            params: self.req.params.clone(),
            query: self.req.query.clone(),
            headers: self.req.headers.clone(),
            ip: self.req.client_ip(),
            session: self.session_arc().cloned(),
        };
        let path = self.req.path().to_string();
        let json_only = self.socket_options.map(|o| o.json).unwrap_or(false);
        let ping_interval = self.configured_ping_interval();
        let configure: Box<dyn FnOnce(&crate::websocket::room::SocketRoom) + Send> =
            Box::new(configure);

        self.accept_upgrade(Box::new(move |socket| {
            Box::pin(crate::websocket::room::run(
                socket,
                path,
                handshake,
                configure,
                ping_interval,
                json_only,
            ))
        }))
    }

    /// The keepalive period from `[websocket] ping_interval`, `None` when off.
    fn configured_ping_interval(&self) -> Option<Duration> {
        let seconds = crate::configuration::CONF::all()
            .map(|config| config.websocket.ping_interval)
            .unwrap_or_else(|| crate::config::WebSocketSettings::default().ping_interval);

        match seconds {
            0 => None,
            seconds => Some(Duration::from_secs(seconds)),
        }
    }

    /// Is a WebSocket upgrade queued on this context?
    ///
    /// True after a [`websocket`](Context::websocket) or
    /// [`socket`](Context::socket) call that accepted the handshake; false
    /// when it was rejected and `ctx.res` holds the error response.
    pub fn is_upgrading(&self) -> bool {
        self.pending_upgrade.is_some()
    }

    /// Shared body of `websocket` and `socket`.
    pub(crate) fn accept_upgrade(&mut self, task: SocketTask) -> Result<()> {
        // Config comes from the global CONF, as everywhere else in Context —
        // a handler has no application handle to ask. Uninitialised (unit
        // tests, benches) falls back to the documented defaults.
        let settings = crate::configuration::CONF::all()
            .map(|config| config.websocket.clone())
            .unwrap_or_default();

        let options = HandshakeOptions {
            allowed_origins: settings.allowed_origins.clone(),
            accept_subprotocols: settings.accept_subprotocols.clone(),
            require_origin: settings.require_origin,
        };

        // Refuse once shutdown has begun. A keep-alive HTTP connection that was
        // accepted before the drain started could otherwise upgrade after it,
        // and the new socket would outlive the server.
        if is_shutting_down() {
            self.set_response(service_unavailable(
                "The server is shutting down and is not accepting WebSocket connections.",
            ));
            return Ok(());
        }

        // A connection is cheap to open and long-lived to hold, so without a
        // cap one client can occupy every task and queue the process has. The
        // slot is taken now and released by `Drop` if anything below refuses
        // the handshake.
        let Some(permit) = ConnectionPermit::acquire(settings.max_connections) else {
            log::warn!(
                "Refusing a WebSocket handshake for {}: at the configured limit of {} \
                 connections",
                self.req.path(),
                settings.max_connections
            );
            self.set_response(service_unavailable(
                "The server is at its WebSocket connection limit.",
            ));
            return Ok(());
        };

        let accepted = match handshake::validate(&self.req, &options) {
            Ok(accepted) => accepted,
            Err(response) => {
                self.set_response(response);
                return Ok(());
            }
        };

        // Refuse to upgrade from a route that was not declared `SOCKET`. The
        // handshake itself would succeed, but the route would have none of the
        // protections that come with the keyword — most importantly, a plain
        // `GET` would run the handler instead of getting a `426`.
        //
        // Checked before the upgrade handle is taken, so this error path
        // leaves the request intact.
        let route_options = match self.socket_options {
            Some(options) => options,
            None => return Err(not_a_socket_route(self.req.path())),
        };

        // Headers can describe a handshake that this connection cannot serve:
        // HTTP/2 carries no upgrade token, and hyper only offers one on
        // HTTP/1.1. Say so plainly rather than emitting a `101` onto a
        // connection that will never switch protocols.
        let on_upgrade = match self.req.take_upgrade() {
            Some(on_upgrade) => on_upgrade,
            None => {
                self.set_response(handshake::upgrade_required(
                    "WebSocket requires HTTP/1.1; this connection cannot be upgraded.",
                ));
                return Ok(());
            }
        };

        self.pending_upgrade = Some(PendingUpgrade {
            on_upgrade,
            config: socket_config(&settings, &route_options),
            json_only: route_options.json,
            ping_interval: match settings.ping_interval {
                0 => None,
                seconds => Some(Duration::from_secs(seconds)),
            },
            path: self.req.path().to_string(),
            task,
            permit,
            accept_key: accepted.accept_key.clone(),
            subprotocol: accepted.subprotocol.clone(),
        });

        self.set_response(handshake::accept_response(&accepted));
        Ok(())
    }
}

/// Every upgraded connection currently being served, by id.
///
/// Upgraded sockets leave hyper's control the moment the `101` goes out, so
/// `GracefulShutdown` never sees them. Holding the `JoinHandle`s is what makes
/// shutdown able to *finish*: a raw `ctx.websocket` handler owns its own loop
/// and cannot be closed from outside, so after the grace period the task is
/// aborted rather than merely logged about.
static LIVE_SOCKETS: once_cell::sync::Lazy<
    dashmap::DashMap<u64, tokio::task::JoinHandle<()>>,
> = once_cell::sync::Lazy::new(dashmap::DashMap::new);

/// Woken whenever a connection ends, so the drain does not have to poll.
static SOCKETS_DRAINED: once_cell::sync::Lazy<tokio::sync::Notify> =
    once_cell::sync::Lazy::new(tokio::sync::Notify::new);

static NEXT_SOCKET_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// Connections reserved or live, counted for `[websocket] max_connections`.
///
/// Reserved at accept time rather than counted at spawn time: two handshakes
/// arriving together would both see room under the limit and both be accepted,
/// so a limit checked by observation is not a limit.
static RESERVED_SOCKETS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// A reserved connection slot, released when dropped — whether the handshake
/// is rejected later, the response is replaced, or the connection ends.
pub(crate) struct ConnectionPermit;

impl ConnectionPermit {
    /// Take a slot, or `None` when the server is already at `limit`.
    ///
    /// `limit` of 0 means no limit, and then no counting is done at all.
    fn acquire(limit: usize) -> Option<Self> {
        if limit == 0 {
            return Some(Self);
        }

        let mut current = RESERVED_SOCKETS.load(std::sync::atomic::Ordering::Acquire);
        loop {
            if current >= limit {
                return None;
            }
            match RESERVED_SOCKETS.compare_exchange_weak(
                current,
                current + 1,
                std::sync::atomic::Ordering::AcqRel,
                std::sync::atomic::Ordering::Acquire,
            ) {
                Ok(_) => return Some(Self),
                Err(actual) => current = actual,
            }
        }
    }
}

impl Drop for ConnectionPermit {
    fn drop(&mut self) {
        // Saturating: a zero-limit permit never incremented, and underflow here
        // would silently disable the cap for the life of the process.
        let _ = RESERVED_SOCKETS.fetch_update(
            std::sync::atomic::Ordering::AcqRel,
            std::sync::atomic::Ordering::Acquire,
            |current| Some(current.saturating_sub(1)),
        );
    }
}

/// Set once shutdown begins. No further handshake is accepted after this.
static SHUTTING_DOWN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// How many upgraded connections are currently being served.
pub fn live_socket_count() -> usize {
    LIVE_SOCKETS.len()
}

/// Is the server refusing new WebSocket connections?
pub fn is_shutting_down() -> bool {
    SHUTTING_DOWN.load(std::sync::atomic::Ordering::SeqCst)
}

/// Spawn a connection and keep its handle until it finishes.
///
/// Returns `false` when shutdown began before the connection could be
/// registered; the caller must then refuse the handshake instead of sending the
/// `101`.
///
/// The ordering is what makes that safe. This registers, then checks the
/// shutdown flag; `shutdown_sockets` sets the flag, then reads the registry.
/// One of the two must observe the other, so a connection can neither escape
/// the drain nor be started after it.
///
/// The `ready` gate exists so the handle is in the map before the task can
/// remove itself: without it, a connection that ends immediately would leave a
/// finished handle behind forever.
pub(crate) fn spawn_tracked(pending: PendingUpgrade) -> bool {
    let id = NEXT_SOCKET_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel::<()>();

    let handle = tokio::spawn(async move {
        let _ = ready_rx.await;
        // Deregistration through `Drop` so it happens on every exit, including
        // a panic in an application callback. A handle left behind would hold a
        // connection slot and make every later shutdown wait out its timeout.
        let _registration = Registration(id);
        pending.run().await;
    });

    LIVE_SOCKETS.insert(id, handle);

    if is_shutting_down() {
        // Shutdown started while this was being registered. Back out: the
        // drain may already have decided the server was empty.
        if let Some((_, handle)) = LIVE_SOCKETS.remove(&id) {
            handle.abort();
        }
        SOCKETS_DRAINED.notify_waiters();
        return false;
    }

    let _ = ready_tx.send(());
    true
}

/// Removes a connection from the live set when its task ends, however it ends.
struct Registration(u64);

impl Drop for Registration {
    fn drop(&mut self) {
        LIVE_SOCKETS.remove(&self.0);
        SOCKETS_DRAINED.notify_waiters();
    }
}

/// Close every socket and wait for the connections to finish.
///
/// Room sockets get a proper `1001 Going Away` close frame. A raw
/// [`Context::websocket`](crate::Context::websocket) handler owns its own loop
/// and cannot be closed from outside, so anything still running after
/// `timeout` is aborted.
///
/// Sets the shutting-down flag first, so a handshake arriving on an already
/// accepted keep-alive connection is refused rather than escaping the drain.
pub async fn shutdown_sockets(timeout: Duration) {
    SHUTTING_DOWN.store(true, std::sync::atomic::Ordering::SeqCst);

    crate::websocket::room::SOCKET::shutdown_all();

    if live_socket_count() == 0 {
        return;
    }

    log::info!("Closing {} WebSocket connection(s)...", live_socket_count());

    let drained = async {
        while live_socket_count() > 0 {
            // Register for the wake-up *before* re-reading the count, so a
            // connection that ends in between cannot be missed.
            let notified = SOCKETS_DRAINED.notified();
            if live_socket_count() == 0 {
                break;
            }
            notified.await;
        }
    };

    if tokio::time::timeout(timeout, drained).await.is_err() {
        let remaining: Vec<u64> = LIVE_SOCKETS.iter().map(|entry| *entry.key()).collect();
        log::warn!(
            "{} WebSocket connection(s) did not finish within the shutdown timeout; \
             aborting them. A handler written against `ctx.websocket` should return \
             when `recv()` yields `None`.",
            remaining.len()
        );

        for id in remaining {
            if let Some((_, handle)) = LIVE_SOCKETS.remove(&id) {
                handle.abort();
            }
        }
    }
}

/// Re-open the server to WebSocket connections.
///
/// Called when a server starts listening. The shutting-down flag is global, so
/// without this a process that starts a second server after stopping the first
/// — which integration tests do — would refuse every handshake forever.
pub(crate) fn reset_shutdown_state() {
    SHUTTING_DOWN.store(false, std::sync::atomic::Ordering::SeqCst);
}

impl PendingUpgrade {
    /// Is the response that is actually going out still the handshake we
    /// negotiated?
    ///
    /// Outbound middleware runs after the handler and can rewrite anything.
    /// A `101` whose `Upgrade`, `Connection` or `Sec-WebSocket-Accept` has been
    /// changed is one the client will reject, so upgrading behind it would
    /// leave a task waiting on a connection that never switches protocols.
    pub(crate) fn matches(&self, response: &crate::http::Response) -> bool {
        // Every occurrence, not the first: `Response::with_header` appends, so
        // middleware can add a second `Sec-WebSocket-Accept` that a client
        // would see and reject while a first-match check found nothing wrong.
        let values = |name: &str| -> Vec<&str> {
            response
                .headers
                .iter()
                .filter(|(key, _)| key.eq_ignore_ascii_case(name))
                .map(|(_, value)| value.as_str())
                .collect()
        };

        /// A header that must appear exactly once, with this value.
        fn exactly_one(values: &[&str], expected: &str) -> bool {
            values.len() == 1 && values[0].eq_ignore_ascii_case(expected)
        }

        if response.status != hyper::StatusCode::SWITCHING_PROTOCOLS {
            return false;
        }

        if !exactly_one(&values("Upgrade"), "websocket") {
            return false;
        }

        // `Sec-WebSocket-Accept` is compared byte-for-byte: it is base64, and
        // case is significant.
        let accept = values("Sec-WebSocket-Accept");
        if accept.len() != 1 || accept[0] != self.accept_key {
            return false;
        }

        // `Connection` is a token list, and may legitimately be split across
        // several header lines, so gather every token from all of them.
        let has_upgrade_token = values("Connection")
            .iter()
            .flat_map(|value| value.split(','))
            .any(|token| token.trim().eq_ignore_ascii_case("upgrade"));
        if !has_upgrade_token {
            return false;
        }

        // Exactly what was negotiated, no more: a subprotocol the client never
        // offered, or an extension nothing agreed to, is a handshake the client
        // is entitled to reject.
        let protocol = values("Sec-WebSocket-Protocol");
        let protocol_matches = match self.subprotocol.as_deref() {
            Some(negotiated) => protocol.len() == 1 && protocol[0] == negotiated,
            None => protocol.is_empty(),
        };

        protocol_matches && values("Sec-WebSocket-Extensions").is_empty()
    }
}

/// `503 Service Unavailable` — the handshake was refused for capacity reasons.
fn service_unavailable(message: &str) -> crate::http::Response {
    crate::http::Response::new(hyper::StatusCode::SERVICE_UNAVAILABLE)
        .with_header("Content-Type", "text/plain; charset=utf-8")
        .with_body(message.as_bytes().to_vec())
}

/// Error used when a handler asks for a socket outside a `SOCKET` route.
pub(crate) fn not_a_socket_route(path: &str) -> Error {
    Error::WebSocket(format!(
        "`{path}` is not a SOCKET route; declare it with \
         `routes![SOCKET \"{path}\" => handler]`"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_zero_limit_means_no_limit() {
        let permits: Vec<ConnectionPermit> =
            (0..1000).filter_map(|_| ConnectionPermit::acquire(0)).collect();
        assert_eq!(permits.len(), 1000);
    }

    #[test]
    fn permits_are_capped_and_released_on_drop() {
        // Serialised against the other test that touches the global counter by
        // using a limit high enough that they cannot interfere: this test only
        // asserts on its own deltas.
        let base = RESERVED_SOCKETS.load(std::sync::atomic::Ordering::Acquire);
        let limit = base + 2;

        let first = ConnectionPermit::acquire(limit);
        let second = ConnectionPermit::acquire(limit);
        assert!(first.is_some() && second.is_some());

        assert!(
            ConnectionPermit::acquire(limit).is_none(),
            "a third connection must be refused at the limit"
        );

        drop(second);
        let third = ConnectionPermit::acquire(limit);
        assert!(
            third.is_some(),
            "dropping a permit must free the slot it held"
        );

        drop(first);
        drop(third);
        assert_eq!(
            RESERVED_SOCKETS.load(std::sync::atomic::Ordering::Acquire),
            base,
            "every permit must return its slot"
        );
    }
}
