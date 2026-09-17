//! The Total.js socket model: one room per URL, shared by everyone on it.
//!
//! In Total.js, `ROUTE('SOCKET /chat/{room}/', action)` runs `action` once,
//! when the first client connects to a given URL, and `this` inside it *is*
//! the room. Later clients on the same URL join the room that action
//! configured. [`SocketRoom`] is that object, and
//! [`Context::socket`](crate::Context::socket) is that route.
//!
//! ```ignore
//! async fn chat(ctx: &mut Context) -> rustf::Result<()> {
//!     ctx.socket(|room| {
//!         room.on_open(|client| async move {
//!             client.room().send_json_except(
//!                 &json!({ "type": "joined", "id": client.id() }),
//!                 client.id(),
//!             )
//!         });
//!         room.on_message(|client, message| async move {
//!             client.room().send_except(message, client.id())
//!         });
//!     })
//! }
//! ```

use crate::error::{Error, Result};
use crate::session::Session;
use crate::websocket::message::{CloseCode, Message};
use crate::websocket::socket::{Received, WebSocket, WebSocketSender, CLOSE_HANDSHAKE_TIMEOUT};
use dashmap::mapref::entry::Entry;
use dashmap::DashMap;
use futures::future::BoxFuture;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{mpsc, watch};

/// Identifies one connection within a room. Unique per room, not globally.
pub type ClientId = u64;

/// How many messages may be queued for one client before it is disconnected.
///
/// A client that stops reading must not be able to make the server buffer
/// without limit, and must not stall the senders fanning out to everyone else.
///
/// This is a *message* count, not a byte budget: with the default 1 MiB
/// message cap the worst case is 64 MiB queued for a single stalled client, so
/// size the cap in `[websocket] max_message_size` for what your routes
/// actually carry, and use `[websocket] max_connections` to bound how many
/// clients can be in that state at once.
const CLIENT_QUEUE_CAPACITY: usize = 64;

/// What the per-connection writer task is asked to send.
#[derive(Debug)]
pub(crate) enum Outbound {
    Send(Message),
}

/// Why a connection's writer stopped.
#[derive(Debug)]
enum WriterOutcome {
    /// We sent a close frame with this code; the peer may still reply.
    ClosedByUs(CloseCode),
    /// Every handle on the outbound queue was dropped.
    QueueClosed,
    /// Writing failed. The connection is gone.
    Failed(Error),
}

type OpenHandler = Arc<dyn Fn(SocketClient) -> BoxFuture<'static, Result<()>> + Send + Sync>;
type MessageHandler =
    Arc<dyn Fn(SocketClient, Message) -> BoxFuture<'static, Result<()>> + Send + Sync>;
type CloseHandler =
    Arc<dyn Fn(SocketClient, CloseCode) -> BoxFuture<'static, Result<()>> + Send + Sync>;
type ErrorHandler = Arc<dyn Fn(SocketClient, Error) -> BoxFuture<'static, Result<()>> + Send + Sync>;

#[derive(Default, Clone)]
struct Handlers {
    open: Option<OpenHandler>,
    message: Option<MessageHandler>,
    close: Option<CloseHandler>,
    error: Option<ErrorHandler>,
}

/// A room: every client connected to one URL.
///
/// Created on the first handshake for a path and configured once per
/// *generation*, before any client joins it.
///
/// A generation ends when the room does: with `autodestroy` on (the default) a
/// room disappears when its last client leaves, so the next connection to that
/// URL creates and configures a new one. A configure closure that panics also
/// ends its generation. Either way the closure can run more than once over the
/// life of the process — keep external side effects out of it.
///
/// The room is published to [`SOCKET`] *while* it is being configured — that is
/// what lets a configure closure use `SOCKET::room` — so a registry lookup can
/// briefly see a room whose handlers are not installed yet. Only joining
/// waits.
pub struct SocketRoom {
    path: String,
    clients: DashMap<ClientId, SocketClient>,
    /// Written once, during configuration; read on every event.
    ///
    /// A `Mutex` rather than a lock-free cell because registration happens
    /// once per room and reads clone an `Arc` out of it — the contention that
    /// would justify anything cleverer never materialises.
    handlers: Mutex<Handlers>,
    autodestroy: AtomicBool,
    next_id: AtomicU64,
    /// How far configuration has got: [`CONFIGURING`], [`CONFIGURED`] or
    /// [`CONFIGURE_FAILED`].
    ///
    /// Configuration is *not* done under the registry lock: the closure is
    /// application code and may call back into `SOCKET::room`, which would
    /// deadlock on the shard it is already holding. So the room is published
    /// first and joiners wait on this instead.
    state: std::sync::atomic::AtomicU8,
    ready_notify: tokio::sync::Notify,
}

/// The configure closure has not finished.
const CONFIGURING: u8 = 0;
/// The configure closure finished; the room is usable.
const CONFIGURED: u8 = 1;
/// The configure closure panicked. The room is unusable and must not be joined.
const CONFIGURE_FAILED: u8 = 2;

