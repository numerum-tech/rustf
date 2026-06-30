//! Bootstrap event handlers for the sample app.

use rustf::events::{EventContext, EventEmitter};
use rustf::prelude::Error;
use sqlx::Row;
use std::fs;
use std::path::{Path, PathBuf};

pub fn install(emitter: &mut EventEmitter) {
    emitter.on("config.loaded", |ctx| Box::pin(on_config_loaded(ctx)));
    emitter.on("database.ready", |ctx| Box::pin(on_database_ready(ctx)));
}

async fn on_config_loaded(ctx: EventContext) -> rustf::Result<()> {
    let Some(database_url) = ctx.config.database.url.as_deref() else {
        return Ok(());
    };

    if !database_url.starts_with("sqlite://") {
        return Ok(());
    }

    if let Some(path) = sqlite_file_path(database_url) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
    }

    fs::create_dir_all("private/uploads")?;
    Ok(())
}

async fn on_database_ready(ctx: EventContext) -> rustf::Result<()> {
    let Some(database_url) = ctx.config.database.url.as_deref() else {
        return Ok(());
    };

    if !database_url.starts_with("sqlite://") {
        return Ok(());
    }

    let pool = rustf::db::DB::sqlite_pool()?;
    let existing = sqlx::query(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name IN ('users', 'task_lists', 'tasks')",
    )
    .fetch_all(&*pool)
    .await
    .map_err(|e| Error::internal(format!("Failed to inspect SQLite schema: {}", e)))?;

    if existing.len() == 3 {
        return Ok(());
    }

    let schema_sql = fs::read_to_string("sql/schema.sql")?;
    let normalized = schema_sql
        .lines()
        .filter(|line| !line.trim_start().starts_with("--"))
        .map(|line| line.replace("CREATE TABLE ", "CREATE TABLE IF NOT EXISTS "))
        .collect::<Vec<_>>()
        .join("\n");

    for statement in normalized.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        sqlx::query(statement)
            .execute(&*pool)
            .await
            .map_err(|e| Error::internal(format!("Failed to apply SQLite schema: {}", e)))?;
    }

    let remaining = sqlx::query(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name IN ('users', 'task_lists', 'tasks')",
    )
    .fetch_all(&*pool)
    .await
    .map_err(|e| Error::internal(format!("Failed to verify SQLite schema: {}", e)))?;

    if remaining.len() != 3 {
        let found = remaining
            .iter()
            .map(|row| row.get::<String, _>("name"))
            .collect::<Vec<_>>()
            .join(", ");
        return Err(Error::internal(format!(
            "SQLite bootstrap incomplete; expected users/task_lists/tasks, found [{}]",
            found
        )));
    }

    Ok(())
}

fn sqlite_file_path(database_url: &str) -> Option<PathBuf> {
    let raw = database_url.strip_prefix("sqlite://")?;
    if raw.is_empty() || raw == ":memory:" {
        return None;
    }

    let path = if raw.starts_with('/') {
        PathBuf::from(raw)
    } else {
        Path::new(raw).to_path_buf()
    };

    Some(path)
}

#[cfg(test)]
mod tests {
    use super::sqlite_file_path;
    use std::path::PathBuf;

    #[test]
    fn sqlite_file_path_handles_relative_urls() {
        assert_eq!(
            sqlite_file_path("sqlite://./private/data/app.db").unwrap(),
            PathBuf::from("./private/data/app.db")
        );
    }

    #[test]
    fn sqlite_file_path_ignores_memory_urls() {
        assert!(sqlite_file_path("sqlite://:memory:").is_none());
        assert!(sqlite_file_path("sqlite://").is_none());
    }
}
