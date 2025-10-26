use super::memory::MemoryCache;
/// Database query result caching with invalidation support
///
/// Features:
/// - SQL query result caching with parameterized query support
/// - Table-based cache invalidation
/// - Query fingerprinting for cache keys
/// - Prepared statement parameter handling
/// - Cache warming and preloading
/// - Query execution time tracking
use super::{cache_key_with_hash, Cache, CacheConfig, CacheKey};
use crate::error::Result;
use log::{debug, info};
use serde_json::Value;
use std::collections::HashSet;
use std::time::Duration;

/// Cached query result with metadata
#[derive(Debug, Clone)]
pub struct QueryCacheEntry {
    /// The cached query result as JSON
    pub result: Value,
    /// SQL query that produced this result
    pub query: String,
    /// Parameters used in the query
    pub params: Vec<Value>,
    /// Tables referenced in the query (for invalidation)
    pub referenced_tables: HashSet<String>,
    /// Query execution time (for performance monitoring)
    pub execution_time_ms: u64,
    /// Cache creation timestamp
    pub created_at: u64,
    /// Number of rows in the result
    pub row_count: usize,
}

impl QueryCacheEntry {
    pub fn new(
        result: Value,
        query: String,
        params: Vec<Value>,
        referenced_tables: HashSet<String>,
        execution_time: Duration,
    ) -> Self {
        let row_count = Self::count_rows(&result);
        Self {
            result,
            query,
            params,
            referenced_tables,
            execution_time_ms: execution_time.as_millis() as u64,
            created_at: super::current_timestamp(),
            row_count,
        }
    }

    fn count_rows(result: &Value) -> usize {
        match result {
            Value::Array(arr) => arr.len(),
            Value::Object(_) => 1,
            _ => 0,
        }
    }
}

/// Query cache configuration
#[derive(Debug, Clone)]
pub struct QueryCacheConfig {
    /// Base cache configuration
    pub cache_config: CacheConfig,
    /// Enable automatic table invalidation
    pub enable_table_invalidation: bool,
    /// Cache only SELECT queries
    pub cache_select_only: bool,
    /// Minimum execution time to cache (skip fast queries)
    pub min_execution_time_ms: u64,
    /// Maximum result size to cache (prevent memory issues)
    pub max_result_size_bytes: usize,
    /// Enable query parameter normalization
    pub normalize_parameters: bool,
}

impl Default for QueryCacheConfig {
    fn default() -> Self {
        Self {
            cache_config: CacheConfig {
                max_entries: 500,                             // Smaller default for query cache
                default_ttl: Some(Duration::from_secs(1800)), // 30 minutes
                enable_stats: true,
                cleanup_interval: Duration::from_secs(300),
            },
            enable_table_invalidation: true,
            cache_select_only: true,
            min_execution_time_ms: 10, // Cache queries taking >10ms
            max_result_size_bytes: 1024 * 1024, // 1MB max result size
            normalize_parameters: true,
        }
    }
}

/// Database query cache with table-based invalidation
pub struct QueryCache {
    cache: MemoryCache<QueryCacheEntry>,
    config: QueryCacheConfig,
    /// Maps table names to cache keys for invalidation
    table_to_keys: MemoryCache<HashSet<CacheKey>>,
}

impl QueryCache {
    /// Create new query cache with default configuration
    pub fn new() -> Self {
        Self::with_config(QueryCacheConfig::default())
    }

    /// Create new query cache with custom configuration
    pub fn with_config(config: QueryCacheConfig) -> Self {
        let cache = MemoryCache::with_config(config.cache_config.clone());
        let table_to_keys = MemoryCache::with_capacity(100); // Reasonable number of tables

        Self {
            cache,
            config,
            table_to_keys,
        }
    }

    /// Generate cache key for a query with parameters
    pub fn generate_cache_key(&self, query: &str, params: &[Value]) -> CacheKey {
        let normalized_query = if self.config.normalize_parameters {
            self.normalize_query(query)
        } else {
            query.to_string()
        };

        let params_str = params
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(",");

        cache_key_with_hash(&[&normalized_query, &params_str])
    }

    /// Normalize query for consistent caching (remove extra whitespace, etc.)
    fn normalize_query(&self, query: &str) -> String {
        query
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase()
    }

