use crate::config::SessionStorageConfig;
use crate::error::Result;
use crate::session::{storage::MemorySessionStorage, FingerprintMode, SessionStorage};
use std::sync::Arc;
use std::time::Duration;

/// Session storage factory for creating storage backends from configuration
pub struct SessionStorageFactory;

impl SessionStorageFactory {
    /// Create a session storage backend from configuration
    ///
    /// This method first checks for user-defined session storage in definitions,
    /// then falls back to native implementations based on configuration.
    pub async fn create_storage(
        config: &SessionStorageConfig,
        fingerprint_mode: FingerprintMode,
    ) -> Result<Arc<dyn SessionStorage>> {
        // Check for user-defined session storage from definitions
        let definitions = crate::definitions::get().await;
        let defs = definitions.read().await;

        if let Some(factory) = defs.get_session_storage_factory() {
            log::info!("Using custom session storage from definitions");

            // Convert SessionStorageConfig to SessionConfig for the factory
            // The factory expects SessionConfig, not SessionStorageConfig
            let session_config = crate::config::SessionConfig {
                enabled: true,
                cookie_name: "rustf.sid".to_string(), // Default, can be overridden
                idle_timeout: 1800,                   // 30 minutes default
                absolute_timeout: 86400,              // 24 hours default
                same_site: "lax".to_string(),
                fingerprint_mode: "soft".to_string(),
                exempt_routes: vec![],
                storage: config.clone(),
            };

            // Call the custom factory
            match factory(&session_config) {
                Ok(storage) => {
                    log::info!("Successfully created custom session storage");
                    return Ok(storage);
                }
                Err(e) => {
                    log::warn!(
                        "Failed to create custom session storage: {}, falling back to default",
                        e
                    );
                    // Fall through to default implementation
                }
            }
        }

        // Configuration-based creation for native implementations
        match config {
            SessionStorageConfig::Memory { cleanup_interval } => {
                let storage = Arc::new(MemorySessionStorage::with_config(
                    Duration::from_secs(30 * 60), // 30 minutes default timeout
                    Duration::from_secs(*cleanup_interval),
                    fingerprint_mode,
                ));
                Ok(storage)
            }

            SessionStorageConfig::Redis {
                url,
                prefix,
                pool_size,
                connection_timeout: _,
                command_timeout: _,
            } => {
                use crate::session::redis::RedisSessionStorage;
                let storage = Arc::new(
                    RedisSessionStorage::from_url(url, prefix, *pool_size, fingerprint_mode)
                        .await?,
                );
                Ok(storage)
            }

            SessionStorageConfig::Database {
                table: _,
                connection_url: _,
                cleanup_interval: _,
            } => {
                // Database storage should be implemented by users via the SessionStorage trait
                // This allows for custom database backends and configurations
                Err(crate::error::Error::internal(
                    "Database session storage must be implemented by the application. \
                     Please implement the SessionStorage trait for your database backend. \
                     See the documentation for examples using SQLx or other database libraries."
                        .to_string(),
                ))
            }
        }
    }

    /// Create default memory storage
    pub fn create_memory_storage() -> Arc<dyn SessionStorage> {
        Arc::new(MemorySessionStorage::new())
    }

    /// Create Redis storage with URL
    pub async fn create_redis_storage(
        redis_url: &str,
        fingerprint_mode: FingerprintMode,
    ) -> Result<Arc<dyn SessionStorage>> {
        use crate::session::redis::RedisSessionStorage;
        let storage = Arc::new(
            RedisSessionStorage::from_url(redis_url, "rustf:session:", 10, fingerprint_mode)
                .await?,
        );
        Ok(storage)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SessionStorageConfig;
    use crate::session::FingerprintMode;

    #[tokio::test]
    async fn test_create_memory_storage() {
        let config = SessionStorageConfig::Memory {
            cleanup_interval: 300,
        };

        let storage = SessionStorageFactory::create_storage(&config, FingerprintMode::Soft)
            .await
            .unwrap();
        assert_eq!(storage.backend_name(), "memory");
    }

    #[tokio::test]
    async fn test_create_redis_storage() {
        let config = SessionStorageConfig::Redis {
            url: "redis://localhost:6379".to_string(),
            prefix: "test:session:".to_string(),
            pool_size: 5,
            connection_timeout: 5000,
            command_timeout: 3000,
        };

        // This test will only pass if Redis is running
        if let Ok(storage) =
            SessionStorageFactory::create_storage(&config, FingerprintMode::Soft).await
        {
            assert_eq!(storage.backend_name(), "redis");
        } else {
            println!("Redis not available, skipping Redis storage test");
        }
    }

    #[tokio::test]
    async fn test_create_default_memory_storage() {
        let storage = SessionStorageFactory::create_memory_storage();
        assert_eq!(storage.backend_name(), "memory");
    }
}
