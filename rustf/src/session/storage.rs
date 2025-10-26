use crate::error::Result;
use crate::session::{
    FingerprintMode, SessionData, SessionFingerprint, SessionStorage, StorageStats,
};
use async_trait::async_trait;
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::interval;

/// In-memory session storage implementation using DashMap for high performance
///
/// This implementation provides lock-free concurrent access to session data
/// with automatic expiration and cleanup. Perfect for single-server deployments
/// or development environments.
#[derive(Clone)]
pub struct MemorySessionStorage {
    sessions: Arc<DashMap<String, SessionData>>,
    cleanup_interval: Duration,
    session_timeout: Duration,
    fingerprint_mode: FingerprintMode,
}

impl MemorySessionStorage {
    /// Create new memory session storage with default settings
    pub fn new() -> Self {
        Self::with_timeout(
            Duration::from_secs(30 * 60), // 30 minutes timeout
            Duration::from_secs(5 * 60),  // 5 minutes cleanup interval
        )
    }

    /// Create memory session storage with custom timeout settings
    pub fn with_timeout(session_timeout: Duration, cleanup_interval: Duration) -> Self {
        Self::with_config(session_timeout, cleanup_interval, FingerprintMode::Soft)
    }

    /// Create memory session storage with full configuration
    pub fn with_config(
        session_timeout: Duration,
        cleanup_interval: Duration,
        fingerprint_mode: FingerprintMode,
    ) -> Self {
        let storage = Self {
            sessions: Arc::new(DashMap::new()),
            cleanup_interval,
            session_timeout,
            fingerprint_mode,
        };

        // Start background cleanup task
        storage.start_cleanup_task();

        storage
    }

    /// Get current Unix timestamp in seconds
    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Get the current number of sessions (for testing)
    #[cfg(test)]
    pub(crate) fn session_count(&self) -> usize {
        self.sessions.len()
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

    /// Start background cleanup task to remove expired sessions
    fn start_cleanup_task(&self) {
        let sessions = Arc::clone(&self.sessions);
        let cleanup_interval = self.cleanup_interval;
        let session_timeout = self.session_timeout;

        tokio::spawn(async move {
            let mut interval = interval(cleanup_interval);

            loop {
                interval.tick().await;

                let now = Self::now();
                let timeout_secs = session_timeout.as_secs();
                let mut cleaned_up = 0;

                // Collect expired session IDs
                let expired_ids: Vec<String> = sessions
                    .iter()
                    .filter_map(|entry| {
                        let is_expired = (now - entry.value().last_accessed) > timeout_secs;
                        if is_expired {
                            Some(entry.key().clone())
                        } else {
                            None
                        }
                    })
                    .collect();

                // Remove expired sessions
                for session_id in expired_ids {
                    if sessions.remove(&session_id).is_some() {
                        cleaned_up += 1;
                    }
                }

                if cleaned_up > 0 {
                    log::info!("Memory storage cleaned up {} expired sessions", cleaned_up);
                }
            }
        });
    }
}

impl Default for MemorySessionStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SessionStorage for MemorySessionStorage {
    async fn get(
        &self,
        session_id: &str,
        current_fingerprint: Option<&SessionFingerprint>,
    ) -> Result<Option<SessionData>> {
        if let Some(mut session_data) = self.sessions.get_mut(session_id) {
            // Check if session is expired
            if session_data.is_expired(self.session_timeout.as_secs()) {
                // Remove expired session
                drop(session_data);
                self.sessions.remove(session_id);
                log::debug!("MemoryStorage: Session {} expired and removed", session_id);
                return Ok(None);
            }

            // Validate fingerprint if provided
            if let Some(current_fp) = current_fingerprint {
                if let Some(ref stored_fp) = session_data.fingerprint {
                    if !self.validate_fingerprint(stored_fp, current_fp) {
                        log::warn!(
                            "MemoryStorage: Session {} failed fingerprint validation",
                            session_id
                        );
                        return Ok(None);
                    }
                }
            }

            // Update last accessed time
            session_data.touch();
            let data_clone = session_data.clone();
            log::debug!(
                "MemoryStorage: Retrieved session {} with {} data entries",
                session_id,
                if let serde_json::Value::Object(ref map) = data_clone.data {
                    map.len()
                } else {
                    0
                }
            );
            Ok(Some(data_clone))
        } else {
            log::debug!("MemoryStorage: Session {} not found", session_id);
            Ok(None)
        }
    }

