use crate::error::{Error, Result};
use crate::http::Request;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub mod config_adapter;
pub mod factory;
pub mod manager;
pub mod security;
pub mod storage;

#[cfg(feature = "redis")]
pub mod redis;

/// Security fingerprint for session validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionFingerprint {
    /// Full IP address
    pub ip: String,
    /// Full user agent string
    pub user_agent: String,
    /// Creation timestamp
    pub created_at: u64,
}

impl SessionFingerprint {
    /// Create fingerprint from request
    pub fn from_request(request: &Request) -> Self {
        let created_at = unix_timestamp();
        let ip = request.client_ip();
        let user_agent = request.user_agent().unwrap_or("unknown").to_string();

        Self {
            ip,
            user_agent,
            created_at,
        }
    }
}

/// Fingerprint validation mode
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FingerprintMode {
    /// No fingerprint validation
    Disabled,
    /// Soft validation (IP prefix + UA hash)
    Soft,
    /// Strict validation (exact match)
    Strict,
}

impl Default for FingerprintMode {
    fn default() -> Self {
        Self::Soft
    }
}

/// Cookie SameSite attribute
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SameSite {
    Strict,
    Lax,
    None,
}

impl Default for SameSite {
    fn default() -> Self {
        Self::Lax
    }
}

impl std::fmt::Display for SameSite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Strict => write!(f, "Strict"),
            Self::Lax => write!(f, "Lax"),
            Self::None => write!(f, "None"),
        }
    }
}

/// Session data structure for external storage backends
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    /// Session data as JSON object
    pub data: Value,
    /// Flash messages as JSON object
    pub flash: Value,
    /// Security fingerprint
    pub fingerprint: Option<SessionFingerprint>,
    /// Session creation timestamp (Unix seconds)
    pub created_at: u64,
    /// Last accessed timestamp (Unix seconds)
    pub last_accessed: u64,
    /// Absolute timeout timestamp (Unix seconds)
    pub absolute_timeout: u64,
    /// Current privilege level
    pub privilege_level: u32,
}

impl Default for SessionData {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionData {
    /// Create new session data
    pub fn new() -> Self {
        let now = unix_timestamp();

        Self {
            data: Value::Object(serde_json::Map::new()),
            flash: Value::Object(serde_json::Map::new()),
            fingerprint: None,
            created_at: now,
            last_accessed: now,
            absolute_timeout: now + (24 * 60 * 60), // Default 24 hours
            privilege_level: 0,
        }
    }

    /// Create new session data with security settings
    pub fn new_with_security(fingerprint: SessionFingerprint, absolute_timeout_secs: u64) -> Self {
        let now = unix_timestamp();

        Self {
            data: Value::Object(serde_json::Map::new()),
            flash: Value::Object(serde_json::Map::new()),
            fingerprint: Some(fingerprint),
            created_at: now,
            last_accessed: now,
            absolute_timeout: now + absolute_timeout_secs,
            privilege_level: 0,
        }
    }

