//! Database introspection and schema generation commands

mod common;
mod mysql;
mod postgres;
mod sqlite;

use anyhow::Result;
use clap::{Args, Subcommand};
use rustf::config::AppConfig;
use std::path::{Path, PathBuf};

// Re-export common types
use self::{mysql::MySqlIntrospector, postgres::PostgresIntrospector, sqlite::SqliteIntrospector};
use async_trait::async_trait;
pub use common::*;

/// Database introspector trait for database-specific implementations
#[async_trait]
pub trait DatabaseIntrospector: Send + Sync {
    /// List all tables in the database
    async fn list_tables(&self, metadata: bool) -> Result<Vec<TableInfo>>;

    /// Describe a specific table structure
    async fn describe_table(&self, table_name: &str) -> Result<TableDescription>;

    /// Generate YAML schemas for all tables
    async fn generate_schemas(
        &self,
        output_dir: &PathBuf,
        force: bool,
        filter_tables: &[String],
    ) -> Result<()>;

    /// Generate meta YAML file
    async fn generate_meta_yaml(&self) -> Result<String>;

    /// Get database name
    async fn get_database_name(&self) -> Result<String>;

    /// Export data from a query
    async fn export_data(&self, query: &str, format: &str) -> Result<String>;
}

/// Create a database introspector based on the database URL
async fn create_introspector(database_url: &str) -> Result<Box<dyn DatabaseIntrospector>> {
    if database_url.starts_with("mysql://") {
        Ok(Box::new(MySqlIntrospector::new(database_url).await?))
    } else if database_url.starts_with("postgresql://") || database_url.starts_with("postgres://") {
        Ok(Box::new(PostgresIntrospector::new(database_url).await?))
    } else if database_url.starts_with("sqlite://") {
        Ok(Box::new(SqliteIntrospector::new(database_url).await?))
    } else {
        anyhow::bail!("Unsupported database type. Supported: MySQL, PostgreSQL, SQLite")
    }
}

#[derive(Debug, Args)]
pub struct DbCommand {
    #[command(subcommand)]
    pub action: DbAction,
}

#[derive(Debug, Subcommand)]
pub enum DbAction {
    /// Describe table structure
    Describe {
        /// Table name to describe
        table_name: String,

        /// Named connection to use (defaults to primary)
        #[arg(long)]
        connection: Option<String>,

        /// Output format
        #[arg(long, default_value = "table")]
        format: String,
    },

    /// Compare database structure with existing schema
    DiffSchema {
        /// Schema file to compare against
        schema_file: PathBuf,

        /// Named connection to use (defaults to primary)
        #[arg(long)]
        connection: Option<String>,
    },

