/// In-memory cache implementation with LRU eviction and TTL support
///
/// Features:
/// - Thread-safe with RwLock
/// - LRU eviction when capacity is reached
/// - TTL support with automatic expiration
/// - Cache statistics and monitoring
/// - Background cleanup of expired entries
use super::{current_timestamp, Cache, CacheConfig, CacheEntry, CacheKey, CacheStats};
use crate::error::{Error, Result};
use log::{debug, info, warn};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

/// Thread-safe in-memory cache with LRU eviction
pub struct MemoryCache<T: Clone> {
    data: Arc<RwLock<HashMap<CacheKey, CacheEntry<T>>>>,
    config: CacheConfig,
    stats: Arc<RwLock<CacheStats>>,
}

impl<T: Clone> MemoryCache<T> {
    /// Create a new memory cache with default configuration
    pub fn new() -> Self {
        Self::with_config(CacheConfig::default())
    }

    /// Create a new memory cache with custom configuration
    pub fn with_config(config: CacheConfig) -> Self {
        let mut stats = CacheStats::default();
        stats.max_entries = config.max_entries as u64;

        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            config,
            stats: Arc::new(RwLock::new(stats)),
        }
    }

    /// Create a new memory cache with capacity limit
    pub fn with_capacity(max_entries: usize) -> Self {
        let mut config = CacheConfig::default();
        config.max_entries = max_entries;
        Self::with_config(config)
    }

    /// Update cache statistics after a hit
    fn record_hit(&self) {
        if self.config.enable_stats {
            if let Ok(mut stats) = self.stats.write() {
                stats.hits += 1;
            }
        }
    }

    /// Update cache statistics after a miss
    fn record_miss(&self) {
        if self.config.enable_stats {
            if let Ok(mut stats) = self.stats.write() {
                stats.misses += 1;
            }
        }
    }

    /// Update cache statistics after an eviction
    fn record_eviction(&self) {
        if self.config.enable_stats {
            if let Ok(mut stats) = self.stats.write() {
                stats.evictions += 1;
            }
        }
    }

    /// Update cache statistics after expired cleanup
    fn record_cleanup(&self, cleaned_count: usize) {
        if self.config.enable_stats {
            if let Ok(mut stats) = self.stats.write() {
                stats.expired_cleanups += cleaned_count as u64;
            }
        }
    }

    /// Update cache statistics entries count
    fn update_entries_count(&self, count: usize) {
        if self.config.enable_stats {
            if let Ok(mut stats) = self.stats.write() {
                stats.entries = count as u64;
            }
        }
    }

    /// Evict least recently used entry to make space
    fn evict_lru(&self, data: &mut HashMap<CacheKey, CacheEntry<T>>) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }

        // Find the entry with the oldest last_access time
        let lru_key = data
            .iter()
            .min_by_key(|(_, entry)| entry.last_access)
            .map(|(key, _)| key.clone())
            .ok_or_else(|| Error::internal("Failed to find LRU entry for eviction"))?;

        data.remove(&lru_key);
        self.record_eviction();
        debug!("Evicted LRU cache entry: {}", lru_key);

        Ok(())
    }

    /// Remove expired entries from cache
    fn cleanup_expired_internal(&self, data: &mut HashMap<CacheKey, CacheEntry<T>>) -> usize {
        let _now = current_timestamp();
        let mut expired_keys = Vec::new();

        // Collect expired keys
        for (key, entry) in data.iter() {
            if entry.is_expired() {
                expired_keys.push(key.clone());
            }
        }

        // Remove expired entries
        let count = expired_keys.len();
        for key in expired_keys {
            data.remove(&key);
            debug!("Cleaned up expired cache entry: {}", key);
        }

        if count > 0 {
            self.record_cleanup(count);
            debug!("Cleaned up {} expired cache entries", count);
        }

        count
    }

    /// Get cache entry without updating access statistics (internal use)
    fn get_entry_internal(&self, key: &CacheKey) -> Option<CacheEntry<T>> {
        if let Ok(data) = self.data.read() {
            if let Some(entry) = data.get(key) {
                if !entry.is_expired() {
                    return Some(entry.clone());
                }
                // Entry is expired, will be cleaned up later
                debug!("Cache entry '{}' is expired", key);
            }
        }
        None
    }
}