    /// Update last accessed timestamp
    pub fn touch(&mut self) {
        self.last_accessed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// Check if session data is expired (dual timeout)
    pub fn is_expired(&self, idle_timeout_secs: u64) -> bool {
        let now = unix_timestamp();

        // Check absolute timeout
        if now > self.absolute_timeout {
            return true;
        }

        // Check idle timeout
        (now - self.last_accessed) > idle_timeout_secs
    }
}

/// Get current Unix timestamp in seconds
pub fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Generate cryptographically secure session ID
pub fn generate_secure_id(length: usize) -> String {
    use rand::distributions::Alphanumeric;
    use rand::{thread_rng, Rng};

    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

/// Abstract session storage backend trait
///
/// This trait allows RustF to support multiple session storage backends
/// including memory, Redis, database, and hybrid strategies.
#[async_trait]
pub trait SessionStorage: Send + Sync {
    /// Get session data by ID with optional fingerprint validation
    /// Returns None if session doesn't exist, is expired, or fails fingerprint validation
    /// The current_fingerprint is provided for validation if the storage implements it
    async fn get(
        &self,
        session_id: &str,
        current_fingerprint: Option<&SessionFingerprint>,
    ) -> Result<Option<SessionData>>;

    /// Store session data with TTL
    /// The TTL parameter indicates when the session should expire
    async fn set(&self, session_id: &str, data: &SessionData, ttl: Duration) -> Result<()>;

    /// Delete session data by ID
    async fn delete(&self, session_id: &str) -> Result<()>;

    /// Check if session exists and is not expired
    async fn exists(&self, session_id: &str) -> Result<bool>;

    /// Clean up expired sessions
    /// Returns the number of sessions cleaned up
    async fn cleanup_expired(&self) -> Result<usize>;

    /// Generate a new cryptographically secure session ID
    fn generate_id(&self) -> String {
        use rand::distributions::Alphanumeric;
        use rand::{thread_rng, Rng};

        thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect()
    }

    /// Get storage backend name for logging/debugging
    fn backend_name(&self) -> &'static str;

    /// Get storage backend statistics if supported
    async fn stats(&self) -> Result<StorageStats> {
        Ok(StorageStats::default())
    }
}

/// Storage backend statistics
#[derive(Debug, Clone, Default)]
pub struct StorageStats {
    /// Total number of sessions
    pub total_sessions: usize,
    /// Number of active (non-expired) sessions
    pub active_sessions: usize,
    /// Number of expired sessions
    pub expired_sessions: usize,
    /// Backend-specific metrics
    pub backend_metrics: HashMap<String, String>,
}

/// High-performance session store with pluggable storage backends
///
/// This store provides a unified interface for session management while allowing
/// different storage backends (memory, Redis, database) to be used transparently.
/// Handles session lifecycle, automatic expiration, and cleanup.
#[derive(Clone)]
pub struct SessionStore {
    storage: Arc<dyn SessionStorage>,
    session_timeout: Duration,
}

/// Thread-safe session with JSON-native storage and security features
///
/// Uses JSON (serde_json::Value) internally for zero-cost view rendering
/// and optimal storage backend compatibility. Includes security features like
/// fingerprinting, dual timeouts, and privilege tracking.
#[derive(Clone)]
pub struct Session {
    /// Opaque random session ID
    id: String,
    /// Session data as JSON (zero-copy for templates)
    data: Arc<RwLock<Value>>,
    /// Flash messages as JSON
    flash: Arc<RwLock<Value>>,
    /// Security fingerprint
    fingerprint: Option<SessionFingerprint>,
    /// Creation timestamp
    created_at: u64,
    /// Last accessed timestamp (for idle timeout)
    last_accessed: Arc<Mutex<u64>>,
    /// Absolute timeout timestamp
    absolute_timeout: u64,
    /// Current privilege level
    privilege_level: Arc<Mutex<u32>>,
    /// Flag indicating session needs rotation
    requires_rotation: Arc<Mutex<bool>>,
    /// Flag indicating session has been modified
    dirty: Arc<Mutex<bool>>,
}

impl Default for Session {
    fn default() -> Self {
        let now = unix_timestamp();
        Self {
            id: String::new(),
            data: Arc::new(RwLock::new(Value::Object(Map::new()))),
            flash: Arc::new(RwLock::new(Value::Object(Map::new()))),
            fingerprint: None,
            created_at: now,
            last_accessed: Arc::new(Mutex::new(now)),
            absolute_timeout: now + (24 * 60 * 60), // Default 24 hours
            privilege_level: Arc::new(Mutex::new(0)),
            requires_rotation: Arc::new(Mutex::new(false)),
            dirty: Arc::new(Mutex::new(false)),
        }
    }
}

impl Session {
    /// Create a new session with the given ID
    pub fn new(id: &str) -> Self {
        let now = unix_timestamp();
        Self {
            id: id.to_string(),
            data: Arc::new(RwLock::new(Value::Object(Map::new()))),
            flash: Arc::new(RwLock::new(Value::Object(Map::new()))),
            fingerprint: None,
            created_at: now,
            last_accessed: Arc::new(Mutex::new(now)),
            absolute_timeout: now + (24 * 60 * 60), // Default 24 hours
            privilege_level: Arc::new(Mutex::new(0)),
            requires_rotation: Arc::new(Mutex::new(false)),
            dirty: Arc::new(Mutex::new(false)),
        }
    }

    /// Create new secure session with fingerprint and timeouts
    pub fn new_secure(
        id: String,
        fingerprint: SessionFingerprint,
        _idle_timeout_secs: u64,
        absolute_timeout_secs: u64,
    ) -> Self {
        let now = unix_timestamp();

        Self {
            id,
            data: Arc::new(RwLock::new(Value::Object(Map::new()))),
            flash: Arc::new(RwLock::new(Value::Object(Map::new()))),
            fingerprint: Some(fingerprint),
            created_at: now,
            last_accessed: Arc::new(Mutex::new(now)),
            absolute_timeout: now + absolute_timeout_secs,
            privilege_level: Arc::new(Mutex::new(0)),
            requires_rotation: Arc::new(Mutex::new(false)),
            dirty: Arc::new(Mutex::new(false)),
        }
    }

    /// Create session from SessionData (for storage backend integration)
    pub fn from_data(id: &str, session_data: SessionData) -> Self {
        Self {
            id: id.to_string(),
            data: Arc::new(RwLock::new(session_data.data)),
            flash: Arc::new(RwLock::new(session_data.flash)),
            fingerprint: session_data.fingerprint,
            created_at: session_data.created_at,
            last_accessed: Arc::new(Mutex::new(session_data.last_accessed)),
            absolute_timeout: session_data.absolute_timeout,
            privilege_level: Arc::new(Mutex::new(session_data.privilege_level)),
            requires_rotation: Arc::new(Mutex::new(false)),
            dirty: Arc::new(Mutex::new(false)),
        }
    }

    /// Convert session to SessionData (for storage backend integration)
    pub fn to_data(&self) -> Result<SessionData> {
        let data = self
            .data
            .read()
            .map_err(|_| {
                Error::Session("Failed to acquire read lock for session data".to_string())
            })?
            .clone();
        let flash = self
            .flash
            .read()
            .map_err(|_| Error::Session("Failed to acquire read lock for flash data".to_string()))?
            .clone();
        let last_accessed = *self
            .last_accessed
            .lock()
            .map_err(|_| Error::Session("Failed to acquire lock for last_accessed".to_string()))?;
        let privilege_level = *self.privilege_level.lock().map_err(|_| {
            Error::Session("Failed to acquire lock for privilege_level".to_string())
        })?;

        Ok(SessionData {
            data,
            flash,
            fingerprint: self.fingerprint.clone(),
            created_at: self.created_at,
            last_accessed,
            absolute_timeout: self.absolute_timeout,
            privilege_level,
        })
    }

    /// Set a session value
    pub fn set<T: serde::Serialize>(&self, key: &str, value: T) -> Result<()> {
        let value = serde_json::to_value(value)?;
        let mut data = self.data.write().map_err(|_| {
            Error::Session("Failed to acquire write lock for session data".to_string())
        })?;
        if let Value::Object(ref mut map) = *data {
            map.insert(key.to_string(), value);
        }

        // Mark as dirty for save
        *self
            .dirty
            .lock()
            .map_err(|_| Error::Session("Failed to acquire lock for dirty flag".to_string()))? =
            true;

        Ok(())
    }

    /// Get a session value
    pub fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        let data = self.data.read().ok()?;
        if let Value::Object(ref map) = *data {
            map.get(key)
                .and_then(|v| serde_json::from_value(v.clone()).ok())
        } else {
            None
        }
    }

    /// Remove a session value
    pub fn remove(&self, key: &str) -> Option<Value> {
        let mut data = self.data.write().ok()?;
        let result = if let Value::Object(ref mut map) = *data {
            map.remove(key)
        } else {
            None
        };

        if result.is_some() {
            if let Ok(mut dirty) = self.dirty.lock() {
                *dirty = true;
            }
        }

        result
    }

    /// Set a flash message
    pub fn flash_set<T: serde::Serialize>(&self, key: &str, value: T) -> Result<()> {
        let value = serde_json::to_value(value)?;
        let mut flash = self.flash.write().map_err(|_| {
            Error::Session("Failed to acquire write lock for flash data".to_string())
        })?;
        if let Value::Object(ref mut map) = *flash {
            map.insert(key.to_string(), value);
        }
        Ok(())
    }

    /// Get and consume a flash message
    pub fn flash_get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        let mut flash = self.flash.write().ok()?;
        if let Value::Object(ref mut map) = *flash {
            map.remove(key).and_then(|v| serde_json::from_value(v).ok())
        } else {
            None
        }
    }

