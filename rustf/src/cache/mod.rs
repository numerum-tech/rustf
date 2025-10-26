/// RustF Caching System
///
/// Provides multi-layer caching capabilities for RustF applications:
/// - HTTP Response caching with ETags and expiration
/// - Database query result caching with invalidation
/// - General-purpose memory cache with TTL support
/// - Cache statistics and monitoring
pub mod memory;
pub mod query;
pub mod response;
pub mod stats;

use crate::error::Result;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Cache key type for consistency across all cache implementations
pub type CacheKey = String;

/// Cache value that can expire
#[derive(Debug, Clone)]
pub struct CacheEntry<T> {
    pub value: T,
    pub created_at: u64,
    pub expires_at: Option<u64>,
    pub access_count: u64,
    pub last_access: u64,
}

impl<T> CacheEntry<T> {
    pub fn new(value: T, ttl: Option<Duration>) -> Self {
        let now = current_timestamp();
        let expires_at = ttl.map(|d| now + d.as_millis() as u64);

        Self {
            value,
            created_at: now,
            expires_at,
            access_count: 1,
            last_access: now,
        }
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            current_timestamp() > expires_at
        } else {
            false
        }
    }

    pub fn mark_accessed(&mut self) {
        self.access_count += 1;
        self.last_access = current_timestamp();
    }

    pub fn age_seconds(&self) -> u64 {
        current_timestamp().saturating_sub(self.created_at)
    }
}

/// Cache configuration for all cache types
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of entries in the cache
    pub max_entries: usize,
    /// Default TTL for cache entries (None = no expiration)
    pub default_ttl: Option<Duration>,
    /// Enable cache statistics collection
    pub enable_stats: bool,
    /// Cleanup interval for expired entries
    pub cleanup_interval: Duration,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 1000,
            default_ttl: Some(Duration::from_secs(3600)), // 1 hour
            enable_stats: true,
            cleanup_interval: Duration::from_secs(300), // 5 minutes
        }
    }
}

/// Cache statistics for monitoring and debugging
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub entries: u64,
    pub max_entries: u64,
    pub evictions: u64,
    pub expired_cleanups: u64,
    pub total_access_count: u64,
    pub average_entry_age: f64,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    pub fn utilization(&self) -> f64 {
        if self.max_entries == 0 {
            0.0
        } else {
            self.entries as f64 / self.max_entries as f64
        }
    }
}

/// Generic cache trait for different cache implementations
pub trait Cache<T: Clone> {
    /// Get a value from the cache
    fn get(&self, key: &CacheKey) -> Option<T>;

    /// Put a value into the cache with optional TTL
    fn put(&self, key: CacheKey, value: T, ttl: Option<Duration>) -> Result<()>;

    /// Remove a value from the cache
    fn remove(&self, key: &CacheKey) -> Option<T>;

    /// Clear all entries from the cache
    fn clear(&self);

    /// Get cache statistics
    fn stats(&self) -> CacheStats;

    /// Check if a key exists in the cache
    fn contains_key(&self, key: &CacheKey) -> bool;

    /// Get the number of entries in the cache
    fn len(&self) -> usize;

    /// Check if the cache is empty
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Cleanup expired entries (if applicable)
    fn cleanup_expired(&self) -> usize;
}

/// Get current timestamp in milliseconds since Unix epoch
pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Generate cache key from components
pub fn cache_key(components: &[&str]) -> CacheKey {
    components.join(":")
}

/// Generate cache key with hash for long keys
pub fn cache_key_with_hash(components: &[&str]) -> CacheKey {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let key = components.join(":");
    if key.len() <= 250 {
        key
    } else {
        // Hash long keys to prevent memory issues
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        format!("hash:{:x}", hasher.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_entry_expiration() {
        let entry = CacheEntry::new("test".to_string(), Some(Duration::from_secs(1)));
        assert!(!entry.is_expired());

        // Test expired entry (artificially expired)
        let mut expired_entry = CacheEntry::new("test".to_string(), Some(Duration::from_secs(0)));
        expired_entry.expires_at = Some(current_timestamp() - 1);
        assert!(expired_entry.is_expired());
    }

    #[test]
    fn test_cache_key_generation() {
        assert_eq!(cache_key(&["user", "123", "profile"]), "user:123:profile");

        // Test hash key generation for very long keys
        let long_components: Vec<&str> = (0..100).map(|_| "very_long_component").collect();
        let key = cache_key_with_hash(&long_components);
        assert!(key.starts_with("hash:"));
        assert!(key.len() < 50);
    }

    #[test]
    fn test_cache_stats() {
        let mut stats = CacheStats::default();
        stats.hits = 80;
        stats.misses = 20;
        stats.entries = 100;
        stats.max_entries = 1000;

        assert_eq!(stats.hit_rate(), 0.8);
        assert_eq!(stats.utilization(), 0.1);
    }
}