    async fn set(&self, session_id: &str, data: &SessionData, _ttl: Duration) -> Result<()> {
        log::debug!(
            "MemoryStorage: Storing session {} with {} data entries",
            session_id,
            if let serde_json::Value::Object(ref map) = data.data {
                map.len()
            } else {
                0
            }
        );
        self.sessions.insert(session_id.to_string(), data.clone());
        Ok(())
    }

    async fn delete(&self, session_id: &str) -> Result<()> {
        self.sessions.remove(session_id);
        Ok(())
    }

    async fn exists(&self, session_id: &str) -> Result<bool> {
        if let Some(session_data) = self.sessions.get(session_id) {
            Ok(!session_data.is_expired(self.session_timeout.as_secs()))
        } else {
            Ok(false)
        }
    }

    async fn cleanup_expired(&self) -> Result<usize> {
        let now = Self::now();
        let timeout_secs = self.session_timeout.as_secs();
        let mut cleaned_up = 0;

        // Collect expired session IDs
        let expired_ids: Vec<String> = self
            .sessions
            .iter()
            .filter_map(|entry| {
                let is_expired = (now - entry.value().last_accessed) > timeout_secs;
                if is_expired {
                    Some(entry.key().clone())
                } else {
                    None
                }
            })
            .collect();

        // Remove expired sessions
        for session_id in expired_ids {
            if self.sessions.remove(&session_id).is_some() {
                cleaned_up += 1;
            }
        }

        if cleaned_up > 0 {
            log::info!("Manual cleanup removed {} expired sessions", cleaned_up);
        }

        Ok(cleaned_up)
    }

    fn backend_name(&self) -> &'static str {
        "memory"
    }

