use crate::config;
use crate::session::manager::{SaveStrategy, SessionConfig as InternalSessionConfig};
use crate::session::{FingerprintMode, SameSite};
use std::time::Duration;

/// Convert from config file SessionConfig to internal SessionConfig
impl From<config::SessionConfig> for InternalSessionConfig {
    fn from(cfg: config::SessionConfig) -> Self {
        Self {
            // Essential user-configurable settings
            cookie_name: cfg.cookie_name,
            idle_timeout: Duration::from_secs(cfg.idle_timeout),
            absolute_timeout: Duration::from_secs(cfg.absolute_timeout),
            same_site: parse_same_site(&cfg.same_site),
            exempt_routes: cfg.exempt_routes,
            enabled: cfg.enabled,

            // Security settings - auto-determined
            secure: is_production(),
            http_only: true, // Always true for security

            // Fixed secure defaults
            domain: None,
            path: "/".to_string(),
            rotation_on_privilege: true,
            fingerprint_mode: parse_fingerprint_mode(&cfg.fingerprint_mode),
            secure_id_length: 32,
            save_strategy: SaveStrategy::EndOfRequest,
        }
    }
}

/// Parse SameSite string from config
fn parse_same_site(value: &str) -> SameSite {
    match value.to_lowercase().as_str() {
        "strict" => SameSite::Strict,
        "lax" => SameSite::Lax,
        "none" => SameSite::None,
        _ => {
            log::warn!("Invalid SameSite value '{}', defaulting to Lax", value);
            SameSite::Lax
        }
    }
}

/// Parse FingerprintMode string from config
fn parse_fingerprint_mode(value: &str) -> FingerprintMode {
    match value.to_lowercase().as_str() {
        "disabled" => FingerprintMode::Disabled,
        "soft" => FingerprintMode::Soft,
        "strict" => FingerprintMode::Strict,
        _ => {
            log::warn!(
                "Invalid FingerprintMode value '{}', defaulting to Soft",
                value
            );
            FingerprintMode::Soft
        }
    }
}

/// Determine if we're in production based on environment
fn is_production() -> bool {
    // Check RustF environment setting
    if let Ok(env) = std::env::var("RUSTF_ENVIRONMENT") {
        return env.to_lowercase() == "production" || env.to_lowercase() == "prod";
    }

    // Check standard Rust environment
    if let Ok(env) = std::env::var("RUST_ENV") {
        return env.to_lowercase() == "production" || env.to_lowercase() == "prod";
    }

    // Check NODE_ENV for compatibility
    if let Ok(env) = std::env::var("NODE_ENV") {
        return env.to_lowercase() == "production" || env.to_lowercase() == "prod";
    }

    // In release builds, default to secure
    #[cfg(not(debug_assertions))]
    {
        true
    }

    // In debug builds, default to insecure
    #[cfg(debug_assertions)]
    {
        false
    }
}

/// Create SessionMiddleware from app config (dual-phase version)
pub async fn create_session_middleware(
    config: &config::AppConfig,
) -> Option<crate::middleware::builtin::session::SessionMiddleware> {
    if !config.session.enabled {
        return None;
    }

    let session_config: InternalSessionConfig = config.session.clone().into();

    // Try to use configured storage (checks definitions first, then config)
    match crate::middleware::builtin::session::SessionMiddleware::with_configured_storage(
        session_config.clone(),
        &config.session.storage,
    )
    .await
    {
        Ok(middleware) => Some(middleware),
        Err(e) => {
            log::warn!(
                "Failed to create configured session storage: {}, falling back to memory",
                e
            );
            Some(crate::middleware::builtin::session::SessionMiddleware::new(
                session_config,
            ))
        }
    }
}
