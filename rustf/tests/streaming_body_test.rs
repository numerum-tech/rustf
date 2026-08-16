//! End-to-end checks for streamed response bodies over a real socket.
//!
//! These exercise the wire format, which is where streaming actually differs
//! from buffering: a stream that declares its size must produce a
//! `Content-Length` and no chunked framing, while an open-ended stream must
//! fall back to `Transfer-Encoding: chunked`. Neither is observable from a
//! unit test against `Body` alone.

use bytes::Bytes;
use rustf::http::Body;
use rustf::prelude::*;
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Larger than one 64 KiB chunk, so the file genuinely spans several reads
/// and any chunk misordering or duplication shows up as a byte mismatch.
const PAYLOAD_LEN: usize = 200_000;

/// Cargo hands integration tests a scratch directory inside `target/`, which
/// keeps generated fixtures within the project tree.
fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("streaming_body_test")
}

/// Position-dependent bytes: a shifted, truncated, or repeated chunk changes
/// the content, unlike a constant fill which would hide such bugs.
fn payload() -> Vec<u8> {
    (0..PAYLOAD_LEN).map(|i| (i % 251) as u8).collect()
}

async fn write_fixture(name: &str, contents: &[u8]) -> PathBuf {
    let dir = fixture_dir();
    tokio::fs::create_dir_all(&dir).await.unwrap();
    let path = dir.join(name);
    tokio::fs::write(&path, contents).await.unwrap();
    path
}

async fn sized_stream_route(ctx: &mut Context) -> rustf::Result<()> {
    let response =
        rustf::http::Response::file_inline_stream_from(fixture_dir(), "large.bin").await?;
    ctx.set_response(response);
    Ok(())
}

async fn open_ended_stream_route(ctx: &mut Context) -> rustf::Result<()> {
    let chunks = futures::stream::iter(vec![
        Ok(Bytes::from_static(b"alpha")),
        Ok(Bytes::from_static(b"beta")),
        Ok(Bytes::from_static(b"gamma")),
    ]);

    ctx.set_response(
        rustf::http::Response::ok()
            .with_header("Content-Type", "text/plain")
            .with_body(Body::from_stream(chunks)),
    );
    Ok(())
}

fn install_routes() -> Vec<Route> {
    routes![
        GET "/large" => sized_stream_route,
        GET "/open" => open_ended_stream_route,
    ]
}

/// Issue a GET and return the raw response bytes, split into headers and body.
async fn http_get(addr: SocketAddr, path: &str) -> (String, Vec<u8>) {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        path, addr
    );
    stream.write_all(request.as_bytes()).await.unwrap();

    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).await.unwrap();

    let split = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .expect("response must contain a header terminator");
    let headers = String::from_utf8(raw[..split].to_vec()).expect("headers are ASCII");
    let body = raw[split + 4..].to_vec();

    (headers, body)
}

/// Decode `Transfer-Encoding: chunked` framing into the underlying bytes.
fn dechunk(mut body: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();

    loop {
        let line_end = body
            .windows(2)
            .position(|w| w == b"\r\n")
            .expect("chunk size line must be CRLF-terminated");
        let size_line = std::str::from_utf8(&body[..line_end]).expect("chunk size is ASCII");
        // A chunk header may carry `;extension` metadata after the size.
        let size_hex = size_line.split(';').next().unwrap().trim();
        let size = usize::from_str_radix(size_hex, 16).expect("chunk size is hexadecimal");

        body = &body[line_end + 2..];
        if size == 0 {
            break;
        }

        out.extend_from_slice(&body[..size]);
        // Skip the payload and its trailing CRLF.
        body = &body[size + 2..];
    }

    out
}

/// Both wire-format cases share one server.
///
/// The framework's configuration is a process-global singleton, so a second
/// `RustF::new()` in the same test binary fails with "Configuration has
/// already been initialized". Tests in a binary run as parallel threads of one
/// process, so exactly one of them may boot an app.
#[tokio::test]
async fn streams_use_the_correct_wire_format_for_known_and_unknown_lengths() {
    let expected = payload();
    write_fixture("large.bin", &expected).await;

    let app = RustF::new().controllers(install_routes());
    let running = app.serve_with_handle("127.0.0.1:0").await.unwrap();
    let addr = running.local_addr;

    // --- a file of known length: Content-Length, no chunked framing ---
    let (headers, body) = http_get(addr, "/large").await;

    assert!(
        headers.starts_with("HTTP/1.1 200"),
        "unexpected status line in: {headers}"
    );

    // A file of known length must advertise it, so clients can render real
    // progress rather than an indeterminate spinner.
    let lower = headers.to_lowercase();
    assert!(
        lower.contains(&format!("content-length: {PAYLOAD_LEN}")),
        "expected content-length {PAYLOAD_LEN} in: {headers}"
    );
    assert!(
        !lower.contains("transfer-encoding: chunked"),
        "a sized stream must not be chunked: {headers}"
    );

    assert_eq!(body.len(), PAYLOAD_LEN, "streamed body length mismatch");
    assert_eq!(
        body, expected,
        "streamed bytes differ from the file on disk"
    );

    // --- a stream of unknown length: chunked framing, no Content-Length ---
    let (headers, body) = http_get(addr, "/open").await;

    assert!(
        headers.starts_with("HTTP/1.1 200"),
        "unexpected status line in: {headers}"
    );

    let lower = headers.to_lowercase();
    assert!(
        lower.contains("transfer-encoding: chunked"),
        "a stream of unknown length must be chunked: {headers}"
    );
    assert!(
        !lower.contains("content-length:"),
        "an unsized stream cannot claim a content length: {headers}"
    );

    assert_eq!(dechunk(&body), b"alphabetagamma".to_vec());

    running.handle.shutdown().await.unwrap();
}

#[tokio::test]
async fn streamed_file_matches_the_buffered_response_byte_for_byte() {
    let expected = payload();
    write_fixture("large.bin", &expected).await;

    // The streaming path must be a drop-in replacement: same bytes, same
    // content type. Only the memory profile differs.
    let buffered = rustf::http::Response::file_inline_from(fixture_dir(), "large.bin")
        .await
        .unwrap();
    let streamed = rustf::http::Response::file_inline_stream_from(fixture_dir(), "large.bin")
        .await
        .unwrap();

    assert_eq!(buffered.status, streamed.status);
    assert_eq!(
        buffered.body.as_slice().map(<[u8]>::to_vec),
        Some(expected.clone())
    );

    // The buffered response holds every byte; the streamed one holds none yet
    // but knows how many are coming.
    assert!(streamed.body.is_stream());
    assert_eq!(streamed.body.len_hint(), Some(PAYLOAD_LEN));

    let streamed_type = streamed
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
        .map(|(_, value)| value.clone());
    let buffered_type = buffered
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
        .map(|(_, value)| value.clone());
    assert_eq!(streamed_type, buffered_type);
}

#[tokio::test]
async fn streaming_a_directory_is_rejected_before_any_bytes_are_sent() {
    let dir = fixture_dir();
    tokio::fs::create_dir_all(dir.join("subdir")).await.unwrap();

    // Must fail as an ordinary error while the status is still changeable —
    // a directory's metadata length describes no readable bytes, so accepting
    // it would emit a Content-Length the body never satisfies.
    let result = rustf::http::Response::file_inline_stream_from(&dir, "subdir").await;

    assert!(result.is_err(), "streaming a directory must not succeed");
}
