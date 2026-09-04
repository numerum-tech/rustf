//! End-to-end checks for the streaming helpers an app author actually calls:
//! `ctx.stream`, `ctx.sse`, `ctx.sse_with_keep_alive`, `ctx.file_download_stream_from`,
//! and static-file serving above the streaming threshold.
//!
//! One server for the whole binary: the framework's configuration is a
//! process-global singleton, so a second `RustF::new()` in the same test
//! binary fails. Every check below hits that one server.

use bytes::Bytes;
use futures::StreamExt;
use rustf::http::SseEvent;
use rustf::prelude::*;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Above `STATIC_STREAM_THRESHOLD` (1 MiB) so the static path streams, and
/// spanning many 64 KiB chunks so ordering bugs show up as byte mismatches.
const BIG_LEN: usize = 1024 * 1024 + 4096;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("streaming_helpers_test")
}

fn big_payload() -> Vec<u8> {
    (0..BIG_LEN).map(|i| (i % 253) as u8).collect()
}

async fn write_fixture(name: &str, contents: &[u8]) -> PathBuf {
    let dir = fixture_dir();
    tokio::fs::create_dir_all(&dir).await.unwrap();
    let path = dir.join(name);
    tokio::fs::write(&path, contents).await.unwrap();
    path
}

// ---------------------------------------------------------------- routes

async fn legacy_buffered_stream(ctx: &mut Context) -> rustf::Result<()> {
    // The pre-1.0 `ctx.stream(Vec<u8>, ..)` call shape must keep compiling
    // and must no longer emit a bogus `Transfer-Encoding: chunked` next to
    // its `Content-Length`.
    ctx.stream(
        b"buffered bytes".to_vec(),
        "application/octet-stream",
        Some("legacy.bin"),
    )
}

async fn csv_stream(ctx: &mut Context) -> rustf::Result<()> {
    let rows = futures::stream::iter(vec![
        Ok(Bytes::from_static(b"id,name\n")),
        Ok(Bytes::from_static(b"1,alice\n")),
        Ok(Bytes::from_static(b"2,bob\n")),
    ]);
    ctx.stream(Body::from_stream(rows), "text/csv", Some("rows.csv"))
}

async fn broken_stream(ctx: &mut Context) -> rustf::Result<()> {
    let chunks = futures::stream::iter(vec![
        Ok(Bytes::from_static(b"partial")),
        Err(rustf::Error::internal("disk read failed")),
    ]);
    ctx.stream(Body::from_stream(chunks), "text/plain", None)
}

async fn events(ctx: &mut Context) -> rustf::Result<()> {
    let feed = futures::stream::iter(vec![
        SseEvent::new("hello").id("1"),
        SseEvent::json(&serde_json::json!({"n": 2}))
            .unwrap()
            .event("tick")
            .id("2"),
        SseEvent::new("line one\nline two").id("3"),
    ]);
    ctx.sse(feed)
}

async fn events_keep_alive(ctx: &mut Context) -> rustf::Result<()> {
    // One real event, then silence: the keep-alive must put a comment on the
    // wire on its own while the source is idle.
    let feed =
        futures::stream::iter(vec![SseEvent::new("first")]).chain(futures::stream::pending());
    ctx.sse_with_keep_alive(feed, Duration::from_millis(50))
}

async fn download(ctx: &mut Context) -> rustf::Result<()> {
    ctx.file_download_stream_from(fixture_dir(), "big.bin", Some("export.bin"))
        .await
}

async fn empty_download(ctx: &mut Context) -> rustf::Result<()> {
    ctx.file_inline_stream_from(fixture_dir(), "empty.bin")
        .await
}

async fn nothing(ctx: &mut Context) -> rustf::Result<()> {
    ctx.empty()
}

fn install_routes() -> Vec<Route> {
    routes![
        GET "/legacy" => legacy_buffered_stream,
        GET "/csv" => csv_stream,
        GET "/broken" => broken_stream,
        GET "/events" => events,
        GET "/events/keepalive" => events_keep_alive,
        GET "/download" => download,
        GET "/empty" => empty_download,
        GET "/nothing" => nothing,
    ]
}

