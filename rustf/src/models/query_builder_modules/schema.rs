//! Schema building for RustF query builder
//!
//! This module provides database-agnostic schema building functionality
//! for creating tables and other database structures.

use crate::models::query_builder::dialects::{create_dialect, DatabaseBackend, SqlDialect};

/// Schema builder that generates dialect-specific DDL
pub struct SchemaBuilder {
    dialect: Box<dyn SqlDialect>,
    backend: DatabaseBackend,
}

impl SchemaBuilder {
    pub fn new(backend: DatabaseBackend) -> Self {
        let dialect = create_dialect(backend);
        SchemaBuilder { dialect, backend }
    }

    /// Create a CREATE TABLE statement
    pub fn create_table(&self, table_name: &str) -> CreateTableBuilder {
        CreateTableBuilder {
            dialect: &self.dialect,
            backend: self.backend,
            table_name: table_name.to_string(),
            columns: Vec::new(),
            constraints: Vec::new(),
        }
    }
}

pub struct CreateTableBuilder<'a> {
    dialect: &'a Box<dyn SqlDialect>,
    backend: DatabaseBackend,
    table_name: String,
    columns: Vec<ColumnDefinition>,
    constraints: Vec<String>,
}

#[derive(Clone)]
struct ColumnDefinition {
    name: String,
    data_type: String,
    nullable: bool,
    default: Option<String>,
    constraints: Vec<String>,
}

impl<'a> CreateTableBuilder<'a> {
    /// Add an auto-incrementing ID column
    pub fn id(mut self) -> Self {
        let data_type = match self.backend {
            DatabaseBackend::Postgres => "SERIAL PRIMARY KEY",
            DatabaseBackend::MySQL | DatabaseBackend::MariaDB => "INT AUTO_INCREMENT PRIMARY KEY",
            DatabaseBackend::SQLite => "INTEGER PRIMARY KEY AUTOINCREMENT",
        };

        self.columns.push(ColumnDefinition {
            name: "id".to_string(),
            data_type: data_type.to_string(),
            nullable: false,
            default: None,
            constraints: Vec::new(),
        });
        self
    }

    /// Add a string column
    pub fn string(mut self, name: &str, max_length: Option<usize>) -> Self {
        let data_type = match (self.backend, max_length) {
            (DatabaseBackend::Postgres, Some(n)) if n <= 255 => format!("VARCHAR({})", n),
            (DatabaseBackend::Postgres, _) => "TEXT".to_string(),
            (DatabaseBackend::MySQL | DatabaseBackend::MariaDB, Some(n)) if n <= 255 => {
                format!("VARCHAR({})", n)
            }
            (DatabaseBackend::MySQL | DatabaseBackend::MariaDB, Some(n)) if n <= 65535 => {
                "TEXT".to_string()
            }
            (DatabaseBackend::MySQL | DatabaseBackend::MariaDB, _) => "LONGTEXT".to_string(),
            (DatabaseBackend::SQLite, _) => "TEXT".to_string(),
        };

        self.columns.push(ColumnDefinition {
            name: name.to_string(),
            data_type,
            nullable: true,
            default: None,
            constraints: Vec::new(),
        });
        self
    }

    /// Add a boolean column
    pub fn boolean(mut self, name: &str) -> Self {
        let data_type = self.dialect.boolean_type().to_string();

        self.columns.push(ColumnDefinition {
            name: name.to_string(),
            data_type,
            nullable: false,
            default: Some("FALSE".to_string()),
            constraints: Vec::new(),
        });
        self
    }

    /// Build the CREATE TABLE statement
    pub fn build(&self) -> String {
        let mut sql = format!(
            "CREATE TABLE {} (\n",
            self.dialect.quote_identifier(&self.table_name)
        );

        let column_defs: Vec<String> = self
            .columns
            .iter()
            .map(|col| {
                let mut def = format!(
                    "  {} {}",
                    self.dialect.quote_identifier(&col.name),
                    col.data_type
                );

                if !col.nullable {
                    def.push_str(" NOT NULL");
                }

                if let Some(default) = &col.default {
                    def.push_str(&format!(" DEFAULT {}", default));
                }

                for constraint in &col.constraints {
                    def.push(' ');
                    def.push_str(constraint);
                }

                def
            })
            .collect();

        sql.push_str(&column_defs.join(",\n"));

        if !self.constraints.is_empty() {
            sql.push_str(",\n  ");
            sql.push_str(&self.constraints.join(",\n  "));
        }

        sql.push_str("\n)");
        sql
    }
}