    /// Export table data
    ExportData {
        /// Table name to export
        table_name: String,

        /// Named connection to use (defaults to primary)
        #[arg(long)]
        connection: Option<String>,

        /// Output format (json, csv)
        #[arg(long, default_value = "json")]
        format: String,

        /// Limit number of rows
        #[arg(long)]
        limit: Option<u32>,

        /// Output file (stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Generate RustF schema from database structure
    GenerateSchema {
        /// Named connection to use (defaults to primary)
        #[arg(long)]
        connection: Option<String>,

        /// Overwrite existing schema files
        #[arg(long)]
        force: bool,

        /// Output directory for schema files
        #[arg(short, long, default_value = "schemas")]
        output: PathBuf,

        /// Only generate schema for specific tables
        #[arg(long)]
        tables: Vec<String>,

        /// Skip emitting the full SQL DDL dump alongside the YAML files.
        /// By default, `db generate-schema` also writes
        /// `<output>/_database_dump.sql` (the raw live-database DDL) so the
        /// canonical DB structure stays in source control. This is distinct
        /// from `schema generate sql`, which writes DDL *generated from the
        /// YAML* to `sql/schema.sql`.
        #[arg(long)]
        no_sql: bool,
    },

    /// Dump the full database schema as SQL DDL to source control.
    ///
    /// Shells out to the native tool for your dialect (`pg_dump`,
    /// `mysqldump`, or `sqlite3 .schema`) with schema-only, no-data
    /// flags. Output is deterministic enough for git diffs.
    DumpSchema {
        /// Named connection to use (defaults to primary)
        #[arg(long)]
        connection: Option<String>,

        /// Output file for the SQL dump
        #[arg(short, long, default_value = "schemas/_database_dump.sql")]
        output: PathBuf,
    },

    /// List all tables in the database
    ListTables {
        /// Named connection to use (defaults to primary)
        #[arg(long)]
        connection: Option<String>,

        /// Output format
        #[arg(long, default_value = "table")]
        format: String,

        /// Include table metadata (row counts, sizes)
        #[arg(long)]
        metadata: bool,
    },

    /// Test database connection
    TestConnection {
        /// Named connection to use (defaults to primary)
        #[arg(long)]
        connection: Option<String>,
    },
}

impl DbCommand {
    pub async fn execute(self, project_path: PathBuf) -> Result<()> {
        match self.action {
            DbAction::Describe {
                table_name,
                connection,
                format,
            } => describe_table(project_path, table_name, connection, format).await,
            DbAction::DiffSchema {
                schema_file,
                connection,
            } => diff_schema(project_path, schema_file, connection).await,
            DbAction::ExportData {
                table_name,
                connection,
                format,
                limit,
                output,
            } => export_data(project_path, table_name, format, output, connection, limit).await,
            DbAction::GenerateSchema {
                connection,
                force,
                output,
                tables,
                no_sql,
            } => generate_schema(project_path, output, connection, force, tables, no_sql).await,
            DbAction::DumpSchema { connection, output } => {
                dump_schema(project_path, output, connection).await
            }
            DbAction::ListTables {
                connection,
                format,
                metadata,
            } => list_tables(project_path, connection, metadata, format).await,
            DbAction::TestConnection { connection } => {
                test_connection(project_path, connection).await
            }
        }
    }
}

/// Load configuration for CLI operations
/// Always loads base config.toml and merges config.dev.toml if present
/// This ensures CLI operates with development settings for each project folder
async fn load_cli_config(project_path: &PathBuf) -> Result<AppConfig> {
    // Load config using the new TOML-level merging approach
    // This automatically handles config.toml + config.dev.toml merging
    let config = AppConfig::load_with_base_dir(project_path)?;
    log::debug!("Configuration loaded and merged successfully");

    Ok(config)
}

/// Get database URL from configuration or environment
/// Priority: 1. --connection arg, 2. DATABASE_URL env var, 3. project config files
async fn get_database_url(project_path: PathBuf, connection: Option<String>) -> Result<String> {
    let config = load_cli_config(&project_path).await?;

    // Priority order: explicit connection arg > DATABASE_URL env > config files
    let database_url = connection.unwrap_or_else(|| {
        std::env::var("DATABASE_URL").unwrap_or_else(|_| config.database.url.unwrap_or_default())
    });

    if database_url.is_empty() {
        anyhow::bail!(
            "No database URL found. Set DATABASE_URL environment variable or configure in config.toml/config.dev.toml"
        );
    }

    Ok(database_url)
}

/// Test database connection
async fn test_connection(project_path: PathBuf, connection: Option<String>) -> Result<()> {
    println!("🔌 Testing database connection...");

    let database_url = get_database_url(project_path, connection).await?;

    // Mask sensitive parts of the database URL for display
    let masked_url = if database_url.contains("://") {
        let parts: Vec<&str> = database_url.split("://").collect();
        if parts.len() == 2 {
            let protocol = parts[0];
            let rest = parts[1];
            if let Some(at_pos) = rest.rfind('@') {
                let host_part = &rest[at_pos..];
                format!("{}://***{}", protocol, host_part)
            } else {
                format!("{}://***", protocol)
            }
        } else {
            "***".to_string()
        }
    } else {
        "***".to_string()
    };

    println!("📍 Connecting to: {}", masked_url);

    let introspector = create_introspector(&database_url).await?;
    let db_name = introspector.get_database_name().await?;

    println!("✅ Connection successful!");
    println!("📊 Database: {}", db_name);

    // Try to get table count
    let tables = introspector.list_tables(false).await?;
    println!("📋 Tables found: {}", tables.len());

    Ok(())
}

/// List all tables in the database
async fn list_tables(
    project_path: PathBuf,
    connection: Option<String>,
    metadata: bool,
    format: String,
) -> Result<()> {
    let database_url = get_database_url(project_path, connection).await?;
    let introspector = create_introspector(&database_url).await?;

    let tables = introspector.list_tables(metadata).await?;

    match format.as_str() {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&tables)?);
        }
        "table" | _ => {
            if tables.is_empty() {
                println!("No tables found in the database.");
            } else {
                println!("📋 Tables in database:\n");
                println!(
                    "{:<30} {:<15} {:<15} {:<15}",
                    "Table Name", "Type", "Rows", "Size (MB)"
                );
                println!("{:-<75}", "");

                for table in tables {
                    let row_count = table
                        .row_count
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| "-".to_string());
                    let size = table
                        .size_bytes
                        .map(|s| format!("{:.2}", s as f64 / 1_048_576.0))
                        .unwrap_or_else(|| "-".to_string());

                    println!(
                        "{:<30} {:<15} {:<15} {:<15}",
                        table.name, table.table_type, row_count, size
                    );
                }
            }
        }
    }

    Ok(())
}