// --------------------------------------------------------------- helpers

async fn send_get(addr: SocketAddr, path: &str) -> TcpStream {
    send_request(addr, "GET", path).await
}

async fn send_request(addr: SocketAddr, method: &str, path: &str) -> TcpStream {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    let request = format!(
        "{} {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        method, path, addr
    );
    stream.write_all(request.as_bytes()).await.unwrap();
    stream
}

/// HEAD and read until the server closes the connection.
async fn http_head(addr: SocketAddr, path: &str) -> (String, Vec<u8>) {
    let mut stream = send_request(addr, "HEAD", path).await;
    let mut raw = Vec::new();
    let _ = stream.read_to_end(&mut raw).await;
    split_response(&raw)
}

fn split_response(raw: &[u8]) -> (String, Vec<u8>) {
    let split = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .unwrap_or_else(|| {
            panic!(
                "response must contain a header terminator, got {} bytes: {:?}",
                raw.len(),
                String::from_utf8_lossy(raw)
            )
        });
    let headers = String::from_utf8(raw[..split].to_vec()).expect("headers are ASCII");
    (headers, raw[split + 4..].to_vec())
}

/// GET and read until the server closes the connection.
async fn http_get(addr: SocketAddr, path: &str) -> (String, Vec<u8>) {
    let mut stream = send_get(addr, path).await;
    let mut raw = Vec::new();
    // A server that aborts mid-body may reset instead of closing cleanly;
    // whatever arrived before that is what the client would have seen.
    let _ = stream.read_to_end(&mut raw).await;
    split_response(&raw)
}

/// GET and return whatever bytes arrived before the server closed or reset
/// the connection, without insisting on a well-formed response.
async fn http_get_raw(addr: SocketAddr, path: &str) -> Vec<u8> {
    let mut stream = send_get(addr, path).await;
    let mut raw = Vec::new();
    let _ = stream.read_to_end(&mut raw).await;
    raw
}

/// GET a chunked response and read until `needle` appears in the dechunked
/// payload or `limit` elapses. Returns the headers and the dechunked payload.
async fn http_get_until(
    addr: SocketAddr,
    path: &str,
    needle: &[u8],
    limit: Duration,
) -> (String, Vec<u8>) {
    let mut stream = send_get(addr, path).await;
    let mut raw = Vec::new();
    let mut buf = [0u8; 4096];
    let deadline = tokio::time::Instant::now() + limit;

    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        let read = tokio::time::timeout(remaining, stream.read(&mut buf))
            .await
            .unwrap_or_else(|_| {
                panic!(
                    "timed out waiting for {:?}; received so far: {:?}",
                    String::from_utf8_lossy(needle),
                    String::from_utf8_lossy(&raw)
                )
            })
            .unwrap();
        if read == 0 {
            break;
        }
        raw.extend_from_slice(&buf[..read]);

        if raw.windows(4).any(|w| w == b"\r\n\r\n") {
            let (_, body) = split_response(&raw);
            let (payload, _) = dechunk(&body);
            if payload.windows(needle.len()).any(|w| w == needle) {
                break;
            }
        }
    }

    let (headers, body) = split_response(&raw);
    let (payload, _) = dechunk(&body);
    (headers, payload)
}

/// Decode `Transfer-Encoding: chunked` framing. Returns the payload and
/// whether the terminating zero-length chunk was present.
fn dechunk(mut body: &[u8]) -> (Vec<u8>, bool) {
    let mut out = Vec::new();
    loop {
        let Some(line_end) = body.windows(2).position(|w| w == b"\r\n") else {
            return (out, false);
        };
        let size_line = std::str::from_utf8(&body[..line_end]).unwrap();
        let size = usize::from_str_radix(size_line.split(';').next().unwrap().trim(), 16).unwrap();
        body = &body[line_end + 2..];
        if size == 0 {
            return (out, true);
        }
        if body.len() < size + 2 {
            out.extend_from_slice(body);
            return (out, false);
        }
        out.extend_from_slice(&body[..size]);
        body = &body[size + 2..];
    }
}

