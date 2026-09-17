//! End-to-end checks for the raw WebSocket path: the `SOCKET` route keyword,
//! `ctx.websocket`, handshake rejection, and the per-route `[json]` /
//! `[max = ..]` flags — all over a real socket against a real server.
//!
//! One server for the whole binary: the framework's configuration is a
//! process-global singleton, so a second `RustF::new()` in the same test
//! binary fails. Every check below hits that one server.

use futures::{SinkExt, StreamExt};
use rustf::prelude::*;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::Message as ClientMessage;

/// What the tungstenite client stream yields. Spelled out because
/// `rustf::prelude::*` brings its own one-parameter `Result` into scope.
type ClientItem = std::result::Result<ClientMessage, tokio_tungstenite::tungstenite::Error>;

// ---------------------------------------------------------------- handlers

/// Echoes every message back, unchanged.
async fn echo(ctx: &mut Context) -> rustf::Result<()> {
    ctx.websocket(|mut socket| async move {
        while let Some(message) = socket.recv().await {
            socket.send(message?).await?;
        }
        Ok(())
    })
}

/// Sends the route parameters back, so we can prove `{name}` is extracted
/// before the upgrade.
async fn greet(ctx: &mut Context) -> rustf::Result<()> {
    let name = ctx.param_str("name").unwrap_or_default();
    ctx.websocket(move |mut socket| async move {
        socket.send_text(format!("hello {name}")).await?;
        while socket.recv().await.is_some() {}
        Ok(())
    })
}

/// Requires an authenticated session. Nothing here has one, so it must 401
/// and never upgrade.
///
/// Written as an explicit `throw401` rather than `ctx.require_auth()?`
/// because the framework currently flattens every handler `Err` to a bare
/// `500` (`http/server.rs`), so `?` would not produce the status this is
/// checking. That is a pre-existing gap, unrelated to WebSocket.
async fn guarded(ctx: &mut Context) -> rustf::Result<()> {
    if ctx.session().map(|session| session.is_authenticated()) != Some(true) {
        return ctx.throw401(Some("Authentication required"));
    }

    ctx.websocket(|mut socket| async move {
        while socket.recv().await.is_some() {}
        Ok(())
    })
}

/// `[json]`: non-JSON text must be refused by the framework, not the handler.
async fn json_only(ctx: &mut Context) -> rustf::Result<()> {
    ctx.websocket(|mut socket| async move {
        while let Some(message) = socket.recv().await {
            socket.send(message?).await?;
        }
        Ok(())
    })
}

/// `[max = 64]`: anything larger must be refused by the framework.
async fn small(ctx: &mut Context) -> rustf::Result<()> {
    ctx.websocket(|mut socket| async move {
        while let Some(message) = socket.recv().await {
            socket.send(message?).await?;
        }
        Ok(())
    })
}

/// A handler that asks for a socket from a route that is not `SOCKET`.
async fn not_a_socket(ctx: &mut Context) -> rustf::Result<()> {
    ctx.websocket(|mut socket| async move {
        while socket.recv().await.is_some() {}
        Ok(())
    })
}

/// An ordinary page, so the socket routes are not the only thing registered.
async fn page(ctx: &mut Context) -> rustf::Result<()> {
    ctx.plain("chat page")
}

fn install_routes() -> Vec<Route> {
    routes![
        GET    "/chat"          => page,
        GET    "/mistake"       => not_a_socket,
        SOCKET "/echo"          => echo,
        SOCKET "/greet/{name}"  => greet,
        SOCKET "/guarded"       => guarded,
        SOCKET "/json"          => json_only, [json],
        SOCKET "/small"         => small, [max = 64],
    ]
}

// ----------------------------------------------------------------- helpers

