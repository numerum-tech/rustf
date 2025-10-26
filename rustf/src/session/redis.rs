use crate::error::Result;
use crate::session::{
    FingerprintMode, SessionData, SessionFingerprint, SessionStorage, StorageStats,
};
use async_trait::async_trait;
use deadpool_redis::{Config, Pool, Runtime};
use redis::AsyncCommands;
use serde_json;
#[cfg(feature = "redis")]
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Redis-based session storage implementation with connection pooling
///
/// This implementation provides persistent session storage using Redis as the backend.
/// Features connection pooling for high performance and automatic TTL management.
#[cfg(feature = "redis")]
#[derive(Clone)]
pub struct RedisSessionStorage {
    pool: Pool,
    prefix: String,
    fingerprint_mode: FingerprintMode,
}

#[cfg(feature = "redis")]
impl RedisSessionStorage {
    /// Create a new Redis session storage with the default configuration
    ///
    /// Uses Redis URL: redis://localhost:6379 with prefix "rustf:session:"
    pub async fn new() -> Result<Self> {
        Self::from_url(
            "redis://localhost:6379",
            "rustf:session:",
            10,
            FingerprintMode::Soft,
        )
        .await
    }

    /// Create Redis session storage with custom URL and settings
    pub async fn from_url(
        redis_url: &str,
        prefix: &str,
        _pool_size: usize,
        fingerprint_mode: FingerprintMode,
    ) -> Result<Self> {
        let cfg = Config::from_url(redis_url);
        let pool = cfg.create_pool(Some(Runtime::Tokio1))?;

        // Test the connection
        let mut conn = pool.get().await?;
        redis::cmd("PING").query_async::<String>(&mut conn).await?;

        Ok(Self {
            pool,
            prefix: prefix.to_string(),
            fingerprint_mode,
        })
    }

    /// Create Redis session storage from deadpool config
    pub fn from_config(
        config: Config,
        prefix: &str,
        fingerprint_mode: FingerprintMode,
    ) -> Result<Self> {
        let pool = config.create_pool(Some(Runtime::Tokio1))?;

        Ok(Self {
            pool,
            prefix: prefix.to_string(),
            fingerprint_mode,
        })
    }

    /// Get the Redis key for a session ID
    fn session_key(&self, session_id: &str) -> String {
        format!("{}{}", self.prefix, session_id)
    }

    /// Get current Unix timestamp in seconds
    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Validate fingerprint based on configured mode
    fn validate_fingerprint(
        &self,
        stored: &SessionFingerprint,
        current: &SessionFingerprint,
    ) -> bool {
        match self.fingerprint_mode {
            FingerprintMode::Disabled => true,
            FingerprintMode::Soft => {
                // Compare IP prefix (first 3 octets) and user agent hash
                let stored_ip_prefix = Self::extract_ip_prefix(&stored.ip);
                let current_ip_prefix = Self::extract_ip_prefix(&current.ip);
                let stored_ua_hash = Self::hash_user_agent(&stored.user_agent);
                let current_ua_hash = Self::hash_user_agent(&current.user_agent);

                stored_ip_prefix == current_ip_prefix && stored_ua_hash == current_ua_hash
            }
            FingerprintMode::Strict => {
                // Exact match on both IP and user agent
                stored.ip == current.ip && stored.user_agent == current.user_agent
            }
        }
    }

    /// Extract IP prefix (first 3 octets) for soft validation
    fn extract_ip_prefix(ip: &str) -> String {
        // Handle both IPv4 and IPv6
        if ip.contains(':') {
            // IPv6: take first 3 segments
            ip.split(':').take(3).collect::<Vec<_>>().join(":")
        } else {
            // IPv4: take first 3 octets
            ip.split('.').take(3).collect::<Vec<_>>().join(".")
        }
    }