impl std::fmt::Debug for SocketRoom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SocketRoom")
            .field("path", &self.path)
            .field("online", &self.online())
            .field("autodestroy", &self.autodestroy.load(Ordering::Relaxed))
            .finish()
    }
}

impl SocketRoom {
    fn new(path: String) -> Self {
        Self {
            path,
            clients: DashMap::new(),
            handlers: Mutex::new(Handlers::default()),
            // Total.js rooms are transient by default; a room nobody is in is
            // just a leak waiting to happen.
            autodestroy: AtomicBool::new(true),
            next_id: AtomicU64::new(1),
            state: std::sync::atomic::AtomicU8::new(CONFIGURING),
            ready_notify: tokio::sync::Notify::new(),
        }
    }

    /// Record how configuration ended and release anyone waiting on it.
    fn finish_configuring(&self, state: u8) {
        self.state.store(state, Ordering::Release);
        self.ready_notify.notify_waiters();
    }

    /// Wait for the configure closure. `false` means it panicked and this room
    /// must not be used.
    async fn wait_configured(&self) -> bool {
        loop {
            match self.state.load(Ordering::Acquire) {
                CONFIGURED => return true,
                CONFIGURE_FAILED => return false,
                _ => {}
            }

            // Subscribe before re-checking, so a room that finishes in between
            // cannot leave this waiting for a notification already sent.
            let notified = self.ready_notify.notified();
            match self.state.load(Ordering::Acquire) {
                CONFIGURED => return true,
                CONFIGURE_FAILED => return false,
                _ => notified.await,
            }
        }
    }

    /// The URL this room belongs to, exactly as matched.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// How many clients are currently connected.
    pub fn online(&self) -> usize {
        self.clients.len()
    }

    /// Is the room empty?
    pub fn is_empty(&self) -> bool {
        self.clients.is_empty()
    }

    /// The ids of every connected client.
    pub fn client_ids(&self) -> Vec<ClientId> {
        self.clients.iter().map(|entry| *entry.key()).collect()
    }

    /// Look up one client by id.
    pub fn find(&self, id: ClientId) -> Option<SocketClient> {
        self.clients.get(&id).map(|entry| entry.clone())
    }

    /// Destroy the room when the last client leaves. On by default.
    ///
    /// Turn it off for a room that must outlive its members — a lobby that
    /// keeps state, say — and call [`destroy`](SocketRoom::destroy) yourself.
    pub fn autodestroy(&self, enabled: bool) -> &Self {
        self.autodestroy.store(enabled, Ordering::Relaxed);
        self
    }

