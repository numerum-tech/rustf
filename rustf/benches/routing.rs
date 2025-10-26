use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rustf::context::Context;
use rustf::error::Result;
use rustf::routing::{Route, Router};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

// Mock handler for benchmarking - matches the actual RouteHandler signature
fn mock_handler(_ctx: &mut Context) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
    Box::pin(async move {
        // Handler logic would go here
        Ok(())
    })
}

fn benchmark_static_routes(c: &mut Criterion) {
    let mut router = Router::new();

    // Add various static routes
    router.add_route(Route::new("GET", "/", mock_handler));
    router.add_route(Route::new("GET", "/about", mock_handler));
    router.add_route(Route::new("GET", "/contact", mock_handler));
    router.add_route(Route::new("GET", "/api/users", mock_handler));
    router.add_route(Route::new("GET", "/api/posts", mock_handler));

    let router = Arc::new(router);

    c.bench_function("static_route_match", |b| {
        b.iter(|| {
            let matched = router.match_route("GET", black_box("/api/users"));
            black_box(matched);
        })
    });

    c.bench_function("static_route_miss", |b| {
        b.iter(|| {
            let matched = router.match_route("GET", black_box("/nonexistent"));
            black_box(matched);
        })
    });
}

fn benchmark_dynamic_routes(c: &mut Criterion) {
    let mut router = Router::new();

    // Add dynamic routes with parameters
    router.add_route(Route::new("GET", "/users/{id}", mock_handler));
    router.add_route(Route::new("GET", "/users/{id}/posts/{post_id}", mock_handler));
    router.add_route(Route::new(
        "GET",
        "/api/v1/resources/{type}/{id}",
        mock_handler,
    ));

    let router = Arc::new(router);

    c.bench_function("dynamic_route_match", |b| {
        b.iter(|| {
            let matched = router.match_route("GET", black_box("/users/123/posts/456"));
            black_box(matched);
        })
    });

    c.bench_function("dynamic_single_param", |b| {
        b.iter(|| {
            let matched = router.match_route("GET", black_box("/users/789"));
            black_box(matched);
        })
    });
}

fn benchmark_wildcard_routes(c: &mut Criterion) {
    let mut router = Router::new();

    // Add wildcard routes
    router.add_route(Route::new("GET", "/static/*path", mock_handler));
    router.add_route(Route::new("GET", "/downloads/*file", mock_handler));
    router.add_route(Route::new("GET", "/api/*", mock_handler));

    let router = Arc::new(router);

    c.bench_function("wildcard_route_match", |b| {
        b.iter(|| {
            let matched = router.match_route("GET", black_box("/static/css/style.css"));
            black_box(matched);
        })
    });

    c.bench_function("wildcard_deep_path", |b| {
        b.iter(|| {
            let matched = router.match_route("GET", black_box("/static/js/vendor/lib/module.js"));
            black_box(matched);
        })
    });
}

fn benchmark_large_router(c: &mut Criterion) {
    let mut router = Router::new();

    // Add many routes to test scalability
    for i in 0..100 {
        router.add_route(Route::new("GET", &format!("/route{}", i), mock_handler));
        router.add_route(Route::new(
            "POST",
            &format!("/api/route{}", i),
            mock_handler,
        ));
        router.add_route(Route::new(
            "GET",
            &format!("/users/{}/profile", i),
            mock_handler,
        ));
    }

    let router = Arc::new(router);

    c.bench_function("large_router_match_early", |b| {
        b.iter(|| {
            let matched = router.match_route("GET", black_box("/route5"));
            black_box(matched);
        })
    });

    c.bench_function("large_router_match_late", |b| {
        b.iter(|| {
            let matched = router.match_route("GET", black_box("/route95"));
            black_box(matched);
        })
    });

    c.bench_function("large_router_miss", |b| {
        b.iter(|| {
            let matched = router.match_route("GET", black_box("/nonexistent"));
            black_box(matched);
        })
    });
}

fn benchmark_method_routing(c: &mut Criterion) {
    let mut router = Router::new();

    // Add routes with different HTTP methods
    router.add_route(Route::new("GET", "/api/resource", mock_handler));
    router.add_route(Route::new("POST", "/api/resource", mock_handler));
    router.add_route(Route::new("PUT", "/api/resource", mock_handler));
    router.add_route(Route::new("DELETE", "/api/resource", mock_handler));
    router.add_route(Route::new("PATCH", "/api/resource", mock_handler));

    let router = Arc::new(router);

    c.bench_function("method_routing_get", |b| {
        b.iter(|| {
            let matched = router.match_route(black_box("GET"), "/api/resource");
            black_box(matched);
        })
    });

    c.bench_function("method_routing_post", |b| {
        b.iter(|| {
            let matched = router.match_route(black_box("POST"), "/api/resource");
            black_box(matched);
        })
    });

    c.bench_function("method_routing_invalid", |b| {
        b.iter(|| {
            let matched = router.match_route(black_box("INVALID"), "/api/resource");
            black_box(matched);
        })
    });
}

criterion_group!(
    benches,
    benchmark_static_routes,
    benchmark_dynamic_routes,
    benchmark_wildcard_routes,
    benchmark_large_router,
    benchmark_method_routing
);
criterion_main!(benches);