/// Minimal HTTP/1.1 GET, returning the response head (status line plus
/// headers).
///
/// Reads up to the header terminator rather than to EOF: an accepted
/// handshake answers `101` and then keeps the connection open forever, so
/// `read_to_end` would never return. `Connection: close` is deliberately not
/// sent either — it is meaningless on an upgrade request.
async fn http_get(addr: SocketAddr, path: &str, extra_headers: &str) -> String {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    let request = format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\n{extra_headers}\r\n");
    stream.write_all(request.as_bytes()).await.unwrap();

    let mut response = Vec::new();
    let mut byte = [0u8; 1];
    let read = async {
        loop {
            match stream.read(&mut byte).await.unwrap() {
                0 => break,
                _ => {
                    response.push(byte[0]);
                    if response.ends_with(b"\r\n\r\n") {
                        break;
                    }
                }
            }
        }
    };

    tokio::time::timeout(Duration::from_secs(5), read)
        .await
        .unwrap_or_else(|_| panic!("timed out reading the response head for {path}"));

    String::from_utf8_lossy(&response).into_owned()
}

/// Status line of a raw HTTP response, e.g. `HTTP/1.1 426 Upgrade Required`.
fn status_line(response: &str) -> &str {
    response.lines().next().unwrap_or("")
}

async fn connect(
    addr: SocketAddr,
    path: &str,
) -> tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>> {
    let (socket, _) = tokio_tungstenite::connect_async(format!("ws://{addr}{path}"))
        .await
        .unwrap_or_else(|e| panic!("connecting to {path} failed: {e}"));
    socket
}

/// Read the next message, failing the test rather than hanging forever.
async fn next_message<S>(socket: &mut S) -> Option<ClientMessage>
where
    S: StreamExt<Item = ClientItem> + Unpin,
{
    match tokio::time::timeout(Duration::from_secs(5), socket.next()).await {
        Ok(Some(Ok(message))) => Some(message),
        Ok(Some(Err(error))) => panic!("socket error: {error}"),
        Ok(None) => None,
        Err(_) => panic!("timed out waiting for a message"),
    }
}

/// Drain until a close frame arrives, returning its code.
///
/// Keeps reading afterwards, which is how a normal client loop answers a close:
/// tungstenite queues the reply frame and flushes it on the next read. Without
/// that the server would sit out its five-second wait for a reply that never
/// comes, on every route that closes from the server side.
async fn close_code<S>(socket: &mut S) -> Option<CloseCode>
where
    S: StreamExt<Item = ClientItem> + Unpin,
{
    let mut code = None;

    loop {
        match tokio::time::timeout(Duration::from_secs(5), socket.next()).await {
            Ok(Some(Ok(ClientMessage::Close(frame)))) => {
                code = frame.map(|f| f.code);
                // Keep going: the next read is what puts our reply on the wire.
                continue;
            }
            Ok(Some(Ok(_))) => continue,
            // The stream ends once the close handshake is complete.
            Ok(Some(Err(_))) | Ok(None) => return code,
            Err(_) => panic!("timed out waiting for a close frame"),
        }
    }
}

// -------------------------------------------------------------------- test