/// Describe table structure
async fn describe_table(
    project_path: PathBuf,
    table_name: String,
    connection: Option<String>,
    format: String,
) -> Result<()> {
    let database_url = get_database_url(project_path, connection).await?;
    let introspector = create_introspector(&database_url).await?;

    let description = introspector.describe_table(&table_name).await?;

    match format.as_str() {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&description)?);
        }
        "table" | _ => {
            println!("📊 Table: {}\n", description.table.name);

            if let Some(comment) = &description.table.comment {
                if !comment.is_empty() {
                    println!("📝 Description: {}\n", comment);
                }
            }

            println!("📋 Columns:");
            println!(
                "{:<25} {:<20} {:<10} {:<10} {:<15}",
                "Column", "Type", "Nullable", "Key", "Default"
            );
            println!("{:-<80}", "");

            for column in &description.columns {
                let key_info = if column.is_primary_key {
                    "PRI"
                } else if column.is_foreign_key {
                    "FK"
                } else if column.is_unique {
                    "UNI"
                } else {
                    ""
                };

                let nullable = if column.is_nullable { "YES" } else { "NO" };
                let default = column.default_value.as_deref().unwrap_or("-");

                println!(
                    "{:<25} {:<20} {:<10} {:<10} {:<15}",
                    column.name, column.data_type, nullable, key_info, default
                );
            }

            if !description.indexes.is_empty() {
                println!("\n📑 Indexes:");
                for index in &description.indexes {
                    let unique = if index.is_unique { "UNIQUE" } else { "" };
                    println!(
                        "  - {} ({}) {}",
                        index.name,
                        index.columns.join(", "),
                        unique
                    );
                }
            }

            if !description.triggers.is_empty() {
                println!("\n⚡ Triggers:");
                for trigger in &description.triggers {
                    println!(
                        "  - {} ({} {} FOR EACH {})",
                        trigger.name, trigger.timing, trigger.event, trigger.for_each
                    );
                }
            }
        }
    }

    Ok(())
}

/// Generate RustF schema from database structure
async fn generate_schema(
    project_path: PathBuf,
    output: PathBuf,
    connection: Option<String>,
    force: bool,
    tables: Vec<String>,
    no_sql: bool,
) -> Result<()> {
    println!("🚀 Generating RustF YAML schemas from database...");
    println!("📂 Project: {:?}", project_path);
    println!("📁 Output: {:?}", output);

    let database_url = get_database_url(project_path.clone(), connection.clone()).await?;

    // Mask sensitive parts for display
    let masked_url = if database_url.contains("://") {
        let parts: Vec<&str> = database_url.split("://").collect();
        if parts.len() == 2 {
            let protocol = parts[0];
            let rest = parts[1];
            if let Some(at_pos) = rest.rfind('@') {
                let host_part = &rest[at_pos..];
                format!("{}://***{}", protocol, host_part)
            } else {
                format!("{}://***", protocol)
            }
        } else {
            "***".to_string()
        }
    } else {
        "***".to_string()
    };

    println!("📍 Database: {}", masked_url);

    // Create backup if forcing overwrite of existing schemas
    if force && output.exists() && !crate::utils::backup::is_empty_directory(&output)? {
        use crate::utils::backup::BackupManager;
        let backup_manager = BackupManager::new()?;
        backup_manager.backup_directory(&output, "schemas")?;
    }

    // Create output directory if it doesn't exist
    if !output.exists() {
        tokio::fs::create_dir_all(&output).await?;
        println!("📁 Created output directory: {:?}", output);
    }

    let introspector = create_introspector(&database_url).await?;
    introspector
        .generate_schemas(&output, force, &tables)
        .await?;

    println!("🎉 Schema generation completed successfully!");

    // Database-first workflow: keep the canonical DDL in source control
    // alongside the YAML so reviewers see real structural diffs and a
    // fresh environment can rebuild the DB from the SQL dump.
    if !no_sql {
        let sql_output = output.join("_database_dump.sql");
        match run_dump_to_file(&database_url, &sql_output).await {
            Ok(()) => {
                println!("🗄️  Dumped DDL to: {:?}", sql_output);
            }
            Err(e) => {
                // Don't fail the whole command — YAML generation already
                // succeeded. Print a clear warning so the user can fix
                // the dump step independently (install tool, etc.).
                eprintln!("⚠️  SQL dump skipped: {}", e);
                eprintln!("    Pass --no-sql to silence this, or run `rustf-cli db dump-schema` separately.");
            }
        }
    }

    Ok(())
}

