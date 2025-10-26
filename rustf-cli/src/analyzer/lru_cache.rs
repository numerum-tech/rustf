use std::collections::HashMap;
use std::hash::Hash;
use serde::Serialize;
use chrono::{DateTime, Utc};

/// LRU Cache implementation for analysis results with TTL support
#[derive(Debug)]
pub struct LruCache<K, V> {
    capacity: usize,
    cache: HashMap<K, CacheNode<V>>,
    access_order: Vec<K>,
    ttl_seconds: i64,
}

#[derive(Debug, Clone)]
struct CacheNode<V> {
    value: V,
    created_at: DateTime<Utc>,
    last_accessed: DateTime<Utc>,
    access_count: u64,
}

#[derive(Debug, Serialize)]
pub struct CacheStatistics {
    pub capacity: usize,
    pub current_size: usize,
    pub hit_count: u64,
    pub miss_count: u64,
    pub eviction_count: u64,
    pub hit_rate: f64,
    pub average_age_seconds: f64,
    pub oldest_entry_age_seconds: f64,
    pub most_accessed_keys: Vec<String>,
}

impl<K, V> LruCache<K, V>
where
    K: Hash + Eq + Clone + std::fmt::Debug,
    V: Clone,
{
    pub fn new(capacity: usize, ttl_seconds: i64) -> Self {
        Self {
            capacity,
            cache: HashMap::with_capacity(capacity),
            access_order: Vec::with_capacity(capacity),
            ttl_seconds,
        }
    }

    pub fn get(&mut self, key: &K) -> Option<V> {
        // Check if key exists and is not expired
        if let Some(node) = self.cache.get_mut(key) {
            let now = Utc::now();
            
            // Check TTL
            if (now - node.created_at).num_seconds() > self.ttl_seconds {
                self.cache.remove(key);
                self.access_order.retain(|k| k != key);
                return None;
            }

            // Update access information
            node.last_accessed = now;
            node.access_count += 1;

            // Move to front of access order
            self.access_order.retain(|k| k != key);
            self.access_order.push(key.clone());

            Some(node.value.clone())
        } else {
            None
        }
    }

    pub fn put(&mut self, key: K, value: V) {
        let now = Utc::now();
        
        // Remove existing entry if present
        if self.cache.contains_key(&key) {
            self.access_order.retain(|k| k != &key);
        } else if self.cache.len() >= self.capacity {
            // Evict least recently used item
            self.evict_lru();
        }

        // Insert new entry
        let node = CacheNode {
            value,
            created_at: now,
            last_accessed: now,
            access_count: 1,
        };

        self.cache.insert(key.clone(), node);
        self.access_order.push(key);
    }

    pub fn contains_key(&self, key: &K) -> bool {
        if let Some(node) = self.cache.get(key) {
            let now = Utc::now();
            (now - node.created_at).num_seconds() <= self.ttl_seconds
        } else {
            false
        }
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        if let Some(node) = self.cache.remove(key) {
            self.access_order.retain(|k| k != key);
            Some(node.value)
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.cache.clear();
        self.access_order.clear();
    }

    pub fn len(&self) -> usize {
        self.cache.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Remove expired entries
    pub fn cleanup_expired(&mut self) -> usize {
        let now = Utc::now();
        let mut expired_keys = Vec::new();

        for (key, node) in &self.cache {
            if (now - node.created_at).num_seconds() > self.ttl_seconds {
                expired_keys.push(key.clone());
            }
        }

        let count = expired_keys.len();
        for key in expired_keys {
            self.cache.remove(&key);
            self.access_order.retain(|k| k != &key);
        }

        count
    }

    fn evict_lru(&mut self) {
        if let Some(lru_key) = self.access_order.first().cloned() {
            self.cache.remove(&lru_key);
            self.access_order.remove(0);
        }
    }

    pub fn get_statistics(&self) -> CacheStatistics {
        let now = Utc::now();
        let mut total_age = 0i64;
        let mut oldest_age = 0i64;
        let mut access_counts: Vec<(String, u64)> = Vec::new();
        let mut hit_count = 0u64;

        for (key, node) in &self.cache {
            let age = (now - node.created_at).num_seconds();
            total_age += age;
            oldest_age = oldest_age.max(age);
            hit_count += node.access_count;
            
            access_counts.push((format!("{:?}", key), node.access_count));
        }

        // Sort by access count and take top 10
        access_counts.sort_by(|a, b| b.1.cmp(&a.1));
        let most_accessed_keys: Vec<String> = access_counts
            .into_iter()
            .take(10)
            .map(|(key, _)| key)
            .collect();

        let current_size = self.cache.len();
        let average_age = if current_size > 0 {
            total_age as f64 / current_size as f64
        } else {
            0.0
        };

        // Note: We don't have miss_count and eviction_count tracking here
        // In a real implementation, you'd want to track these separately
        
        CacheStatistics {
            capacity: self.capacity,
            current_size,
            hit_count,
            miss_count: 0, // Would need separate tracking
            eviction_count: 0, // Would need separate tracking
            hit_rate: 0.0, // Would need hit/miss tracking
            average_age_seconds: average_age,
            oldest_entry_age_seconds: oldest_age as f64,
            most_accessed_keys,
        }
    }
}

impl<K, V> Default for LruCache<K, V>
where
    K: Hash + Eq + Clone + std::fmt::Debug,
    V: Clone,
{
    fn default() -> Self {
        Self::new(1000, 3600) // 1000 entries, 1 hour TTL
    }
}

/// Thread-safe LRU cache wrapper
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
pub struct ThreadSafeLruCache<K, V> {
    inner: Arc<RwLock<LruCache<K, V>>>,
}

impl<K, V> ThreadSafeLruCache<K, V>
where
    K: Hash + Eq + Clone + std::fmt::Debug,
    V: Clone,
{
    pub fn new(capacity: usize, ttl_seconds: i64) -> Self {
        Self {
            inner: Arc::new(RwLock::new(LruCache::new(capacity, ttl_seconds))),
        }
    }

    pub fn get(&self, key: &K) -> Option<V> {
        self.inner.write().ok()?.get(key)
    }

    pub fn put(&self, key: K, value: V) {
        if let Ok(mut cache) = self.inner.write() {
            cache.put(key, value);
        }
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.inner.read().ok().map(|cache| cache.contains_key(key)).unwrap_or(false)
    }

    pub fn remove(&self, key: &K) -> Option<V> {
        self.inner.write().ok()?.remove(key)
    }

    pub fn clear(&self) {
        if let Ok(mut cache) = self.inner.write() {
            cache.clear();
        }
    }

    pub fn len(&self) -> usize {
        self.inner.read().ok().map(|cache| cache.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.inner.read().ok().map(|cache| cache.is_empty()).unwrap_or(true)
    }

    pub fn cleanup_expired(&self) -> usize {
        self.inner.write().ok().map(|mut cache| cache.cleanup_expired()).unwrap_or(0)
    }

    pub fn get_statistics(&self) -> CacheStatistics {
        self.inner.read().ok().map(|cache| cache.get_statistics()).unwrap_or_else(|| {
            CacheStatistics {
                capacity: 0,
                current_size: 0,
                hit_count: 0,
                miss_count: 0,
                eviction_count: 0,
                hit_rate: 0.0,
                average_age_seconds: 0.0,
                oldest_entry_age_seconds: 0.0,
                most_accessed_keys: Vec::new(),
            }
        })
    }
}

impl<K, V> Default for ThreadSafeLruCache<K, V>
where
    K: Hash + Eq + Clone + std::fmt::Debug,
    V: Clone,
{
    fn default() -> Self {
        Self::new(1000, 3600)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_basic_lru_operations() {
        let mut cache = LruCache::new(3, 60);
        
        cache.put("a", 1);
        cache.put("b", 2);
        cache.put("c", 3);
        
        assert_eq!(cache.get(&"a"), Some(1));
        assert_eq!(cache.get(&"b"), Some(2));
        assert_eq!(cache.get(&"c"), Some(3));
        assert_eq!(cache.len(), 3);
    }

    #[test]
    fn test_lru_eviction() {
        let mut cache = LruCache::new(2, 60);
        
        cache.put("a", 1);
        cache.put("b", 2);
        cache.put("c", 3); // Should evict "a"
        
        assert_eq!(cache.get(&"a"), None);
        assert_eq!(cache.get(&"b"), Some(2));
        assert_eq!(cache.get(&"c"), Some(3));
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_lru_access_order() {
        let mut cache = LruCache::new(2, 60);
        
        cache.put("a", 1);
        cache.put("b", 2);
        
        // Access "a" to make it recently used
        assert_eq!(cache.get(&"a"), Some(1));
        
        cache.put("c", 3); // Should evict "b" (least recently used)
        
        assert_eq!(cache.get(&"a"), Some(1));
        assert_eq!(cache.get(&"b"), None);
        assert_eq!(cache.get(&"c"), Some(3));
    }

    #[test]
    fn test_ttl_expiration() {
        let mut cache = LruCache::new(10, 1); // 1 second TTL
        
        cache.put("short-lived", "value");
        assert_eq!(cache.get(&"short-lived"), Some("value"));
        
        // Simulate time passing (in real test, would need to wait)
        // For this test, we'll modify the node's creation time
        if let Some(node) = cache.cache.get_mut(&"short-lived") {
            node.created_at = Utc::now() - chrono::Duration::seconds(2);
        }
        
        assert_eq!(cache.get(&"short-lived"), None);
    }

    #[test]
    fn test_thread_safe_cache() {
        let cache: ThreadSafeLruCache<String, String> = ThreadSafeLruCache::new(10, 60);
        
        // Test basic operations
        cache.put("key1".to_string(), "value1".to_string());
        assert_eq!(cache.get(&"key1".to_string()), Some("value1".to_string()));
        assert_eq!(cache.len(), 1);
        
        // Test concurrent access
        let cache_clone = cache.clone();
        let handle = thread::spawn(move || {
            for i in 0..5 {
                cache_clone.put(format!("key{}", i), format!("value{}", i));
            }
        });
        
        for i in 5..10 {
            cache.put(format!("key{}", i), format!("value{}", i));
        }
        
        handle.join().unwrap();
        
        // Should have around 10 entries (depending on timing)
        assert!(cache.len() >= 5);
    }

    #[test]
    fn test_cache_statistics() {
        let mut cache = LruCache::new(5, 60);
        
        cache.put("a", 1);
        cache.put("b", 2);
        cache.put("c", 3);
        
        // Access some entries multiple times
        cache.get(&"a");
        cache.get(&"a");
        cache.get(&"b");
        
        let stats = cache.get_statistics();
        assert_eq!(stats.current_size, 3);
        assert_eq!(stats.capacity, 5);
        assert!(stats.hit_count > 0);
        assert!(!stats.most_accessed_keys.is_empty());
    }
}