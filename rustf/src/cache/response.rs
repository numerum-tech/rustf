use super::memory::MemoryCache;
/// HTTP Response caching with ETags, Last-Modified, and conditional requests
///
/// Features:
/// - ETag generation and validation
/// - Last-Modified header support
/// - Conditional GET requests (304 Not Modified)
/// - Cache-Control header handling
/// - Vary header support for content negotiation
/// - Response compression awareness
use super::{cache_key_with_hash, Cache, CacheConfig, CacheKey};
use crate::error::Result;
use crate::http::response::Response;
use log::debug;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// HTTP response cache entry with metadata
#[derive(Debug, Clone)]
pub struct ResponseCacheEntry {
    /// The cached response body
    pub body: String,
    /// HTTP headers to include in cached response
    pub headers: HashMap<String, String>,
    /// Status code of the cached response
    pub status_code: u16,
    /// ETag for the cached response
    pub etag: String,
    /// Last modified timestamp
    pub last_modified: u64,
    /// Content type of the response
    pub content_type: String,
    /// Whether the response was compressed
    pub compressed: bool,
    /// Cache creation timestamp
    pub created_at: u64,
    /// Cache expiration timestamp (if any)
    pub expires_at: Option<u64>,
}

impl ResponseCacheEntry {
    /// Create new response cache entry
    pub fn new(
        body: String,
        status_code: u16,
        content_type: String,
        headers: HashMap<String, String>,
        ttl: Option<Duration>,
    ) -> Self {
        let now = current_timestamp();
        let etag = generate_etag(&body);
        let expires_at = ttl.map(|d| now + d.as_secs());

        Self {
            body,
            headers,
            status_code,
            etag,
            last_modified: now,
            content_type,
            compressed: false,
            created_at: now,
            expires_at,
        }
    }

    /// Check if this cache entry is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            current_timestamp() > expires_at
        } else {
            false
        }
    }

    /// Convert to HTTP Response
    pub fn to_response(&self) -> Response {
        use hyper::StatusCode;

        let status = StatusCode::from_u16(self.status_code).unwrap_or(StatusCode::OK);
        let mut response = Response::new(status)
            .with_body(self.body.as_bytes().to_vec())
            .with_header("Content-Type", &self.content_type)
            .with_header("ETag", &self.etag)
            .with_header("Last-Modified", &format_http_date(self.last_modified));

        // Add custom headers
        for (key, value) in &self.headers {
            response = response.with_header(key, value);
        }

        // Add cache headers
        if let Some(expires_at) = self.expires_at {
            let max_age = expires_at.saturating_sub(current_timestamp());
            response =
                response.with_header("Cache-Control", &format!("public, max-age={}", max_age));
        }

        response
    }
}

/// HTTP Response Cache with ETag and conditional request support
pub struct ResponseCache {
    cache: MemoryCache<ResponseCacheEntry>,
    config: ResponseCacheConfig,
}

/// Configuration for response caching
#[derive(Debug, Clone)]
pub struct ResponseCacheConfig {
    /// Base cache configuration
    pub cache_config: CacheConfig,
    /// Enable ETag generation
    pub enable_etags: bool,
    /// Enable Last-Modified headers
    pub enable_last_modified: bool,
    /// Enable conditional requests (304 responses)
    pub enable_conditional_requests: bool,
    /// Headers to include in cache key for Vary support
    pub vary_headers: Vec<String>,
    /// Content types to cache (empty = cache all)
    pub cacheable_content_types: Vec<String>,
    /// Status codes to cache
    pub cacheable_status_codes: Vec<u16>,
}

impl Default for ResponseCacheConfig {
    fn default() -> Self {
        Self {
            cache_config: CacheConfig::default(),
            enable_etags: true,
            enable_last_modified: true,
            enable_conditional_requests: true,
            vary_headers: vec!["Accept-Encoding".to_string(), "Accept-Language".to_string()],
            cacheable_content_types: vec![
                "text/html".to_string(),
                "text/css".to_string(),
                "text/javascript".to_string(),
                "application/javascript".to_string(),
                "application/json".to_string(),
                "application/xml".to_string(),
                "text/xml".to_string(),
            ],
            cacheable_status_codes: vec![200, 203, 300, 301, 302, 304, 404, 410],
        }
    }
}