/// Standalone SQL dump command — same mechanism, no YAML regeneration.
async fn dump_schema(
    project_path: PathBuf,
    output: PathBuf,
    connection: Option<String>,
) -> Result<()> {
    // Relative output paths are resolved against the project dir, not CWD.
    let resolved = if output.is_absolute() {
        output.clone()
    } else {
        project_path.join(&output)
    };
    println!("🗄️  Dumping database DDL to {:?}", resolved);
    let database_url = get_database_url(project_path, connection).await?;
    run_dump_to_file(&database_url, &resolved).await?;
    println!("✅ Dump complete.");
    Ok(())
}

/// Shell out to the dialect-specific native tool and write its stdout
/// to `output`. The parent directory is created if needed. Errors are
/// reported with enough detail to diagnose missing-tool vs connection-
/// refused vs auth-failed.
async fn run_dump_to_file(database_url: &str, output: &Path) -> Result<()> {
    use std::process::Stdio;
    use tokio::io::AsyncWriteExt;

    // Ensure parent exists so writing the file doesn't race a missing dir.
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            tokio::fs::create_dir_all(parent).await?;
        }
    }

    let spec = DumpCommand::from_url(database_url)?;
    log::debug!(
        "Invoking {} with {} args (secrets redacted)",
        spec.program,
        spec.args.len()
    );

    let mut cmd = tokio::process::Command::new(&spec.program);
    cmd.args(&spec.args);
    for (k, v) in &spec.env {
        cmd.env(k, v);
    }
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            anyhow::anyhow!(
                "`{}` not found on PATH. Install it to enable SQL dumps \
                 (brew install {} / apt install {} / equivalent).",
                spec.program,
                spec.install_hint,
                spec.install_hint
            )
        } else {
            anyhow::anyhow!("Failed to spawn `{}`: {}", spec.program, e)
        }
    })?;

    let stdout = child.stdout.take().expect("stdout piped");
    let stderr = child.stderr.take().expect("stderr piped");

    // Stream stdout → file.
    let output_path = output.to_path_buf();
    let write_task = tokio::spawn(async move {
        let mut reader = stdout;
        let mut file = tokio::fs::File::create(&output_path).await?;
        tokio::io::copy(&mut reader, &mut file).await?;
        file.flush().await?;
        Ok::<(), std::io::Error>(())
    });

    // Collect stderr for error messaging.
    let stderr_task = tokio::spawn(async move {
        use tokio::io::AsyncReadExt;
        let mut reader = stderr;
        let mut buf = Vec::new();
        let _ = reader.read_to_end(&mut buf).await;
        buf
    });

    let status = child.wait().await?;
    let stderr_bytes = stderr_task.await.unwrap_or_default();
    write_task.await??;

    // Determine whether anything actually got written — a partial dump
    // beats no dump (mysqldump on DBs with broken views still emits
    // most tables correctly before erroring out).
    let dump_size = tokio::fs::metadata(output)
        .await
        .map(|m| m.len())
        .unwrap_or(0);

    if !status.success() {
        let stderr_text = String::from_utf8_lossy(&stderr_bytes);
        if dump_size == 0 {
            // Nothing useful written — remove the empty file and bail.
            let _ = tokio::fs::remove_file(output).await;
            anyhow::bail!(
                "`{}` exited with status {} without producing output: {}",
                spec.program,
                status.code().map(|c| c.to_string()).unwrap_or_else(|| "signal".into()),
                stderr_text.trim()
            );
        }
        // Partial dump — keep it, warn with the first few stderr lines
        // so the user knows the output is incomplete.
        let stderr_snippet: String = stderr_text
            .lines()
            .take(5)
            .collect::<Vec<_>>()
            .join("\n    ");
        eprintln!(
            "⚠️  `{}` exited with status {} but wrote {} bytes — keeping partial dump.",
            spec.program,
            status.code().map(|c| c.to_string()).unwrap_or_else(|| "signal".into()),
            dump_size
        );
        eprintln!("    First errors:\n    {}", stderr_snippet);
    }

    Ok(())
}

