//! Simple migration system for RustF framework
//!
//! This provides a basic migration system that works with the existing
//! RustF infrastructure without complex sqlx integration.

use crate::error::{Error, Result};
use chrono::{DateTime, TimeZone, Utc};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Simple migration representation
#[derive(Debug, Clone)]
pub struct SimpleMigration {
    pub id: String,
    pub name: String,
    pub up_sql: String,
    pub down_sql: String,
    pub file_path: PathBuf,
    pub created_at: DateTime<Utc>,
}

impl SimpleMigration {
    /// Create a new migration from a file
    pub fn from_file(file_path: impl AsRef<Path>) -> Result<Self> {
        let path = file_path.as_ref();
        let content = fs::read_to_string(path).map_err(|e| {
            Error::template(format!(
                "Failed to read migration file {}: {}",
                path.display(),
                e
            ))
        })?;

        // Parse filename to extract ID and name
        let filename = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| Error::template("Invalid migration filename".to_string()))?;

        let parts: Vec<&str> = filename.splitn(2, '_').collect();
        if parts.len() != 2 {
            return Err(Error::template(format!(
                "Migration filename must follow format: {{timestamp}}_{{name}}.sql: {}",
                filename
            )));
        }

        let id = parts[0].to_string();
        let name = parts[1].replace('_', " ");

        // Parse timestamp from ID
        let created_at = Self::parse_timestamp(&id)?;

        // Split content into up and down parts
        let (up_sql, down_sql) = Self::parse_migration_content(&content)?;

        Ok(SimpleMigration {
            id,
            name,
            up_sql,
            down_sql,
            file_path: path.to_path_buf(),
            created_at,
        })
    }

    /// Parse migration file content to extract up and down SQL
    fn parse_migration_content(content: &str) -> Result<(String, String)> {
        let mut parts = content.split("-- Down");

        let up_part = parts.next().ok_or_else(|| {
            Error::template("Migration file must contain SQL content".to_string())
        })?;

        let down_part = parts.next().unwrap_or(""); // Down migrations are optional

        // Clean up the up part - extract actual SQL (skip comment lines)
        let up_sql = up_part
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.starts_with("--") && !trimmed.is_empty()
            })
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string();

        let down_sql = down_part
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.starts_with("--") && !trimmed.is_empty()
            })
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string();

        // For template files, we allow empty up SQL if it contains placeholder text
        if up_sql.is_empty() && !content.contains("Add your up migration SQL here") {
            return Err(Error::template("Migration must contain up SQL".to_string()));
        }

        Ok((up_sql, down_sql))
    }

    /// Parse timestamp from migration ID
    fn parse_timestamp(id: &str) -> Result<DateTime<Utc>> {
        // Expected format: YYYYMMDDHHMMSS
        if id.len() != 14 {
            return Err(Error::template(format!(
                "Migration ID must be 14 digits (YYYYMMDDHHMMSS): {}",
                id
            )));
        }

        let year: i32 = id[0..4]
            .parse()
            .map_err(|_| Error::template(format!("Invalid year in migration ID: {}", id)))?;
        let month: u32 = id[4..6]
            .parse()
            .map_err(|_| Error::template(format!("Invalid month in migration ID: {}", id)))?;
        let day: u32 = id[6..8]
            .parse()
            .map_err(|_| Error::template(format!("Invalid day in migration ID: {}", id)))?;
        let hour: u32 = id[8..10]
            .parse()
            .map_err(|_| Error::template(format!("Invalid hour in migration ID: {}", id)))?;
        let minute: u32 = id[10..12]
            .parse()
            .map_err(|_| Error::template(format!("Invalid minute in migration ID: {}", id)))?;
        let second: u32 = id[12..14]
            .parse()
            .map_err(|_| Error::template(format!("Invalid second in migration ID: {}", id)))?;

        chrono::Utc
            .with_ymd_and_hms(year, month, day, hour, minute, second)
            .single()
            .ok_or_else(|| Error::template(format!("Invalid datetime in migration ID: {}", id)))
    }

    /// Generate a new migration ID based on current timestamp
    pub fn generate_id() -> String {
        Utc::now().format("%Y%m%d%H%M%S").to_string()
    }

    /// Create a new migration file template
    pub fn create_template(name: &str, migrations_dir: impl AsRef<Path>) -> Result<PathBuf> {
        let id = Self::generate_id();
        let filename = format!("{}_{}.sql", id, name.replace(' ', "_").to_lowercase());
        let file_path = migrations_dir.as_ref().join(filename);

        let template = format!(
            r#"-- Up
-- Migration: {}
-- Created: {}

-- Add your up migration SQL here
-- Example:
-- CREATE TABLE example (
--     id SERIAL PRIMARY KEY,
--     name VARCHAR(255) NOT NULL,
--     created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
-- );

-- Down
-- Add your down migration SQL here
-- Example:
-- DROP TABLE IF EXISTS example;
"#,
            name,
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        );

        // Ensure migrations directory exists
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                Error::template(format!("Failed to create migrations directory: {}", e))
            })?;
        }

        fs::write(&file_path, template)
            .map_err(|e| Error::template(format!("Failed to write migration file: {}", e)))?;

        Ok(file_path)
    }
}