    /// Run when a client joins.
    ///
    /// Registered during configuration, which completes before any client
    /// joins — so the first client's `on_open` always sees the handlers the
    /// configure closure installed.
    pub fn on_open<F, Fut>(&self, handler: F) -> &Self
    where
        F: Fn(SocketClient) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        self.lock_handlers().open = Some(Arc::new(move |client| Box::pin(handler(client))));
        self
    }

    /// Run for every message a client sends.
    ///
    /// Awaited inline on that client's connection, in order: while it runs,
    /// this connection reads nothing else, so a slow handler delays that
    /// client's later messages and its close. Other clients are unaffected,
    /// and keepalive pings keep flowing from the connection's separate writer
    /// task. Move slow work to a `WORKER` if the ordering guarantee is not
    /// worth the latency.
    ///
    /// A handler that never returns is cancelled when the connection ends for
    /// any reason the writer can see — a peer that stops answering pings, a
    /// close, server shutdown — so it cannot hold the connection open
    /// indefinitely. It is not cancelled by the peer merely disconnecting,
    /// because that is only observable by the read this handler is blocking.
    pub fn on_message<F, Fut>(&self, handler: F) -> &Self
    where
        F: Fn(SocketClient, Message) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        self.lock_handlers().message = Some(Arc::new(move |client, message| {
            Box::pin(handler(client, message))
        }));
        self
    }

    /// Run when a client leaves, with the close code it left under.
    ///
    /// The code is the peer's own when it sent one, `1006` when the connection
    /// vanished without a close frame, or the code the server closed with.
    ///
    /// **Best effort, and bounded at five seconds.** The client has already
    /// left the room by the time this runs, and the connection is being torn
    /// down behind it; a handler that takes longer is cancelled at its next
    /// await point and the fact is logged. Anything that must complete — and
    /// must not be left half-done by cancellation — belongs in a `WORKER`, not
    /// here.
    pub fn on_close<F, Fut>(&self, handler: F) -> &Self
    where
        F: Fn(SocketClient, CloseCode) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        self.lock_handlers().close =
            Some(Arc::new(move |client, code| Box::pin(handler(client, code))));
        self
    }

    /// Run when a connection fails — a protocol error, or a failed write.
    ///
    /// Without one, failures are logged at `error` and the client is dropped.
    ///
    /// **Best effort, and bounded at five seconds**, like
    /// [`on_close`](SocketRoom::on_close) and for the same reason: most
    /// failures happen while the connection is being torn down. A handler that
    /// takes longer is cancelled at its next await point and the fact is
    /// logged, so anything that cannot safely be interrupted halfway belongs in
    /// a `WORKER`, not here.
    pub fn on_error<F, Fut>(&self, handler: F) -> &Self
    where
        F: Fn(SocketClient, Error) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        self.lock_handlers().error =
            Some(Arc::new(move |client, error| Box::pin(handler(client, error))));
        self
    }

    /// Send to every client in the room.
    ///
    /// Never blocks: each client has a bounded queue, and one that has stopped
    /// reading is disconnected with `1008` rather than allowed to stall the
    /// others.
    pub fn send(&self, message: Message) -> Result<()> {
        self.fan_out(message, None)
    }

    /// Send to everyone except one client — the Total.js `self.send(msg, sender)`.
    pub fn send_except(&self, message: Message, except: ClientId) -> Result<()> {
        self.fan_out(message, Some(except))
    }

    /// Send to one client. `Ok(())` even if that client has already left.
    pub fn send_to(&self, id: ClientId, message: Message) -> Result<()> {
        match self.clients.get(&id) {
            Some(client) => client.send(message),
            None => Ok(()),
        }
    }

    /// Serialise `value` as JSON and send it to everyone.
    pub fn send_json<T: serde::Serialize + ?Sized>(&self, value: &T) -> Result<()> {
        self.send(Message::json(value)?)
    }

    /// Serialise `value` as JSON and send it to everyone but `except`.
    pub fn send_json_except<T: serde::Serialize + ?Sized>(
        &self,
        value: &T,
        except: ClientId,
    ) -> Result<()> {
        self.send_except(Message::json(value)?, except)
    }

    /// Disconnect everyone and remove the room from [`SOCKET`].
    pub fn destroy(self: &Arc<Self>) {
        // Collect first: holding a `clients` reference while taking the
        // registry lock below would reverse this module's lock order.
        let clients: Vec<SocketClient> =
            self.clients.iter().map(|entry| entry.value().clone()).collect();
        for client in clients {
            client.close(CloseCode::GOING_AWAY, "room closed");
        }

        // Only if this is still the registered room: an `Arc` kept from before
        // an autodestroy must not evict the room that replaced it.
        registry().remove_if(&self.path, |_, current| Arc::ptr_eq(current, self));
    }

    fn fan_out(&self, message: Message, except: Option<ClientId>) -> Result<()> {
        for entry in self.clients.iter() {
            if Some(*entry.key()) == except {
                continue;
            }
            // One client's failure must not stop the broadcast; `send` has
            // already disconnected it.
            let _ = entry.value().send(message.clone());
        }
        Ok(())
    }

    fn handlers(&self) -> Handlers {
        self.lock_handlers().clone()
    }

    /// The handler registry, recovering from a poisoned lock.
    ///
    /// Poisoning means a handler panicked while registering. The registry is a
    /// plain struct of `Option<Arc<..>>` — a panic cannot leave it in a state
    /// that is unsafe to read — so taking the value back is strictly better
    /// than killing every other connection in the room.
    fn lock_handlers(&self) -> std::sync::MutexGuard<'_, Handlers> {
        self.handlers
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn next_client_id(&self) -> ClientId {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }
}

/// A single connection within a room.
///
/// A cheap, cloneable handle: everything it carries is either an `Arc` or a
/// channel sender, so it can be moved into handlers and stored freely.
#[derive(Clone)]
pub struct SocketClient {
    id: ClientId,
    room: Arc<SocketRoom>,
    handshake: Arc<Handshake>,
    outbound: mpsc::Sender<Outbound>,
    /// Closing travels on its own channel, not through `outbound`.
    ///
    /// The whole point of closing a client is that its outbound queue is full
    /// or its peer is gone; a close queued behind the messages it is meant to
    /// abandon would never be sent.
    closing: Arc<watch::Sender<Option<(CloseCode, String)>>>,
}

/// The parts of the opening request a socket handler may still need.
///
/// Snapshotted at handshake time: the `Request` itself is gone by the time the
/// connection is live.
pub(crate) struct Handshake {
    pub(crate) params: HashMap<String, String>,
    pub(crate) query: HashMap<String, String>,
    pub(crate) headers: HashMap<String, String>,
    pub(crate) ip: String,
    pub(crate) session: Option<Arc<Session>>,
}

impl std::fmt::Debug for SocketClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SocketClient")
            .field("id", &self.id)
            .field("room", &self.room.path)
            .field("ip", &self.handshake.ip)
            .finish()
    }
}

impl SocketClient {
    /// This client's id, unique within its room.
    pub fn id(&self) -> ClientId {
        self.id
    }

    /// The room this client belongs to.
    pub fn room(&self) -> &Arc<SocketRoom> {
        &self.room
    }

    /// Route parameters from the socket URL, e.g. `{room}` in `/chat/{room}`.
    pub fn params(&self) -> &HashMap<String, String> {
        &self.handshake.params
    }

    /// One route parameter.
    pub fn param(&self, name: &str) -> Option<&str> {
        self.handshake.params.get(name).map(String::as_str)
    }

    /// Query-string parameters from the handshake URL.
    pub fn query(&self) -> &HashMap<String, String> {
        &self.handshake.query
    }

