//! End-to-end checks for the Total.js room model: one room per URL, a
//! configure closure that runs once, broadcast with and without the sender,
//! reaching a room from ordinary HTTP, autodestroy, and a `1001` on shutdown.
//!
//! One server for the whole binary: the framework's configuration is a
//! process-global singleton, so a second `RustF::new()` in the same test
//! binary fails. Every check below hits that one server, and the shutdown
//! check comes last because it takes the server down.

use futures::{SinkExt, StreamExt};
use rustf::prelude::*;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::Message as ClientMessage;

type ClientItem = std::result::Result<ClientMessage, tokio_tungstenite::tungstenite::Error>;
type Client = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>;

/// How many times a room's configure closure has run, per path.
static CONFIGURED: AtomicUsize = AtomicUsize::new(0);
/// How many `on_close` callbacks have fired.
static CLOSED: AtomicUsize = AtomicUsize::new(0);

// ---------------------------------------------------------------- handlers

/// The close code the last departing client left under, so the test can prove
/// `on_close` reports the peer's own code rather than a default.
static LAST_CLOSE_CODE: AtomicUsize = AtomicUsize::new(0);

async fn chat(ctx: &mut Context) -> rustf::Result<()> {
    ctx.socket(|room| {
        CONFIGURED.fetch_add(1, Ordering::SeqCst);

        room.on_open(|client| async move {
            // Tell the newcomer who it is, and everyone else that it arrived.
            client.send_json(&json!({
                "type": "welcome",
                "id": client.id(),
                "room": client.param("room").unwrap_or_default(),
                "online": client.room().online(),
            }))?;
            client.room().send_json_except(
                &json!({ "type": "joined", "id": client.id() }),
                client.id(),
            )
        });

        room.on_message(|client, message| async move {
            // Relay to everyone else — Total.js `self.send(msg, sender)`.
            client.room().send_except(message, client.id())
        });

        room.on_close(|_client, code| async move {
            CLOSED.fetch_add(1, Ordering::SeqCst);
            LAST_CLOSE_CODE.store(code.as_u16() as usize, Ordering::SeqCst);
            Ok(())
        });
    })
}

/// A socket that never destroys its room, so it can be inspected after
/// everyone has left.
async fn lobby(ctx: &mut Context) -> rustf::Result<()> {
    ctx.socket(|room| {
        room.autodestroy(false);
        room.on_message(|client, message| async move { client.send(message) });
    })
}

/// What the reentrant room's configure closure observed about the registry.
static REENTRANT: AtomicUsize = AtomicUsize::new(0);

/// A room whose configure closure calls back into the registry.
///
/// This is the shape that deadlocks if configuration runs while the registry
/// shard lock is held: the closure asks for the very room being created.
async fn reentrant(ctx: &mut Context) -> rustf::Result<()> {
    ctx.socket(|room| {
        // 1 = the room is already published, 2 = plus the registry is readable.
        if SOCKET::room(room.path()).is_some() {
            REENTRANT.fetch_add(1, Ordering::SeqCst);
        }
        if !SOCKET::paths().is_empty() {
            REENTRANT.fetch_add(1, Ordering::SeqCst);
        }

        room.on_message(|client, message| async move { client.send(message) });
    })
}

/// Push into a live room from an ordinary HTTP handler — `F.connections[url]`.
async fn announce(ctx: &mut Context) -> rustf::Result<()> {
    let path = ctx.query_str("path").unwrap_or_default();
    let text = ctx.query_str("text").unwrap_or_default();

    match SOCKET::room(&path) {
        Some(room) => {
            room.send_json(&json!({ "type": "announcement", "text": text }))?;
            ctx.plain("sent")
        }
        None => ctx.plain("no such room"),
    }
}

/// Report registry state, so the test can assert on it from outside.
async fn rooms(ctx: &mut Context) -> rustf::Result<()> {
    let mut paths = SOCKET::paths();
    paths.sort();

    ctx.json(json!({
        "paths": paths,
        "online": SOCKET::online(),
        "configured": CONFIGURED.load(Ordering::SeqCst),
        "closed": CLOSED.load(Ordering::SeqCst),
        "last_close_code": LAST_CLOSE_CODE.load(Ordering::SeqCst),
    }))
}

fn install_routes() -> Vec<Route> {
    routes![
        GET    "/rooms"        => rooms,
        GET    "/announce"     => announce,
        SOCKET "/chat/{room}"  => chat,
        SOCKET "/lobby"        => lobby,
        SOCKET "/reentrant"    => reentrant,
    ]
}

// ----------------------------------------------------------------- helpers

async fn connect(addr: SocketAddr, path: &str) -> Client {
    // Bounded, so a deadlock in room setup fails the test instead of hanging
    // the whole suite.
    let connecting = tokio_tungstenite::connect_async(format!("ws://{addr}{path}"));
    let (socket, _) = tokio::time::timeout(Duration::from_secs(10), connecting)
        .await
        .unwrap_or_else(|_| panic!("connecting to {path} timed out"))
        .unwrap_or_else(|e| panic!("connecting to {path} failed: {e}"));
    socket
}