/// Resolved shell-out for a given database URL. Keeps program, args,
/// env vars, and a human-friendly install hint together.
struct DumpCommand {
    program: &'static str,
    args: Vec<String>,
    env: Vec<(&'static str, String)>,
    install_hint: &'static str,
}

impl DumpCommand {
    fn from_url(database_url: &str) -> Result<Self> {
        if database_url.starts_with("mysql://") {
            Self::mysql(database_url)
        } else if database_url.starts_with("postgres://") || database_url.starts_with("postgresql://") {
            Self::postgres(database_url)
        } else if database_url.starts_with("sqlite:") {
            Self::sqlite(database_url)
        } else {
            anyhow::bail!(
                "Unsupported database URL for SQL dump: expected mysql://, postgresql://, or sqlite:"
            )
        }
    }

    fn mysql(database_url: &str) -> Result<Self> {
        let parsed = url::Url::parse(database_url)
            .map_err(|e| anyhow::anyhow!("Invalid MySQL URL: {}", e))?;

        let host = parsed.host_str().unwrap_or("localhost").to_string();
        let port = parsed.port().unwrap_or(3306).to_string();
        let user = parsed.username();
        if user.is_empty() {
            anyhow::bail!("MySQL URL missing user");
        }
        let user = user.to_string();
        let db_name = parsed.path().trim_start_matches('/');
        if db_name.is_empty() {
            anyhow::bail!("MySQL URL missing database name");
        }

        let mut args: Vec<String> = vec![
            "--no-data".into(),
            "--skip-comments".into(),
            "--skip-dump-date".into(),
            "--skip-add-locks".into(),
            // Schema-only dump — no point locking, and locks fail when
            // the DB has views/routines with DEFINER users that no
            // longer exist (MySQL error 1449). Skip locking outright.
            "--skip-lock-tables".into(),
            "--single-transaction".into(),
            "--skip-set-charset".into(),
            "--compact".into(),
            // Continue past errors so broken views / routines don't wipe
            // out an otherwise-useful dump of the tables. A partial
            // dump is flagged with a warning (see run_dump_to_file).
            "--force".into(),
            format!("--host={}", host),
            format!("--port={}", port),
            format!("--user={}", user),
            db_name.to_string(),
        ];
        // Routines + triggers make the dump a faithful full structure.
        args.insert(0, "--routines".into());
        args.insert(0, "--triggers".into());

        // Password via MYSQL_PWD so it doesn't appear in `ps`.
        let mut env: Vec<(&'static str, String)> = Vec::new();
        if let Some(pw) = parsed.password() {
            env.push(("MYSQL_PWD", pw.to_string()));
        }

        Ok(Self {
            program: "mysqldump",
            args,
            env,
            install_hint: "mysql-client",
        })
    }

    fn postgres(database_url: &str) -> Result<Self> {
        // pg_dump accepts the connection URL directly; no parsing needed.
        Ok(Self {
            program: "pg_dump",
            args: vec![
                "--schema-only".into(),
                "--no-owner".into(),
                "--no-privileges".into(),
                "--no-comments".into(),
                database_url.to_string(),
            ],
            env: Vec::new(),
            install_hint: "postgresql-client",
        })
    }

    fn sqlite(database_url: &str) -> Result<Self> {
        // Supported URL shapes: sqlite:PATH  sqlite://PATH  sqlite:///ABSPATH
        let path = database_url
            .trim_start_matches("sqlite:")
            .trim_start_matches("//");
        if path.is_empty() || path == ":memory:" {
            anyhow::bail!("Cannot dump in-memory SQLite database");
        }
        Ok(Self {
            program: "sqlite3",
            args: vec![path.to_string(), ".schema".into()],
            env: Vec::new(),
            install_hint: "sqlite3",
        })
    }
}

/// Compare database structure with existing schema
async fn diff_schema(
    _project_path: PathBuf,
    _schema_file: PathBuf,
    _connection: Option<String>,
) -> Result<()> {
    // TODO: Implement schema diff functionality
    println!("⚠️  Schema diff functionality not yet implemented");
    Ok(())
}

/// Export table data
async fn export_data(
    project_path: PathBuf,
    table_name: String,
    format: String,
    output: Option<PathBuf>,
    connection: Option<String>,
    limit: Option<u32>,
) -> Result<()> {
    let database_url = get_database_url(project_path, connection).await?;
    let introspector = create_introspector(&database_url).await?;

    // Build query
    let query = if let Some(limit) = limit {
        format!("SELECT * FROM {} LIMIT {}", table_name, limit)
    } else {
        format!("SELECT * FROM {}", table_name)
    };

    let data = introspector.export_data(&query, &format).await?;

    // Output to file or stdout
    if let Some(output_file) = output {
        tokio::fs::write(output_file, data).await?;
        println!("✅ Data exported successfully");
    } else {
        println!("{}", data);
    }

    Ok(())
}