    /// One query-string parameter.
    pub fn query_param(&self, name: &str) -> Option<&str> {
        self.handshake.query.get(name).map(String::as_str)
    }

    /// Handshake headers, keyed by lowercase name.
    pub fn headers(&self) -> &HashMap<String, String> {
        &self.handshake.headers
    }

    /// One handshake header. The name is matched case-insensitively.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.handshake
            .headers
            .get(&name.to_ascii_lowercase())
            .map(String::as_str)
    }

    /// The client's IP, honouring trusted proxy headers as HTTP requests do.
    pub fn ip(&self) -> &str {
        &self.handshake.ip
    }

    /// The session the handshake carried, if any.
    ///
    /// A snapshot: it stays readable for the life of the connection, but a
    /// login that happens elsewhere afterwards is not reflected here.
    pub fn session(&self) -> Option<&Session> {
        self.handshake.session.as_deref()
    }

    /// Send a message to this client.
    ///
    /// Queues rather than blocks. A client whose queue is full has stopped
    /// reading, so it is disconnected with `1008` and this returns an error.
    pub fn send(&self, message: Message) -> Result<()> {
        match self.outbound.try_send(Outbound::Send(message)) {
            Ok(()) => Ok(()),
            Err(mpsc::error::TrySendError::Full(_)) => {
                log::warn!(
                    "WebSocket client {} on {} is not reading; disconnecting",
                    self.id,
                    self.room.path
                );
                self.close(CloseCode::POLICY_VIOLATION, "client is not reading");
                Err(Error::WebSocket(format!(
                    "client {} is not reading its socket",
                    self.id
                )))
            }
            Err(mpsc::error::TrySendError::Closed(_)) => Err(Error::WebSocket(format!(
                "client {} has disconnected",
                self.id
            ))),
        }
    }

    /// Send a text message.
    pub fn send_text<S: Into<String>>(&self, text: S) -> Result<()> {
        self.send(Message::text(text))
    }

    /// Serialise `value` as JSON and send it as text.
    pub fn send_json<T: serde::Serialize + ?Sized>(&self, value: &T) -> Result<()> {
        self.send(Message::json(value)?)
    }

    /// Close this client's connection.
    ///
    /// Takes effect even when the client's outbound queue is full, which is
    /// the case that matters: a stalled client is exactly the one that needs
    /// disconnecting. Best effort — a client that has already gone is simply
    /// already gone.
    pub fn close(&self, code: CloseCode, reason: &str) {
        let _ = self.closing.send(Some((code, reason.to_string())));
    }
}

// --------------------------------------------------------------- registry

fn registry() -> &'static DashMap<String, Arc<SocketRoom>> {
    static ROOMS: Lazy<DashMap<String, Arc<SocketRoom>>> = Lazy::new(DashMap::new);
    &ROOMS
}

/// Global access to live socket rooms — RustF's `F.connections[url]`.
///
/// Named after the route keyword, so `SOCKET::room` sits next to
/// `routes![SOCKET ...]` in autocomplete, the way `WORKER::run` sits next to
/// `WORKER::register`.
///
/// ```ignore
/// // Push from an ordinary HTTP handler, a worker, or a scheduler:
/// if let Some(room) = SOCKET::room("/chat/general") {
///     room.send_json(&json!({ "type": "announcement", "text": text }))?;
/// }
/// ```
pub struct SOCKET;

impl SOCKET {
    /// The room serving `path`, if anyone is connected to it.
    pub fn room(path: &str) -> Option<Arc<SocketRoom>> {
        registry().get(path).map(|entry| entry.value().clone())
    }

