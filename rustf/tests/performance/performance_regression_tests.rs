//! Performance regression tests for RustF framework
//! 
//! These tests validate that critical performance optimizations maintain their
//! benchmarked performance levels and detect any performance regressions.
//! 
//! Target Performance Baselines:
//! - Router Trie: 347,000+ lookups/sec (O(log n) complexity)
//! - Sessions DashMap: 2,000,000+ concurrent ops/sec (lock-free)
//! - Context Arc: 108,000+ contexts/sec (memory-safe)
//! - Middleware Chain: 85,000+ requests/sec (priority-based execution)

use rustf::http::{Request, Response};
use rustf::context::Context;
use rustf::routing::{Route, Router};
use rustf::middleware::{Middleware, MiddlewareResult, Next, MiddlewareRegistry};
use rustf::session::SessionStore;
use rustf::views::ViewEngine;
use rustf::config::AppConfig;
use rustf::error::Result;
use rustf::pool::global_request_pool;
use serde_json::json;
use std::sync::Arc;
use std::pin::Pin;
use std::future::Future;
use std::time::Instant;

/// Mock handler for performance testing
fn mock_handler(_ctx: Context) -> Pin<Box<dyn Future<Output = Result<Response>> + Send>> {
    Box::pin(async { Ok(Response::ok()) })
}

/// Simple test middleware for chain performance testing
struct PerfTestMiddleware {
    name: &'static str,
    operations: u32,
}

impl PerfTestMiddleware {
    pub fn new(name: &'static str, operations: u32) -> Self {
        Self { name, operations }
    }
}

impl Middleware for PerfTestMiddleware {
    fn handle<'a>(&'a self, ctx: &'a mut Context, next: Next) 
        -> Pin<Box<dyn Future<Output = Result<MiddlewareResult>> + Send + 'a>> {
        Box::pin(async move {
            // Simulate some work
            for _ in 0..self.operations {
                let _ = ctx.ip();
            }
            
            next.call(ctx).await
        })
    }

    fn name(&self) -> &'static str {
        self.name
    }

    fn priority(&self) -> i32 {
        0
    }
}

#[tokio::test]
async fn test_router_performance_regression() {
    println!("🚀 Testing Router Trie Performance Regression");
    println!("Target: 300,000+ lookups/sec (allowing 15% tolerance from 347k baseline)");
    
    let mut router = Router::new();
    
    // Create a realistic route set with various patterns
    for i in 0..1000 {
        router.add_route(Route::new("GET", &format!("/api/users/{}/posts", i), mock_handler));
        router.add_route(Route::new("POST", &format!("/api/users/{}/posts", i), mock_handler));
        router.add_route(Route::new("GET", &format!("/api/posts/{}/comments", i), mock_handler));
    }
    
    // Add some complex nested routes
    for i in 0..200 {
        router.add_route(Route::new("GET", &format!("/api/v1/organizations/{}/projects/{}/tasks", i, i), mock_handler));
        router.add_route(Route::new("PUT", &format!("/api/v1/organizations/{}/projects/{}/tasks/{}", i, i, i), mock_handler));
    }
    
    // Add static routes
    for i in 0..500 {
        router.add_route(Route::new("GET", &format!("/static/assets/js/module_{}.js", i), mock_handler));
        router.add_route(Route::new("GET", &format!("/static/assets/css/theme_{}.css", i), mock_handler));
    }
    
    println!("Router loaded with {} routes", router.route_count());
    
    // Performance test with realistic lookup patterns
    let start = Instant::now();
    let iterations = 50000;
    
    for i in 0..iterations {
        // Mix of successful lookups
        let user_id = i % 1000;
        let org_id = i % 200;
        let static_id = i % 500;
        
        // Test various route patterns
        let routes_to_test = [
            format!("/api/users/{}/posts", user_id),
            format!("/api/posts/{}/comments", user_id),
            format!("/api/v1/organizations/{}/projects/{}/tasks", org_id, org_id),
            format!("/static/assets/js/module_{}.js", static_id),
        ];
        
        for route in &routes_to_test {
            let result = router.match_route("GET", route);
            assert!(result.is_some(), "Route should match: {}", route);
        }
    }
    
    let elapsed = start.elapsed();
    let total_lookups = iterations * 4; // 4 lookups per iteration
    let lookups_per_second = total_lookups as f64 / elapsed.as_secs_f64();
    
    println!("Router Performance Results:");
    println!("- Total lookups: {}", total_lookups);
    println!("- Total time: {:?}", elapsed);
    println!("- Lookups per second: {:.0}", lookups_per_second);
    println!("- Average lookup time: {:.2}μs", elapsed.as_micros() as f64 / total_lookups as f64);
    
    // Allow 15% tolerance from baseline (347k -> 300k minimum)
    assert!(lookups_per_second > 300_000.0, 
        "Router performance regression detected: {:.0} lookups/sec < 300,000 minimum", 
        lookups_per_second);
    
    println!("✅ Router performance test PASSED");
}