/// Simple migration manager for basic operations
pub struct SimpleMigrationManager {
    migrations_dir: PathBuf,
}

impl SimpleMigrationManager {
    /// Create a new simple migration manager
    pub fn new(migrations_dir: impl AsRef<Path>) -> Result<Self> {
        let migrations_dir = migrations_dir.as_ref().to_path_buf();

        // Ensure migrations directory exists
        if !migrations_dir.exists() {
            fs::create_dir_all(&migrations_dir).map_err(|e| {
                Error::template(format!(
                    "Failed to create migrations directory {}: {}",
                    migrations_dir.display(),
                    e
                ))
            })?;
            println!(
                "📁 Created migrations directory: {}",
                migrations_dir.display()
            );
        }

        Ok(Self { migrations_dir })
    }

    /// Create a new migration file
    pub fn create_migration(&self, name: &str) -> Result<PathBuf> {
        // Validate migration name
        if name.is_empty() {
            return Err(Error::template(
                "Migration name cannot be empty".to_string(),
            ));
        }

        if name.len() > 100 {
            return Err(Error::template(
                "Migration name too long (max 100 characters)".to_string(),
            ));
        }

        // Check for invalid characters
        if !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == ' ' || c == '-')
        {
            return Err(Error::template("Migration name can only contain letters, numbers, spaces, hyphens, and underscores".to_string()));
        }

        let file_path = SimpleMigration::create_template(name, &self.migrations_dir)?;

        println!("📝 Created migration file: {}", file_path.display());
        println!("   Migration name: {}", name);
        println!("   ID: {}", SimpleMigration::generate_id());

        Ok(file_path)
    }

    /// Load all migration files from directory
    pub fn load_migrations(&self) -> Result<Vec<SimpleMigration>> {
        if !self.migrations_dir.exists() {
            return Ok(Vec::new());
        }

        let mut migrations = Vec::new();

        let entries = fs::read_dir(&self.migrations_dir).map_err(|e| {
            Error::template(format!(
                "Failed to read migrations directory {}: {}",
                self.migrations_dir.display(),
                e
            ))
        })?;

        for entry in entries {
            let entry = entry
                .map_err(|e| Error::template(format!("Failed to read directory entry: {}", e)))?;
            let file_path = entry.path();

            // Only process .sql files
            if file_path.extension().and_then(|s| s.to_str()) == Some("sql") {
                match SimpleMigration::from_file(&file_path) {
                    Ok(migration) => migrations.push(migration),
                    Err(e) => {
                        eprintln!(
                            "Warning: Failed to parse migration file {}: {}",
                            file_path.display(),
                            e
                        );
                        continue;
                    }
                }
            }
        }

        // Sort migrations by ID (timestamp)
        migrations.sort_by(|a, b| a.id.cmp(&b.id));

        Ok(migrations)
    }

    /// List all migrations with their basic information
    pub fn list_migrations(&self) -> Result<Vec<MigrationInfo>> {
        let migrations = self.load_migrations()?;

        let items = migrations
            .into_iter()
            .map(|migration| MigrationInfo {
                id: migration.id.clone(),
                name: migration.name.clone(),
                file_path: migration.file_path.clone(),
                created_at: migration.created_at,
                has_down_migration: !migration.down_sql.trim().is_empty(),
            })
            .collect();

        Ok(items)
    }

    /// Validate all migration files
    pub fn validate_migrations(&self) -> Result<ValidationResult> {
        let migrations = self.load_migrations()?;

        let mut result = ValidationResult {
            valid_count: 0,
            total_count: migrations.len(),
            errors: Vec::new(),
            warnings: Vec::new(),
        };

        // Check for duplicate IDs
        let mut id_count = HashMap::new();
        for migration in &migrations {
            *id_count.entry(migration.id.clone()).or_insert(0) += 1;
        }

        for migration in migrations {
            let mut has_errors = false;

            // Check for duplicate ID
            if *id_count.get(&migration.id).unwrap_or(&0) > 1 {
                result
                    .errors
                    .push(format!("Migration {} has duplicate ID", migration.id));
                has_errors = true;
            }

            // Check up SQL
            if !has_errors && migration.up_sql.trim().is_empty() {
                result
                    .errors
                    .push(format!("Migration {} has no up SQL", migration.id));
                has_errors = true;
            }

            // Check down SQL (warning if missing)
            if migration.down_sql.trim().is_empty() {
                result.warnings.push(format!(
                    "Migration {} has no down SQL - rollback will not be possible",
                    migration.id
                ));
            }

            // Check for potentially dangerous operations
            let dangerous_patterns = [
                ("DROP TABLE", "Dropping tables"),
                ("DROP DATABASE", "Dropping databases"),
                ("DELETE FROM", "Deleting data"),
                ("TRUNCATE", "Truncating tables"),
            ];

            for (pattern, description) in dangerous_patterns {
                if migration.up_sql.to_uppercase().contains(pattern) {
                    result.warnings.push(format!(
                        "Migration {} contains {}: {}",
                        migration.id, description, pattern
                    ));
                }
            }

            if !has_errors {
                result.valid_count += 1;
            }
        }

        Ok(result)
    }

    /// Get migrations directory path
    pub fn migrations_dir(&self) -> &Path {
        &self.migrations_dir
    }
}