    /// Every live room.
    pub fn rooms() -> Vec<Arc<SocketRoom>> {
        registry()
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Every live room's path.
    pub fn paths() -> Vec<String> {
        registry().iter().map(|entry| entry.key().clone()).collect()
    }

    /// Total clients connected across all rooms.
    pub fn online() -> usize {
        registry().iter().map(|entry| entry.value().online()).sum()
    }

    /// Close every socket on the server with `1001 Going Away`.
    ///
    /// Used by graceful shutdown; safe to call directly.
    pub fn shutdown_all() {
        for room in Self::rooms() {
            let clients: Vec<SocketClient> = room
                .clients
                .iter()
                .map(|entry| entry.value().clone())
                .collect();
            for client in clients {
                client.close(CloseCode::GOING_AWAY, "server shutting down");
            }
        }
    }
}

// ------------------------------------------------------------- connection

/// Join `path`, creating and configuring the room if this is the first client.
///
/// Three phases, because the configure closure is application code that may
/// call back into the registry (`SOCKET::room`), and running it under a
/// DashMap shard lock would deadlock against itself:
///
/// 1. **Publish or find**, holding the registry entry for a map lookup only.
/// 2. **Configure** with no lock held — or, for a joiner, wait for whoever is
///    configuring. Joiners wait on the room's own readiness state, so nobody
///    can fire `on_open` against a room whose handlers are not installed.
/// 3. **Register**, holding the entry again, and only into the room that is
///    still the registered one. Membership is established before that lock is
///    released, so the last client of a *previous* generation cannot
///    autodestroy the room this one just joined.
///
/// Phase 3 can find the path empty or occupied by a different room — an
/// autodestroy or a `destroy()` in between — and then loops. `configure` is a
/// `FnOnce`: once spent it cannot configure a second generation, which is the
/// one case that gives up rather than retrying.
async fn join(
    path: &str,
    configure: Box<dyn FnOnce(&SocketRoom) + Send>,
    outbound: mpsc::Sender<Outbound>,
    closing: Arc<watch::Sender<Option<(CloseCode, String)>>>,
    handshake: Handshake,
) -> Result<(Arc<SocketRoom>, SocketClient)> {
    let mut configure = Some(configure);
    let handshake = Arc::new(handshake);

    loop {
        // Phase 1 — publish or find the room. The lock is held for a map
        // lookup and nothing else; no application code runs under it.
        let (room, ours) = match registry().entry(path.to_string()) {
            Entry::Occupied(occupied) => (occupied.get().clone(), false),
            // Creating a room requires an initializer. After a retry this
            // connection's `FnOnce` may already be spent — it configured a
            // generation that has since been destroyed or replaced — and a room
            // created without one would be published with no handlers at all.
            Entry::Vacant(_) if configure.is_none() => {
                return Err(Error::WebSocket(format!(
                    "the room for `{path}` was replaced while it was being configured, and \
                     this connection's configure closure has already been used; reconnect"
                )));
            }
            Entry::Vacant(vacant) => {
                let room = Arc::new(SocketRoom::new(path.to_string()));
                (vacant.insert(room).clone(), true)
            }
        };

        // Phase 2 — configure, or wait for whoever is configuring. No lock is
        // held here, so a configure closure may use `SOCKET::room` freely.
        if ours {
            // A guard, so a panicking configure closure unpublishes the
            // half-built room and releases the waiters, instead of leaving
            // every later connection to this path joining something that was
            // never finished.
            let finish = FinishConfiguring(&room);
            if let Some(configure) = configure.take() {
                configure(&room);
            }
            drop(finish);
        } else if !room.wait_configured().await {
            // Whoever was configuring panicked and withdrew the room. Start
            // again: this connection still holds its own configure closure.
            continue;
        }

        // Phase 3 — register, but only into the room that is actually
        // registered. Re-inserting a room that is no longer there would undo
        // the `destroy()` or autodestroy that removed it.
        let guard = registry().entry(path.to_string());
        match guard {
            Entry::Occupied(occupied) if Arc::ptr_eq(occupied.get(), &room) => {
                let client = SocketClient {
                    id: room.next_client_id(),
                    room: Arc::clone(&room),
                    handshake,
                    outbound,
                    closing,
                };
                // Membership is established before the lock is released, so the
                // last client of a previous generation cannot autodestroy the
                // room this one just joined.
                room.clients.insert(client.id, client.clone());
                drop(occupied);
                return Ok((room, client));
            }
            // Another room won this path while we were configuring. Join that
            // one on the next turn: it is configured by whoever created it, and
            // our own closure — if we still hold it — is not needed.
            Entry::Occupied(_) => continue,
            // The room went away while this connection was configuring or
            // waiting — an ordinary autodestroy of the previous generation, or
            // an explicit `destroy()`. With an unspent closure this connection
            // simply configures the next generation.
            Entry::Vacant(_) if configure.is_some() => continue,
            // Without one there is nothing to configure a new generation with,
            // and publishing a handler-less room would be worse than refusing.
            // Only reachable if the room this connection configured was
            // destroyed and then replaced and destroyed again.
            Entry::Vacant(_) => {
                return Err(Error::WebSocket(format!(
                    "the room for `{path}` was destroyed while it was being configured, and \
                     this connection's configure closure has already been used; reconnect"
                )));
            }
        }
    }
}

/// Records how configuration ended, including while unwinding from a panic in
/// the configure closure.
struct FinishConfiguring<'a>(&'a Arc<SocketRoom>);

impl Drop for FinishConfiguring<'_> {
    fn drop(&mut self) {
        if std::thread::panicking() {
            log::error!(
                "the configure closure for WebSocket room `{}` panicked; the room is \
                 withdrawn and the next connection will configure it again",
                self.0.path
            );
            // Withdraw this exact room, so the next connection starts clean
            // rather than joining one whose handlers were never installed.
            registry().remove_if(&self.0.path, |_, current| Arc::ptr_eq(current, self.0));
            self.0.finish_configuring(CONFIGURE_FAILED);
        } else {
            self.0.finish_configuring(CONFIGURED);
        }
    }
}

/// Removes a client from its room when dropped, however the connection ends —
/// including when the task is aborted at shutdown, where no code of ours runs.
struct Membership {
    room: Arc<SocketRoom>,
    id: ClientId,
}

impl Drop for Membership {
    fn drop(&mut self) {
        leave(&self.room, self.id);
    }
}

