use crate::config;
use crate::session::manager::{SaveStrategy, SessionConfig as InternalSessionConfig};
use crate::session::{FingerprintMode, SameSite};
use std::time::Duration;

impl InternalSessionConfig {
    /// The session settings of a loaded configuration, with the cookie's
    /// `Secure` flag set from its environment: secure in production, plain
    /// HTTP allowed in development. Use this when building a
    /// `SessionMiddleware` yourself.
    ///
    /// There is deliberately no `From<config::SessionConfig>`: the session
    /// section alone does not say whether the application runs in
    /// production, and guessing let a production app issue cookies without
    /// `Secure`.
    pub fn from_app_config(app_config: &config::AppConfig) -> Self {
        let cfg = &app_config.session;
        Self {
            // Essential user-configurable settings
            cookie_name: cfg.cookie_name.clone(),
            idle_timeout: Duration::from_secs(cfg.idle_timeout),
            absolute_timeout: Duration::from_secs(cfg.absolute_timeout),
            same_site: parse_same_site(&cfg.same_site),
            exempt_routes: cfg.exempt_routes.clone(),
            enabled: cfg.enabled,

            // Security settings
            secure: app_config.environment.is_production(),
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

/// Create SessionMiddleware from app config (dual-phase version)
pub async fn create_session_middleware(
    config: &config::AppConfig,
) -> Option<crate::middleware::builtin::session::SessionMiddleware> {
    if !config.session.enabled {
        return None;
    }

    let session_config = InternalSessionConfig::from_app_config(config);

    // Try to use configured storage (checks definitions first, then config)
    match crate::middleware::builtin::session::SessionMiddleware::with_configured_storage(
        session_config.clone(),
        &config.session,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AppConfig, Environment};

    #[test]
    fn from_app_config_follows_the_configured_environment() {
        let production = AppConfig {
            environment: Environment::Production,
            ..AppConfig::default()
        };
        assert!(InternalSessionConfig::from_app_config(&production).secure);
        assert!(!InternalSessionConfig::from_app_config(&AppConfig::default()).secure);
    }
}