fn header<'a>(headers: &'a str, name: &str) -> Option<&'a str> {
    headers.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        key.trim().eq_ignore_ascii_case(name).then(|| value.trim())
    })
}

// ------------------------------------------------------------------ test

#[tokio::test]
async fn streaming_helpers_produce_correct_wire_output() {
    let big = big_payload();
    write_fixture("big.bin", &big).await;
    write_fixture("small.txt", b"tiny static asset").await;
    write_fixture("empty.bin", b"").await;

    let app = RustF::new()
        .controllers(install_routes())
        .static_files("/public", fixture_dir().to_str().unwrap());
    let running = app.serve_with_handle("127.0.0.1:0").await.unwrap();
    let addr = running.local_addr;

    // --- ctx.stream(Vec<u8>): still buffered, no longer self-contradictory ---
    let (headers, body) = http_get(addr, "/legacy").await;
    assert!(headers.starts_with("HTTP/1.1 200"), "{headers}");
    assert_eq!(header(&headers, "content-length"), Some("14"));
    assert!(
        header(&headers, "transfer-encoding").is_none(),
        "a buffered body must not claim chunked encoding: {headers}"
    );
    assert_eq!(
        header(&headers, "content-disposition"),
        Some("attachment; filename=\"legacy.bin\"; filename*=UTF-8''legacy.bin")
    );
    assert_eq!(body, b"buffered bytes");

    // --- ctx.stream(Body::from_stream): chunked, download headers intact ---
    let (headers, body) = http_get(addr, "/csv").await;
    assert!(headers.starts_with("HTTP/1.1 200"), "{headers}");
    assert_eq!(header(&headers, "content-type"), Some("text/csv"));
    assert_eq!(header(&headers, "transfer-encoding"), Some("chunked"));
    assert!(header(&headers, "content-length").is_none(), "{headers}");
    assert!(
        header(&headers, "content-disposition")
            .unwrap()
            .starts_with("attachment; filename=\"rows.csv\""),
        "{headers}"
    );
    let (payload, terminated) = dechunk(&body);
    assert!(
        terminated,
        "a complete stream must send the final zero chunk"
    );
    assert_eq!(payload, b"id,name\n1,alice\n2,bob\n");

    // --- a stream that fails mid-flight must never look complete ---
    // Hyper aborts the connection on a body error. Whether the client sees
    // nothing at all (error hit before the first flush) or a 200 with a
    // truncated body depends on write buffering; what must never happen is
    // a terminating zero chunk, which would pass the truncation off as the
    // whole payload.
    let raw = http_get_raw(addr, "/broken").await;
    if !raw.is_empty() {
        let (headers, body) = split_response(&raw);
        assert!(headers.starts_with("HTTP/1.1 200"), "{headers}");
        let (payload, terminated) = dechunk(&body);
        assert!(
            b"partial".starts_with(&payload),
            "only bytes produced before the error may reach the client"
        );
        assert!(
            !terminated,
            "an aborted stream must not send the final zero chunk"
        );
    }

    // --- ctx.sse: event-stream headers and wire format ---
    let (headers, body) = http_get(addr, "/events").await;
    assert!(headers.starts_with("HTTP/1.1 200"), "{headers}");
    assert_eq!(header(&headers, "content-type"), Some("text/event-stream"));
    assert_eq!(header(&headers, "cache-control"), Some("no-cache"));
    assert_eq!(header(&headers, "x-accel-buffering"), Some("no"));
    assert_eq!(header(&headers, "transfer-encoding"), Some("chunked"));
    let (payload, terminated) = dechunk(&body);
    assert!(terminated);
    assert_eq!(
        String::from_utf8(payload).unwrap(),
        "id: 1\ndata: hello\n\n\
         id: 2\nevent: tick\ndata: {\"n\":2}\n\n\
         id: 3\ndata: line one\ndata: line two\n\n"
    );

    // --- ctx.sse_with_keep_alive: comment arrives while the source idles ---
    let (headers, payload) = http_get_until(
        addr,
        "/events/keepalive",
        b"data: first\n\n:\n\n",
        Duration::from_secs(5),
    )
    .await;
    assert!(headers.starts_with("HTTP/1.1 200"), "{headers}");
    assert_eq!(header(&headers, "content-type"), Some("text/event-stream"));
    let text = String::from_utf8(payload).unwrap();
    assert!(
        text.starts_with("data: first\n\n:\n\n"),
        "expected the event then a keep-alive comment, got: {text:?}"
    );

    // --- ctx.file_download_stream_from: sized stream, attachment headers ---
    let (headers, body) = http_get(addr, "/download").await;
    assert!(headers.starts_with("HTTP/1.1 200"), "{headers}");
    assert_eq!(
        header(&headers, "content-length"),
        Some(BIG_LEN.to_string().as_str())
    );
    assert!(header(&headers, "transfer-encoding").is_none(), "{headers}");
    assert!(
        header(&headers, "content-disposition")
            .unwrap()
            .starts_with("attachment; filename=\"export.bin\""),
        "{headers}"
    );
    assert_eq!(body.len(), BIG_LEN);
    assert_eq!(body, big);

    // --- static file above the threshold: streamed with cache headers ---
    let (headers, body) = http_get(addr, "/public/big.bin").await;
    assert!(headers.starts_with("HTTP/1.1 200"), "{headers}");
    assert_eq!(
        header(&headers, "content-length"),
        Some(BIG_LEN.to_string().as_str())
    );
    assert!(header(&headers, "transfer-encoding").is_none(), "{headers}");
    assert!(
        header(&headers, "etag").is_some(),
        "static files keep their ETag: {headers}"
    );
    assert_eq!(body, big);

    // --- static file below the threshold: unchanged buffered behaviour ---
    let (headers, body) = http_get(addr, "/public/small.txt").await;
    assert!(headers.starts_with("HTTP/1.1 200"), "{headers}");
    assert_eq!(header(&headers, "content-length"), Some("17"));
    assert_eq!(body, b"tiny static asset");

    // --- zero-length streamed file: Content-Length: 0, empty body, no hang ---
    let (headers, body) = http_get(addr, "/empty").await;
    assert!(headers.starts_with("HTTP/1.1 200"), "{headers}");
    assert_eq!(header(&headers, "content-length"), Some("0"));
    assert!(header(&headers, "transfer-encoding").is_none(), "{headers}");
    assert!(body.is_empty());

    // --- HEAD on a streamed static file: GET's headers, no body ---
    let (headers, body) = http_head(addr, "/public/big.bin").await;
    assert!(headers.starts_with("HTTP/1.1 200"), "{headers}");
    assert_eq!(
        header(&headers, "content-length"),
        Some(BIG_LEN.to_string().as_str()),
        "HEAD must advertise the length GET would send: {headers}"
    );
    assert!(header(&headers, "etag").is_some(), "{headers}");
    assert!(body.is_empty(), "HEAD must not send a body");

    // --- HEAD on a GET route with no HEAD route: handler runs, body dropped ---
    let (headers, body) = http_head(addr, "/legacy").await;
    assert!(headers.starts_with("HTTP/1.1 200"), "{headers}");
    assert_eq!(header(&headers, "content-length"), Some("14"));
    assert_eq!(
        header(&headers, "content-type"),
        Some("application/octet-stream")
    );
    assert!(body.is_empty(), "HEAD must not send a body");

    // --- HEAD on an unsized stream: no body, and no invented length ---
    let (headers, body) = http_head(addr, "/csv").await;
    assert!(headers.starts_with("HTTP/1.1 200"), "{headers}");
    assert_eq!(header(&headers, "content-type"), Some("text/csv"));
    assert_ne!(
        header(&headers, "content-length"),
        Some("0"),
        "HEAD must not claim the GET body is empty: {headers}"
    );
    assert!(body.is_empty(), "HEAD must not send a body");

    // --- HEAD on a 204: no Content-Length at all ---
    let (headers, body) = http_head(addr, "/nothing").await;
    assert!(headers.starts_with("HTTP/1.1 204"), "{headers}");
    assert!(
        header(&headers, "content-length").is_none(),
        "204 never carries Content-Length: {headers}"
    );
    assert!(body.is_empty());

    running.handle.shutdown().await.unwrap();
}