impl<T: Clone> Cache<T> for MemoryCache<T> {
    fn get(&self, key: &CacheKey) -> Option<T> {
        // First try to get the entry
        if let Some(mut entry) = self.get_entry_internal(key) {
            // Update access statistics
            entry.mark_accessed();

            // Update the entry in the cache with new access statistics
            if let Ok(mut data) = self.data.write() {
                if let Some(cache_entry) = data.get_mut(key) {
                    cache_entry.mark_accessed();
                }
            }

            self.record_hit();
            return Some(entry.value);
        }

        self.record_miss();
        None
    }

    fn put(&self, key: CacheKey, value: T, ttl: Option<Duration>) -> Result<()> {
        let ttl = ttl.or(self.config.default_ttl);
        let entry = CacheEntry::new(value, ttl);

        if let Ok(mut data) = self.data.write() {
            // Check if we need to make space
            while data.len() >= self.config.max_entries && !data.is_empty() {
                self.evict_lru(&mut data)?;
            }

            data.insert(key.clone(), entry);
            self.update_entries_count(data.len());

            debug!("Cached entry '{}' (TTL: {:?})", key, ttl);
        } else {
            return Err(Error::internal("Failed to acquire write lock for cache"));
        }

        Ok(())
    }

    fn remove(&self, key: &CacheKey) -> Option<T> {
        if let Ok(mut data) = self.data.write() {
            if let Some(entry) = data.remove(key) {
                self.update_entries_count(data.len());
                debug!("Removed cache entry '{}'", key);
                return Some(entry.value);
            }
        }
        None
    }

    fn clear(&self) {
        if let Ok(mut data) = self.data.write() {
            let count = data.len();
            data.clear();
            self.update_entries_count(0);
            info!("Cleared cache ({} entries)", count);
        }
    }

    fn stats(&self) -> CacheStats {
        let mut stats = if let Ok(stats) = self.stats.read() {
            stats.clone()
        } else {
            CacheStats::default()
        };

        // Update real-time statistics
        if let Ok(data) = self.data.read() {
            stats.entries = data.len() as u64;

            // Calculate average entry age
            if !data.is_empty() {
                let total_age: u64 = data.values().map(|e| e.age_seconds()).sum();
                stats.average_entry_age = total_age as f64 / data.len() as f64;

                // Calculate total access count
                stats.total_access_count = data.values().map(|e| e.access_count).sum();
            }
        }

        stats
    }

    fn contains_key(&self, key: &CacheKey) -> bool {
        if let Ok(data) = self.data.read() {
            if let Some(entry) = data.get(key) {
                return !entry.is_expired();
            }
        }
        false
    }

    fn len(&self) -> usize {
        if let Ok(data) = self.data.read() {
            data.len()
        } else {
            0
        }
    }

    fn cleanup_expired(&self) -> usize {
        if let Ok(mut data) = self.data.write() {
            let count = self.cleanup_expired_internal(&mut data);
            self.update_entries_count(data.len());
            count
        } else {
            warn!("Failed to acquire write lock for cleanup");
            0
        }
    }
}

