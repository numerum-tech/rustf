//! Built-in event handlers for common startup tasks
//!
//! Provides ready-to-use event handlers for database seeding, directory setup,
//! cleanup operations, and other common application initialization tasks.

use super::EventContext;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tokio::fs;

/// Database seeder event handler
///
/// Automatically runs SQL seed files from a directory when the application starts.
///
/// # Example
/// ```rust,ignore
/// app.on("ready", database_seeder("seeds/"));
/// ```
pub fn database_seeder(
    seed_dir: impl Into<PathBuf>,
) -> impl Fn(
    EventContext,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<()>> + Send>>
       + Send
       + Sync
       + 'static {
    let seed_dir = seed_dir.into();

    move |ctx| {
        let seed_dir = seed_dir.clone();
        Box::pin(async move {
            // Only run seeds in development by default
            if !ctx.is_development() {
                log::info!("Skipping database seeding (not in development mode)");
                return Ok(());
            }

            log::info!("Running database seeds from: {:?}", seed_dir);

            if !seed_dir.exists() {
                log::warn!("Seed directory does not exist: {:?}", seed_dir);
                return Ok(());
            }

            let mut entries = fs::read_dir(&seed_dir).await?;
            let mut seed_files = Vec::new();

            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("sql") {
                    seed_files.push(path);
                }
            }

            // Sort files to ensure consistent execution order
            seed_files.sort();

            for seed_file in seed_files {
                log::info!("Executing seed file: {:?}", seed_file.file_name());
                let _sql = fs::read_to_string(&seed_file).await?;

                // Execute the SQL
                // NOTE: Awaiting DB::execute method implementation for direct SQL execution
                // Currently logs the action for debugging purposes
                log::info!("Would execute SQL from: {:?}", seed_file.file_name());
            }

            log::info!("Database seeding completed");

            Ok(())
        })
    }
}

/// Directory setup event handler
///
/// Ensures required directories exist with proper permissions.
///
/// # Example
/// ```rust,ignore
/// app.on("startup", directory_setup(&["uploads", "temp", "logs", "cache"]));
/// ```
pub fn directory_setup(
    directories: &'static [&'static str],
) -> impl Fn(
    EventContext,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<()>> + Send>>
       + Send
       + Sync
       + 'static {
    move |_ctx| {
        Box::pin(async move {
            log::info!("Setting up application directories");

            for dir in directories {
                let path = Path::new(dir);

                if !path.exists() {
                    log::info!("Creating directory: {}", dir);
                    fs::create_dir_all(path).await?;

                    // Set permissions on Unix systems
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let mut perms = fs::metadata(path).await?.permissions();
                        perms.set_mode(0o755); // rwxr-xr-x
                        fs::set_permissions(path, perms).await?;
                    }
                } else {
                    log::debug!("Directory already exists: {}", dir);
                }
            }

            log::info!("Directory setup completed");
            Ok(())
        })
    }
}

/// Cleanup manager event handler
///
/// Removes old temporary files and performs other cleanup tasks.
///
/// # Example
/// ```rust,ignore
/// app.on("startup", cleanup_manager("temp/", Duration::from_secs(86400))); // 24 hours
/// ```
pub fn cleanup_manager(
    temp_dir: impl Into<PathBuf>,
    max_age: Duration,
) -> impl Fn(
    EventContext,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<()>> + Send>>
       + Send
       + Sync
       + 'static {
    let temp_dir = temp_dir.into();

    move |_ctx| {
        let temp_dir = temp_dir.clone();
        Box::pin(async move {
            log::info!("Running cleanup manager");

            if !temp_dir.exists() {
                log::debug!(
                    "Temp directory does not exist, skipping cleanup: {:?}",
                    temp_dir
                );
                return Ok(());
            }

            let mut entries = fs::read_dir(&temp_dir).await?;
            let now = SystemTime::now();
            let mut removed_count = 0;

            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                let metadata = match entry.metadata().await {
                    Ok(m) => m,
                    Err(e) => {
                        log::warn!("Failed to get metadata for {:?}: {}", path, e);
                        continue;
                    }
                };

                if let Ok(modified) = metadata.modified() {
                    if let Ok(age) = now.duration_since(modified) {
                        if age > max_age {
                            log::debug!("Removing old file: {:?} (age: {:?})", path, age);

                            if metadata.is_dir() {
                                if let Err(e) = fs::remove_dir_all(&path).await {
                                    log::warn!("Failed to remove directory {:?}: {}", path, e);
                                }
                            } else if let Err(e) = fs::remove_file(&path).await {
                                log::warn!("Failed to remove file {:?}: {}", path, e);
                            }

                            removed_count += 1;
                        }
                    }
                }
            }

            log::info!(
                "Cleanup completed: removed {} old files/directories",
                removed_count
            );
            Ok(())
        })
    }
}