    /// Get and consume all flash messages
    pub fn flash_get_all(&self) -> HashMap<String, Value> {
        let mut flash = match self.flash.write() {
            Ok(f) => f,
            Err(_) => return HashMap::new(),
        };

        if let Value::Object(ref mut map) = *flash {
            // Clone the map and clear the original
            let all_flash: HashMap<String, Value> = map.clone().into_iter().collect();
            map.clear();
            all_flash
        } else {
            HashMap::new()
        }
    }

    /// Clear all flash messages without consuming them
    ///
    /// This is used for manual flash message management when you need to
    /// clear flash messages without rendering a view.
    pub fn flash_clear(&self) {
        if let Ok(mut flash) = self.flash.write() {
            if let Value::Object(ref mut map) = *flash {
                map.clear();
            }
        }
    }

    /// Remove a specific flash message by key
    ///
    /// Returns the removed value if it existed, None otherwise.
    /// This allows selective removal of flash messages.
    pub fn flash_remove(&self, key: &str) -> Option<Value> {
        let mut flash = self.flash.write().ok()?;
        if let Value::Object(ref mut map) = *flash {
            map.remove(key)
        } else {
            None
        }
    }

    /// Get the session ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Check if session has any data
    pub fn is_empty(&self) -> bool {
        let data = match self.data.read() {
            Ok(d) => d,
            Err(_) => return true, // Consider corrupted session as empty
        };
        let flash = match self.flash.read() {
            Ok(f) => f,
            Err(_) => return true, // Consider corrupted session as empty
        };

        let data_empty = if let Value::Object(ref map) = *data {
            map.is_empty()
        } else {
            true
        };

        let flash_empty = if let Value::Object(ref map) = *flash {
            map.is_empty()
        } else {
            true
        };

        data_empty && flash_empty
    }

