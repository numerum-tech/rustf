//! A handler that returns `Err` must produce the status the error names, not
//! a blanket `500`.
//!
//! `ctx.require_auth()?` is the case that matters most — it is how every guarded
//! route in a RustF app is written — but the mapping is general: `Error::status_code`
//! decides, and `Error::client_message` decides how much of the error the client
//! is allowed to read.
//!
//! One server for the whole binary: the framework's configuration is a
//! process-global singleton, so a second `RustF::new()` in the same test binary
//! fails.

use async_trait::async_trait;
use rustf::middleware::{InboundAction, InboundMiddleware, OutboundMiddleware};
use rustf::prelude::*;
use rustf::Error;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

// ------------------------------------------------------------- middleware

/// Stamps every outgoing response. Registered *before* the failing middleware,
/// so it is on the outbound stack when that one errors — which is how this
/// test proves an error response is not sent bare.
struct Stamp;

#[async_trait]
impl OutboundMiddleware for Stamp {
    async fn process_response(&self, ctx: &mut Context) -> rustf::Result<()> {
        if let Some(response) = ctx.res.as_mut() {
            response.add_header("X-Stamped", "yes");
        }
        Ok(())
    }
}

/// Fails for one path only, the way a real auth middleware fails.
struct RejectGuardedPath;

#[async_trait]
impl InboundMiddleware for RejectGuardedPath {
    async fn process_request(&self, ctx: &mut Context) -> rustf::Result<InboundAction> {
        if ctx.req.path() == "/by-middleware" {
            return Err(Error::authentication("Middleware says no"));
        }
        Ok(InboundAction::Continue)
    }
}

// ---------------------------------------------------------------- handlers

/// The canonical guarded route. Nothing here has a session, so `?` must turn
/// the authentication error into a `401`.
async fn guarded(ctx: &mut Context) -> rustf::Result<()> {
    ctx.require_auth()?;
    ctx.plain("secret")
}

/// A 4xx whose message belongs to the caller and must survive.
async fn validated(_ctx: &mut Context) -> rustf::Result<()> {
    Err(Error::validation("email is not a valid address"))
}

async fn refused(_ctx: &mut Context) -> rustf::Result<()> {
    Err(Error::authorization("administrators only"))
}

/// A 5xx whose message names internals and must NOT reach the client.
async fn boom(_ctx: &mut Context) -> rustf::Result<()> {
    Err(Error::internal(
        "postgres://admin:hunter2@10.0.0.7/billing is unreachable",
    ))
}

/// A 404 whose message names a server-side model and must not be echoed.
async fn missing(_ctx: &mut Context) -> rustf::Result<()> {
    Err(Error::ModelNotFound("internal_billing_ledger".into()))
}

/// `with_context` records internal detail; only the wrapped error decides what
/// the client sees.
async fn wrapped(_ctx: &mut Context) -> rustf::Result<()> {
    Err(Error::authentication("Session expired")
        .with_context("while verifying the admin token against /etc/rustf/keys/admin.pem"))
}

/// A handler that errors *after* writing a response. The partial response must
/// be discarded in favour of the error's.
async fn half_written(ctx: &mut Context) -> rustf::Result<()> {
    ctx.plain("half of a page")?;
    Err(Error::validation("changed my mind"))
}

async fn ok_page(ctx: &mut Context) -> rustf::Result<()> {
    ctx.plain("fine")
}

fn install_routes() -> Vec<Route> {
    routes![
        GET "/guarded"     => guarded,
        GET "/validated"   => validated,
        GET "/refused"     => refused,
        GET "/boom"        => boom,
        GET "/missing"     => missing,
        GET "/wrapped"     => wrapped,
        GET "/half"        => half_written,
        GET "/ok"          => ok_page,
        // Never reached: the middleware rejects this path first.
        GET "/by-middleware" => ok_page,
    ]
}

/// A second controller, whose `before` hook fails for every route under it.
fn install_hooked_routes() -> Vec<Route> {
    async fn before(ctx: &mut Context) -> rustf::Result<BeforeAction> {
        ctx.require_auth()?;
        Ok(BeforeAction::Continue)
    }

    routes![
        before: before,
        GET "/hooked" => ok_page,
    ]
}

// ----------------------------------------------------------------- helpers

struct HttpResponse {
    status: u16,
    headers: String,
    body: String,
}

/// Minimal HTTP/1.1 GET that reads the head and then exactly `Content-Length`
/// body bytes. `Connection: close` keeps the server from holding the socket
/// open for a keep-alive that this test will never use.
async fn http_get(addr: SocketAddr, path: &str, accept: Option<&str>) -> HttpResponse {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    let accept = match accept {
        Some(value) => format!("Accept: {value}\r\n"),
        None => String::new(),
    };
    let request =
        format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\n{accept}Connection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).await.unwrap();

    let read = async {
        let mut raw = Vec::new();
        let mut byte = [0u8; 1];
        loop {
            if stream.read(&mut byte).await.unwrap() == 0 {
                panic!("connection closed before the header terminator for {path}");
            }
            raw.push(byte[0]);
            if raw.ends_with(b"\r\n\r\n") {
                break;
            }
        }

        let head = String::from_utf8_lossy(&raw).into_owned();
        let status = head
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|code| code.parse::<u16>().ok())
            .unwrap_or_else(|| panic!("no status line in {head}"));

        let length = head
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())?
            })
            .unwrap_or(0);

        let mut body = vec![0u8; length];
        stream.read_exact(&mut body).await.unwrap();

        HttpResponse {
            status,
            headers: head,
            body: String::from_utf8_lossy(&body).into_owned(),
        }
    };

    tokio::time::timeout(Duration::from_secs(5), read)
        .await
        .unwrap_or_else(|_| panic!("timed out reading the response for {path}"))
}