    /// Extract table names from SQL query (simplified parser)
    fn extract_table_names(&self, query: &str) -> HashSet<String> {
        let mut tables = HashSet::new();
        let query_lower = query.to_lowercase();

        // Simple regex-based extraction (in production, use proper SQL parser)
        // Look for FROM, JOIN, UPDATE, INSERT INTO, DELETE FROM patterns
        let patterns = [
            r"from\s+(\w+)",
            r"join\s+(\w+)",
            r"update\s+(\w+)",
            r"insert\s+into\s+(\w+)",
            r"delete\s+from\s+(\w+)",
        ];

        for pattern in &patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                for captures in re.captures_iter(&query_lower) {
                    if let Some(table_name) = captures.get(1) {
                        tables.insert(table_name.as_str().to_string());
                    }
                }
            }
        }

        // Fallback: simple word extraction if regex fails
        if tables.is_empty() {
            for word in query_lower.split_whitespace() {
                if word.chars().all(|c| c.is_alphanumeric() || c == '_') && word.len() > 2 {
                    // This is a very naive approach - in production use proper SQL parsing
                    if !is_sql_keyword(word) {
                        tables.insert(word.to_string());
                    }
                }
            }
        }

        debug!("Extracted tables from query: {:?}", tables);
        tables
    }

    /// Check if query should be cached
    pub fn should_cache_query(&self, query: &str, execution_time: Duration) -> bool {
        // Check execution time threshold
        if execution_time.as_millis() < self.config.min_execution_time_ms as u128 {
            return false;
        }

        // Check if SELECT only
        if self.config.cache_select_only {
            let query_trimmed = query.trim().to_lowercase();
            if !query_trimmed.starts_with("select") {
                return false;
            }
        }

        true
    }

    /// Cache query result
    pub fn cache_query_result(
        &self,
        query: &str,
        params: Vec<Value>,
        result: Value,
        execution_time: Duration,
        ttl: Option<Duration>,
    ) -> Result<()> {
        if !self.should_cache_query(query, execution_time) {
            return Ok(());
        }

        // Check result size
        let result_size = serde_json::to_string(&result)?.len();
        if result_size > self.config.max_result_size_bytes {
            debug!("Query result too large to cache: {} bytes", result_size);
            return Ok(());
        }

        let cache_key = self.generate_cache_key(query, &params);
        let referenced_tables = self.extract_table_names(query);

        let entry = QueryCacheEntry::new(
            result,
            query.to_string(),
            params,
            referenced_tables.clone(),
            execution_time,
        );

        // Cache the query result
        self.cache.put(cache_key.clone(), entry, ttl)?;

        // Update table-to-keys mapping for invalidation
        if self.config.enable_table_invalidation {
            for table_name in referenced_tables {
                self.add_key_to_table_mapping(&table_name, &cache_key)?;
            }
        }

        debug!(
            "Cached query result: key={}, execution_time={}ms",
            cache_key,
            execution_time.as_millis()
        );

        Ok(())
    }

    /// Get cached query result
    pub fn get_cached_result(&self, query: &str, params: &[Value]) -> Option<QueryCacheEntry> {
        let cache_key = self.generate_cache_key(query, params);
        self.cache.get(&cache_key)
    }

    /// Add cache key to table mapping for invalidation
    fn add_key_to_table_mapping(&self, table_name: &str, cache_key: &CacheKey) -> Result<()> {
        let mut keys = self
            .table_to_keys
            .get(&table_name.to_string())
            .unwrap_or_else(HashSet::new);
        keys.insert(cache_key.clone());
        self.table_to_keys.put(table_name.to_string(), keys, None)?;
        Ok(())
    }

    /// Invalidate cache entries for specific tables
    pub fn invalidate_table(&self, table_name: &str) -> Result<usize> {
        if !self.config.enable_table_invalidation {
            return Ok(0);
        }

        let mut invalidated_count = 0;

        if let Some(cache_keys) = self.table_to_keys.get(&table_name.to_string()) {
            for cache_key in cache_keys {
                if self.cache.remove(&cache_key).is_some() {
                    invalidated_count += 1;
                    debug!("Invalidated cached query: {}", cache_key);
                }
            }

            // Clear the table mapping
            self.table_to_keys.remove(&table_name.to_string());

            info!(
                "Invalidated {} cached queries for table '{}'",
                invalidated_count, table_name
            );
        }

        Ok(invalidated_count)
    }

    /// Invalidate cache entries for multiple tables
    pub fn invalidate_tables(&self, table_names: &[&str]) -> Result<usize> {
        let mut total_invalidated = 0;

        for table_name in table_names {
            total_invalidated += self.invalidate_table(table_name)?;
        }

        Ok(total_invalidated)
    }

    /// Clear all cached queries
    pub fn clear(&self) {
        self.cache.clear();
        self.table_to_keys.clear();
    }

    /// Get cache statistics
    pub fn stats(&self) -> QueryCacheStats {
        let base_stats = self.cache.stats();

        QueryCacheStats {
            base_stats,
            cached_tables: self.table_to_keys.len(),
        }
    }

    /// Cleanup expired entries
    pub fn cleanup_expired(&self) -> usize {
        self.cache.cleanup_expired() + self.table_to_keys.cleanup_expired()
    }

    /// Warm cache with commonly used queries
    pub fn warm_cache(&self, queries: Vec<(String, Vec<Value>)>) -> Result<usize> {
        info!("Warming query cache with {} queries", queries.len());
        let mut warmed = 0;

        for (query, params) in queries {
            // In a real implementation, you'd execute these queries
            // and cache the results. For now, we'll just create placeholder entries.
            let cache_key = self.generate_cache_key(&query, &params);

            if !self.cache.contains_key(&cache_key) {
                // Execute query and cache result (implementation-specific)
                // This is a placeholder - actual implementation would execute the query
                debug!("Would warm cache for query: {}", query);
                warmed += 1;
            }
        }

        info!("Warmed {} queries in cache", warmed);
        Ok(warmed)
    }
}

