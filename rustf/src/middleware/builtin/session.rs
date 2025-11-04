use crate::context::Context;
use crate::error::Result;
use crate::middleware::traits::{InboundAction, InboundMiddleware, OutboundMiddleware};
use crate::session::manager::{SaveStrategy, SessionConfig, SessionManager};
use async_trait::async_trait;
use std::sync::Arc;

/// Session middleware using the dual-phase pattern
///
/// This middleware handles session loading in the inbound phase
/// and cookie setting in the outbound phase, cleanly separating concerns.
#[derive(Clone)]
pub struct SessionMiddleware {
    manager: Arc<SessionManager>,
}

impl SessionMiddleware {
    /// Create new session middleware with custom configuration
    /// Uses default memory storage for backwards compatibility
    pub fn new(config: SessionConfig) -> Self {
        Self {
            manager: SessionManager::with_default_storage(config),
        }
    }

    /// Create with storage from configuration (async)
    /// This checks for custom storage in definitions first,
    /// then falls back to config-based storage (Memory/Redis)
    pub async fn with_configured_storage(
        config: SessionConfig,
        storage_config: &crate::config::SessionStorageConfig,
    ) -> crate::error::Result<Self> {
        let manager = SessionManager::with_configured_storage(config, storage_config).await?;
        Ok(Self { manager })
    }

    /// Create with custom manager
    pub fn with_manager(manager: Arc<SessionManager>) -> Self {
        Self { manager }
    }

    /// Create with default configuration
    pub fn default() -> Self {
        Self::new(SessionConfig::default())
    }

    /// Create with custom storage backend
    pub fn with_storage(
        storage: Arc<dyn crate::session::SessionStorage>,
        config: SessionConfig,
    ) -> Self {
        Self {
            manager: SessionManager::new(storage, config),
        }
    }

    /// Check if route is exempt from session handling
    fn is_route_exempt(&self, path: &str) -> bool {
        for exempt_pattern in &self.manager.config.exempt_routes {
            if self.matches_pattern(path, exempt_pattern) {
                return true;
            }
        }
        false
    }

    /// Simple glob pattern matching for routes
    fn matches_pattern(&self, path: &str, pattern: &str) -> bool {
        if let Some(prefix) = pattern.strip_suffix("/*") {
            // For /* pattern, path must start with prefix AND have a separator after it
            // OR be exactly the prefix followed by a slash
            if let Some(remaining) = path.strip_prefix(prefix) {
                // Must be followed by / or be exactly the prefix
                remaining.starts_with('/') || remaining.is_empty()
            } else {
                false
            }
        } else if pattern.contains('*') {
            // More complex glob patterns could be implemented here
            // For now, just handle the /* suffix case
            false
        } else {
            path == pattern
        }
    }
}

#[async_trait]
impl InboundMiddleware for SessionMiddleware {
    async fn process_request(&self, ctx: &mut Context) -> Result<InboundAction> {
        // Check if sessions are globally disabled
        if !self.manager.config.enabled {
            return Ok(InboundAction::Continue);
        }

        // Check if this path is exempt from session handling
        let request_path = ctx.req.path();
        if self.is_route_exempt(request_path) {
            return Ok(InboundAction::Continue);
        }

        // Extract session ID from cookie
        let session_id = ctx.req.cookie(&self.manager.config.cookie_name);

        // Load or create session (now fully async)
        let session = match session_id {
            Some(ref id) => {
                // Try to load existing session
                match self.manager.load_session(id, &ctx.req).await {
                    Ok(Some(session)) => session,
                    Ok(None) => {
                        // Session invalid or expired, create new one
                        self.manager.create_session(&ctx.req).await?
                    }
                    Err(e) => {
                        log::error!("Session middleware: error loading session {}: {}", id, e);
                        return Err(e);
                    }
                }
            }
            None => {
                // No session cookie, create new session
                self.manager.create_session(&ctx.req).await?
            }
        };

        // Store session metadata for outbound phase
        let is_new = session_id.is_none();
        let needs_rotation = session.needs_rotation();
        ctx.set("session_is_new", is_new)?;
        ctx.set("session_needs_rotation", needs_rotation)?;

        // Handle rotation if needed (now fully async)
        let session = if needs_rotation {
            log::info!("Session {} needs rotation", session.id());
            self.manager.rotate_session(&session, &ctx.req).await?
        } else {
            session
        };

        // Store session in context
        let session_arc = Arc::new(session);
        ctx.set_session(Some(Arc::clone(&session_arc)));

        // We always want to process the response to handle cookies
        Ok(InboundAction::Capture)
    }

    fn name(&self) -> &'static str {
        "session"
    }

    fn priority(&self) -> i32 {
        -500 // Run early in the middleware chain, after CORS but before custom middleware
    }
}

#[async_trait]
impl OutboundMiddleware for SessionMiddleware {
    async fn process_response(&self, ctx: &mut Context) -> Result<()> {
        // Skip if sessions are disabled or path is exempt
        let request_path = ctx.req.path();
        if !self.manager.config.enabled || self.is_route_exempt(request_path) {
            return Ok(());
        }

        // Save session if needed and get session ID for cookie (now fully async)
        let session_id = if let Some(session) = ctx.session_arc() {
            // Save session if using EndOfRequest strategy
            if matches!(
                self.manager.config.save_strategy,
                SaveStrategy::EndOfRequest
            ) {
                self.manager.force_save(session).await?;
            } else if matches!(self.manager.config.save_strategy, SaveStrategy::Immediate) {
                // For immediate strategy, save if dirty
                if session.is_dirty() {
                    self.manager.save_session(session).await?;
                }
            }

            Some(session.id().to_string())
        } else {
            None
        };

        // Set cookie after session work is done to avoid borrow conflicts
        if let Some(session_id) = session_id {
            if let Some(response) = ctx.res.as_mut() {
                // Always send session cookie to ensure browser has current valid session
                // This handles all cases: new sessions, recreated sessions, and refreshes expiry
                let cookie = self.manager.create_cookie(&session_id);
                response.add_header("Set-Cookie", &cookie);
            }
        } else {
            // Session was destroyed, send deletion cookie
            if let Some(response) = ctx.res.as_mut() {
                let cookie = self.manager.create_destroy_cookie();
                response.add_header("Set-Cookie", &cookie);
            }
        }

        Ok(())
    }
}