#[tokio::test]
async fn websocket_routes_behave_correctly_on_the_wire() {
    let app = RustF::new().controllers(install_routes());
    let running = app.serve_with_handle("127.0.0.1:0").await.unwrap();
    let addr = running.local_addr;

    // --- ordinary routes still work next to socket routes ---
    let response = http_get(addr, "/chat", "").await;
    assert!(
        status_line(&response).starts_with("HTTP/1.1 200"),
        "{response}"
    );

    // --- a plain GET on a SOCKET route is 426, and the handler never runs ---
    let response = http_get(addr, "/echo", "").await;
    assert!(
        status_line(&response).starts_with("HTTP/1.1 426"),
        "a non-handshake GET on a SOCKET route must be 426: {response}"
    );
    assert!(
        response.to_lowercase().contains("upgrade: websocket"),
        "426 must advertise the protocol to switch to: {response}"
    );
    assert!(
        response.to_lowercase().contains("sec-websocket-version: 13"),
        "426 must advertise the supported version: {response}"
    );

    // --- a handshake with the wrong version is refused ---
    let response = http_get(
        addr,
        "/echo",
        "Upgrade: websocket\r\nConnection: Upgrade\r\n\
         Sec-WebSocket-Version: 8\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n",
    )
    .await;
    assert!(
        status_line(&response).starts_with("HTTP/1.1 426"),
        "an unsupported version must be refused: {response}"
    );

    // --- a handshake with a malformed key is a 400 ---
    let response = http_get(
        addr,
        "/echo",
        "Upgrade: websocket\r\nConnection: Upgrade\r\n\
         Sec-WebSocket-Version: 13\r\nSec-WebSocket-Key: nope\r\n",
    )
    .await;
    assert!(
        status_line(&response).starts_with("HTTP/1.1 400"),
        "a malformed Sec-WebSocket-Key must be refused: {response}"
    );

    // --- a handshake from a foreign origin is a 403, and never upgrades ---
    let response = http_get(
        addr,
        "/echo",
        "Upgrade: websocket\r\nConnection: Upgrade\r\n\
         Sec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
         Origin: http://evil.example\r\n",
    )
    .await;
    assert!(
        status_line(&response).starts_with("HTTP/1.1 403"),
        "a cross-origin handshake must be refused: {response}"
    );

    // --- the site's own origin is accepted ---
    let response = http_get(
        addr,
        "/echo",
        &format!(
            "Upgrade: websocket\r\nConnection: Upgrade\r\n\
             Sec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
             Origin: http://{addr}\r\n"
        ),
    )
    .await;
    assert!(
        status_line(&response).starts_with("HTTP/1.1 101"),
        "a same-origin handshake must be accepted: {response}"
    );
    assert!(
        response.contains("s3pPLMBiTxaQ9kYGzzhZRbK+xOo="),
        "the 101 must carry the RFC-derived Sec-WebSocket-Accept: {response}"
    );

    // --- an unauthenticated request to a guarded socket is 401, no upgrade ---
    let response = http_get(
        addr,
        "/guarded",
        "Upgrade: websocket\r\nConnection: Upgrade\r\n\
         Sec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n",
    )
    .await;
    assert!(
        status_line(&response).starts_with("HTTP/1.1 401"),
        "an auth check must reject before the upgrade: {response}"
    );

    // --- a duplicated singleton handshake header is refused ---
    // Two `Origin` headers collapse into one map entry, so whichever survived
    // would decide the origin check. RFC 6455 allows exactly one.
    let response = http_get(
        addr,
        "/echo",
        "Upgrade: websocket\r\nConnection: Upgrade\r\n\
         Sec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
         Origin: http://evil.example\r\nOrigin: http://example.com\r\n",
    )
    .await;
    assert!(
        status_line(&response).starts_with("HTTP/1.1 400"),
        "a duplicated Origin must be refused, not silently resolved: {response}"
    );

    let response = http_get(
        addr,
        "/echo",
        "Upgrade: websocket\r\nConnection: Upgrade\r\n\
         Sec-WebSocket-Version: 13\r\n\
         Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
         Sec-WebSocket-Key: AAAAAAAAAAAAAAAAAAAAAA==\r\n",
    )
    .await;
    assert!(
        status_line(&response).starts_with("HTTP/1.1 400"),
        "a duplicated Sec-WebSocket-Key must be refused: {response}"
    );

    // --- ctx.websocket outside a SOCKET route is a server error, not a socket ---
    let response = http_get(
        addr,
        "/mistake",
        "Upgrade: websocket\r\nConnection: Upgrade\r\n\
         Sec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n",
    )
    .await;
    assert!(
        status_line(&response).starts_with("HTTP/1.1 500"),
        "asking for a socket from a GET route must fail loudly: {response}"
    );

    // --- a real echo round trip, then a clean close handshake ---
    let mut socket = connect(addr, "/echo").await;
    socket
        .send(ClientMessage::Text("hello".into()))
        .await
        .unwrap();
    assert_eq!(
        next_message(&mut socket).await,
        Some(ClientMessage::Text("hello".into()))
    );

    socket
        .send(ClientMessage::Binary(vec![1, 2, 3].into()))
        .await
        .unwrap();
    match next_message(&mut socket).await {
        Some(ClientMessage::Binary(bytes)) => assert_eq!(&bytes[..], &[1, 2, 3]),
        other => panic!("expected the binary frame back, got {other:?}"),
    }

    socket.close(None).await.unwrap();
    // A completed close handshake ends the stream cleanly. Anything else —
    // notably a reset — means the server dropped the socket without replying.
    while next_message(&mut socket).await.is_some() {}

    // --- route parameters are extracted before the upgrade ---
    let mut socket = connect(addr, "/greet/morle").await;
    assert_eq!(
        next_message(&mut socket).await,
        Some(ClientMessage::Text("hello morle".into()))
    );
    socket.close(None).await.unwrap();

    // --- [json]: valid JSON passes through ---
    let mut socket = connect(addr, "/json").await;
    socket
        .send(ClientMessage::Text(r#"{"a":1}"#.into()))
        .await
        .unwrap();
    assert_eq!(
        next_message(&mut socket).await,
        Some(ClientMessage::Text(r#"{"a":1}"#.into()))
    );

    // --- [json]: anything else is closed with 1003 ---
    socket
        .send(ClientMessage::Text("not json".into()))
        .await
        .unwrap();
    assert_eq!(
        close_code(&mut socket).await,
        Some(CloseCode::Unsupported),
        "non-JSON text on a [json] route must close with 1003"
    );

    // --- [max = 64]: a larger message is closed with 1009 ---
    let mut socket = connect(addr, "/small").await;
    socket
        .send(ClientMessage::Text("x".repeat(16).into()))
        .await
        .unwrap();
    assert_eq!(
        next_message(&mut socket).await,
        Some(ClientMessage::Text("x".repeat(16).into())),
        "a message within the cap must still be echoed"
    );

    socket
        .send(ClientMessage::Text("x".repeat(256).into()))
        .await
        .unwrap();
    assert_eq!(
        close_code(&mut socket).await,
        Some(CloseCode::Size),
        "a message over the route's [max] must close with 1009"
    );

    // --- the handshake a real client builds is answered correctly ---
    assert_handshake_response_is_correct(addr).await;

    running.handle.shutdown().await.unwrap();
}

/// The handshake a real client builds, checked against what RustF answers.
///
/// Driven through the same server as the test above — one server per binary —
/// so this asserts on the actual `101`, not just on the request headers.
async fn assert_handshake_response_is_correct(addr: SocketAddr) {
    use base64::Engine as _;
    use sha1::Digest as _;

    let request = format!("ws://{addr}/echo").into_client_request().unwrap();
    assert_eq!(
        request.headers().get("sec-websocket-version").unwrap(),
        "13",
        "a client must ask for version 13"
    );
    let key = request
        .headers()
        .get("sec-websocket-key")
        .expect("a client must send a key")
        .to_str()
        .unwrap()
        .to_string();

    // What RFC 6455 §4.2.2 says the server must derive from that key.
    let mut hasher = sha1::Sha1::new();
    hasher.update(key.as_bytes());
    hasher.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
    let expected = base64::engine::general_purpose::STANDARD.encode(hasher.finalize());

    let response = http_get(
        addr,
        "/echo",
        &format!(
            "Upgrade: websocket\r\nConnection: Upgrade\r\n\
             Sec-WebSocket-Version: 13\r\nSec-WebSocket-Key: {key}\r\n"
        ),
    )
    .await;

    assert!(
        status_line(&response).starts_with("HTTP/1.1 101"),
        "a well-formed handshake must be accepted: {response}"
    );
    let lower = response.to_lowercase();
    assert!(lower.contains("upgrade: websocket"), "{response}");
    assert!(lower.contains("connection: upgrade"), "{response}");
    assert!(
        response.contains(&expected),
        "the 101 must carry the accept key derived from this client's key: {response}"
    );
    assert!(
        !lower.contains("sec-websocket-extensions"),
        "nothing negotiated an extension: {response}"
    );
}
