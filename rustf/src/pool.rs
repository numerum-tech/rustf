//! Object pooling system for Request structures
//!
//! ⚠️ **NOT RECOMMENDED FOR USE IN PRODUCTION**
//!
//! Benchmarks demonstrate this implementation is approximately **2x SLOWER** than direct
//! allocation for Request objects due to:
//! - Mutex lock overhead (4 separate lock acquisitions per get/return cycle)
//! - Request::reset() overhead (clearing 6 collections)
//! - Statistics tracking overhead
//! - Modern Rust allocators (jemalloc/mimalloc) are already excellent for small objects
//!
//! **Benchmark Results:**
//! - Pooled allocation: ~90-105ns
//! - Direct allocation: ~40-57ns
//! - Pool is 2x SLOWER than `Request::default()`
//!
//! This module is kept for reference and experimentation, but the RustF framework
//! itself does not use it. For real-world applications, use `Request::default()` directly.
//!
//! See `rustf/benches/pool.rs` for detailed benchmark results.

use crate::http::Request;
use std::sync::{Arc, Mutex};

/// Thread-safe object pool for Request structures
///
/// Maintains a pool of pre-allocated Request objects that can be reused
/// across multiple HTTP requests. Uses LIFO (Last In, First Out) strategy
/// to improve cache locality.
#[derive(Clone)]
pub struct RequestPool {
    pool: Arc<Mutex<Vec<Request>>>,
    max_size: usize,
    created_count: Arc<Mutex<usize>>,
    borrowed_count: Arc<Mutex<usize>>,
    returned_count: Arc<Mutex<usize>>,
}

impl RequestPool {
    /// Create a new request pool with specified maximum size
    ///
    /// # Arguments
    /// * `max_size` - Maximum number of objects to keep in pool
    ///
    /// # Performance
    /// - Larger pools reduce allocation frequency but use more memory
    /// - Recommended: 2-4x your expected concurrent request count
    pub fn new(max_size: usize) -> Self {
        Self {
            pool: Arc::new(Mutex::new(Vec::with_capacity(max_size))),
            max_size,
            created_count: Arc::new(Mutex::new(0)),
            borrowed_count: Arc::new(Mutex::new(0)),
            returned_count: Arc::new(Mutex::new(0)),
        }
    }

    /// Get a Request from the pool or create a new one
    ///
    /// Returns a `PooledRequest` which automatically returns the
    /// object to the pool when dropped (RAII pattern).
    pub fn get(&self) -> PooledRequest {
        let request = {
            let mut pool = match self.pool.lock() {
                Ok(p) => p,
                Err(_) => {
                    // Lock poisoned, create new request
                    return PooledRequest {
                        request: Some(Request::default()),
                        pool: self.clone(),
                    };
                }
            };
            match pool.pop() {
                Some(mut req) => {
                    // Reset the request for reuse
                    req.reset();
                    req
                }
                None => {
                    // Pool empty, create new request
                    if let Ok(mut count) = self.created_count.lock() {
                        *count += 1;
                    }
                    Request::default()
                }
            }
        };

        if let Ok(mut count) = self.borrowed_count.lock() {
            *count += 1;
        }

        PooledRequest {
            request: Some(request),
            pool: self.clone(),
        }
    }

    /// Return a Request to the pool
    ///
    /// Called automatically by PooledRequest::drop(), but can be
    /// called manually for explicit control.
    fn return_request(&self, mut request: Request) {
        if let Ok(mut pool) = self.pool.lock() {
            if pool.len() < self.max_size {
                // Reset request state before returning to pool
                request.reset();
                pool.push(request);
                if let Ok(mut count) = self.returned_count.lock() {
                    *count += 1;
                }
            }
        }
        // If pool is full or lock failed, let the request be dropped
    }

    /// Get pool statistics for monitoring
    pub fn stats(&self) -> PoolStats {
        let pool_size = self.pool.lock().map(|p| p.len()).unwrap_or(0);
        let created = self.created_count.lock().map(|c| *c).unwrap_or(0);
        let borrowed = self.borrowed_count.lock().map(|b| *b).unwrap_or(0);
        let returned = self.returned_count.lock().map(|r| *r).unwrap_or(0);

        PoolStats {
            pool_size,
            max_size: self.max_size,
            created_count: created,
            borrowed_count: borrowed,
            returned_count: returned,
            hit_rate: if borrowed > 0 && borrowed >= created {
                (borrowed - created) as f64 / borrowed as f64 * 100.0
            } else {
                0.0
            },
        }
    }

    /// Clear all objects from the pool
    ///
    /// Useful for memory cleanup or during testing
    pub fn clear(&self) {
        if let Ok(mut pool) = self.pool.lock() {
            pool.clear();
        }
    }

    /// Pre-populate the pool with objects
    ///
    /// Creates objects in advance to avoid allocation during request handling.
    /// Recommended to call during application startup.
    pub fn warm_up(&self, count: usize) {
        let count = count.min(self.max_size);
        let mut pool = match self.pool.lock() {
            Ok(p) => p,
            Err(_) => return, // Can't warm up with poisoned lock
        };

        for _ in 0..count {
            if pool.len() < self.max_size {
                pool.push(Request::default());
                if let Ok(mut created) = self.created_count.lock() {
                    *created += 1;
                }
            }
        }

        log::info!("Warmed up RequestPool with {} objects", pool.len());
    }
}

/// RAII wrapper for pooled Request objects
///
/// Automatically returns the Request to the pool when dropped.
/// Implements Deref/DerefMut for transparent access to Request methods.
pub struct PooledRequest {
    request: Option<Request>,
    pool: RequestPool,
}