    /// Hash user agent for soft validation
    fn hash_user_agent(user_agent: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        user_agent.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

#[cfg(feature = "redis")]
#[async_trait]
impl SessionStorage for RedisSessionStorage {
    async fn get(
        &self,
        session_id: &str,
        current_fingerprint: Option<&SessionFingerprint>,
    ) -> Result<Option<SessionData>> {
        let mut conn = self.pool.get().await?;
        let key = self.session_key(session_id);

        // Get the JSON data from Redis
        let json_data: Option<String> = conn.get(&key).await?;

        match json_data {
            Some(data) => {
                // Deserialize the session data
                let mut session_data: SessionData = serde_json::from_str(&data)?;

                // Validate fingerprint if provided
                if let Some(current_fp) = current_fingerprint {
                    if let Some(ref stored_fp) = session_data.fingerprint {
                        if !self.validate_fingerprint(stored_fp, current_fp) {
                            log::warn!(
                                "RedisStorage: Session {} failed fingerprint validation",
                                session_id
                            );
                            return Ok(None);
                        }
                    }
                }

                // Update last accessed time
                session_data.touch();

                // Update the session in Redis with new access time
                let updated_json = serde_json::to_string(&session_data)?;
                let _: () = conn.set(&key, &updated_json).await?;

                Ok(Some(session_data))
            }
            None => Ok(None),
        }
    }

    async fn set(&self, session_id: &str, data: &SessionData, ttl: Duration) -> Result<()> {
        let mut conn = self.pool.get().await?;
        let key = self.session_key(session_id);

        // Serialize session data to JSON
        let json_data = serde_json::to_string(data)?;

        // Store in Redis with TTL
        let ttl_seconds = ttl.as_secs();
        let _: () = conn.set_ex(&key, &json_data, ttl_seconds).await?;

        Ok(())
    }

    async fn delete(&self, session_id: &str) -> Result<()> {
        let mut conn = self.pool.get().await?;
        let key = self.session_key(session_id);

        let _: i32 = conn.del(&key).await?;
        Ok(())
    }

    async fn exists(&self, session_id: &str) -> Result<bool> {
        let mut conn = self.pool.get().await?;
        let key = self.session_key(session_id);

        let exists: bool = conn.exists(&key).await?;
        Ok(exists)
    }

    async fn cleanup_expired(&self) -> Result<usize> {
        // Redis automatically handles TTL expiration, so we don't need manual cleanup.
        // We could implement a scan for expired sessions if needed, but it's typically
        // not necessary with Redis as it handles expiration automatically.

        // Return 0 as Redis handles cleanup automatically
        Ok(0)
    }

    fn backend_name(&self) -> &'static str {
        "redis"
    }

    async fn stats(&self) -> Result<StorageStats> {
        let mut conn = self.pool.get().await?;

        // Use SCAN to count sessions with our prefix
        let pattern = format!("{}*", self.prefix);
        let mut total_sessions = 0;
        let mut cursor = 0u64;

        // Count sessions using SCAN (non-blocking iteration)
        loop {
            let (new_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&pattern)
                .arg("COUNT")
                .arg(100) // Scan in batches of 100
                .query_async(&mut conn)
                .await?;

            total_sessions += keys.len();
            cursor = new_cursor;

            if cursor == 0 {
                break;
            }
        }

        // Get Redis info for additional metrics
        let info: String = redis::cmd("INFO")
            .arg("memory")
            .query_async(&mut conn)
            .await?;

        let mut backend_metrics = HashMap::new();
        backend_metrics.insert("redis_pattern".to_string(), pattern);
        backend_metrics.insert("scan_method".to_string(), "non-blocking".to_string());

        // Parse memory info from Redis INFO command
        for line in info.lines() {
            if line.starts_with("used_memory_human:") {
                let memory = line.split(':').nth(1).unwrap_or("unknown").trim();
                backend_metrics.insert("redis_memory_used".to_string(), memory.to_string());
            }
            if line.starts_with("used_memory_peak_human:") {
                let memory = line.split(':').nth(1).unwrap_or("unknown").trim();
                backend_metrics.insert("redis_memory_peak".to_string(), memory.to_string());
            }
        }

        Ok(StorageStats {
            total_sessions,
            active_sessions: total_sessions, // Redis only stores active sessions
            expired_sessions: 0,             // Redis automatically removes expired sessions
            backend_metrics,
        })
    }
}

#[cfg(feature = "redis")]
#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;

    async fn create_test_storage() -> RedisSessionStorage {
        // Skip tests if Redis is not available
        match RedisSessionStorage::new().await {
            Ok(storage) => storage,
            Err(_) => {
                println!("Redis not available, skipping tests");
                panic!("Redis connection failed - tests require running Redis server");
            }
        }
    }