async fn next_json<S>(socket: &mut S, what: &str) -> serde_json::Value
where
    S: StreamExt<Item = ClientItem> + Unpin,
{
    match tokio::time::timeout(Duration::from_secs(5), socket.next()).await {
        Ok(Some(Ok(ClientMessage::Text(text)))) => serde_json::from_str(text.as_str())
            .unwrap_or_else(|e| panic!("{what}: not JSON ({e}): {text}")),
        Ok(other) => panic!("{what}: expected a text frame, got {other:?}"),
        Err(_) => panic!("{what}: timed out"),
    }
}

/// Assert that nothing arrives within a short window.
async fn expect_silence<S>(socket: &mut S, what: &str)
where
    S: StreamExt<Item = ClientItem> + Unpin,
{
    if let Ok(message) = tokio::time::timeout(Duration::from_millis(300), socket.next()).await {
        panic!("{what}: expected nothing, got {message:?}");
    }
}

async fn http_get(addr: SocketAddr, path: &str) -> String {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    let request = format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).await.unwrap();

    let mut response = Vec::new();
    tokio::time::timeout(
        Duration::from_secs(5),
        stream.read_to_end(&mut response),
    )
    .await
    .unwrap_or_else(|_| panic!("timed out reading {path}"))
    .unwrap();

    String::from_utf8_lossy(&response).into_owned()
}

/// The JSON body of an HTTP response.
async fn http_json(addr: SocketAddr, path: &str) -> serde_json::Value {
    let response = http_get(addr, path).await;
    let body = response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .unwrap_or("");
    serde_json::from_str(body).unwrap_or_else(|e| panic!("{path}: not JSON ({e}): {response}"))
}

// -------------------------------------------------------------------- test