impl PooledRequest {
    /// Get mutable reference to the underlying Request
    pub fn get_mut(&mut self) -> Option<&mut Request> {
        self.request.as_mut()
    }

    /// Get immutable reference to the underlying Request
    pub fn get(&self) -> Option<&Request> {
        self.request.as_ref()
    }

    /// Take ownership of the Request, consuming the PooledRequest
    ///
    /// The Request will NOT be returned to the pool in this case.
    /// Returns None if the request has already been taken.
    pub fn into_inner(mut self) -> Option<Request> {
        self.request.take()
    }
}

impl std::ops::Deref for PooledRequest {
    type Target = Request;

    fn deref(&self) -> &Self::Target {
        self.request
            .as_ref()
            .expect("PooledRequest: request already taken")
    }
}

impl std::ops::DerefMut for PooledRequest {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.request
            .as_mut()
            .expect("PooledRequest: request already taken")
    }
}

impl Drop for PooledRequest {
    fn drop(&mut self) {
        if let Some(request) = self.request.take() {
            self.pool.return_request(request);
        }
    }
}

/// Pool statistics for monitoring and optimization
#[derive(Debug, Clone)]
pub struct PoolStats {
    /// Current number of objects in the pool
    pub pool_size: usize,
    /// Maximum pool size
    pub max_size: usize,
    /// Total objects created (including initial pool)
    pub created_count: usize,
    /// Total objects borrowed from pool
    pub borrowed_count: usize,
    /// Total objects returned to pool
    pub returned_count: usize,
    /// Cache hit rate (percentage of borrows that didn't require allocation)
    pub hit_rate: f64,
}

/// Global request pool instance
///
/// Provides a default pool that can be used across the application.
/// Pool size automatically determined based on available memory and CPU cores.
static REQUEST_POOL: std::sync::OnceLock<RequestPool> = std::sync::OnceLock::new();

/// Get the global request pool instance
///
/// Lazily initializes the pool with optimized defaults:
/// - Pool size: 4x CPU core count (good for CPU-bound workloads)
/// - Minimum 16, maximum 512 objects
pub fn global_request_pool() -> &'static RequestPool {
    REQUEST_POOL.get_or_init(|| {
        let core_count = num_cpus::get();
        let pool_size = (core_count * 4).max(16).min(512);

        log::info!(
            "Initializing global RequestPool with {} objects (detected {} CPU cores)",
            pool_size,
            core_count
        );

        let pool = RequestPool::new(pool_size);
        pool.warm_up(pool_size / 2); // Pre-populate 50% of pool
        pool
    })
}

/// Initialize global request pool with custom size
///
/// Must be called before first use of global_request_pool().
/// Useful for applications with specific memory/performance requirements.
pub fn init_global_pool(
    max_size: usize,
    warm_up_count: usize,
) -> std::result::Result<(), &'static str> {
    let pool = RequestPool::new(max_size);
    pool.warm_up(warm_up_count);

    REQUEST_POOL
        .set(pool)
        .map_err(|_| "Global pool already initialized")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_basic_functionality() {
        let pool = RequestPool::new(5);

        // Get request from empty pool (should create new)
        let mut req1 = pool.get();
        req1.method = "GET".to_string();
        req1.uri = "/test".to_string();

        let stats = pool.stats();
        assert_eq!(stats.created_count, 1);
        assert_eq!(stats.borrowed_count, 1);
        assert_eq!(stats.pool_size, 0);

        // Return to pool
        drop(req1);

        let stats = pool.stats();
        assert_eq!(stats.returned_count, 1);
        assert_eq!(stats.pool_size, 1);

        // Get again (should reuse)
        let req2 = pool.get();
        assert!(req2.method.is_empty()); // Should be reset
        assert!(req2.uri.is_empty());

        let stats = pool.stats();
        assert_eq!(stats.created_count, 1); // No new creation
        assert_eq!(stats.borrowed_count, 2);
        assert_eq!(stats.pool_size, 0);
        assert!(stats.hit_rate > 0.0);
    }

    #[test]
    fn test_pool_max_size() {
        let pool = RequestPool::new(2);

        // Fill pool beyond capacity
        let req1 = pool.get();
        let req2 = pool.get();
        let req3 = pool.get();

        drop(req1);
        drop(req2);
        drop(req3);

        let stats = pool.stats();
        assert_eq!(stats.pool_size, 2); // Should not exceed max_size
        assert_eq!(stats.returned_count, 2);
    }

    #[test]
    fn test_request_reset() {
        let pool = RequestPool::new(5);

        {
            let mut req = pool.get();
            req.method = "POST".to_string();
            req.uri = "/api/test".to_string();
            req.headers
                .insert("Content-Type".to_string(), "application/json".to_string());
            req.query.insert("param".to_string(), "value".to_string());
            // body_bytes is private, test via body_as_string after setting
        } // Return to pool

        // Get same request back
        let req = pool.get();
        assert!(req.method.is_empty());
        assert!(req.uri.is_empty());
        assert!(req.headers.is_empty());
        assert!(req.query.is_empty());
        assert!(req.body_as_string().is_empty());
    }

    #[test]
    fn test_pool_warm_up() {
        let pool = RequestPool::new(10);
        pool.warm_up(5);

        let stats = pool.stats();
        assert_eq!(stats.pool_size, 5);
        assert_eq!(stats.created_count, 5);
        assert_eq!(stats.borrowed_count, 0);
    }

    #[test]
    fn test_global_pool() {
        let pool1 = global_request_pool();
        let pool2 = global_request_pool();

        // Should be same instance
        assert!(std::ptr::eq(pool1, pool2));

        let _req = pool1.get();
        let stats = pool1.stats();
        assert!(stats.max_size >= 16); // Should have reasonable size
    }
}
