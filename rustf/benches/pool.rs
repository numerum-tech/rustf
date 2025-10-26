//! Object Pool Benchmarks
//!
//! These benchmarks demonstrate why RustF does NOT use object pooling for Request objects.
//!
//! **Results Summary:**
//! - Pooled allocation: ~90-105ns
//! - Direct allocation: ~40-57ns
//! - **Pool is approximately 2x SLOWER than direct allocation**
//!
//! **Why pooling doesn't help here:**
//! - Mutex lock overhead (4 separate locks per get/return cycle)
//! - Request::reset() overhead (clearing 6 collections)
//! - Statistics tracking overhead
//! - Rust's allocator (jemalloc/mimalloc) is already excellent for small objects
//! - Cache locality gains from LIFO strategy are outweighed by lock contention
//!
//! This benchmark is kept to document the decision to avoid using object pooling
//! in the RustF framework, despite it being a common optimization in other languages.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use hyper::StatusCode;
use rustf::http::{Request, Response};
use rustf::pool::global_request_pool;

fn benchmark_request_pool_allocation(c: &mut Criterion) {
    let pool = global_request_pool();

    c.bench_function("request_pool_get_return", |b| {
        b.iter(|| {
            let mut request = pool.get();
            request.uri = black_box("/test".to_string());
            request.method = black_box("GET".to_string());
            // Request automatically returned to pool when dropped
        })
    });
}

fn benchmark_request_direct_allocation(c: &mut Criterion) {
    c.bench_function("request_direct_new", |b| {
        b.iter(|| {
            let mut request = Request::default();
            request.uri = black_box("/test".to_string());
            request.method = black_box("GET".to_string());
            // Request dropped and deallocated
        })
    });
}

fn benchmark_response_direct_allocation(c: &mut Criterion) {
    c.bench_function("response_direct_new", |b| {
        b.iter(|| {
            let mut response = Response::ok();
            response.status = black_box(StatusCode::OK);
            response.body = black_box(b"Hello World".to_vec());
            // Response dropped and deallocated
        })
    });
}

fn benchmark_pool_with_payload_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("pool_path_sizes");
    let pool = global_request_pool();

    for size in [10, 100, 500, 1000].iter() {
        let path = format!("/api/v1/resource/{}", "x".repeat(*size));

        group.bench_with_input(BenchmarkId::new("pooled", size), size, |b, _| {
            b.iter(|| {
                let mut request = pool.get();
                request.uri = black_box(path.clone());
                request.method = black_box("POST".to_string());
                // Auto-return to pool
            })
        });

        group.bench_with_input(BenchmarkId::new("direct", size), size, |b, _| {
            b.iter(|| {
                let mut request = Request::default();
                request.uri = black_box(path.clone());
                request.method = black_box("POST".to_string());
                // Deallocated
            })
        });
    }
    group.finish();
}

fn benchmark_pool_stats(c: &mut Criterion) {
    let pool = global_request_pool();

    c.bench_function("pool_stats_collection", |b| {
        b.iter(|| {
            let stats = pool.stats();
            black_box(stats.hit_rate);
            black_box(stats.borrowed_count);
            black_box(stats.returned_count);
        })
    });
}

fn benchmark_concurrent_pool_access(c: &mut Criterion) {
    use std::sync::Arc;
    use std::thread;

    let pool = Arc::new(global_request_pool());

    c.bench_function("concurrent_pool_access", |b| {
        b.iter(|| {
            let handles: Vec<_> = (0..4)
                .map(|_| {
                    let pool_clone = pool.clone();
                    thread::spawn(move || {
                        for _ in 0..10 {
                            let mut req = pool_clone.get();
                            req.uri = "/test".to_string();
                            req.method = "GET".to_string();
                        }
                    })
                })
                .collect();

            for handle in handles {
                handle.join().unwrap();
            }
        })
    });
}

criterion_group!(
    benches,
    benchmark_request_pool_allocation,
    benchmark_request_direct_allocation,
    benchmark_response_direct_allocation,
    benchmark_pool_with_payload_sizes,
    benchmark_pool_stats,
    benchmark_concurrent_pool_access
);
criterion_main!(benches);
