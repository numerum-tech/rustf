//! Template definitions for code generation
//! 
//! This module contains the actual template strings used for generating
//! various types of code from RustF schemas.

/// SQLx model template
pub const SQLX_MODEL_TEMPLATE: &str = include_str!("templates/sqlx_model.hbs");

/// SQLx CRUD operations template  
pub const SQLX_CRUD_TEMPLATE: &str = include_str!("templates/sqlx_crud.hbs");

/// SQLx relations template
pub const SQLX_RELATIONS_TEMPLATE: &str = include_str!("templates/sqlx_relations.hbs");

/// SQL migration template
pub const SQL_MIGRATION_TEMPLATE: &str = include_str!("templates/sql_migration.hbs");