    /// Get the number of data entries in the session
    pub fn data_count(&self) -> usize {
        let data = match self.data.read() {
            Ok(d) => d,
            Err(_) => return 0,
        };
        if let Value::Object(ref map) = *data {
            map.len()
        } else {
            0
        }
    }

    /// Get the number of flash messages in the session
    pub fn flash_count(&self) -> usize {
        let flash = match self.flash.read() {
            Ok(f) => f,
            Err(_) => return 0,
        };
        if let Value::Object(ref map) = *flash {
            map.len()
        } else {
            0
        }
    }

    /// Clear all session data and flash messages but keep session active
    ///
    /// This removes all data from the session but keeps the session ID intact,
    /// which is useful for scenarios like user logout where you want to clear
    /// data but maintain the session for tracking.
    pub fn clear(&self) {
        let data_had_items = if let Ok(mut data) = self.data.write() {
            if let Value::Object(ref mut map) = *data {
                let had_items = !map.is_empty();
                map.clear();
                had_items
            } else {
                false
            }
        } else {
            false
        };

        let flash_had_items = if let Ok(mut flash) = self.flash.write() {
            if let Value::Object(ref mut map) = *flash {
                let had_items = !map.is_empty();
                map.clear();
                had_items
            } else {
                false
            }
        } else {
            false
        };

        if data_had_items || flash_had_items {
            if let Ok(mut dirty) = self.dirty.lock() {
                *dirty = true;
            }
        }
    }

    /// Alias for clear() - removes all session data and flash messages
    ///
    /// This method provides Laravel-style compatibility for developers
    /// familiar with the flush() method name.
    pub fn flush(&self) {
        self.clear();
    }

