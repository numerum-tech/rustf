use crate::error::Result;
use crate::http::Request;
use crate::session::{
    generate_secure_id, unix_timestamp, FingerprintMode, SameSite, Session, SessionData,
    SessionFingerprint, SessionStorage,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::interval;

/// Session save strategy
#[derive(Debug, Clone)]
pub enum SaveStrategy {
    /// Save immediately on every change
    Immediate,
    /// Batch saves with debounce delay
    Batched(Duration),
    /// Save only at end of request (default)
    EndOfRequest,
}

impl Default for SaveStrategy {
    fn default() -> Self {
        Self::EndOfRequest
    }
}

/// Session manager with security features and event-driven saves
pub struct SessionManager {
    /// Storage backend
    storage: Arc<dyn SessionStorage>,
    /// Session configuration
    pub config: SessionConfig,
    /// Save strategy
    save_strategy: SaveStrategy,
    /// Pending saves for batching
    pending_saves: Arc<Mutex<HashMap<String, PendingSave>>>,
}

/// Pending save task
struct PendingSave {
    session_data: SessionData,
    _scheduled_at: u64,
}

/// Secure session configuration
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Cookie name
    pub cookie_name: String,
    /// Cookie secure flag (HTTPS only)
    pub secure: bool,
    /// Cookie HttpOnly flag
    pub http_only: bool,
    /// Cookie SameSite attribute
    pub same_site: SameSite,
    /// Cookie domain
    pub domain: Option<String>,
    /// Cookie path
    pub path: String,

    /// Idle timeout (sliding window)
    pub idle_timeout: Duration,
    /// Absolute timeout (maximum lifetime)
    pub absolute_timeout: Duration,

    /// Rotate session on privilege changes
    pub rotation_on_privilege: bool,
    /// Fingerprint validation mode
    pub fingerprint_mode: FingerprintMode,
    /// Secure ID length
    pub secure_id_length: usize,

    /// Save strategy
    pub save_strategy: SaveStrategy,

    /// Routes to exempt from session handling (supports glob patterns)
    pub exempt_routes: Vec<String>,
    /// Whether sessions are enabled globally
    pub enabled: bool,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            cookie_name: "rustf_sid".to_string(),
            secure: true,
            http_only: true,
            same_site: SameSite::Lax,
            domain: None,
            path: "/".to_string(),

            idle_timeout: Duration::from_secs(15 * 60), // 15 minutes
            absolute_timeout: Duration::from_secs(8 * 60 * 60), // 8 hours

            rotation_on_privilege: true,
            fingerprint_mode: FingerprintMode::Soft,
            secure_id_length: 32,

            save_strategy: SaveStrategy::EndOfRequest,
            exempt_routes: vec![],
            enabled: true,
        }
    }
}

impl SessionConfig {
    /// Create new session configuration with defaults
    pub fn new() -> Self {
        Self::default()
    }
}

impl SessionManager {
    /// Create new session manager
    pub fn new(storage: Arc<dyn SessionStorage>, config: SessionConfig) -> Arc<Self> {
        let manager = Arc::new(Self {
            storage,
            save_strategy: config.save_strategy.clone(),
            config,
            pending_saves: Arc::new(Mutex::new(HashMap::new())),
        });

        // Start background save task if using batched strategy
        if let SaveStrategy::Batched(delay) = &manager.save_strategy {
            let manager_clone = Arc::clone(&manager);
            let delay = *delay;
            tokio::spawn(async move {
                manager_clone.background_save_task(delay).await;
            });
        }

        manager
    }

    /// Create session with storage based on configuration
    ///
    /// This will use custom storage from definitions if available,
    /// otherwise falls back to configuration-based storage (Memory/Redis).
    pub async fn with_configured_storage(
        config: SessionConfig,
        storage_config: &crate::config::SessionStorageConfig,
    ) -> Result<Arc<Self>> {
        use crate::session::factory::SessionStorageFactory;

        let storage =
            SessionStorageFactory::create_storage(storage_config, config.fingerprint_mode).await?;
        Ok(Self::new(storage, config))
    }

    /// Create session with default memory storage (for backwards compatibility)
    pub fn with_default_storage(config: SessionConfig) -> Arc<Self> {
        use crate::session::storage::MemorySessionStorage;
        let storage = Arc::new(MemorySessionStorage::new());
        Self::new(storage, config)
    }

    /// Background task for batched saves
    async fn background_save_task(&self, delay: Duration) {
        let mut interval = interval(delay);

        loop {
            interval.tick().await;

            let pending = {
                let mut pending = self.pending_saves.lock().await;
                std::mem::take(&mut *pending)
            };

            for (id, save) in pending {
                if let Err(e) = self
                    .storage
                    .set(&id, &save.session_data, self.config.idle_timeout)
                    .await
                {
                    log::error!("Failed to save session {}: {}", id, e);
                }
            }
        }
    }