    #[test]
    async fn test_redis_storage_basic_operations() {
        let storage = create_test_storage().await;
        let session_id = "test_redis_session_123";

        // Test session doesn't exist initially
        assert!(storage.get(session_id).await.unwrap().is_none());
        assert!(!storage.exists(session_id).await.unwrap());

        // Create and store session data
        let mut session_data = SessionData::new();
        session_data
            .data
            .insert("user_id".to_string(), serde_json::Value::Number(123.into()));
        session_data.flash.insert(
            "message".to_string(),
            serde_json::Value::String("Hello Redis".to_string()),
        );

        storage
            .set(session_id, &session_data, Duration::from_secs(3600))
            .await
            .unwrap();

        // Test session exists and can be retrieved
        assert!(storage.exists(session_id).await.unwrap());
        let retrieved = storage.get(session_id).await.unwrap().unwrap();
        assert_eq!(
            retrieved.data.get("user_id").unwrap(),
            &serde_json::Value::Number(123.into())
        );
        assert_eq!(
            retrieved.flash.get("message").unwrap(),
            &serde_json::Value::String("Hello Redis".to_string())
        );

        // Test session deletion
        storage.delete(session_id).await.unwrap();
        assert!(storage.get(session_id).await.unwrap().is_none());
        assert!(!storage.exists(session_id).await.unwrap());
    }

    #[test]
    async fn test_redis_storage_ttl() {
        let storage = create_test_storage().await;
        let session_id = "expiring_redis_session";

        // Store session data with very short TTL
        let session_data = SessionData::new();
        storage
            .set(session_id, &session_data, Duration::from_secs(1))
            .await
            .unwrap();

        // Session should exist initially
        assert!(storage.exists(session_id).await.unwrap());

        // Wait for TTL to expire
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Session should be expired and return None
        assert!(storage.get(session_id).await.unwrap().is_none());
        assert!(!storage.exists(session_id).await.unwrap());
    }

    #[test]
    async fn test_redis_storage_stats() {
        let storage = create_test_storage().await;

        // Clean up any existing test sessions
        for i in 0..3 {
            let session_id = format!("redis_stats_session_{}", i);
            let _ = storage.delete(&session_id).await;
        }

        // Add some test sessions
        for i in 0..3 {
            let session_id = format!("redis_stats_session_{}", i);
            let mut session_data = SessionData::new();
            session_data
                .data
                .insert("counter".to_string(), serde_json::Value::Number(i.into()));
            storage
                .set(&session_id, &session_data, Duration::from_secs(3600))
                .await
                .unwrap();
        }

        let stats = storage.stats().await.unwrap();
        assert!(stats.total_sessions >= 3); // At least our 3 test sessions
        assert_eq!(stats.active_sessions, stats.total_sessions); // Redis only stores active sessions
        assert_eq!(stats.expired_sessions, 0); // Redis auto-expires
        assert!(stats.backend_metrics.contains_key("redis_pattern"));

        // Clean up test sessions
        for i in 0..3 {
            let session_id = format!("redis_stats_session_{}", i);
            let _ = storage.delete(&session_id).await;
        }
    }

    #[test]
    async fn test_redis_storage_concurrent_access() {
        let storage = create_test_storage().await;
        let session_id = "concurrent_redis_session";

        // Clean up any existing session
        let _ = storage.delete(session_id).await;

        // Create initial session
        let session_data = SessionData::new();
        storage
            .set(session_id, &session_data, Duration::from_secs(3600))
            .await
            .unwrap();

        // Spawn multiple tasks that access the same session
        let handles: Vec<_> = (0..10)
            .map(|i| {
                let storage_clone = storage.clone();
                let session_id = session_id.to_string();

                tokio::spawn(async move {
                    // Get session
                    let session_opt = storage_clone.get(&session_id).await.unwrap();
                    assert!(session_opt.is_some());

                    // Update session data
                    let mut session_data = session_opt.unwrap();
                    session_data.data.insert(
                        format!("task_{}", i),
                        serde_json::Value::String(format!("value_{}", i)),
                    );

                    // Save back to Redis
                    storage_clone
                        .set(&session_id, &session_data, Duration::from_secs(3600))
                        .await
                        .unwrap();
                })
            })
            .collect();

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }

        // Verify final session state
        let final_session = storage.get(session_id).await.unwrap().unwrap();

        // Should have data from all tasks (though exact count may vary due to race conditions)
        assert!(!final_session.data.is_empty());

        // Clean up
        storage.delete(session_id).await.unwrap();
    }
}
