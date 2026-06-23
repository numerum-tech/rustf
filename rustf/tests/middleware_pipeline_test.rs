use bytes::Bytes;
use http_body_util::Full;
use hyper::Request as HyperRequest;
use rustf::middleware::builtin::{CorsMiddleware, RateLimitMiddleware};
use rustf::prelude::*;
use rustf::RustF;

fn install_text_route() -> Vec<Route> {
    async fn index(ctx: &mut Context) -> rustf::Result<()> {
        ctx.text("compress-me ".repeat(200))
    }

    routes![
        GET "/" => index,
    ]
}

#[tokio::test]
async fn outbound_only_compression_runs_through_app_pipeline() {
    let app = RustF::new()
        .with_compression()
        .controllers(install_text_route());

    let req = HyperRequest::builder()
        .method("GET")
        .uri("/")
        .header("Accept-Encoding", "gzip")
        .body(Full::<Bytes>::new(Bytes::new()))
        .unwrap();

    let res = app.handle_request(req).await.unwrap();

    assert_eq!(res.status, hyper::StatusCode::OK);
    assert!(res
        .headers
        .iter()
        .any(|(name, value)| { name.eq_ignore_ascii_case("content-encoding") && value == "gzip" }));
}

#[tokio::test]
async fn compression_removes_identity_etag_when_body_is_gzipped() {
    async fn with_etag(ctx: &mut Context) -> rustf::Result<()> {
        ctx.set_response(
            Response::new(hyper::StatusCode::OK)
                .with_header("Content-Type", "text/plain")
                .with_header("ETag", "\"identity-etag\"")
                .with_body("compress-me ".repeat(200).into_bytes()),
        );
        Ok(())
    }

    let app = RustF::new()
        .with_compression()
        .controllers(routes![GET "/" => with_etag]);

    let req = HyperRequest::builder()
        .method("GET")
        .uri("/")
        .header("Accept-Encoding", "gzip")
        .body(Full::<Bytes>::new(Bytes::new()))
        .unwrap();

    let res = app.handle_request(req).await.unwrap();

    assert_eq!(res.status, hyper::StatusCode::OK);
    assert!(res
        .headers
        .iter()
        .any(|(name, value)| { name.eq_ignore_ascii_case("content-encoding") && value == "gzip" }));
    assert!(
        !res.headers
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case("etag")),
        "gzip response must not keep the identity ETag"
    );
}

#[tokio::test]
async fn cors_preflight_short_circuits_route_and_keeps_headers() {
    let app = RustF::new()
        .middleware_from(|registry| registry.register_dual("cors", CorsMiddleware::new()))
        .controllers(install_text_route());

    let req = HyperRequest::builder()
        .method("OPTIONS")
        .uri("/missing-preflight-target")
        .header("Origin", "https://example.com")
        .header("Access-Control-Request-Method", "POST")
        .body(Full::<Bytes>::new(Bytes::new()))
        .unwrap();

    let res = app.handle_request(req).await.unwrap();

    assert_eq!(res.status, hyper::StatusCode::NO_CONTENT);
    assert!(res.headers.iter().any(|(name, value)| {
        name.eq_ignore_ascii_case("access-control-allow-origin") && value == "*"
    }));
}

#[tokio::test]
async fn rate_limit_success_headers_are_emitted_on_normal_responses() {
    let app = RustF::new()
        .middleware_from(|registry| {
            registry.register_dual("rate_limit", RateLimitMiddleware::new(3, 60))
        })
        .controllers(install_text_route());

    let req = HyperRequest::builder()
        .method("GET")
        .uri("/")
        .body(Full::<Bytes>::new(Bytes::new()))
        .unwrap();

    let res = app.handle_request(req).await.unwrap();

    assert_eq!(res.status, hyper::StatusCode::OK);
    assert!(res
        .headers
        .iter()
        .any(|(name, value)| { name.eq_ignore_ascii_case("x-ratelimit-limit") && value == "3" }));
    assert!(res.headers.iter().any(|(name, value)| {
        name.eq_ignore_ascii_case("x-ratelimit-remaining") && value == "2"
    }));
    assert!(res
        .headers
        .iter()
        .any(|(name, _)| { name.eq_ignore_ascii_case("x-ratelimit-reset") }));
}