impl<T: Clone> Default for MemoryCache<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> Clone for MemoryCache<T> {
    fn clone(&self) -> Self {
        Self {
            data: Arc::clone(&self.data),
            config: self.config.clone(),
            stats: Arc::clone(&self.stats),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_basic_cache_operations() {
        let cache: MemoryCache<String> = MemoryCache::new();

        // Test put and get
        cache
            .put("key1".to_string(), "value1".to_string(), None)
            .unwrap();
        assert_eq!(cache.get(&"key1".to_string()), Some("value1".to_string()));

        // Test contains_key
        assert!(cache.contains_key(&"key1".to_string()));
        assert!(!cache.contains_key(&"key2".to_string()));

        // Test remove
        assert_eq!(
            cache.remove(&"key1".to_string()),
            Some("value1".to_string())
        );
        assert_eq!(cache.get(&"key1".to_string()), None);
    }

    #[test]
    fn test_cache_capacity_and_eviction() {
        let cache: MemoryCache<String> = MemoryCache::with_capacity(2);

        // Fill cache to capacity
        cache
            .put("key1".to_string(), "value1".to_string(), None)
            .unwrap();
        // Small delay to ensure different millisecond timestamps
        thread::sleep(Duration::from_millis(10));
        cache
            .put("key2".to_string(), "value2".to_string(), None)
            .unwrap();
        assert_eq!(cache.len(), 2);

        // Adding third item should evict LRU (key1)
        cache
            .put("key3".to_string(), "value3".to_string(), None)
            .unwrap();
        assert_eq!(cache.len(), 2);

        // key1 should be evicted (LRU)
        assert_eq!(cache.get(&"key1".to_string()), None);
        assert_eq!(cache.get(&"key2".to_string()), Some("value2".to_string()));
        assert_eq!(cache.get(&"key3".to_string()), Some("value3".to_string()));
    }

    #[test]
    fn test_ttl_expiration() {
        let cache: MemoryCache<String> = MemoryCache::new();

        // Put with 1 second TTL (cache now uses millisecond precision)
        cache
            .put(
                "key1".to_string(),
                "value1".to_string(),
                Some(Duration::from_secs(1)),
            )
            .unwrap();
        assert_eq!(cache.get(&"key1".to_string()), Some("value1".to_string()));

        // Wait for expiration (slightly more than TTL to account for timing)
        thread::sleep(Duration::from_secs(2));
        assert_eq!(cache.get(&"key1".to_string()), None);
    }

    #[test]
    fn test_cache_statistics() {
        let cache: MemoryCache<String> = MemoryCache::new();

        // Generate some cache activity
        cache
            .put("key1".to_string(), "value1".to_string(), None)
            .unwrap();
        cache.get(&"key1".to_string()); // hit
        cache.get(&"key2".to_string()); // miss

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.entries, 1);
        assert_eq!(stats.hit_rate(), 0.5);
    }

    #[test]
    fn test_cleanup_expired() {
        let cache: MemoryCache<String> = MemoryCache::new();

        // Add entries with different TTLs (cache uses second precision)
        cache
            .put(
                "key1".to_string(),
                "value1".to_string(),
                Some(Duration::from_secs(1)),
            )
            .unwrap();
        cache
            .put("key2".to_string(), "value2".to_string(), None)
            .unwrap(); // No expiration

        // Wait for expiration
        thread::sleep(Duration::from_secs(2));

        let cleaned = cache.cleanup_expired();
        assert_eq!(cleaned, 1);
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get(&"key2".to_string()), Some("value2".to_string()));
    }

    #[test]
    fn test_thread_safety() {
        let cache: MemoryCache<String> = MemoryCache::new();
        let cache_clone = cache.clone();

        let handle = thread::spawn(move || {
            for i in 0..100 {
                cache_clone
                    .put(format!("key{}", i), format!("value{}", i), None)
                    .unwrap();
            }
        });

        for i in 0..100 {
            cache
                .put(
                    format!("thread1_key{}", i),
                    format!("thread1_value{}", i),
                    None,
                )
                .unwrap();
        }

        handle.join().unwrap();

        // Both threads should have successfully added entries
        assert!(cache.len() > 0);
        let stats = cache.stats();
        assert!(stats.entries > 0);
    }
}