    async fn stats(&self) -> Result<StorageStats> {
        let now = Self::now();
        let timeout_secs = self.session_timeout.as_secs();

        let mut active_sessions = 0;
        let mut expired_sessions = 0;
        let mut oldest_session = now;
        let mut total_data_entries = 0;
        let mut total_flash_entries = 0;

        for entry in self.sessions.iter() {
            let is_expired = (now - entry.value().last_accessed) > timeout_secs;
            if is_expired {
                expired_sessions += 1;
            } else {
                active_sessions += 1;
            }

            // Count entries in JSON objects
            if let serde_json::Value::Object(ref map) = entry.value().data {
                total_data_entries += map.len();
            }
            if let serde_json::Value::Object(ref map) = entry.value().flash {
                total_flash_entries += map.len();
            }

            if entry.value().created_at < oldest_session {
                oldest_session = entry.value().created_at;
            }
        }

        let mut backend_metrics = HashMap::new();
        backend_metrics.insert(
            "total_data_entries".to_string(),
            total_data_entries.to_string(),
        );
        backend_metrics.insert(
            "total_flash_entries".to_string(),
            total_flash_entries.to_string(),
        );
        backend_metrics.insert(
            "oldest_session_age_secs".to_string(),
            now.saturating_sub(oldest_session)
            .to_string(),
        );
        backend_metrics.insert("session_timeout_secs".to_string(), timeout_secs.to_string());
        backend_metrics.insert(
            "cleanup_interval_secs".to_string(),
            self.cleanup_interval.as_secs().to_string(),
        );

        Ok(StorageStats {
            total_sessions: self.sessions.len(),
            active_sessions,
            expired_sessions,
            backend_metrics,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_memory_storage_basic_operations() {
        let storage = MemorySessionStorage::new();
        let session_id = "test_session_123";

        // Test session doesn't exist initially
        assert!(storage.get(session_id, None).await.unwrap().is_none());
        assert!(!storage.exists(session_id).await.unwrap());

        // Create and store session data
        let mut session_data = SessionData::new();
        if let serde_json::Value::Object(ref mut map) = session_data.data {
            map.insert("user_id".to_string(), serde_json::Value::Number(123.into()));
        }
        if let serde_json::Value::Object(ref mut map) = session_data.flash {
            map.insert(
                "message".to_string(),
                serde_json::Value::String("Hello".to_string()),
            );
        }

        storage
            .set(session_id, &session_data, Duration::from_secs(3600))
            .await
            .unwrap();

        // Test session exists and can be retrieved
        assert!(storage.exists(session_id).await.unwrap());
        let retrieved = storage.get(session_id, None).await.unwrap().unwrap();
        if let serde_json::Value::Object(ref data_map) = retrieved.data {
            assert_eq!(
                data_map.get("user_id").unwrap(),
                &serde_json::Value::Number(123.into())
            );
        } else {
            panic!("Expected data to be an object");
        }
        if let serde_json::Value::Object(ref flash_map) = retrieved.flash {
            assert_eq!(
                flash_map.get("message").unwrap(),
                &serde_json::Value::String("Hello".to_string())
            );
        } else {
            panic!("Expected flash to be an object");
        }

        // Test session deletion
        storage.delete(session_id).await.unwrap();
        assert!(storage.get(session_id, None).await.unwrap().is_none());
        assert!(!storage.exists(session_id).await.unwrap());
    }

    #[tokio::test]
    async fn test_memory_storage_expiration() {
        let storage = MemorySessionStorage::with_timeout(
            Duration::from_secs(1), // 1 second timeout for testing
            Duration::from_millis(500),
        );
        let session_id = "expiring_session";

        // Store session data
        let session_data = SessionData::new();
        storage
            .set(session_id, &session_data, Duration::from_secs(1))
            .await
            .unwrap();

        // Session should exist initially
        assert!(storage.exists(session_id).await.unwrap());

        // Wait for session to expire (idle timeout)
        sleep(Duration::from_secs(2)).await;

        // Session should be expired and return None
        assert!(storage.get(session_id, None).await.unwrap().is_none());
        assert!(!storage.exists(session_id).await.unwrap());
    }

    #[tokio::test]
    async fn test_memory_storage_stats() {
        let storage = MemorySessionStorage::new();

        // Add some test sessions
        for i in 0..5 {
            let session_id = format!("session_{}", i);
            let mut session_data = SessionData::new();
            if let serde_json::Value::Object(ref mut map) = session_data.data {
                map.insert("counter".to_string(), serde_json::Value::Number(i.into()));
            }
            storage
                .set(&session_id, &session_data, Duration::from_secs(3600))
                .await
                .unwrap();
        }

        let stats = storage.stats().await.unwrap();
        assert_eq!(stats.total_sessions, 5);
        assert_eq!(stats.active_sessions, 5);
        assert_eq!(stats.expired_sessions, 0);
        assert!(stats.backend_metrics.contains_key("total_data_entries"));
    }

    #[tokio::test]
    async fn test_memory_storage_cleanup() {
        let storage = MemorySessionStorage::with_timeout(
            Duration::from_secs(1),    // 1 second timeout
            Duration::from_secs(3600), // Long cleanup interval to test manual cleanup
        );

        // Add test sessions
        for i in 0..3 {
            let session_id = format!("session_{}", i);
            let session_data = SessionData::new();
            storage
                .set(&session_id, &session_data, Duration::from_secs(1))
                .await
                .unwrap();
        }

        assert_eq!(storage.session_count(), 3);

        // Wait for sessions to expire
        sleep(Duration::from_secs(2)).await;

        // Manual cleanup
        let cleaned = storage.cleanup_expired().await.unwrap();
        assert_eq!(cleaned, 3);
        assert_eq!(storage.session_count(), 0);
    }
}