    /// Create new session
    pub async fn create_session(&self, request: &Request) -> Result<Session> {
        let id = generate_secure_id(self.config.secure_id_length);
        let fingerprint = SessionFingerprint::from_request(request);

        let session = Session::new_secure(
            id.clone(),
            fingerprint,
            self.config.idle_timeout.as_secs(),
            self.config.absolute_timeout.as_secs(),
        );

        // Save immediately to storage
        let storage_data = session.to_data()?;
        self.storage
            .set(&id, &storage_data, self.config.idle_timeout)
            .await?;

        Ok(session)
    }

    /// Load and validate session
    pub async fn load_session(&self, id: &str, request: &Request) -> Result<Option<Session>> {
        // Create current fingerprint from request
        let current_fingerprint = SessionFingerprint::from_request(request);

        // Load from storage with fingerprint validation
        let session_data = match self.storage.get(id, Some(&current_fingerprint)).await? {
            Some(data) => data,
            None => return Ok(None),
        };

        // Check absolute timeout
        if unix_timestamp() > session_data.absolute_timeout {
            self.storage.delete(id).await?;
            log::info!("Session {} expired (absolute timeout)", id);
            return Ok(None);
        }

        // Check idle timeout
        if unix_timestamp() - session_data.last_accessed > self.config.idle_timeout.as_secs() {
            self.storage.delete(id).await?;
            log::info!("Session {} expired (idle timeout)", id);
            return Ok(None);
        }

        // Create session from storage
        let session = Session::from_data(id, session_data);

        // Update last accessed time
        session.touch();

        Ok(Some(session))
    }

    /// Save session based on strategy
    pub async fn save_session(&self, session: &Session) -> Result<()> {
        if !session.is_dirty() {
            return Ok(());
        }

        let storage_data = session.to_data()?;

        match self.save_strategy {
            SaveStrategy::Immediate => {
                // Save immediately
                self.storage
                    .set(session.id(), &storage_data, self.config.idle_timeout)
                    .await?;
                session.mark_clean();
            }
            SaveStrategy::Batched(_) => {
                // Add to pending saves
                let mut pending = self.pending_saves.lock().await;
                pending.insert(
                    session.id().to_string(),
                    PendingSave {
                        session_data: storage_data,
                        _scheduled_at: unix_timestamp(),
                    },
                );
                session.mark_clean();
            }
            SaveStrategy::EndOfRequest => {
                // Will be saved by middleware at end of request
                // Keep dirty flag
            }
        }

        Ok(())
    }

    /// Force save session (used by middleware at end of request)
    pub async fn force_save(&self, session: &Session) -> Result<()> {
        // Always save the session at end of request to ensure data persistence
        let storage_data = session.to_data()?;
        log::debug!(
            "SessionManager: Force saving session {} (dirty: {})",
            session.id(),
            session.is_dirty()
        );
        self.storage
            .set(session.id(), &storage_data, self.config.idle_timeout)
            .await?;
        session.mark_clean();
        Ok(())
    }

    /// Rotate session ID (for security)
    pub async fn rotate_session(&self, session: &Session, request: &Request) -> Result<Session> {
        let old_id = session.id();
        let new_id = generate_secure_id(self.config.secure_id_length);

        // Create new session with same data but new ID
        let mut storage_data = session.to_data()?;
        storage_data.fingerprint = Some(SessionFingerprint::from_request(request)); // Update fingerprint

        // Save new session
        self.storage
            .set(&new_id, &storage_data, self.config.idle_timeout)
            .await?;

        // Delete old session
        self.storage.delete(old_id).await?;

        log::info!("Session rotated: {} -> {}", old_id, new_id);

        // Return new session
        Ok(Session::from_data(&new_id, storage_data))
    }

    /// Destroy session (logout)
    pub async fn destroy_session(&self, id: &str) -> Result<()> {
        self.storage.delete(id).await?;

        // Remove from pending saves if present
        let mut pending = self.pending_saves.lock().await;
        pending.remove(id);

        log::info!("Session {} destroyed", id);
        Ok(())
    }

    /// Get session cookie value
    pub fn create_cookie(&self, session_id: &str) -> String {
        let mut cookie = format!("{}={}", self.config.cookie_name, session_id);

        cookie.push_str(&format!("; Path={}", self.config.path));

        if let Some(ref domain) = self.config.domain {
            cookie.push_str(&format!("; Domain={}", domain));
        }

        if self.config.secure {
            cookie.push_str("; Secure");
        }

        if self.config.http_only {
            cookie.push_str("; HttpOnly");
        }

        cookie.push_str(&format!("; SameSite={}", self.config.same_site));

        // Max-Age for idle timeout
        cookie.push_str(&format!("; Max-Age={}", self.config.idle_timeout.as_secs()));

        cookie
    }

    /// Create cookie for session destruction
    pub fn create_destroy_cookie(&self) -> String {
        let mut cookie = format!("{}=", self.config.cookie_name);

        cookie.push_str(&format!("; Path={}", self.config.path));

        if let Some(ref domain) = self.config.domain {
            cookie.push_str(&format!("; Domain={}", domain));
        }

        // Expire immediately
        cookie.push_str("; Max-Age=0");
        cookie.push_str("; Expires=Thu, 01 Jan 1970 00:00:00 GMT");

        cookie
    }
}