impl Default for ResponseCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ResponseCache {
    /// Create new response cache with default configuration
    pub fn new() -> Self {
        Self::with_config(ResponseCacheConfig::default())
    }

    /// Create new response cache with custom configuration
    pub fn with_config(config: ResponseCacheConfig) -> Self {
        let cache = MemoryCache::with_config(config.cache_config.clone());

        Self { cache, config }
    }

    /// Generate cache key for request
    pub fn generate_cache_key(
        &self,
        method: &str,
        path: &str,
        headers: &HashMap<String, String>,
    ) -> CacheKey {
        let mut components = vec![method, path];

        // Add vary headers to cache key
        for header_name in &self.config.vary_headers {
            if let Some(header_value) = headers.get(&header_name.to_lowercase()) {
                components.push(header_value);
            }
        }

        cache_key_with_hash(&components)
    }

    /// Check if response should be cached
    pub fn should_cache(&self, status_code: u16, content_type: &str) -> bool {
        // Check status code
        if !self.config.cacheable_status_codes.contains(&status_code) {
            return false;
        }

        // Check content type (if specified)
        if !self.config.cacheable_content_types.is_empty() {
            let should_cache = self
                .config
                .cacheable_content_types
                .iter()
                .any(|ct| content_type.starts_with(ct));
            if !should_cache {
                return false;
            }
        }

        true
    }

    /// Cache a response
    pub fn cache_response(
        &self,
        key: CacheKey,
        body: String,
        status_code: u16,
        content_type: String,
        headers: HashMap<String, String>,
        ttl: Option<Duration>,
    ) -> Result<()> {
        if !self.should_cache(status_code, &content_type) {
            debug!(
                "Response not cacheable: status={}, content-type={}",
                status_code, content_type
            );
            return Ok(());
        }

        let entry = ResponseCacheEntry::new(body, status_code, content_type, headers, ttl);
        self.cache.put(key.clone(), entry, ttl)?;

        debug!("Cached response: key={}, status={}", key, status_code);
        Ok(())
    }

    /// Get cached response
    pub fn get_response(&self, key: &CacheKey) -> Option<ResponseCacheEntry> {
        self.cache.get(key)
    }

    /// Handle conditional request (ETag/Last-Modified validation)
    pub fn handle_conditional_request(
        &self,
        key: &CacheKey,
        if_none_match: Option<&str>,    // ETag from If-None-Match header
        if_modified_since: Option<u64>, // Timestamp from If-Modified-Since header
    ) -> Option<ConditionalResponse> {
        if !self.config.enable_conditional_requests {
            return None;
        }

        let cached_entry = self.cache.get(key)?;

        // Check ETag (If-None-Match)
        if let Some(client_etag) = if_none_match {
            if self.config.enable_etags && client_etag == cached_entry.etag {
                debug!("ETag match - returning 304 Not Modified");
                return Some(ConditionalResponse::NotModified(cached_entry));
            }
        }

        // Check Last-Modified (If-Modified-Since)
        if let Some(client_timestamp) = if_modified_since {
            if self.config.enable_last_modified && client_timestamp >= cached_entry.last_modified {
                debug!(
                    "Not modified since {} - returning 304 Not Modified",
                    client_timestamp
                );
                return Some(ConditionalResponse::NotModified(cached_entry));
            }
        }

        Some(ConditionalResponse::Modified(cached_entry))
    }

    /// Clear expired entries
    pub fn cleanup_expired(&self) -> usize {
        self.cache.cleanup_expired()
    }

    /// Get cache statistics
    pub fn stats(&self) -> super::CacheStats {
        self.cache.stats()
    }

    /// Clear all cached responses
    pub fn clear(&self) {
        self.cache.clear()
    }
}