// -------------------------------------------------------------------- test

#[tokio::test]
async fn handler_errors_carry_their_own_status() {
    let app = RustF::new()
        .middleware_from(|registry| {
            registry.register_outbound("stamp", Stamp);
            registry.register_inbound("reject_guarded_path", RejectGuardedPath);
        })
        .controllers(install_routes())
        .controllers(install_hooked_routes());
    let running = app.serve_with_handle("127.0.0.1:0").await.unwrap();
    let addr = running.local_addr;

    // --- a route that does not fail is unaffected ---
    let response = http_get(addr, "/ok", None).await;
    assert_eq!(response.status, 200, "{}", response.headers);
    assert_eq!(response.body, "fine");

    // --- the case this test exists for ---
    let response = http_get(addr, "/guarded", None).await;
    assert_eq!(
        response.status, 401,
        "ctx.require_auth()? must be a 401, not a 500: {}",
        response.headers
    );
    assert_eq!(response.body, "Authentication required");
    assert!(
        response.headers.to_lowercase().contains("text/plain"),
        "a browser request gets plain text: {}",
        response.headers
    );

    // --- the same failure inside a controller `before` hook ---
    let response = http_get(addr, "/hooked", None).await;
    assert_eq!(
        response.status, 401,
        "a before hook's error must map like a handler's: {}",
        response.headers
    );

    // --- content negotiation ---
    let response = http_get(addr, "/guarded", Some("application/json")).await;
    assert_eq!(response.status, 401);
    assert!(
        response.headers.to_lowercase().contains("application/json"),
        "{}",
        response.headers
    );
    let body: serde_json::Value = serde_json::from_str(&response.body)
        .unwrap_or_else(|e| panic!("body was not JSON ({e}): {}", response.body));
    assert_eq!(body["error"], "Authentication required");
    assert_eq!(body["code"], "E_AUTH");

    // A browser's Accept header names text/html and never application/json,
    // so it must not be answered with JSON.
    let response = http_get(
        addr,
        "/guarded",
        Some("text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"),
    )
    .await;
    assert!(
        response.headers.to_lowercase().contains("text/plain"),
        "a browser Accept must not produce JSON: {}",
        response.headers
    );

    // --- the rest of the mapping ---
    let response = http_get(addr, "/validated", None).await;
    assert_eq!(response.status, 400, "{}", response.headers);
    assert_eq!(response.body, "email is not a valid address");

    let response = http_get(addr, "/refused", None).await;
    assert_eq!(response.status, 403, "{}", response.headers);
    assert_eq!(response.body, "administrators only");

    let response = http_get(addr, "/missing", None).await;
    assert_eq!(response.status, 404, "{}", response.headers);
    assert_eq!(
        response.body, "Not Found",
        "a 404 must not echo the model name"
    );

    // --- a 500 keeps its detail to itself ---
    let response = http_get(addr, "/boom", None).await;
    assert_eq!(response.status, 500, "{}", response.headers);
    assert_eq!(response.body, "Internal Server Error");
    assert!(
        !response.body.contains("hunter2") && !response.body.contains("10.0.0.7"),
        "a 500 must not disclose its message: {}",
        response.body
    );

    let response = http_get(addr, "/boom", Some("application/json")).await;
    assert_eq!(response.status, 500);
    let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
    assert_eq!(body["error"], "Internal Server Error");
    assert_eq!(
        body["code"], "E_INTERNAL",
        "a 5xx must not name the variant that failed"
    );

    // --- context is for the log, not the client ---
    let response = http_get(addr, "/wrapped", None).await;
    assert_eq!(response.status, 401, "{}", response.headers);
    assert_eq!(response.body, "Session expired");
    assert!(
        !response.body.contains("admin.pem"),
        "with_context detail must not be disclosed: {}",
        response.body
    );

    // --- a partial response written before the error is discarded ---
    let response = http_get(addr, "/half", None).await;
    assert_eq!(response.status, 400, "{}", response.headers);
    assert_eq!(
        response.body, "changed my mind",
        "the half-written body must not survive the error"
    );

    // --- an inbound middleware's error maps too, and still gets the outbound
    //     stack, so the response is not sent without its headers ---
    let response = http_get(addr, "/by-middleware", None).await;
    assert_eq!(
        response.status, 401,
        "an inbound middleware's error must carry its own status: {}",
        response.headers
    );
    assert_eq!(response.body, "Middleware says no");
    assert!(
        response.headers.to_lowercase().contains("x-stamped: yes"),
        "outbound middleware registered before the failure must still run: {}",
        response.headers
    );

    // The same stamp must be on a handler-raised error, which travels the
    // ordinary outbound path.
    let response = http_get(addr, "/guarded", None).await;
    assert!(
        response.headers.to_lowercase().contains("x-stamped: yes"),
        "a handler error response must go through outbound middleware: {}",
        response.headers
    );

    running.handle.shutdown().await.unwrap();
}
