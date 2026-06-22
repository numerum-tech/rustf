use bytes::Bytes;
use http_body_util::Full;
use hyper::Request as HyperRequest;
use rustf::middleware::builtin::CorsMiddleware;
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
    let app = RustF::new().with_compression().controllers(install_text_route());

    let req = HyperRequest::builder()
        .method("GET")
        .uri("/")
        .header("Accept-Encoding", "gzip")
        .body(Full::<Bytes>::new(Bytes::new()))
        .unwrap();

    let res = app.handle_request(req).await.unwrap();

    assert_eq!(res.status, hyper::StatusCode::OK);
    assert!(res.headers.iter().any(|(name, value)| {
        name.eq_ignore_ascii_case("content-encoding") && value == "gzip"
    }));
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