    /// Regenerate session ID while preserving all data
    ///
    /// This creates a new session ID for security purposes (session fixation protection)
    /// while keeping all existing session data and flash messages intact.
    ///
    /// Note: This only changes the ID in memory. For full session regeneration with
    /// storage backend updates, use SessionStore::regenerate_session_id().
    pub fn regenerate_id(&mut self) {
        use rand::distributions::Alphanumeric;
        use rand::{thread_rng, Rng};

        self.id = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();
    }

    /// Mark session for destruction
    ///
    /// This clears all session data locally. For complete session destruction
    /// including removal from storage backend, use SessionStore::destroy_session().
    ///
    /// After calling this method, the session should be considered invalid.
    pub fn destroy(&self) {
        self.clear();
    }

    /// Convert session data to a serde_json::Value
    ///
    /// This is used to pass session data to view templates.
    /// Returns a JSON object containing all session data (excluding flash messages).
    /// This is now a zero-cost operation since session is already JSON internally.
    pub fn to_value(&self) -> Value {
        self.data
            .read()
            .map(|d| d.clone())
            .unwrap_or_else(|_| Value::Object(Map::new()))
    }

    /// Check if session is dirty (has unsaved changes)
    pub fn is_dirty(&self) -> bool {
        self.dirty.lock().map(|d| *d).unwrap_or(false)
    }

    /// Mark session as clean (saved)
    pub fn mark_clean(&self) {
        if let Ok(mut dirty) = self.dirty.lock() {
            *dirty = false;
        }
    }

    /// Check if session is expired
    pub fn is_expired(&self, idle_timeout_secs: u64) -> bool {
        let now = unix_timestamp();
        let last_accessed = self.last_accessed.lock().map(|l| *l).unwrap_or(0);

        // Check absolute timeout
        if now > self.absolute_timeout {
            return true;
        }

        // Check idle timeout
        if now - last_accessed > idle_timeout_secs {
            return true;
        }

        false
    }

    /// Update last accessed time (sliding window)
    pub fn touch(&self) {
        if let Ok(mut last_accessed) = self.last_accessed.lock() {
            *last_accessed = unix_timestamp();
        }
    }

    /// Get fingerprint for validation
    pub fn fingerprint(&self) -> Option<&SessionFingerprint> {
        self.fingerprint.as_ref()
    }

    /// Mark session for rotation (e.g., on privilege change)
    pub fn mark_for_rotation(&self) {
        if let Ok(mut requires_rotation) = self.requires_rotation.lock() {
            *requires_rotation = true;
        }
    }

    /// Check if session needs rotation
    pub fn needs_rotation(&self) -> bool {
        self.requires_rotation.lock().map(|r| *r).unwrap_or(false)
    }

    /// Set privilege level (marks for rotation if increasing)
    pub fn set_privilege_level(&self, level: u32) {
        if let Ok(mut current) = self.privilege_level.lock() {
            if level > *current {
                self.mark_for_rotation();
            }
            *current = level;
        }
    }

    /// Get current privilege level
    pub fn privilege_level(&self) -> u32 {
        self.privilege_level.lock().map(|p| *p).unwrap_or(0)
    }

    /// Set user ID (common helper)
    pub fn set_user_id(&self, user_id: i64) -> Result<()> {
        self.set("uid", user_id)
    }

    /// Get user ID (common helper)
    pub fn get_user_id(&self) -> Option<i64> {
        self.get("uid")
    }

