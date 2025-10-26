use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rustf::context::Context;
use rustf::error::Result;
use rustf::http::{Request, Response};
use rustf::middleware::{InboundAction, InboundMiddleware, MiddlewareRegistry, OutboundMiddleware};
use rustf::views::ViewEngine;
use async_trait::async_trait;
use std::sync::Arc;

// Mock inbound middleware for benchmarking
struct BenchInboundMiddleware {
    name: &'static str,
    priority: i32,
}

#[async_trait]
impl InboundMiddleware for BenchInboundMiddleware {
    fn name(&self) -> &'static str {
        self.name
    }

    fn priority(&self) -> i32 {
        self.priority
    }

    async fn process_request(&self, _ctx: &mut Context) -> Result<InboundAction> {
        // Simple pass-through
        Ok(InboundAction::Continue)
    }
}

// Mock outbound middleware
struct BenchOutboundMiddleware {
    name: &'static str,
}

#[async_trait]
impl OutboundMiddleware for BenchOutboundMiddleware {
    async fn process_response(&self, ctx: &mut Context) -> Result<()> {
        // Add a header to simulate work
        if let Some(ref mut response) = ctx.res {
            response
                .headers
                .push(("X-Bench".to_string(), self.name.to_string()));
        }
        Ok(())
    }
}

fn benchmark_middleware_registry(c: &mut Criterion) {
    c.bench_function("middleware_registry_5_layers", |b| {
        b.iter(|| {
            let mut registry = MiddlewareRegistry::new();

            // Add 5 middleware layers
            for i in 0..5 {
                registry.register_inbound(
                    &format!("middleware_{}", i),
                    BenchInboundMiddleware {
                        name: "middleware",
                        priority: i * 10,
                    },
                );
            }

            // Get sorted by priority
            let _sorted = registry.get_sorted();

            black_box(registry);
        })
    });
}

fn benchmark_middleware_priority_sorting(c: &mut Criterion) {
    c.bench_function("middleware_priority_sort_20_items", |b| {
        b.iter(|| {
            let mut registry = MiddlewareRegistry::new();

            // Add middleware with random priorities
            for i in 0..20 {
                registry.register_inbound(
                    &format!("middleware_{}", i),
                    BenchInboundMiddleware {
                        name: "middleware",
                        priority: (i * 7) % 13 - 6, // Creates varied priorities
                    },
                );
            }

            // The registry sorts middleware by priority internally
            let _sorted = registry.get_sorted();
            black_box(registry);
        })
    });
}

fn benchmark_inbound_processing(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let views = Arc::new(ViewEngine::new());

    let mut group = c.benchmark_group("inbound_processing");

    // Benchmark with no middleware
    group.bench_function("no_middleware", |b| {
        b.iter(|| {
            let mut request = Request::default();
            request.method = black_box("GET".to_string());
            request.uri = black_box("/test".to_string());
            let context = Context::new(request, Arc::clone(&views));

            black_box(context);
        })
    });

    // Benchmark with 1 middleware
    group.bench_function("1_middleware", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut registry = MiddlewareRegistry::new();
                registry.register_inbound(
                    "single",
                    BenchInboundMiddleware {
                        name: "single",
                        priority: 0,
                    },
                );

                let mut request = Request::default();
                request.method = black_box("GET".to_string());
                request.uri = black_box("/test".to_string());
                let mut context = Context::new(request, Arc::clone(&views));

                // Simulate processing
                for middleware in registry.get_sorted() {
                    if let Some(inbound) = &middleware.inbound {
                        let _ = inbound.process_request(&mut context).await;
                    }
                }

                black_box(context);
            })
        })
    });

    // Benchmark with 10 middleware
    group.bench_function("10_middleware", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut registry = MiddlewareRegistry::new();
                for i in 0..10 {
                    registry.register_inbound(
                        &format!("middleware_{}", i),
                        BenchInboundMiddleware {
                            name: "middleware",
                            priority: i,
                        },
                    );
                }

                let mut request = Request::default();
                request.method = black_box("GET".to_string());
                request.uri = black_box("/test".to_string());
                let mut context = Context::new(request, Arc::clone(&views));

                // Simulate processing
                for middleware in registry.get_sorted() {
                    if let Some(inbound) = &middleware.inbound {
                        let _ = inbound.process_request(&mut context).await;
                    }
                }

                black_box(context);
            })
        })
    });

    group.finish();
}

fn benchmark_outbound_processing(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let views = Arc::new(ViewEngine::new());

    c.bench_function("outbound_processing_5_middleware", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut registry = MiddlewareRegistry::new();

                // Add 5 outbound middleware
                for i in 0..5 {
                    registry.register_outbound(
                        &format!("outbound_{}", i),
                        BenchOutboundMiddleware { name: "outbound" },
                    );
                }

                let request = Request::default();
                let mut context = Context::new(request, Arc::clone(&views));

                // Simulate outbound processing
                for middleware in registry.get_sorted() {
                    if let Some(outbound) = &middleware.outbound {
                        let _ = outbound.process_response(&mut context).await;
                    }
                }

                black_box(context);
            })
        })
    });
}

fn benchmark_dual_phase_middleware(c: &mut Criterion) {
    let views = Arc::new(ViewEngine::new());

    // Dual-phase middleware that does both inbound and outbound
    #[derive(Clone)]
    struct TimingMiddleware;

    #[async_trait]
    impl InboundMiddleware for TimingMiddleware {
        fn name(&self) -> &'static str {
            "timing"
        }

        fn priority(&self) -> i32 {
            -100
        }

        async fn process_request(&self, ctx: &mut Context) -> Result<InboundAction> {
            ctx.set("bench_time", std::time::Instant::now());
            Ok(InboundAction::Capture) // We want to process the response
        }
    }

    #[async_trait]
    impl OutboundMiddleware for TimingMiddleware {
        async fn process_response(&self, ctx: &mut Context) -> Result<()> {
            if let Some(start) = ctx.get::<std::time::Instant>("bench_time") {
                let duration = start.elapsed();
                if let Some(ref mut response) = ctx.res {
                    response.headers.push((
                        "X-Response-Time".to_string(),
                        format!("{}μs", duration.as_micros()),
                    ));
                }
            }
            Ok(())
        }
    }

    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("dual_phase_middleware", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut registry = MiddlewareRegistry::new();
                registry.register_dual("timing", TimingMiddleware);

                let mut request = Request::default();
                request.method = "GET".to_string();
                request.uri = "/test".to_string();
                let mut context = Context::new(request, Arc::clone(&views));

                // Inbound phase
                for middleware in registry.get_sorted() {
                    if let Some(inbound) = &middleware.inbound {
                        let _ = inbound.process_request(&mut context).await;
                    }
                }

                // Outbound phase
                for middleware in registry.get_sorted() {
                    if let Some(outbound) = &middleware.outbound {
                        let _ = outbound.process_response(&mut context).await;
                    }
                }

                black_box(context);
            })
        })
    });
}

criterion_group!(
    benches,
    benchmark_middleware_registry,
    benchmark_middleware_priority_sorting,
    benchmark_inbound_processing,
    benchmark_outbound_processing,
    benchmark_dual_phase_middleware
);
criterion_main!(benches);
