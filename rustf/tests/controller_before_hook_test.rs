//! Integration tests for the controller-level `before` hook.
//!
//! Each test builds a real `RustF` app with controllers + handlers,
//! drives a `hyper::Request<Body>` through `RustF::handle_request`, and
//! asserts on the resulting `Response`. Same pattern as
//! `benches/request_lifecycle.rs`.

use bytes::Bytes;
use http_body_util::Full;
use hyper::Request as HyperRequest;
use rustf::prelude::*;
use rustf::RustF;

// ---------------------------------------------------------------------
// Test 1: before runs and Continue lets the handler run.
// The hook sets a repository value; the handler echoes it as JSON. We
// assert the value reached the response, proving:
//   (a) before was called,
//   (b) Continue actually fell through to the handler,
//   (c) state set by before is visible to the handler.
// ---------------------------------------------------------------------
fn install_continue() -> Vec<Route> {
    // NOTE: `before` is declared AFTER the `routes![]` call — Rust
    // hoists item declarations (fn / async fn / struct / etc.) within a
    // function body, so this order is fine. We deliberately use this
    // shape here as a regression test against the order-mattering
    // assumption, even though the convention in real code is to put
    // `before` first for readability.
    let routes = routes![
        before: before,
        GET "/users/{id}" => show,
    ];

    async fn before(ctx: &mut Context) -> rustf::Result<BeforeAction> {
        ctx.repository_set("section", "users");
        ctx.repository_set("hook_ran", true);
        Ok(BeforeAction::Continue)
    }

    async fn show(ctx: &mut Context) -> rustf::Result<()> {
        let section = ctx
            .repository_get("section")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default();
        let hook_ran = ctx
            .repository_get("hook_ran")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        ctx.json(json!({
            "section": section,
            "hook_ran": hook_ran,
        }))
    }

    routes
}

#[tokio::test]
async fn before_runs_and_continues() {
    let app = RustF::new().controllers(install_continue());

    let req = HyperRequest::builder()
        .method("GET")
        .uri("/users/42")
        .body(Full::<Bytes>::new(Bytes::new()))
        .unwrap();

    let res = app
        .handle_request(req)
        .await
        .expect("handle_request returned Err");

    assert_eq!(
        res.status,
        hyper::StatusCode::OK,
        "expected 200, got {:?}",
        res.status
    );
    let body = String::from_utf8(res.body.clone()).unwrap();
    assert!(
        body.contains(r#""section":"users""#),
        "before's repository value missing from handler response: {}",
        body
    );
    assert!(
        body.contains(r#""hook_ran":true"#),
        "hook_ran flag not propagated: {}",
        body
    );
}

// ---------------------------------------------------------------------
// Test 2: before returning Stop short-circuits the handler.
// The hook calls ctx.redirect and returns Stop. The handler would set
// a sentinel header if it ran. We assert:
//   (a) response is 302,
//   (b) Location header is set by the hook,
//   (c) the sentinel header from the handler is ABSENT (proves the
//       handler never ran).
// ---------------------------------------------------------------------
fn install_stop() -> Vec<Route> {
    async fn before(ctx: &mut Context) -> rustf::Result<BeforeAction> {
        ctx.redirect("/login")?;
        Ok(BeforeAction::Stop)
    }

    async fn protected(ctx: &mut Context) -> rustf::Result<()> {
        // Sentinel — if this header appears in the response, the handler
        // ran and the Stop short-circuit failed.
        ctx.add_header("X-Handler-Ran", "yes");
        ctx.json(json!({"secret": true}))
    }

    routes![
        before: before,
        GET "/protected" => protected,
    ]
}

#[tokio::test]
async fn before_stops_short_circuits_handler() {
    let app = RustF::new().controllers(install_stop());

    let req = HyperRequest::builder()
        .method("GET")
        .uri("/protected")
        .body(Full::<Bytes>::new(Bytes::new()))
        .unwrap();

    let res = app
        .handle_request(req)
        .await
        .expect("handle_request returned Err");

    assert_eq!(
        res.status,
        hyper::StatusCode::FOUND,
        "Stop with redirect should produce 302, got {:?}",
        res.status
    );

    let location = res
        .headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("location"))
        .map(|(_, v)| v.as_str())
        .unwrap_or("");
    assert_eq!(location, "/login", "Location header missing or wrong");

    let handler_ran = res
        .headers
        .iter()
        .any(|(k, _)| k.eq_ignore_ascii_case("X-Handler-Ran"));
    assert!(
        !handler_ran,
        "Stop should have prevented the handler from running, but X-Handler-Ran is set"
    );
}

// ---------------------------------------------------------------------
// Test 3: a controller WITHOUT the `before:` clause still works.
// Pure backward-compat smoke test — the bare routes![] arm must produce
// routes whose handlers are invoked normally and whose `before` is None.
// ---------------------------------------------------------------------
fn install_no_hook() -> Vec<Route> {
    async fn ping(ctx: &mut Context) -> rustf::Result<()> {
        ctx.json(json!({"pong": true}))
    }

    routes![
        GET "/ping" => ping,
    ]
}

#[tokio::test]
async fn before_absent_is_backward_compatible() {
    let routes = install_no_hook();
    assert_eq!(routes.len(), 1);
    assert!(
        routes[0].before.is_none(),
        "bare routes![] must leave Route::before as None"
    );

    let app = RustF::new().controllers(install_no_hook());

    let req = HyperRequest::builder()
        .method("GET")
        .uri("/ping")
        .body(Full::<Bytes>::new(Bytes::new()))
        .unwrap();

    let res = app
        .handle_request(req)
        .await
        .expect("handle_request returned Err");

    assert_eq!(res.status, hyper::StatusCode::OK);
    let body = String::from_utf8(res.body.clone()).unwrap();
    assert!(
        body.contains(r#""pong":true"#),
        "handler did not run: {}",
        body
    );
}
