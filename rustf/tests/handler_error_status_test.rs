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

/// Stamps every outgoing response, the way `SecurityHeadersMiddleware` does.
/// Outbound-only at the default priority 0, so it sits *after* the failing
/// middleware (priority -100) in the sorted list — the inbound loop has not
/// reached it when that one errors or stops. Proves an early exit is still not
/// sent bare.
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

/// Dual-phase and sorted *after* the failing middleware: when that one exits
/// early, this one's inbound phase never ran, so its outbound phase must not
/// run either.
#[derive(Clone)]
struct LateDual;

#[async_trait]
impl InboundMiddleware for LateDual {
    async fn process_request(&self, _ctx: &mut Context) -> rustf::Result<InboundAction> {
        Ok(InboundAction::Capture)
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[async_trait]
impl OutboundMiddleware for LateDual {
    async fn process_response(&self, ctx: &mut Context) -> rustf::Result<()> {
        if let Some(response) = ctx.res.as_mut() {
            response.add_header("X-Late-Dual", "yes");
        }
        Ok(())
    }
}

/// Fails (or stops) for one path only, the way a real auth middleware does —
/// and, like one, runs early.
struct RejectGuardedPath;

#[async_trait]
impl InboundMiddleware for RejectGuardedPath {
    async fn process_request(&self, ctx: &mut Context) -> rustf::Result<InboundAction> {
        match ctx.req.path() {
            "/by-middleware" => Err(Error::authentication("Middleware says no")),
            "/stopped-by-middleware" => {
                ctx.throw403(Some("Stopped here"))?;
                Ok(InboundAction::Stop)
            }
            _ => Ok(InboundAction::Continue),
        }
    }

    fn priority(&self) -> i32 {
        -100
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

/// A dependency that did not answer in time: the server's problem, not the
/// client's, so a 504 — and the worker's name stays in the log.
async fn slow(_ctx: &mut Context) -> rustf::Result<()> {
    Err(Error::timeout("Worker 'ledger-sync' timed out after 30s"))
}

/// A 5xx other than `Internal`: the masking must hold for every variant.
async fn db_down(_ctx: &mut Context) -> rustf::Result<()> {
    Err(Error::database_query(
        "relation \"internal_billing_ledger\" does not exist",
    ))
}

/// A body the client sent and the server cannot parse is the client's error.
async fn parse_json(ctx: &mut Context) -> rustf::Result<()> {
    let value: serde_json::Value = ctx.body_json()?;
    ctx.json(value)
}

/// A typed payload: the deserializer's complaint names a server-side field,
/// which must not reach the client.
#[derive(serde::Deserialize)]
#[allow(dead_code)]
struct Payment {
    internal_account_id: u64,
}

async fn parse_typed(ctx: &mut Context) -> rustf::Result<()> {
    let _payment: Payment = ctx.body_json()?;
    ctx.plain("paid")
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
        GET "/slow"        => slow,
        GET "/db"          => db_down,
        POST "/json"       => parse_json,
        POST "/pay"        => parse_typed,
        // Never reached: the middleware rejects these paths first.
        GET "/by-middleware" => ok_page,
        GET "/stopped-by-middleware" => ok_page,
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

/// GET with an optional `Accept` header.
async fn http_get(addr: SocketAddr, path: &str, accept: Option<&str>) -> HttpResponse {
    let headers: Vec<(&str, &str)> = accept.map(|value| ("Accept", value)).into_iter().collect();
    http_send(addr, "GET", path, &headers, "").await
}

/// Minimal HTTP/1.1 client that reads the head and then exactly
/// `Content-Length` body bytes. `Connection: close` keeps the server from
/// holding the socket open for a keep-alive that this test will never use.
async fn http_send(
    addr: SocketAddr,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: &str,
) -> HttpResponse {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    let mut request = format!("{method} {path} HTTP/1.1\r\nHost: {addr}\r\n");
    for (name, value) in headers {
        request.push_str(&format!("{name}: {value}\r\n"));
    }
    if !body.is_empty() {
        request.push_str(&format!("Content-Length: {}\r\n", body.len()));
    }
    request.push_str("Connection: close\r\n\r\n");
    request.push_str(body);
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
            registry.register_dual("late_dual", LateDual);
        })
        .controllers(install_routes())
        .controllers(install_hooked_routes());
    let running = app.serve_with_handle("127.0.0.1:0").await.unwrap();
    let addr = running.local_addr;

    // --- a route that does not fail is unaffected ---
    let response = http_get(addr, "/ok", None).await;
    assert_eq!(response.status, 200, "{}", response.headers);
    assert_eq!(response.body, "fine");
    assert!(
        response.headers.to_lowercase().contains("x-late-dual: yes"),
        "on a normal request the dual-phase middleware's outbound runs: {}",
        response.headers
    );

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
    //     stack — including outbound-only middleware sorted *after* it, which
    //     is where `SecurityHeadersMiddleware` sits relative to an auth
    //     middleware ---
    let response = http_get(addr, "/by-middleware", None).await;
    assert_eq!(
        response.status, 401,
        "an inbound middleware's error must carry its own status: {}",
        response.headers
    );
    assert_eq!(response.body, "Middleware says no");
    assert!(
        response.headers.to_lowercase().contains("x-stamped: yes"),
        "outbound-only middleware after the failing one must still run: {}",
        response.headers
    );
    assert!(
        !response.headers.to_lowercase().contains("x-late-dual"),
        "a dual-phase middleware whose inbound never ran must not run outbound: {}",
        response.headers
    );

    // The same for an inbound `Stop` from an early middleware.
    let response = http_get(addr, "/stopped-by-middleware", None).await;
    assert_eq!(response.status, 403, "{}", response.headers);
    assert!(
        response.headers.to_lowercase().contains("x-stamped: yes"),
        "an early inbound Stop must still get outbound-only middleware: {}",
        response.headers
    );
    assert!(
        !response.headers.to_lowercase().contains("x-late-dual"),
        "a Stop must not run a later dual-phase middleware's outbound: {}",
        response.headers
    );
    assert_eq!(
        response
            .headers
            .to_lowercase()
            .matches("x-stamped:")
            .count(),
        1,
        "each outbound middleware runs exactly once: {}",
        response.headers
    );

    // --- a dependency timeout is a 504, and names nothing ---
    let response = http_get(addr, "/slow", None).await;
    assert_eq!(response.status, 504, "{}", response.headers);
    assert_eq!(response.body, "Gateway Timeout");
    let response = http_get(addr, "/slow", Some("application/json")).await;
    let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
    assert_eq!(
        body["code"], "E_INTERNAL",
        "a 5xx must not name the variant"
    );

    // --- every 5xx variant is masked, not only `Internal` ---
    let response = http_get(addr, "/db", None).await;
    assert_eq!(response.status, 500, "{}", response.headers);
    assert_eq!(response.body, "Internal Server Error");
    let response = http_get(addr, "/db", Some("application/json")).await;
    assert!(
        !response.body.contains("internal_billing_ledger"),
        "a database error must not leak through JSON: {}",
        response.body
    );

    // --- malformed JSON from the client is a 400, not a 500 ---
    let response = http_send(
        addr,
        "POST",
        "/json",
        &[("Content-Type", "application/json")],
        "{not json",
    )
    .await;
    assert_eq!(response.status, 400, "{}", response.headers);
    assert_eq!(response.body, "Invalid JSON body");
    let response = http_send(
        addr,
        "POST",
        "/pay",
        &[("Content-Type", "application/json")],
        "{}",
    )
    .await;
    assert_eq!(response.status, 400, "{}", response.headers);
    assert!(
        !response.body.contains("internal_account_id"),
        "the deserializer's diagnostic must not name server-side fields: {}",
        response.body
    );
    let response = http_send(
        addr,
        "POST",
        "/json",
        &[("Content-Type", "application/json")],
        r#"{"ok":true}"#,
    )
    .await;
    assert_eq!(response.status, 200, "{}", response.headers);

    // --- htmx swaps the body into the page: it never gets JSON on its own ---
    let response = http_send(addr, "GET", "/guarded", &[("HX-Request", "true")], "").await;
    assert_eq!(response.status, 401);
    assert!(
        response.headers.to_lowercase().contains("text/plain"),
        "htmx must not receive a JSON error: {}",
        response.headers
    );
    // ...while a classic XHR with no Accept preference still does.
    let response = http_send(
        addr,
        "GET",
        "/guarded",
        &[("X-Requested-With", "XMLHttpRequest")],
        "",
    )
    .await;
    assert!(
        response.headers.to_lowercase().contains("application/json"),
        "a classic XHR gets JSON: {}",
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