/// Result of conditional request processing
#[derive(Debug)]
pub enum ConditionalResponse {
    /// Resource not modified - return 304
    NotModified(ResponseCacheEntry),
    /// Resource modified - return cached content with 200
    Modified(ResponseCacheEntry),
}

impl ConditionalResponse {
    /// Convert to HTTP response
    pub fn to_response(self) -> Response {
        use hyper::StatusCode;

        match self {
            ConditionalResponse::NotModified(entry) => Response::new(StatusCode::NOT_MODIFIED)
                .with_header("ETag", &entry.etag)
                .with_header("Last-Modified", &format_http_date(entry.last_modified))
                .with_header("Cache-Control", "public, max-age=3600"),
            ConditionalResponse::Modified(entry) => entry.to_response(),
        }
    }
}

/// Generate ETag for content
fn generate_etag(content: &str) -> String {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("\"{}\"", hasher.finish())
}

/// Format timestamp as HTTP date
fn format_http_date(timestamp: u64) -> String {
    use std::time::{Duration, UNIX_EPOCH};

    let datetime = UNIX_EPOCH + Duration::from_secs(timestamp);

    // Format as RFC 2822 date (simplified)
    // In a real implementation, you'd use chrono or time crate
    format!("{:?}", datetime) // Placeholder - use proper HTTP date formatting
}

/// Get current timestamp
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_cache_basic() {
        let cache = ResponseCache::new();
        let key = "test_key".to_string();

        // Cache a response
        let headers = HashMap::new();
        cache
            .cache_response(
                key.clone(),
                "Hello World".to_string(),
                200,
                "text/html".to_string(),
                headers,
                Some(Duration::from_secs(300)),
            )
            .unwrap();

        // Retrieve cached response
        let cached = cache.get_response(&key).unwrap();
        assert_eq!(cached.body, "Hello World");
        assert_eq!(cached.status_code, 200);
        assert_eq!(cached.content_type, "text/html");
    }

    #[test]
    fn test_etag_generation() {
        let etag1 = generate_etag("Hello World");
        let etag2 = generate_etag("Hello World");
        let etag3 = generate_etag("Different Content");

        assert_eq!(etag1, etag2);
        assert_ne!(etag1, etag3);
        assert!(etag1.starts_with("\""));
        assert!(etag1.ends_with("\""));
    }

    #[test]
    fn test_should_cache() {
        let cache = ResponseCache::new();

        // Should cache HTML
        assert!(cache.should_cache(200, "text/html"));

        // Should cache JSON
        assert!(cache.should_cache(200, "application/json"));

        // Should not cache non-cacheable status
        assert!(!cache.should_cache(500, "text/html"));

        // Should not cache non-specified content type
        assert!(!cache.should_cache(200, "application/octet-stream"));
    }

    #[test]
    fn test_conditional_requests() {
        let cache = ResponseCache::new();
        let key = "test_key".to_string();

        // Cache a response
        let headers = HashMap::new();
        cache
            .cache_response(
                key.clone(),
                "Hello World".to_string(),
                200,
                "text/html".to_string(),
                headers,
                Some(Duration::from_secs(300)),
            )
            .unwrap();

        let cached = cache.get_response(&key).unwrap();
        let etag = cached.etag.clone();

        // Test ETag match (should return 304)
        let result = cache.handle_conditional_request(&key, Some(&etag), None);
        assert!(matches!(result, Some(ConditionalResponse::NotModified(_))));

        // Test ETag mismatch (should return content)
        let result = cache.handle_conditional_request(&key, Some("\"different\""), None);
        assert!(matches!(result, Some(ConditionalResponse::Modified(_))));
    }

    #[test]
    fn test_cache_key_generation() {
        let cache = ResponseCache::new();
        let headers = HashMap::new();

        let key1 = cache.generate_cache_key("GET", "/api/users", &headers);
        let key2 = cache.generate_cache_key("GET", "/api/users", &headers);
        let key3 = cache.generate_cache_key("POST", "/api/users", &headers);

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }
}