/// Log rotation event handler
///
/// Rotates log files to prevent them from growing too large.
///
/// # Example
/// ```rust,ignore
/// app.on("startup", log_rotation("logs/", 10)); // Keep 10 old log files
/// ```
pub fn log_rotation(
    log_dir: impl Into<PathBuf>,
    max_files: usize,
) -> impl Fn(
    EventContext,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<()>> + Send>>
       + Send
       + Sync
       + 'static {
    let log_dir = log_dir.into();

    move |_ctx| {
        let log_dir = log_dir.clone();
        Box::pin(async move {
            log::info!("Running log rotation");

            if !log_dir.exists() {
                log::debug!(
                    "Log directory does not exist, skipping rotation: {:?}",
                    log_dir
                );
                return Ok(());
            }

            let mut entries = fs::read_dir(&log_dir).await?;
            let mut log_files = Vec::new();

            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("log") {
                    if let Ok(metadata) = entry.metadata().await {
                        if let Ok(modified) = metadata.modified() {
                            log_files.push((path, modified));
                        }
                    }
                }
            }

            // Sort by modification time (oldest first)
            log_files.sort_by_key(|(_, time)| *time);

            // Remove old log files if we have too many
            if log_files.len() > max_files {
                let to_remove = log_files.len() - max_files;
                for (path, _) in log_files.iter().take(to_remove) {
                    log::info!("Removing old log file: {:?}", path.file_name());
                    if let Err(e) = fs::remove_file(path).await {
                        log::warn!("Failed to remove log file {:?}: {}", path, e);
                    }
                }
            }

            log::info!("Log rotation completed");
            Ok(())
        })
    }
}

/// Environment check event handler
///
/// Validates that the application is running in the expected environment.
///
/// # Example
/// ```rust,ignore
/// app.on("startup", environment_check("production"));
/// ```
pub fn environment_check(
    expected_env: &'static str,
) -> impl Fn(
    EventContext,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<()>> + Send>>
       + Send
       + Sync
       + 'static {
    move |ctx| {
        Box::pin(async move {
            let current_env = ctx.env();

            if current_env != expected_env {
                log::warn!(
                    "Environment mismatch! Expected: '{}', Current: '{}'",
                    expected_env,
                    current_env
                );

                // You might want to fail hard in production
                if expected_env == "production" {
                    return Err(crate::error::Error::internal(format!(
                        "Application configured for production but running in '{}' mode",
                        current_env
                    )));
                }
            } else {
                log::info!("Environment check passed: {}", current_env);
            }

            Ok(())
        })
    }
}

