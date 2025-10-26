//! Schema management commands for RustF CLI - Modular orchestrator
//! 
//! This module orchestrates database-specific schema generation modules

pub mod postgres;
pub mod mysql;
pub mod sqlite;

use crate::analyzer::OutputFormat;
use clap::{Args, Subcommand};
use rustf_schema::Schema;
use std::path::{Path, PathBuf};

/// Schema management commands
#[derive(Debug, Args)]
pub struct SchemaCommand {
    #[command(subcommand)]
    pub action: SchemaAction,
}

/// Schema actions
#[derive(Debug, Subcommand)]
pub enum SchemaAction {
    /// Analyze schema structure and relationships
    Analyze {
        /// Output format
        #[arg(short, long, value_enum, default_value = "table")]
        format: OutputFormat,
        
        /// Path to schema directory
        #[arg(short = 's', long, default_value = "schemas")]
        path: PathBuf,
    },
    
    /// Check consistency between schema and generated code
    CheckConsistency {
        /// Path to generated models directory
        #[arg(short = 'm', long, default_value = "src/models")]
        models_path: PathBuf,
        
        /// Path to schema directory
        #[arg(short = 'c', long, default_value = "schemas")]
        schema_path: PathBuf,
    },
    
    /// Generate code from schema
    Generate {
        /// What to generate
        #[command(subcommand)]
        target: GenerateTarget,
    },
    
    /// Validate schema files for consistency
    Validate {
        /// Also validate against generated code
        #[arg(long)]
        check_generated: bool,
        
        /// Path to schema directory
        #[arg(short, long, default_value = "schemas")]
        path: PathBuf,
    },
    
    /// Watch schema files for changes and auto-regenerate
    Watch {
        /// Auto-generate models on changes
        #[arg(long)]
        auto_generate: bool,
        
        /// Path to schema directory
        #[arg(short = 'w', long, default_value = "schemas")]
        path: PathBuf,
    },
}

/// Code generation targets
#[derive(Debug, Subcommand, Clone)]
pub enum GenerateTarget {
    /// Generate SQL migrations
    Migrations {
        /// Output directory for migrations
        #[arg(short = 'o', long, default_value = "migrations")]
        output: PathBuf,
        
        /// Path to schema directory
        #[arg(short = 's', long, default_value = "schemas")]
        schema_path: PathBuf,
    },
    
    /// Generate Rust model structs
    Models {
        /// Overwrite existing files
        #[arg(long)]
        force: bool,
        
        /// Output directory for generated models
        #[arg(short, long, default_value = "src/models")]
        output: PathBuf,
        
        /// Path to schema directory
        #[arg(short = 's', long, default_value = "schemas")]
        schema_path: PathBuf,
        
        /// Generate only specific tables (comma-separated)
        #[arg(short = 't', long, value_delimiter = ',')]
        tables: Option<Vec<String>>,
        
        /// Exclude specific tables from generation (comma-separated)
        #[arg(short = 'e', long, value_delimiter = ',')]
        exclude: Option<Vec<String>>,
    },
}

impl SchemaCommand {
    /// Execute the schema command
    pub async fn execute(self) -> anyhow::Result<()> {
        // First, detect the database backend if we're doing generation
        let backend = if let SchemaAction::Generate { ref target } = self.action {
            match target {
                GenerateTarget::Migrations { schema_path, .. } |
                GenerateTarget::Models { schema_path, .. } => {
                    detect_database_backend_from_path(schema_path).await?
                }
            }
        } else if let SchemaAction::Validate { ref path, .. } = self.action {
            detect_database_backend_from_path(path).await?
        } else {
            // Default to PostgreSQL for non-generation commands
            "Postgres".to_string()
        };

        // Route to the appropriate database-specific module
        match backend.as_str() {
            "Postgres" | "PostgreSQL" => {
                postgres::execute_schema_command(self).await
            }
            "MySql" | "MySQL" => {
                mysql::execute_schema_command(self).await
            }
            "Sqlite" | "SQLite" => {
                sqlite::execute_schema_command(self).await
            }
            _ => {
                // Default to PostgreSQL
                postgres::execute_schema_command(self).await
            }
        }
    }
}

/// Detect database backend from schema path
async fn detect_database_backend_from_path(schema_path: &Path) -> anyhow::Result<String> {
    // Try to load the schema to detect the backend
    let schema = Schema::load_from_directory(schema_path).await?;
    Ok(detect_database_backend(&schema))
}

/// Helper function to determine database backend from schema
fn detect_database_backend(schema: &Schema) -> String {
    if let Some(meta) = &schema.meta {
        // Check description field for database type hints
        if let Some(desc) = &meta.description {
            if desc.contains("MySQL") || desc.contains("mysql") {
                return "MySql".to_string();
            } else if desc.contains("SQLite") || desc.contains("sqlite") {
                return "Sqlite".to_string();
            } else if desc.contains("PostgreSQL") || desc.contains("postgres") {
                return "Postgres".to_string();
            }
        }
        
        // Note: database_type field type varies between rustf-schema versions
        // We'll just comment this out for now and rely on description field
        /*
        if let Some(db_type) = &meta.database_type {
            return match db_type.as_str() {
                "mysql" => "MySql".to_string(),
                "postgres" | "postgresql" => "Postgres".to_string(),
                "sqlite" => "Sqlite".to_string(),
                _ => db_type.clone(),
            };
        }
        */
    }
    
    // Default to PostgreSQL
    "Postgres".to_string()
}

/// Shared utility: Escape Rust reserved keywords
pub fn escape_rust_keyword(field_name: &str) -> String {
    const RUST_KEYWORDS: &[&str] = &[
        "type", "match", "if", "else", "while", "for", "loop", "fn", "let", "mut", 
        "const", "static", "struct", "enum", "trait", "impl", "mod", "use", "pub",
        "return", "break", "continue", "true", "false", "self", "Self", "super",
        "crate", "in", "as", "where", "async", "await", "dyn", "move", "ref",
        "macro", "union", "unsafe", "extern", "yield", "try", "catch", "typeof"
    ];
    
    if RUST_KEYWORDS.contains(&field_name) {
        format!("r#{}", field_name)
    } else {
        field_name.to_string()
    }
}

/// Convert string to PascalCase
pub fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect()
}

/// Convert string to snake_case
pub fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut prev_is_upper = false;
    
    for (i, ch) in s.chars().enumerate() {
        if ch == '_' {
            result.push(ch);
            prev_is_upper = false;
        } else if ch.is_uppercase() {
            if i > 0 && !prev_is_upper {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().unwrap());
            prev_is_upper = true;
        } else {
            result.push(ch);
            prev_is_upper = false;
        }
    }
    
    result
}