#[tokio::test]
async fn socket_rooms_behave_like_totaljs_connections() {
    let app = RustF::new().controllers(install_routes());
    let running = app.serve_with_handle("127.0.0.1:0").await.unwrap();
    let addr = running.local_addr;

    // --- first client into /chat/a: configure runs, welcome arrives ---
    let mut alice = connect(addr, "/chat/a").await;
    let welcome = next_json(&mut alice, "alice welcome").await;
    assert_eq!(welcome["type"], "welcome");
    assert_eq!(welcome["room"], "a", "route parameters reach the room");
    assert_eq!(welcome["online"], 1);
    let alice_id = welcome["id"].as_u64().unwrap();

    assert_eq!(
        CONFIGURED.load(Ordering::SeqCst),
        1,
        "the configure closure runs on the first connection"
    );

    // --- second client into the same room joins it, does not reconfigure ---
    let mut bob = connect(addr, "/chat/a").await;
    let welcome = next_json(&mut bob, "bob welcome").await;
    assert_eq!(welcome["online"], 2, "bob joined alice's room");
    let bob_id = welcome["id"].as_u64().unwrap();
    assert_ne!(alice_id, bob_id, "client ids are unique within a room");

    assert_eq!(
        CONFIGURED.load(Ordering::SeqCst),
        1,
        "a second connection to the same URL must not re-run configure"
    );

    // alice is told bob arrived; bob is not told about himself.
    let joined = next_json(&mut alice, "alice sees bob join").await;
    assert_eq!(joined["type"], "joined");
    assert_eq!(joined["id"], bob_id);

    // --- a third client on a different URL gets its own room ---
    let mut carol = connect(addr, "/chat/b").await;
    let welcome = next_json(&mut carol, "carol welcome").await;
    assert_eq!(welcome["room"], "b");
    assert_eq!(welcome["online"], 1, "/chat/b is a separate room");
    assert_eq!(
        CONFIGURED.load(Ordering::SeqCst),
        2,
        "a new URL is a new room, so configure runs again"
    );

    // --- send_except relays to the room, skipping the sender ---
    alice
        .send(ClientMessage::Text(r#"{"say":"hello"}"#.into()))
        .await
        .unwrap();

    let relayed = next_json(&mut bob, "bob receives alice's message").await;
    assert_eq!(relayed["say"], "hello");
    expect_silence(&mut alice, "the sender must not receive its own message").await;
    expect_silence(&mut carol, "a message must not cross into another room").await;

    // --- the registry sees both rooms ---
    let state = http_json(addr, "/rooms").await;
    assert_eq!(
        state["paths"],
        json!(["/chat/a", "/chat/b"]),
        "both rooms are registered under their matched URLs"
    );
    assert_eq!(state["online"], 3);

    // --- SOCKET::room reaches a room from an ordinary HTTP handler ---
    let response = http_get(addr, "/announce?path=/chat/a&text=maintenance").await;
    assert!(response.contains("sent"), "{response}");

    let announcement = next_json(&mut alice, "alice receives the announcement").await;
    assert_eq!(announcement["type"], "announcement");
    assert_eq!(announcement["text"], "maintenance");
    let announcement = next_json(&mut bob, "bob receives the announcement").await;
    assert_eq!(announcement["text"], "maintenance");
    expect_silence(&mut carol, "the announcement must not reach /chat/b").await;

    // --- leaving fires on_close and, with autodestroy, removes the room ---
    carol
        .close(Some(tokio_tungstenite::tungstenite::protocol::CloseFrame {
            code: CloseCode::Policy,
            reason: "done here".into(),
        }))
        .await
        .unwrap();
    while tokio::time::timeout(Duration::from_secs(5), carol.next())
        .await
        .unwrap()
        .is_some()
    {}

    // The close is processed asynchronously, and membership is released just
    // before `on_close` runs, so wait for both to have happened.
    let mut state = http_json(addr, "/rooms").await;
    for _ in 0..100 {
        if state["paths"] == json!(["/chat/a"]) && state["closed"] == json!(1) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
        state = http_json(addr, "/rooms").await;
    }
    assert_eq!(
        state["paths"],
        json!(["/chat/a"]),
        "an empty room with autodestroy on must leave the registry"
    );
    assert_eq!(state["online"], 2);
    assert_eq!(state["closed"], 1, "on_close fired for carol");
    assert_eq!(
        state["last_close_code"], 1008,
        "on_close must report the code the peer actually sent, not a default"
    );

    // --- autodestroy(false) keeps an empty room registered ---
    let mut visitor = connect(addr, "/lobby").await;
    visitor
        .send(ClientMessage::Text("echo me".into()))
        .await
        .unwrap();
    match tokio::time::timeout(Duration::from_secs(5), visitor.next()).await {
        Ok(Some(Ok(ClientMessage::Text(text)))) => assert_eq!(text.as_str(), "echo me"),
        other => panic!("the lobby must echo back, got {other:?}"),
    }
    visitor.close(None).await.unwrap();
    while tokio::time::timeout(Duration::from_secs(5), visitor.next())
        .await
        .unwrap()
        .is_some()
    {}

    let mut state = http_json(addr, "/rooms").await;
    for _ in 0..100 {
        if state["online"] == json!(2) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
        state = http_json(addr, "/rooms").await;
    }
    let paths = state["paths"].as_array().unwrap();
    assert!(
        paths.contains(&json!("/lobby")),
        "autodestroy(false) must keep the room after the last client leaves: {paths:?}"
    );

    // --- a configure closure may use the registry it is being created in ---
    // Configuration deliberately does not run under the registry lock, because
    // application code is entitled to call SOCKET::room / SOCKET::paths from
    // it. Under a lock this connect would never return.
    let mut reentrant = connect(addr, "/reentrant").await;
    reentrant
        .send(ClientMessage::Text("still alive".into()))
        .await
        .unwrap();
    match tokio::time::timeout(Duration::from_secs(5), reentrant.next()).await {
        Ok(Some(Ok(ClientMessage::Text(text)))) => assert_eq!(text.as_str(), "still alive"),
        other => panic!("the reentrant room must work normally, got {other:?}"),
    }
    assert_eq!(
        REENTRANT.load(Ordering::SeqCst),
        2,
        "the configure closure must see its own room in the registry"
    );
    reentrant.close(None).await.unwrap();
    while tokio::time::timeout(Duration::from_secs(5), reentrant.next())
        .await
        .unwrap_or(None)
        .is_some()
    {}

    // --- shutdown closes live sockets with 1001 and waits for their replies ---
    let mut shutdown = tokio::spawn(async move { running.handle.shutdown().await });

    // Collect both close frames first, without answering either. Asserting
    // between them would prove nothing: the *other* connection would keep
    // shutdown pending on its own.
    for (name, socket) in [("alice", &mut alice), ("bob", &mut bob)] {
        let mut code = None;
        while let Ok(Some(Ok(message))) =
            tokio::time::timeout(Duration::from_secs(5), socket.next()).await
        {
            if let ClientMessage::Close(frame) = message {
                code = frame.map(|f| f.code);
                break;
            }
        }
        assert_eq!(
            code,
            Some(CloseCode::Away),
            "{name} must be closed with 1001 on shutdown"
        );
    }

    // Both peers have been told to go away and neither has answered, so the
    // server is still owed two close replies and must still be draining. An
    // implementation that sent 1001 and dropped the connections would have
    // finished by now.
    assert!(
        tokio::time::timeout(Duration::from_millis(500), &mut shutdown)
            .await
            .is_err(),
        "shutdown must not complete while both peers still owe a close reply"
    );

    // Answer, by continuing to read: tungstenite queued each reply frame and
    // flushes it on the next read.
    for socket in [&mut alice, &mut bob] {
        while tokio::time::timeout(Duration::from_secs(5), socket.next())
            .await
            .unwrap_or(None)
            .is_some()
        {}
    }

    shutdown.await.unwrap().unwrap();
}