/// Aborts a task when dropped, so a cancelled parent does not leave its writer
/// running detached.
struct AbortOnDrop(tokio::task::JoinHandle<WriterOutcome>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// Remove a client and, if that emptied an autodestroy room, unregister it.
///
/// Held under the registry entry lock for the same reason as [`join`]: the
/// emptiness check and the removal must not straddle a new client joining.
fn leave(room: &Arc<SocketRoom>, id: ClientId) {
    let entry = registry().entry(room.path.clone());

    room.clients.remove(&id);

    if let Entry::Occupied(occupied) = entry {
        if Arc::ptr_eq(occupied.get(), room)
            && room.autodestroy.load(Ordering::Relaxed)
            && room.clients.is_empty()
        {
            occupied.remove();
        }
    }
}

/// Fire `on_open`, if one was registered.
async fn fire_open(client: &SocketClient) {
    if let Some(handler) = client.room.handlers().open {
        if let Err(error) = handler(client.clone()).await {
            report(client, error).await;
        }
    }
}

/// Fire `on_message`, if one was registered.
async fn fire_message(client: &SocketClient, message: Message) {
    if let Some(handler) = client.room.handlers().message {
        if let Err(error) = handler(client.clone(), message).await {
            report(client, error).await;
        }
    }
}

/// Fire `on_close`. Removing the client from the room is `Membership`'s job,
/// so that it still happens if this task is aborted before getting here.
async fn fire_close(client: &SocketClient, code: CloseCode) {
    if let Some(handler) = client.room.handlers().close {
        if let Err(error) = handler(client.clone(), code).await {
            // The client is leaving anyway; log rather than re-enter on_error,
            // which could itself try to talk to a socket that is closing.
            log::error!(
                "WebSocket on_close for {} ended with an error: {error}",
                client.room.path
            );
        }
    }

}

/// Route an error to `on_error`, or log it when there is none.
///
/// Bounded by [`CLOSE_HANDSHAKE_TIMEOUT`], because most of the places this is
/// called from are on the way out: an `on_error` that never returns would
/// otherwise hold the connection's membership, its task and its slot against
/// `max_connections` until the server shut down, and a peer that can provoke
/// errors could exhaust the limit.
async fn report(client: &SocketClient, error: Error) {
    match client.room.handlers().error {
        Some(handler) => {
            let call = handler(client.clone(), error);
            match tokio::time::timeout(CLOSE_HANDSHAKE_TIMEOUT, call).await {
                Ok(Ok(())) => {}
                Ok(Err(nested)) => {
                    log::error!(
                        "WebSocket on_error for {} itself failed: {nested}",
                        client.room.path
                    );
                }
                Err(_) => {
                    log::error!(
                        "WebSocket on_error for {} did not finish within {}s; abandoning it",
                        client.room.path,
                        CLOSE_HANDSHAKE_TIMEOUT.as_secs()
                    );
                }
            }
        }
        None => {
            log::error!(
                "WebSocket error on {} (client {}): {error}",
                client.room.path,
                client.id
            );
        }
    }
}

/// How long a single socket write may take before the peer is declared gone.
///
/// TCP backpressure from a peer that has stopped reading blocks a write
/// indefinitely, and while it blocks the writer cannot notice the close that
/// was published to disconnect it. Bounding the write is what makes
/// "a client that stops reading is disconnected" true rather than aspirational.
///
/// The write future is cancelled on timeout, so the sink is not reused
/// afterwards: the connection is abandoned instead.
const WRITE_TIMEOUT: Duration = Duration::from_secs(30);

/// Own the sending half for the life of the connection.
///
/// Separate from the read loop so that a broadcast to a thousand clients, or a
/// handler that takes its time, cannot stop this connection from sending
/// keepalive pings or the close frame that ends it.
async fn write_loop(
    mut sender: WebSocketSender,
    mut queue: mpsc::Receiver<Outbound>,
    mut closing: watch::Receiver<Option<(CloseCode, String)>>,
    ping_interval: Option<Duration>,
    reader_busy: crate::websocket::socket::ReaderBusy,
) -> WriterOutcome {
    let mut ping = ping_interval
        .filter(|period| !period.is_zero())
        .map(|period| tokio::time::interval_at(tokio::time::Instant::now() + period, period));

    loop {
        let item = tokio::select! {
            // Closing wins: a client is closed precisely when its queue is
            // full or its peer is gone, so draining first would never finish.
            biased;
            changed = closing.changed() => {
                if changed.is_err() {
                    // Every `SocketClient` handle is gone.
                    return WriterOutcome::QueueClosed;
                }
                let requested = closing.borrow_and_update().clone();
                if let Some((code, reason)) = requested {
                    return match timed(sender.close(code, &reason)).await {
                        Ok(Ok(())) => WriterOutcome::ClosedByUs(code),
                        Ok(Err(error)) => WriterOutcome::Failed(error),
                        // The close frame itself could not be written. The peer
                        // is not reading; there is nothing left to say to it.
                        Err(timeout) => WriterOutcome::Failed(timeout),
                    };
                }
                continue;
            }
            item = queue.recv() => item,
            // `pending()` when the keepalive is off: a `select!` branch that
            // never completes, rather than a guard that would need an unwrap.
            _ = async {
                match ping.as_mut() {
                    Some(interval) => {
                        interval.tick().await;
                    }
                    None => std::future::pending::<()>().await,
                }
            } => {
                match timed(sender.ping_and_check(&reader_busy)).await.unwrap_or_else(Err) {
                    Ok(true) => continue,
                    // Nothing has come back since the previous ping: the peer
                    // is accepting TCP but no longer speaking WebSocket. Close
                    // rather than ping a corpse forever.
                    Ok(false) => {
                        log::debug!("WebSocket peer stopped answering keepalive pings");
                        // The close can fail too — the peer that stopped
                        // answering may have gone entirely — and that failure
                        // is reportable like any other write failure.
                        return match timed(
                            sender.close(CloseCode::GOING_AWAY, "no response to keepalive"),
                        )
                        .await
                        {
                            Ok(Ok(())) => WriterOutcome::ClosedByUs(CloseCode::GOING_AWAY),
                            Ok(Err(error)) | Err(error) => WriterOutcome::Failed(error),
                        };
                    }
                    Err(error) => return WriterOutcome::Failed(error),
                }
            }
        };

        match item {
            Some(Outbound::Send(message)) => {
                match timed(sender.send(message)).await {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) | Err(error) => return WriterOutcome::Failed(error),
                }
            }
            None => return WriterOutcome::QueueClosed,
        }
    }
}