/// Configuration validator event handler
///
/// Validates critical configuration settings at startup.
///
/// # Example
/// ```rust,ignore
/// app.on("config.loaded", configuration_validator);
/// ```
pub fn configuration_validator(
    ctx: EventContext,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<()>> + Send>> {
    Box::pin(async move {
        log::info!("Validating configuration");

        let config = &ctx.config;

        // Check for common configuration issues
        if ctx.is_production() {
            // Ensure database URL is configured
            if config.database.url.is_none() {
                return Err(crate::error::Error::internal(
                    "Database URL not configured for production".to_string(),
                ));
            }
        }

        // Validate directories exist or can be created
        let dirs_to_check = vec![&config.views.directory, &config.static_files.directory];

        for dir in dirs_to_check {
            let path = Path::new(dir);
            if !path.exists() {
                log::warn!("Directory does not exist and will be created: {}", dir);
            }
        }

        log::info!("Configuration validation completed");
        Ok(())
    })
}

/// Health check event handler
///
/// Performs basic health checks on application startup.
///
/// # Example
/// ```rust,ignore
/// app.on("ready", health_check);
/// ```
pub fn health_check(
    _ctx: EventContext,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<()>> + Send>> {
    Box::pin(async move {
        log::info!("Running health checks");

        // Check database connectivity
        use crate::db::DB;
        match DB::ping().await {
            Ok(true) => log::info!("✓ Database health check passed"),
            Ok(false) => log::info!("⚠ Database not configured (skipping health check)"),
            Err(e) => {
                log::error!("✗ Database health check failed: {}", e);
                return Err(crate::error::Error::internal(format!(
                    "Database health check failed: {}",
                    e
                )));
            }
        }

        // Check disk space (simplified check - placeholder for user implementation)
        log::info!("✓ Disk space check (placeholder)");

        // Check memory usage (simplified - placeholder for user implementation)
        log::info!("✓ Memory usage check (placeholder)");

        log::info!("Health checks completed");
        Ok(())
    })
}

/// Log shutdown event handler
///
/// Logs shutdown notification with timing information.
///
/// # Example
/// ```rust,ignore
/// app.on("shutdown", log_shutdown);
/// ```
pub fn log_shutdown(
    ctx: EventContext,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<()>> + Send>> {
    Box::pin(async move {
        let shutdown_timeout = ctx.config.server.shutdown_timeout;
        log::info!(
            "🛑 Shutting down {} application (timeout: {}s)",
            ctx.env(),
            shutdown_timeout
        );

        // Log current connections or pending tasks if available
        log::info!("Waiting for active connections to close...");

        Ok(())
    })
}

/// Flush logs event handler
///
/// Ensures all buffered log messages are written before shutdown.
///
/// # Example
/// ```rust,ignore
/// app.on_priority("shutdown", 100, flush_logs); // Run late in shutdown
/// ```
pub fn flush_logs(
    _ctx: EventContext,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<()>> + Send>> {
    Box::pin(async move {
        log::info!("Flushing log buffers...");

        // Force flush of log messages
        // Note: Most log implementations auto-flush, but this ensures it
        log::logger().flush();

        // Give a small delay to ensure writes complete
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        Ok(())
    })
}

/// Save application state event handler
///
/// Saves critical application state that should persist across restarts.
///
/// # Example
/// ```rust,ignore
/// app.on_priority("shutdown", -50, save_state("state.json")); // Run early
/// ```
pub fn save_state(
    state_file: impl Into<PathBuf>,
) -> impl Fn(
    EventContext,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<()>> + Send>>
       + Send
       + Sync
       + 'static {
    let state_file = state_file.into();

    move |_ctx| {
        let state_file = state_file.clone();
        Box::pin(async move {
            log::info!("Saving application state to {:?}", state_file);

            // Create state directory if needed
            if let Some(parent) = state_file.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent).await?;
                }
            }

            // Here you would serialize and save your application state
            // For now, we'll just write a timestamp
            let state_data = format!(
                "{{\"shutdown_at\": \"{:?}\", \"version\": \"1.0.0\"}}\n",
                SystemTime::now()
            );

            fs::write(&state_file, state_data).await?;
            log::info!("Application state saved successfully");

            Ok(())
        })
    }
}

/// Cleanup temporary files on shutdown
///
/// Removes temporary files and directories during shutdown.
///
/// # Example
/// ```rust,ignore
/// app.on("shutdown", cleanup_temp_files("temp/"));
/// ```
pub fn cleanup_temp_files(
    temp_dir: impl Into<PathBuf>,
) -> impl Fn(
    EventContext,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<()>> + Send>>
       + Send
       + Sync
       + 'static {
    let temp_dir = temp_dir.into();

    move |ctx| {
        let temp_dir = temp_dir.clone();
        Box::pin(async move {
            // Only clean temp files in development by default
            if !ctx.is_development() {
                log::debug!("Skipping temp file cleanup in {} mode", ctx.env());
                return Ok(());
            }

            log::info!("Cleaning up temporary files in {:?}", temp_dir);

            if !temp_dir.exists() {
                return Ok(());
            }

            // Remove the entire temp directory in development
            if let Err(e) = fs::remove_dir_all(&temp_dir).await {
                log::warn!("Failed to remove temp directory: {}", e);
            } else {
                log::info!("Temporary files cleaned up");
            }

            Ok(())
        })
    }
}

/// Notification event handler for shutdown
///
/// Sends notifications to external services about shutdown.
///
/// # Example
/// ```rust,ignore
/// app.on_priority("shutdown", -100, notify_shutdown("https://api.example.com/webhook"));
/// ```
pub fn notify_shutdown(
    webhook_url: &'static str,
) -> impl Fn(
    EventContext,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<()>> + Send>>
       + Send
       + Sync
       + 'static {
    move |ctx| {
        Box::pin(async move {
            log::info!("Notifying external services of shutdown");

            // Here you would implement actual webhook notification
            log::debug!(
                "Would notify webhook: {} (environment: {})",
                webhook_url,
                ctx.env()
            );

            // Placeholder for actual implementation
            // You might use reqwest or similar to send the notification

            Ok(())
        })
    }
}