/// Basic migration information
#[derive(Debug, Clone)]
pub struct MigrationInfo {
    pub id: String,
    pub name: String,
    pub file_path: PathBuf,
    pub created_at: DateTime<Utc>,
    pub has_down_migration: bool,
}

/// Validation result for migrations
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub valid_count: usize,
    pub total_count: usize,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};
    use std::fs::write;
    use tempfile::TempDir;

    #[test]
    fn test_migration_id_generation() {
        let id = SimpleMigration::generate_id();
        assert_eq!(id.len(), 14);
        assert!(id.chars().all(|c| c.is_numeric()));
    }

    #[test]
    fn test_parse_timestamp() {
        let timestamp = "20250104123045";
        let parsed = SimpleMigration::parse_timestamp(timestamp).unwrap();
        assert_eq!(parsed.year(), 2025);
        assert_eq!(parsed.month(), 1);
        assert_eq!(parsed.day(), 4);
        assert_eq!(parsed.hour(), 12);
        assert_eq!(parsed.minute(), 30);
        assert_eq!(parsed.second(), 45);
    }

    #[test]
    fn test_parse_migration_content() {
        let content = r#"
-- Up
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL
);

-- Down
DROP TABLE users;
"#;

        let (up_sql, down_sql) = SimpleMigration::parse_migration_content(content).unwrap();
        assert!(up_sql.contains("CREATE TABLE users"));
        assert!(down_sql.contains("DROP TABLE users"));
    }

    #[test]
    fn test_migration_from_file() {
        let temp_dir = TempDir::new().unwrap();
        let migration_file = temp_dir.path().join("20250104123045_create_users.sql");

        let content = r#"-- Up
CREATE TABLE users (id SERIAL PRIMARY KEY);

-- Down
DROP TABLE users;
"#;

        write(&migration_file, content).unwrap();

        let migration = SimpleMigration::from_file(&migration_file).unwrap();
        assert_eq!(migration.id, "20250104123045");
        assert_eq!(migration.name, "create users");
        assert!(migration.up_sql.contains("CREATE TABLE users"));
        assert!(migration.down_sql.contains("DROP TABLE users"));
    }

    #[test]
    fn test_simple_migration_manager() {
        let temp_dir = TempDir::new().unwrap();
        let manager = SimpleMigrationManager::new(temp_dir.path()).unwrap();

        // Create a test migration
        let file_path = manager.create_migration("test migration").unwrap();
        assert!(file_path.exists());

        // Load migrations
        let migrations = manager.load_migrations().unwrap();
        assert_eq!(migrations.len(), 1);
        assert_eq!(migrations[0].name, "test migration");

        // List migrations
        let info_list = manager.list_migrations().unwrap();
        assert_eq!(info_list.len(), 1);
        assert_eq!(info_list[0].name, "test migration");

        // Validate migrations - template files will have no actual SQL but that's okay for testing
        let validation = manager.validate_migrations().unwrap();

        // Template files don't contain actual SQL, so they won't be valid migrations
        // but we can still test the validation structure
        assert_eq!(validation.total_count, 1);
        assert!(!validation.is_valid()); // Template files aren't valid migrations
        assert_eq!(validation.errors.len(), 1);
        assert!(validation.errors[0].contains("has no up SQL"));
    }
}