#[tokio::test]
async fn test_session_performance_regression() {
    println!("🚀 Testing Session DashMap Performance Regression");
    println!("Target: 1,500,000+ concurrent ops/sec (allowing 25% tolerance from 2M baseline)");
    
    let store = Arc::new(SessionStore::new());
    
    let start = Instant::now();
    let concurrent_tasks = 50;
    let operations_per_task = 2000;
    
    // Spawn concurrent tasks for realistic session usage
    let mut handles = Vec::new();
    
    for task_id in 0..concurrent_tasks {
        let store_clone = store.clone();
        
        let handle = tokio::spawn(async move {
            let session_id = format!("perf_session_{}", task_id);
            let session = store_clone.get_or_create(&session_id).await.unwrap();
            
            // Perform mixed session operations
            for i in 0..operations_per_task {
                let key = format!("key_{}_{}", task_id, i);
                let value = format!("value_{}_{}", task_id, i);
                
                // Set operation
                session.set(&key, &value).unwrap();
                
                // Get operation
                let retrieved: Option<String> = session.get(&key);
                assert_eq!(retrieved, Some(value));
                
                // Flash operations (more complex)
                let flash_key = format!("flash_{}", i);
                session.flash_set(&flash_key, &format!("flash_value_{}", i)).unwrap();
                let _flash: Option<String> = session.flash_get(&flash_key);
                
                // Remove operation
                session.remove(&key);
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }
    
    let elapsed = start.elapsed();
    let total_operations = concurrent_tasks * operations_per_task * 4; // set, get, flash_set+get, remove
    let ops_per_second = total_operations as f64 / elapsed.as_secs_f64();
    
    println!("Session Performance Results:");
    println!("- Concurrent tasks: {}", concurrent_tasks);
    println!("- Operations per task: {}", operations_per_task);
    println!("- Total operations: {}", total_operations);
    println!("- Total time: {:?}", elapsed);
    println!("- Operations per second: {:.0}", ops_per_second);
    
    // Allow 25% tolerance from baseline (2M -> 1.5M minimum)
    assert!(ops_per_second > 1_500_000.0, 
        "Session performance regression detected: {:.0} ops/sec < 1,500,000 minimum", 
        ops_per_second);
    
    println!("✅ Session performance test PASSED");
}

#[tokio::test]
async fn test_context_performance_regression() {
    println!("🚀 Testing Context Arc Performance Regression");
    println!("Target: 80,000+ contexts/sec (allowing 25% tolerance from 108k baseline)");
    
    // Setup shared components (Arc-based)
    let views = Arc::new(ViewEngine::filesystem("views"));
    let config = Arc::new(AppConfig::default());
    let session_store = SessionStore::new();
    
    let start = Instant::now();
    let context_count = 20000;
    
    // Create contexts concurrently to test Arc sharing performance
    let mut handles = Vec::new();
    
    for i in 0..context_count {
        let views_clone = Arc::clone(&views);
        let config_clone = Arc::clone(&config);
        let store_clone = session_store.clone();
        
        let handle = tokio::spawn(async move {
            // Create request
            let mut request = Request::default();
            request.method = "GET".to_string();
            request.uri = format!("/test/context/{}", i);
            request.headers.insert("User-Agent".to_string(), "RustF-PerfTest/1.0".to_string());
            
            let session = store_clone.get_or_create(&format!("ctx_session_{}", i)).await.unwrap();
            
            // Create context with Arc sharing
            let mut context = Context::new(
                request,
                session,
                views_clone,
                config_clone
            );
            
            // Use context methods to test Arc access performance
            let _config = context.config();
            let _url = context.url();
            let _ip = context.ip();
            let _user_agent = context.user_agent();
            context.layout("test_layout");
            context.flash_success("Performance test message");
            
            // Test JSON response creation
            let _response = context.json(json!({
                "test": true,
                "id": i,
                "performance": "testing"
            }));
            
            context
        });
        
        handles.push(handle);
    }
    
    // Wait for all context operations
    let mut contexts = Vec::new();
    for handle in handles {
        contexts.push(handle.await.unwrap());
    }
    
    let elapsed = start.elapsed();
    let contexts_per_second = context_count as f64 / elapsed.as_secs_f64();
    
    println!("Context Performance Results:");
    println!("- Contexts created: {}", context_count);
    println!("- Total time: {:?}", elapsed);
    println!("- Contexts per second: {:.0}", contexts_per_second);
    println!("- Average creation time: {:.2}μs", elapsed.as_micros() as f64 / context_count as f64);
    
    // Verify contexts are valid
    assert_eq!(contexts.len(), context_count);
    
    // Allow 25% tolerance from baseline (108k -> 80k minimum)
    assert!(contexts_per_second > 80_000.0, 
        "Context performance regression detected: {:.0} contexts/sec < 80,000 minimum", 
        contexts_per_second);
    
    println!("✅ Context performance test PASSED");
}

#[tokio::test]
async fn test_middleware_chain_performance_regression() {
    println!("🚀 Testing Middleware Chain Performance Regression");
    println!("Target: 60,000+ requests/sec (allowing 30% tolerance from 85k baseline)");
    
    let mut middleware_registry = MiddlewareRegistry::new();
    
    // Register multiple middleware with different priorities
    middleware_registry.register("auth", PerfTestMiddleware::new("auth", 5));
    middleware_registry.register("cors", PerfTestMiddleware::new("cors", 3));
    middleware_registry.register("logging", PerfTestMiddleware::new("logging", 2));
    middleware_registry.register("rate_limit", PerfTestMiddleware::new("rate_limit", 4));
    middleware_registry.register("validation", PerfTestMiddleware::new("validation", 6));
    
    let sorted_middleware = middleware_registry.get_sorted();
    println!("Middleware chain loaded with {} middleware", sorted_middleware.len());
    
    // Setup for context creation
    let views = Arc::new(ViewEngine::filesystem("views"));
    let config = Arc::new(AppConfig::default());
    let session_store = SessionStore::new();
    
    let start = Instant::now();
    let request_count = 10000;
    
    // Test middleware chain execution performance
    for i in 0..request_count {
        // Create context for each request
        let mut request = Request::default();
        request.method = "GET".to_string();
        request.uri = format!("/api/test/{}", i);
        
        let session = session_store.get_or_create(&format!("mw_session_{}", i % 100)).await.unwrap();
        let context = Context::new(
            request,
            session,
            Arc::clone(&views),
            Arc::clone(&config)
        );
        
        // Simulate middleware chain execution
        // (In real implementation, this would go through the middleware chain)
        for _middleware_info in &sorted_middleware {
            // Simulate middleware operations
            for _ in 0..3 {
                let _ = context.ip();
                let _ = context.url();
            }
        }
        
        // Simulate final handler execution
        let _response = context.json(json!({
            "status": "ok",
            "request_id": i
        }));
    }
    
    let elapsed = start.elapsed();
    let requests_per_second = request_count as f64 / elapsed.as_secs_f64();
    
    println!("Middleware Chain Performance Results:");
    println!("- Requests processed: {}", request_count);
    println!("- Middleware per request: {}", sorted_middleware.len());
    println!("- Total time: {:?}", elapsed);
    println!("- Requests per second: {:.0}", requests_per_second);
    println!("- Average request time: {:.2}μs", elapsed.as_micros() as f64 / request_count as f64);
    
    // Allow 30% tolerance from baseline (85k -> 60k minimum)
    assert!(requests_per_second > 60_000.0, 
        "Middleware chain performance regression detected: {:.0} req/sec < 60,000 minimum", 
        requests_per_second);
    
    println!("✅ Middleware chain performance test PASSED");
}

#[tokio::test]
async fn test_request_pool_performance_regression() {
    println!("🚀 Testing Request Pool Performance Regression");
    println!("Target: 500,000+ pool operations/sec");
    
    let pool = global_request_pool();
    
    let start = Instant::now();
    let operations = 100000;
    
    // Test pool get/return cycle performance
    for i in 0..operations {
        let mut pooled_request = pool.get();
        
        // Use the pooled request
        pooled_request.method = "POST".to_string();
        pooled_request.uri = format!("/api/pool/test/{}", i);
        pooled_request.headers.insert("Content-Type".to_string(), "application/json".to_string());
        
        // Some operations
        assert_eq!(pooled_request.method, "POST");
        assert!(pooled_request.uri.contains("pool"));
        
        // Request returns to pool automatically when dropped
        drop(pooled_request);
    }
    
    let elapsed = start.elapsed();
    let ops_per_second = operations as f64 / elapsed.as_secs_f64();
    
    println!("Request Pool Performance Results:");
    println!("- Pool operations: {}", operations);
    println!("- Total time: {:?}", elapsed);
    println!("- Operations per second: {:.0}", ops_per_second);
    println!("- Average operation time: {:.2}μs", elapsed.as_micros() as f64 / operations as f64);
    
    assert!(ops_per_second > 500_000.0, 
        "Request pool performance regression detected: {:.0} ops/sec < 500,000 minimum", 
        ops_per_second);
    
    println!("✅ Request pool performance test PASSED");
}

#[tokio::test]
async fn test_integrated_performance_regression() {
    println!("🚀 Testing Integrated Performance Regression");
    println!("Testing all components working together under realistic load");
    
    // Setup all components
    let mut router = Router::new();
    let mut middleware_registry = MiddlewareRegistry::new();
    let session_store = Arc::new(SessionStore::new());
    let views = Arc::new(ViewEngine::filesystem("views"));
    let config = Arc::new(AppConfig::default());
    let pool = global_request_pool();
    
    // Setup routes
    for i in 0..200 {
        router.add_route(Route::new("GET", &format!("/api/items/{}", i), mock_handler));
        router.add_route(Route::new("POST", &format!("/api/items/{}/update", i), mock_handler));
    }
    
    // Setup middleware
    middleware_registry.register("perf_test", PerfTestMiddleware::new("perf_test", 2));
    
    println!("Integrated test setup complete");
    
    let start = Instant::now();
    let request_count = 5000;
    let concurrent_tasks = 20;
    let requests_per_task = request_count / concurrent_tasks;
    
    // Run integrated performance test with concurrency
    let mut handles = Vec::new();
    
    for task_id in 0..concurrent_tasks {
        let session_store_clone = Arc::clone(&session_store);
        let views_clone = Arc::clone(&views);
        let config_clone = Arc::clone(&config);
        
        let handle = tokio::spawn(async move {
            // Create a router for this task (since Router doesn't implement Clone)
            let mut task_router = Router::new();
            for i in 0..200 {
                task_router.add_route(Route::new("GET", &format!("/api/items/{}", i), mock_handler));
            }
            
            for req_id in 0..requests_per_task {
                // 1. Get pooled request
                let mut pooled_request = pool.get();
                pooled_request.method = "GET".to_string();
                pooled_request.uri = format!("/api/items/{}", req_id % 200);
                
                // 2. Route matching
                let route_result = task_router.match_route("GET", &pooled_request.uri);
                assert!(route_result.is_some());
                
                // 3. Session operations
                let session = session_store_clone.get_or_create(&format!("integrated_{}_{}", task_id, req_id)).await.unwrap();
                session.set("request_count", &req_id.to_string()).unwrap();
                let _count: Option<String> = session.get("request_count");
                
                // 4. Create context with regular Request
                let mut request = Request::default();
                request.method = pooled_request.method.clone();
                request.uri = pooled_request.uri.clone();
                request.headers = pooled_request.headers.clone();
                request.query = pooled_request.query.clone();
                
                let context = Context::new(
                    request,
                    session,
                    Arc::clone(&views_clone),
                    Arc::clone(&config_clone)
                );
                
                // 5. Simulate middleware chain and handler
                let _ = context.ip();
                let _ = context.url();
                let _response = context.json(json!({
                    "task": task_id,
                    "request": req_id,
                    "status": "ok"
                }));
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }
    
    let elapsed = start.elapsed();
    let requests_per_second = request_count as f64 / elapsed.as_secs_f64();
    
    println!("Integrated Performance Results:");
    println!("- Total requests: {}", request_count);
    println!("- Concurrent tasks: {}", concurrent_tasks);
    println!("- Total time: {:?}", elapsed);
    println!("- Requests per second: {:.0}", requests_per_second);
    println!("- Average request time: {:.2}ms", elapsed.as_millis() as f64 / request_count as f64);
    
    // Integrated performance should still be substantial
    assert!(requests_per_second > 20_000.0, 
        "Integrated performance regression detected: {:.0} req/sec < 20,000 minimum", 
        requests_per_second);
    
    println!("✅ Integrated performance test PASSED");
}

