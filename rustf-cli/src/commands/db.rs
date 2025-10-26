//! Database introspection and schema generation commands

mod common;
mod mysql;
mod postgres;
mod sqlite;

use anyhow::Result;
use clap::{Args, Subcommand};
use std::path::PathBuf;
use rustf::config::AppConfig;

// Re-export common types
pub use common::*;
use self::{mysql::MySqlIntrospector, postgres::PostgresIntrospector, sqlite::SqliteIntrospector};
use async_trait::async_trait;

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
            DbAction::Describe { table_name, connection, format } => {
                describe_table(project_path, table_name, connection, format).await
            }
            DbAction::DiffSchema { schema_file, connection } => {
                diff_schema(project_path, schema_file, connection).await
            }
            DbAction::ExportData { table_name, connection, format, limit, output } => {
                export_data(project_path, table_name, format, output, connection, limit).await
            }
            DbAction::GenerateSchema { connection, force, output, tables } => {
                generate_schema(project_path, output, connection, force, tables).await
            }
            DbAction::ListTables { connection, format, metadata } => {
                list_tables(project_path, connection, metadata, format).await
            }
            DbAction::TestConnection { connection } => {
                test_connection(project_path, connection).await
            }
        }
    }
}

/// Get database URL from configuration or environment
async fn get_database_url(project_path: PathBuf, connection: Option<String>) -> Result<String> {
    let config_file = project_path.join("config.toml");
    let config = if config_file.exists() {
        AppConfig::from_file(config_file)?
    } else {
        AppConfig::from_env()?
    };
    
    let database_url = connection.unwrap_or_else(|| {
        std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| config.database.url.unwrap_or_default())
    });
    
    if database_url.is_empty() {
        anyhow::bail!("No database URL found. Set DATABASE_URL or configure in config.toml");
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
    format: String
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
                println!("{:<30} {:<15} {:<15} {:<15}", "Table Name", "Type", "Rows", "Size (MB)");
                println!("{:-<75}", "");
                
                for table in tables {
                    let row_count = table.row_count
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| "-".to_string());
                    let size = table.size_bytes
                        .map(|s| format!("{:.2}", s as f64 / 1_048_576.0))
                        .unwrap_or_else(|| "-".to_string());
                    
                    println!("{:<30} {:<15} {:<15} {:<15}", 
                        table.name, 
                        table.table_type,
                        row_count,
                        size
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
    format: String
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
            println!("{:<25} {:<20} {:<10} {:<10} {:<15}", 
                "Column", "Type", "Nullable", "Key", "Default");
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
                
                println!("{:<25} {:<20} {:<10} {:<10} {:<15}",
                    column.name,
                    column.data_type,
                    nullable,
                    key_info,
                    default
                );
            }
            
            if !description.indexes.is_empty() {
                println!("\n📑 Indexes:");
                for index in &description.indexes {
                    let unique = if index.is_unique { "UNIQUE" } else { "" };
                    println!("  - {} ({}) {}", 
                        index.name, 
                        index.columns.join(", "),
                        unique
                    );
                }
            }
            
            if !description.triggers.is_empty() {
                println!("\n⚡ Triggers:");
                for trigger in &description.triggers {
                    println!("  - {} ({} {} FOR EACH {})", 
                        trigger.name,
                        trigger.timing,
                        trigger.event,
                        trigger.for_each
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
    tables: Vec<String>
) -> Result<()> {
    println!("🚀 Generating RustF YAML schemas from database...");
    println!("📂 Project: {:?}", project_path);
    println!("📁 Output: {:?}", output);
    
    let database_url = get_database_url(project_path, connection).await?;
    
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
    introspector.generate_schemas(&output, force, &tables).await?;
    
    println!("🎉 Schema generation completed successfully!");
    Ok(())
}

/// Compare database structure with existing schema
async fn diff_schema(
    _project_path: PathBuf,
    _schema_file: PathBuf,
    _connection: Option<String>
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
    limit: Option<u32>
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