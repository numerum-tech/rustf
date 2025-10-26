use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rustf::context::Context;
use rustf::http::Request;
use rustf::session::{Session, SessionStore};
use rustf::views::ViewEngine;
use serde_json::json;
use std::sync::Arc;

fn benchmark_context_creation(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let views = Arc::new(ViewEngine::new());

    c.bench_function("context_creation", |b| {
        b.iter(|| {
            let mut request = Request::default();
            request.method = black_box("GET".to_string());
            request.uri = black_box("/test".to_string());

            let context = Context::new(request, Arc::clone(&views));

            black_box(context);
        })
    });
}

fn benchmark_context_methods(c: &mut Criterion) {
    let views = Arc::new(ViewEngine::new());

    let mut request = Request::default();
    request.method = "GET".to_string();
    request.uri = "/test/path".to_string();
    request
        .headers
        .insert("X-Real-IP".to_string(), "192.168.1.1".to_string());

    let context = Context::new(request, Arc::clone(&views));

    c.bench_function("context_url", |b| {
        b.iter(|| {
            let url = context.url();
            black_box(url);
        })
    });

    c.bench_function("context_ip", |b| {
        b.iter(|| {
            let ip = context.ip();
            black_box(ip);
        })
    });

    c.bench_function("context_is_xhr", |b| {
        b.iter(|| {
            let is_xhr = context.is_xhr();
            black_box(is_xhr);
        })
    });
}

fn benchmark_context_with_session(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let session_store = SessionStore::new();
    let views = Arc::new(ViewEngine::new());

    c.bench_function("context_with_session", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut request = Request::default();
                request.method = "GET".to_string();
                request.uri = "/test".to_string();

                let session = session_store
                    .get_or_create(black_box("bench_session"))
                    .await
                    .unwrap();
                let mut context = Context::new(request, Arc::clone(&views));
                context.set_session(Some(Arc::new(session)));

                black_box(context);
            })
        })
    });
}

fn benchmark_context_flash(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let session_store = SessionStore::new();
    let views = Arc::new(ViewEngine::new());

    c.bench_function("context_flash_operations", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut request = Request::default();
                request.method = "GET".to_string();
                request.uri = "/test".to_string();

                let session = session_store.get_or_create("flash_bench").await.unwrap();
                let session_arc = Arc::new(session);
                let mut context = Context::new(request, Arc::clone(&views));
                context.set_session(Some(session_arc.clone()));

                // Flash operations
                context.flash_success(black_box("Success message"));
                context.flash_error(black_box("Error message"));
                context.flash_info(black_box("Info message"));

                // Get flash messages through session
                let messages = session_arc.flash_get_all();
                black_box(messages);
            })
        })
    });
}

fn benchmark_context_view_data(c: &mut Criterion) {
    let views = Arc::new(ViewEngine::new());

    c.bench_function("context_view_preparation", |b| {
        b.iter(|| {
            let mut request = Request::default();
            request.method = "GET".to_string();
            request.uri = "/test".to_string();

            let mut context = Context::new(request, Arc::clone(&views));

            // Set layout
            context.layout(black_box("application"));

            // Set repository data (accessible in views)
            context.repository_set("title", json!("Test Page"));
            context.repository_set(
                "user",
                json!({
                    "id": 123,
                    "name": "Test User"
                }),
            );

            // Layout is set but no getter method available
            black_box(context);
        })
    });
}

fn benchmark_concurrent_context_access(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let session_store = Arc::new(SessionStore::new());
    let views = Arc::new(ViewEngine::new());

    c.bench_function("concurrent_context_creation", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut handles = Vec::new();

                for i in 0..10 {
                    let store_clone = Arc::clone(&session_store);
                    let views_clone = Arc::clone(&views);

                    let handle = tokio::spawn(async move {
                        let mut request = Request::default();
                        request.method = "GET".to_string();
                        request.uri = format!("/test/{}", i);

                        let session = store_clone
                            .get_or_create(&format!("session_{}", i))
                            .await
                            .unwrap();
                        let mut context = Context::new(request, views_clone);
                        context.set_session(Some(Arc::new(session)));

                        let _url = context.url();
                        let _ip = context.ip();

                        context
                    });

                    handles.push(handle);
                }

                for handle in handles {
                    let _context = handle.await.unwrap();
                }
            })
        })
    });
}

criterion_group!(
    benches,
    benchmark_context_creation,
    benchmark_context_methods,
    benchmark_context_with_session,
    benchmark_context_flash,
    benchmark_context_view_data,
    benchmark_concurrent_context_access
);
criterion_main!(benches);