/// Extended statistics for query cache
#[derive(Debug, Clone)]
pub struct QueryCacheStats {
    pub base_stats: super::CacheStats,
    pub cached_tables: usize,
}

/// Check if a word is a SQL keyword (simplified list)
fn is_sql_keyword(word: &str) -> bool {
    matches!(
        word.to_lowercase().as_str(),
        "select"
            | "from"
            | "where"
            | "join"
            | "inner"
            | "left"
            | "right"
            | "outer"
            | "on"
            | "and"
            | "or"
            | "not"
            | "in"
            | "like"
            | "between"
            | "is"
            | "null"
            | "order"
            | "by"
            | "group"
            | "having"
            | "limit"
            | "offset"
            | "distinct"
            | "count"
            | "sum"
            | "avg"
            | "min"
            | "max"
            | "as"
            | "asc"
            | "desc"
            | "insert"
            | "update"
            | "delete"
            | "into"
            | "values"
            | "set"
    )
}

impl Default for QueryCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_query_cache_basic() {
        let cache = QueryCache::new();
        let query = "SELECT * FROM users WHERE id = ?";
        let params = vec![json!(123)];
        let result = json!([{"id": 123, "name": "John"}]);

        // Cache query result
        cache
            .cache_query_result(
                query,
                params.clone(),
                result.clone(),
                Duration::from_millis(50),
                Some(Duration::from_secs(300)),
            )
            .unwrap();

        // Retrieve cached result
        let cached = cache.get_cached_result(query, &params).unwrap();
        assert_eq!(cached.result, result);
        assert_eq!(cached.execution_time_ms, 50);
    }

    #[test]
    fn test_cache_key_generation() {
        let cache = QueryCache::new();

        let key1 = cache.generate_cache_key("SELECT * FROM users", &[]);
        let key2 = cache.generate_cache_key("SELECT * FROM users", &[]);
        let key3 = cache.generate_cache_key("SELECT * FROM posts", &[]);

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_table_extraction() {
        let cache = QueryCache::new();

        let tables1 = cache.extract_table_names("SELECT * FROM users WHERE id = 1");
        assert!(tables1.contains("users"));

        let tables2 = cache.extract_table_names(
            "SELECT u.name, p.title FROM users u JOIN posts p ON u.id = p.user_id",
        );
        assert!(tables2.contains("users"));
        assert!(tables2.contains("posts"));
    }

    #[test]
    fn test_should_cache_query() {
        let cache = QueryCache::new();

        // Should cache SELECT with sufficient execution time
        assert!(cache.should_cache_query("SELECT * FROM users", Duration::from_millis(20)));

        // Should not cache fast queries
        assert!(!cache.should_cache_query("SELECT * FROM users", Duration::from_millis(5)));

        // Should not cache non-SELECT queries (with default config)
        assert!(!cache.should_cache_query(
            "INSERT INTO users VALUES (1, 'John')",
            Duration::from_millis(20)
        ));
    }

    #[test]
    fn test_table_invalidation() {
        let cache = QueryCache::new();
        let query = "SELECT * FROM users WHERE active = 1";
        let params = vec![];
        let result = json!([{"id": 1, "name": "John"}]);

        // Cache query result
        cache
            .cache_query_result(
                query,
                params.clone(),
                result,
                Duration::from_millis(50),
                Some(Duration::from_secs(300)),
            )
            .unwrap();

        // Verify it's cached
        assert!(cache.get_cached_result(query, &params).is_some());

        // Invalidate the table
        let invalidated = cache.invalidate_table("users").unwrap();
        assert_eq!(invalidated, 1);

        // Verify it's no longer cached
        assert!(cache.get_cached_result(query, &params).is_none());
    }
}