    /// Check if user is authenticated
    pub fn is_authenticated(&self) -> bool {
        self.get_user_id().is_some()
    }
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionStore {
    /// Create a new session store with memory storage backend
    pub fn new() -> Self {
        use crate::session::storage::MemorySessionStorage;
        Self::with_storage(Arc::new(MemorySessionStorage::new()))
    }

    /// Create a session store with custom storage backend
    pub fn with_storage(storage: Arc<dyn SessionStorage>) -> Self {
        Self {
            storage,
            session_timeout: Duration::from_secs(30 * 60), // 30 minutes default
        }
    }

    /// Create a session store with custom timeout
    pub fn with_timeout(storage: Arc<dyn SessionStorage>, session_timeout: Duration) -> Self {
        Self {
            storage,
            session_timeout,
        }
    }

    /// Get or create a session with automatic expiration tracking
    ///
    /// This method fetches session data from the storage backend or creates
    /// a new session if one doesn't exist.
    pub async fn get_or_create(&self, session_id: &str) -> crate::error::Result<Session> {
        // SimpleSessionManager doesn't have request context, so no fingerprint validation
        match self.storage.get(session_id, None).await? {
            Some(session_data) => {
                // Convert SessionData back to Session
                Ok(Session::from_data(session_id, session_data))
            }
            None => {
                // Create new session
                let session = Session::new(session_id);
                let session_data = session.to_data()?;

                // Store the new session
                self.storage
                    .set(session_id, &session_data, self.session_timeout)
                    .await?;

                Ok(session)
            }
        }
    }

    /// Get a session without creating it
    pub async fn get(&self, session_id: &str) -> crate::error::Result<Option<Session>> {
        // SimpleSessionManager doesn't have request context, so no fingerprint validation
        if let Some(session_data) = self.storage.get(session_id, None).await? {
            Ok(Some(Session::from_data(session_id, session_data)))
        } else {
            Ok(None)
        }
    }

    /// Remove a session from the store
    pub async fn remove_session(&self, session_id: &str) -> crate::error::Result<()> {
        self.storage.delete(session_id).await
    }

    /// Perform cleanup of expired sessions
    ///
    /// Returns the number of sessions cleaned up. This method delegates
    /// to the storage backend's cleanup implementation.
    pub async fn cleanup_expired(&self) -> crate::error::Result<usize> {
        self.storage.cleanup_expired().await
    }

    /// Get session store statistics for monitoring
    pub async fn stats(&self) -> crate::error::Result<StorageStats> {
        self.storage.stats().await
    }

    /// Get the storage backend name
    pub fn backend_name(&self) -> &'static str {
        self.storage.backend_name()
    }

    /// Save a session to the storage backend
    pub async fn save_session(&self, session: &Session) -> crate::error::Result<()> {
        let session_data = session.to_data()?;
        self.storage
            .set(session.id(), &session_data, self.session_timeout)
            .await
    }

    /// Check if a session exists
    pub async fn exists(&self, session_id: &str) -> crate::error::Result<bool> {
        self.storage.exists(session_id).await
    }

    /// Completely destroy a session and remove it from storage
    ///
    /// This removes the session from the storage backend entirely. Unlike Session::destroy(),
    /// which only clears data locally, this method ensures the session is completely removed
    /// from persistent storage.
    ///
    /// Use this for complete session invalidation (e.g., logout, security breach).
    pub async fn destroy_session(&self, session_id: &str) -> crate::error::Result<()> {
        self.storage.delete(session_id).await
    }

    /// Regenerate session ID with storage backend updates
    ///
    /// This creates a new session ID, transfers all data to the new session,
    /// and removes the old session from storage. This provides complete session
    /// fixation protection by ensuring both memory and storage are updated.
    ///
    /// Returns the new session with the regenerated ID.
    pub async fn regenerate_session_id(
        &self,
        old_session_id: &str,
    ) -> crate::error::Result<Option<Session>> {
        // Get the existing session data
        if let Some(mut session) = self.get(old_session_id).await? {
            // Generate new session ID
            session.regenerate_id();
            let new_session_id = session.id().to_string();

            // Save session with new ID
            self.save_session(&session).await?;

            // Remove old session
            self.storage.delete(old_session_id).await?;

            log::debug!(
                "Session ID regenerated: {} -> {}",
                old_session_id,
                new_session_id
            );
            Ok(Some(session))
        } else {
            // Session doesn't exist
            Ok(None)
        }
    }

    /// Clear all data from a session but keep it active in storage
    ///
    /// This removes all session data and flash messages while keeping the session
    /// ID and storage entry intact. Useful for partial logout scenarios.
    pub async fn clear_session(&self, session_id: &str) -> crate::error::Result<()> {
        if let Some(session) = self.get(session_id).await? {
            session.clear();
            self.save_session(&session).await?;
        }
        Ok(())
    }
}