/// Bound a socket write, turning a stalled peer into an ordinary error.
async fn timed<T>(
    operation: impl std::future::Future<Output = Result<T>>,
) -> std::result::Result<Result<T>, Error> {
    tokio::time::timeout(WRITE_TIMEOUT, operation)
        .await
        .map_err(|_| {
            Error::WebSocket(format!(
                "a WebSocket write did not complete within {}s; the peer is not reading",
                WRITE_TIMEOUT.as_secs()
            ))
        })
}

/// Drive one connection's life inside a room.
pub(crate) async fn run(
    socket: WebSocket,
    path: String,
    handshake: Handshake,
    configure: Box<dyn FnOnce(&SocketRoom) + Send>,
    ping_interval: Option<Duration>,
    json_only: bool,
) -> Result<()> {
    let (outbound, queue) = mpsc::channel(CLIENT_QUEUE_CAPACITY);
    let (closing, closing_rx) = watch::channel(None);
    let closing = Arc::new(closing);

    let (_room, client) =
        join(&path, configure, outbound, Arc::clone(&closing), handshake).await?;

    // Membership is released by `Drop`, so an aborted task — which is what
    // shutdown does to a connection that overruns its grace period — cannot
    // leave a phantom client in the room.
    let membership = Membership {
        room: Arc::clone(client.room()),
        id: client.id(),
    };

    let reader_busy = crate::websocket::socket::ReaderBusy::default();
    let (sender, mut receiver) = socket.split();

    // Aborted with the parent for the same reason: a detached writer would
    // outlive the connection it belongs to.
    let mut writer = AbortOnDrop(tokio::spawn(write_loop(
        sender,
        queue,
        closing_rx,
        ping_interval,
        reader_busy.clone(),
    )));

    // `1006 Abnormal Closure` is the right default: it is what a connection
    // that ends without a close frame is, and every other path overwrites it.
    let mut code = CloseCode::from_u16(1006);
    let mut writer_finished = false;
    // Whether the peer has already had its say. If it has not, a close this
    // server sends is owed a reply, and teardown waits for it.
    let mut peer_closed = false;

    {
        // Same reason as `on_message`: while an application callback runs, the
        // reader is not reading, so "nothing arrived" says nothing about the
        // peer. And, as there, the callback is raced against the writer so it
        // cannot outlive the connection.
        let _busy = reader_busy.enter();
        tokio::select! {
            biased;
            _ = fire_open(&client) => {}
            result = &mut writer.0 => {
                writer_finished = true;
                code = handle_writer_outcome(&client, result, &mut receiver).await;
            }
        }
    }

    while !writer_finished {
        let received = tokio::select! {
            biased;
            received = receiver.receive() => received,
            result = &mut writer.0, if !writer_finished => {
                writer_finished = true;
                code = handle_writer_outcome(&client, result, &mut receiver).await;
                break;
            }
        };

        match received {
            Received::Message(message) => {
                // The route's `[json]` flag. Enforced here rather than in the
                // socket because the room runtime split the connection, and
                // this side is the one that can close it politely.
                if json_only && !crate::websocket::socket::is_json(&message) {
                    client.close(
                        CloseCode::UNSUPPORTED_DATA,
                        "this endpoint accepts JSON only",
                    );
                    code = CloseCode::UNSUPPORTED_DATA;
                    // Stop reading. Continuing would hand `on_message` the next
                    // frame of a connection that is already being closed.
                    break;
                }

                // Flagged while the handler runs, so the keepalive does not
                // mistake "the reader is busy" for "the peer is gone".
                let _busy = reader_busy.enter();

                // Raced against the writer so that a handler which never
                // returns cannot hold the connection — and its membership and
                // connection slot — open forever. Anything the writer notices
                // (a dead peer, a requested close, shutdown) cancels it.
                tokio::select! {
                    biased;
                    _ = fire_message(&client, message) => {}
                    result = &mut writer.0, if !writer_finished => {
                        writer_finished = true;
                        code = handle_writer_outcome(&client, result, &mut receiver).await;
                        break;
                    }
                }
            }
            // RFC 6455 §7.1.5: a close frame with no payload is reported
            // locally as 1005, not as a normal closure the peer never asked
            // for.
            Received::Closed(peer_code) => {
                code = peer_code.unwrap_or(CloseCode::from_u16(1005));
                peer_closed = true;
                break;
            }
            Received::Eof => {
                // The peer is gone; there is nobody left to exchange frames
                // with, so teardown must not wait for a reply.
                peer_closed = true;
                break;
            }
            Received::Failed(error, failure_code) => {
                code = failure_code;
                // Publish the close *before* the callback: a blocking
                // `on_error` must not stop the peer being told why its
                // connection is ending.
                client.close(code, reason_for_close(code));
                report(&client, error).await;
                break;
            }
        }
    }

    // Ask the writer for the close frame *before* running `on_close`. The
    // reply the peer is waiting for must not be held up by whatever the
    // application does on the way out.
    if !writer_finished {
        client.close(code, reason_for_close(code));
    }

    // The client has left; release its place in the room before the callback,
    // so a slow `on_close` cannot keep a departed client visible to everyone
    // else — or, if it was the last one, hold an empty room open.
    drop(membership);

    // Bounded, for the same reason: `on_close` is cleanup, and cleanup that
    // never returns would otherwise hold this connection's task and its slot
    // against `max_connections` until the server shut down.
    if tokio::time::timeout(CLOSE_HANDSHAKE_TIMEOUT, fire_close(&client, code))
        .await
        .is_err()
    {
        log::error!(
            "WebSocket on_close for {path} did not finish within {}s; abandoning it",
            CLOSE_HANDSHAKE_TIMEOUT.as_secs()
        );
    }

    if !writer_finished {
        let result = (&mut writer.0).await;

        // Report what the writer hit on the way out — the peer may have reset
        // the moment it sent its close — through the same path as any other
        // failure, so `on_error` sees it.
        match result {
            Ok(WriterOutcome::ClosedByUs(_)) if !peer_closed => {
                // This server closed first, so the handshake is only complete
                // once the peer answers.
                drain_until_closed(&mut receiver).await;
            }
            Ok(WriterOutcome::ClosedByUs(_)) | Ok(WriterOutcome::QueueClosed) => {}
            Ok(WriterOutcome::Failed(error)) => report(&client, error).await,
            Err(join_error) => {
                report(
                    &client,
                    Error::WebSocket(format!("socket writer task failed: {join_error}")),
                )
                .await;
            }
        }
    }

    drop(client);
    Ok(())
}

