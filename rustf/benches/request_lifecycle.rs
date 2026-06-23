//! End-to-end request dispatch bench.
//!
//! Drives a hyper `Request<Body>` through `RustF::handle_request` — the same
//! entry point the real server uses — and times the full path: body read,
//! Request::from_hyper, routing, handler execution, Response assembly.
//!
//! Purpose: catch any future regression that slows down the request path as
//! a whole. Individual phases are covered by their own focused benches
//! (`routing.rs`, `middleware.rs`, `context.rs`, `compression.rs`, etc.);
//! this one is the smoke test that ties them together.
//!
//! Two flavours are measured:
//!   - `json_bare`    minimal app, JSON handler, no added middleware.
//!   - `json_compressed` same app with `.with_compression()` enabled and an
//!     `Accept-Encoding: gzip` header — shows the compression tax in situ.

use bytes::Bytes;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use http_body_util::Full;
use hyper::Request as HyperRequest;
use rustf::prelude::*;
use rustf::RustF;
use tokio::runtime::Runtime;

fn build_app(with_compression: bool) -> RustF {
    let app = RustF::new().controllers(vec![Route::new("GET", "/bench", |ctx| {
        Box::pin(async move {
            ctx.json(json!({
                "ok": true,
                "framework": "rustf",
                "echo": ctx.query("q").unwrap_or("").to_string(),
            }))?;
            Ok(())
        })
    })]);
    if with_compression {
        app.with_compression()
    } else {
        app
    }
}

fn make_request(with_gzip_accept: bool) -> HyperRequest<Full<Bytes>> {
    let mut builder = HyperRequest::builder().method("GET").uri("/bench?q=hello");
    if with_gzip_accept {
        builder = builder.header("accept-encoding", "gzip");
    }
    builder.body(Full::<Bytes>::new(Bytes::new())).unwrap()
}

fn bench_request_lifecycle(c: &mut Criterion) {
    let rt = Runtime::new().expect("tokio runtime");

    let bare_app = build_app(false);
    c.bench_function("request_lifecycle/json_bare", |b| {
        b.iter(|| {
            rt.block_on(async {
                let req = make_request(false);
                let res = bare_app
                    .handle_request(black_box(req))
                    .await
                    .expect("handle_request");
                black_box(res);
            })
        });
    });

    let gzip_app = build_app(true);
    c.bench_function("request_lifecycle/json_compressed", |b| {
        b.iter(|| {
            rt.block_on(async {
                let req = make_request(true);
                let res = gzip_app
                    .handle_request(black_box(req))
                    .await
                    .expect("handle_request");
                black_box(res);
            })
        });
    });
}

criterion_group!(benches, bench_request_lifecycle);
criterion_main!(benches);
