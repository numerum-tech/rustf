use super::*;
use std::time::Instant;
use std::pin::Pin;
use std::future::Future;
use tokio::time::{sleep, Duration};

/// Create a test handler that simulates work
fn create_work_handler(work_duration_ms: u64, name: &str) -> impl Fn(EventContext) -> Pin<Box<dyn Future<Output = crate::Result<()>> + Send>> + Send + Sync + 'static {
    let name = name.to_string();
    move |_ctx| {
        let duration = Duration::from_millis(work_duration_ms);
        let handler_name = name.clone();
        Box::pin(async move {
            tokio::time::sleep(duration).await;
            log::debug!("Handler '{}' completed work simulation", handler_name);
            Ok(())
        })
    }
}

#[tokio::test]
async fn test_parallel_vs_sequential_performance() {
    // Setup
    let config = Arc::new(crate::config::AppConfig::default());
    let work_duration = 100; // 100ms per handler
    let handler_count = 4;
    
    // Test sequential execution
    let mut sequential_emitter = EventEmitter::with_config(
        EventEmitterConfig::sequential().with_debug_logging(false)
    );
    
    for i in 0..handler_count {
        sequential_emitter.on("test", create_work_handler(work_duration, &format!("seq_{}", i)));
    }
    
    let start = Instant::now();
    sequential_emitter.emit("test", None, config.clone()).await.unwrap();
    let sequential_duration = start.elapsed();
    
    // Test parallel execution  
    let mut parallel_emitter = EventEmitter::with_config(
        EventEmitterConfig::parallel().with_debug_logging(false)
    );
    
    for i in 0..handler_count {
        parallel_emitter.on("test", create_work_handler(work_duration, &format!("par_{}", i)));
    }
    
    let start = Instant::now();
    parallel_emitter.emit("test", None, config.clone()).await.unwrap();
    let parallel_duration = start.elapsed();
    
    println!("Sequential execution: {:?}", sequential_duration);
    println!("Parallel execution: {:?}", parallel_duration);
    
    // Parallel should be significantly faster
    // With 4 handlers taking 100ms each:
    // - Sequential: ~400ms
    // - Parallel: ~100ms (plus overhead)
    let expected_sequential = Duration::from_millis((handler_count * work_duration) as u64);
    let expected_parallel = Duration::from_millis(work_duration + 50); // 50ms overhead allowance
    
    assert!(sequential_duration >= expected_sequential - Duration::from_millis(50));
    assert!(parallel_duration <= expected_parallel);
    
    // Parallel should be at least 2x faster for this case
    assert!(parallel_duration.as_millis() * 2 <= sequential_duration.as_millis());
}

#[tokio::test] 
async fn test_priority_execution_order() {
    let config = Arc::new(crate::config::AppConfig::default());
    let execution_order = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    
    let mut emitter = EventEmitter::with_config(
        EventEmitterConfig::parallel().with_debug_logging(false)
    );
    
    // Add handlers in different priority groups
    let order_clone = execution_order.clone();
    emitter.on_priority("test", 100, move |_ctx| {
        let order = order_clone.clone();
        Box::pin(async move {
            order.lock().await.push("low_priority_1".to_string());
            Ok(())
        })
    });
    
    let order_clone = execution_order.clone();
    emitter.on_priority("test", -100, move |_ctx| {
        let order = order_clone.clone();
        Box::pin(async move {
            order.lock().await.push("high_priority_1".to_string());
            Ok(())
        })
    });
    
    emitter.emit("test", None, config).await.unwrap();
    
    let final_order = execution_order.lock().await.clone();
    println!("Execution order: {:?}", final_order);
    
    // High priority group should execute before low priority group
    let high_priority_indices: Vec<usize> = final_order
        .iter()
        .enumerate()
        .filter(|(_, item)| item.starts_with("high_priority"))
        .map(|(i, _)| i)
        .collect();
        
    let low_priority_indices: Vec<usize> = final_order
        .iter() 
        .enumerate()
        .filter(|(_, item)| item.starts_with("low_priority"))
        .map(|(i, _)| i)
        .collect();
    
    // All high priority handlers should execute before any low priority handler
    if let (Some(&max_high), Some(&min_low)) = (high_priority_indices.iter().max(), low_priority_indices.iter().min()) {
        assert!(max_high < min_low, 
               "High priority handlers should complete before low priority handlers start");
    }
    
    // Verify we have the expected handlers
    assert_eq!(high_priority_indices.len(), 1);
    assert_eq!(low_priority_indices.len(), 1);
}

#[tokio::test]
async fn test_timeout_protection() {
    let config = Arc::new(crate::config::AppConfig::default());
    let mut emitter = EventEmitter::with_config(
        EventEmitterConfig::parallel()
            .with_timeout(Duration::from_millis(50)) // Very short timeout
            .with_debug_logging(false)
    );
    
    // Add a handler that will timeout
    emitter.on("test", |_ctx| {
        Box::pin(async move {
            tokio::time::sleep(Duration::from_millis(200)).await; // Longer than timeout
            Ok(())
        })
    });
    
    let start = Instant::now();
    let result = emitter.emit("test", None, config).await;
    let duration = start.elapsed();
    
    // Should timeout and return error
    assert!(result.is_err());
    assert!(duration <= Duration::from_millis(100)); // Should timeout quickly
    
    let error_msg = format!("{}", result.unwrap_err());
    assert!(error_msg.contains("timed out"));
}

#[tokio::test]
async fn test_error_isolation_in_parallel() {
    let config = Arc::new(crate::config::AppConfig::default());
    let success_counter = Arc::new(tokio::sync::Mutex::new(0));
    
    let mut emitter = EventEmitter::with_config(
        EventEmitterConfig::parallel().with_debug_logging(false)
    );
    
    // Handler that fails
    emitter.on("test", |_ctx| {
        Box::pin(async move {
            Err(crate::error::Error::internal("Simulated failure".to_string()))
        })
    });
    
    // Handlers that succeed
    for i in 0..3 {
        let counter = success_counter.clone();
        emitter.on("test", move |_ctx| {
            let counter = counter.clone();
            Box::pin(async move {
                *counter.lock().await += 1;
                log::debug!("Successful handler {} executed", i);
                Ok(())
            })
        });
    }
    
    let result = emitter.emit("test", None, config).await;
    
    // Should return error from the failing handler
    assert!(result.is_err());
    
    // But successful handlers should still have executed
    let success_count = *success_counter.lock().await;
    assert_eq!(success_count, 3, "Successful handlers should execute despite one failure");
}