/// Turn the writer's exit into the close code the connection ended under,
/// reporting anything that went wrong.
async fn handle_writer_outcome(
    client: &SocketClient,
    result: std::result::Result<WriterOutcome, tokio::task::JoinError>,
    receiver: &mut crate::websocket::socket::WebSocketReceiver,
) -> CloseCode {
    match result {
        Ok(WriterOutcome::ClosedByUs(code)) => {
            // We sent the close frame; RFC 6455 §7.1.4 says the handshake is
            // complete when the peer's reply arrives. Wait for it, but not
            // forever.
            drain_until_closed(receiver).await;
            code
        }
        Ok(WriterOutcome::QueueClosed) => CloseCode::from_u16(1006),
        Ok(WriterOutcome::Failed(error)) => {
            // A failed write is a real error, and this is the only place it can
            // be observed. Without this it would vanish.
            report(client, error).await;
            CloseCode::from_u16(1006)
        }
        Err(join_error) => {
            report(
                client,
                Error::WebSocket(format!("socket writer task failed: {join_error}")),
            )
            .await;
            CloseCode::INTERNAL_ERROR
        }
    }
}

/// A short, honest reason for a close this server initiated.
fn reason_for_close(code: CloseCode) -> &'static str {
    match code {
        CloseCode::GOING_AWAY => "server shutting down",
        CloseCode::MESSAGE_TOO_BIG => "message exceeds the configured limit",
        CloseCode::UNSUPPORTED_DATA => "this endpoint accepts JSON only",
        CloseCode::INVALID_PAYLOAD => "text frame was not valid UTF-8",
        CloseCode::PROTOCOL_ERROR => "WebSocket protocol violation",
        _ => "",
    }
}

/// Keep reading until the peer answers our close frame, or time runs out.
async fn drain_until_closed(receiver: &mut crate::websocket::socket::WebSocketReceiver) {
    let drain = async {
        // Late messages sent before the peer saw our close are discarded: the
        // connection is ending and the handlers have already been told.
        while let Received::Message(_) = receiver.receive().await {}
    };

    if tokio::time::timeout(CLOSE_HANDSHAKE_TIMEOUT, drain).await.is_err() {
        log::debug!("peer did not answer the close handshake within the timeout");
    }
}
