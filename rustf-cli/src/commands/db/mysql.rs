//! MySQL database introspection implementation

use super::{common::*, DatabaseIntrospector};
use anyhow::Result;
use async_trait::async_trait;
use sqlx::{MySql, Pool, Row};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;

pub struct MySqlIntrospector {
    pool: Pool<MySql>,
    db_name: String,
}

impl MySqlIntrospector {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = sqlx::mysql::MySqlPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;
            
        // Get database name
        let db_name_row = sqlx::query("SELECT DATABASE() as db_name")
            .fetch_one(&pool)
            .await?;
        let db_name: String = db_name_row.try_get("db_name")?;
        
        Ok(Self { pool, db_name })
    }
    
    /// Parse enum values from MySQL COLUMN_TYPE field
    /// Example: "enum('male','female','other')" -> vec!["male", "female", "other"]
    fn parse_enum_values(column_type: &str) -> Vec<String> {
        if !column_type.starts_with("enum(") || !column_type.ends_with(")") {
            return Vec::new();
        }
        
        let values_part = &column_type[5..column_type.len()-1]; // Remove "enum(" and ")"
        let mut values = Vec::new();
        let mut current_value = String::new();
        let mut in_quotes = false;
        let mut escape_next = false;
        
        for ch in values_part.chars() {
            if escape_next {
                current_value.push(ch);
                escape_next = false;
            } else if ch == '\\' {
                escape_next = true;
            } else if ch == '\'' {
                if in_quotes {
                    // End of quoted value
                    values.push(current_value.clone());
                    current_value.clear();
                    in_quotes = false;
                } else {
                    // Start of quoted value
                    in_quotes = true;
                }
            } else if in_quotes {
                current_value.push(ch);
            }
            // Ignore commas and spaces outside quotes
        }
        
        values
    }
    
    fn generate_field_yaml(&self, column: &ColumnInfo) -> String {
        let mut field = String::new();
        let escaped_field_name = escape_yaml_field_name(&column.name);
        field.push_str(&format!("    {}:\n", escaped_field_name));
        
        // Extract enum values if this is an enum type
        let enum_values = if let Some(column_type) = &column.column_type {
            Self::parse_enum_values(column_type)
        } else {
            Vec::new()
        };
        
        // Handle enum types
        if !enum_values.is_empty() {
            field.push_str("      type: enum\n");
            field.push_str("      values:\n");
            for value in &enum_values {
                field.push_str(&format!("        - \"{}\"\n", value));
            }
            // Set default to first enum value if no default specified
            if column.default_value.is_none() {
                if let Some(first_value) = enum_values.first() {
                    field.push_str(&format!("      default: \"{}\"\n", first_value));
                }
            } else if let Some(default) = &column.default_value {
                let clean_default = default.trim_matches('\'').trim_matches('"');
                field.push_str(&format!("      default: \"{}\"\n", clean_default));
            }
        } else {
            // Regular data type
            let data_type = if column.data_type.contains("varchar") || column.data_type.contains("char") {
                if let Some(max_length) = column.max_length {
                    format!("string({})", max_length)
                } else {
                    "string".to_string()
                }
            } else if column.data_type == "text" {
                "text".to_string()  // Don't parameterize text type
            } else if column.data_type.contains("decimal") {
                if let (Some(precision), Some(scale)) = (column.numeric_precision, column.numeric_scale) {
                    format!("decimal({},{})", precision, scale)
                } else {
                    "decimal".to_string()
                }
            } else {
                column.data_type.clone()
            };
            field.push_str(&format!("      type: {}\n", data_type));
            
            // Add default value or auto-increment flag
            if let Some(default) = &column.default_value {
                if default == "AUTO_INCREMENT" {
                    field.push_str("      auto: true\n");
                } else if !default.is_empty() && default != "NULL" {
                    field.push_str(&format!("      default: {}\n", default));
                }
            }
        }
        
        // Add lang_type
        let lang_type = self.map_to_rust_type(&column.data_type, column.is_nullable);
        field.push_str(&format!("      lang_type: {}\n", lang_type));
        
        // Add nullable flag
        if column.is_nullable {
            field.push_str("      nullable: true\n");
        } else {
            field.push_str("      required: true\n");
        }
        
        // Add primary key flag
        if column.is_primary_key {
            field.push_str("      primary_key: true\n");
        }
        
        // Add unique flag
        if column.is_unique {
            field.push_str("      unique: true\n");
        }
        
        // Add AI hint
        let ai_hint = generate_field_ai_hint(&column.name, &column.data_type, column.is_foreign_key);
        field.push_str(&format!("      ai: \"{}\"\n", ai_hint));
        
        // Add foreign key reference
        if column.is_foreign_key {
            if let (Some(table), Some(col)) = (&column.foreign_table, &column.foreign_column) {
                field.push_str(&format!("      foreign_key: {}.{}\n", table, col));
            }
        }
        
        // Add column comment if available
        if let Some(comment) = &column.comment {
            if !comment.is_empty() {
                field.push_str(&format!("      column_comment: \"{}\"\n", comment));
            }
        }
        
        field.push_str("\n");
        field
    }
    
    fn map_to_rust_type(&self, data_type: &str, is_nullable: bool) -> String {
        let base_type = match data_type {
            "int" | "integer" => "i32",
            "bigint" => "i64",
            "smallint" => "i16",
            "tinyint" => "i8",
            "mediumint" => "i32",
            "float" => "f32",
            "double" | "real" => "f64",
            "decimal" | "numeric" => "Decimal",
            "boolean" | "bool" => "bool",
            "varchar" | "char" | "text" | "longtext" | "mediumtext" | "tinytext" => "String",
            "date" => "NaiveDate",
            "time" => "NaiveTime",
            "datetime" | "timestamp" => "DateTime<Utc>",
            "json" => "serde_json::Value",
            "enum" => "String",
            _ => "String",
        };
        
        if is_nullable {
            format!("Option<{}>", base_type)
        } else {
            base_type.to_string()
        }
    }
    
    fn generate_table_schema_yaml(&self, description: &TableDescription) -> Result<String> {
        let mut yaml = String::new();
        
        // Add header comment
        yaml.push_str(&format!("# {} entity - Generated from database\n\n", description.table.name));
        
        // Model name (convert table name to PascalCase)
        let model_name = to_pascal_case(&description.table.name);
        yaml.push_str(&format!("{}:\n", model_name));
        yaml.push_str(&format!("  table: {}\n", description.table.name));
        yaml.push_str(&format!("  database_type: mysql\n"));
        yaml.push_str(&format!("  database_name: {}\n", self.db_name));
        
        // Determine element type (table or view)
        let element_type = if description.table.table_type.to_lowercase().contains("view") {
            "view"
        } else {
            "table"
        };
        yaml.push_str(&format!("  element_type: {}\n", element_type));
        yaml.push_str("  version: 1\n");
        
        // Add description if available
        if let Some(comment) = &description.table.comment {
            if !comment.is_empty() {
                yaml.push_str(&format!("  description: \"{}\"\n", comment));
            }
        }
        
        // Add AI context based on table name
        let ai_context = generate_table_ai_context(&description.table.name);
        yaml.push_str(&format!("  ai_context: \"{}\"\n", ai_context));
        
        yaml.push_str("  \n  fields:\n");
        
        // Generate fields
        for column in &description.columns {
            yaml.push_str(&self.generate_field_yaml(column));
        }
        
        // Generate relations if foreign keys exist
        let relations = generate_relations_yaml(&description.columns);
        if !relations.is_empty() {
            yaml.push_str("  \n  relations:\n");
            yaml.push_str(&relations);
        }
        
        // Generate indexes
        if !description.indexes.is_empty() {
            yaml.push_str("  \n  indexes:\n");
            for index in &description.indexes {
                if index.columns.len() == 1 {
                    yaml.push_str(&format!("    - {}\n", index.columns[0]));
                } else {
                    yaml.push_str(&format!("    - [{}]\n", index.columns.join(", ")));
                }
            }
        }
        
        // Generate triggers
        if !description.triggers.is_empty() {
            yaml.push_str("  \n  triggers:\n");
            for trigger in &description.triggers {
                yaml.push_str(&format!("    - name: {}\n", trigger.name));
                yaml.push_str(&format!("      event: {}\n", trigger.event));
                yaml.push_str(&format!("      timing: {}\n", trigger.timing));
                yaml.push_str(&format!("      for_each: {}\n", trigger.for_each));
                yaml.push_str(&format!("      ai: \"Database trigger - executes automatically on table changes. Consider application logic implications.\"\n"));
                yaml.push_str("\n");
            }
        }
        
        yaml.push_str("\n");
        Ok(yaml)
    }
}

#[async_trait]
impl DatabaseIntrospector for MySqlIntrospector {
    async fn list_tables(&self, metadata: bool) -> Result<Vec<TableInfo>> {
        let query = if metadata {
            r#"
            SELECT 
                t.TABLE_NAME as table_name,
                t.TABLE_SCHEMA as table_schema,
                CAST(t.TABLE_TYPE AS CHAR) as table_type,
                t.TABLE_ROWS as row_count,
                (t.DATA_LENGTH + t.INDEX_LENGTH) as size_bytes,
                CAST(t.TABLE_COMMENT AS CHAR) as comment
            FROM information_schema.tables t
            WHERE t.TABLE_SCHEMA = ?
            ORDER BY t.TABLE_NAME
            "#
        } else {
            r#"
            SELECT 
                TABLE_NAME as table_name,
                TABLE_SCHEMA as table_schema,
                CAST(TABLE_TYPE AS CHAR) as table_type,
                NULL as row_count,
                NULL as size_bytes,
                NULL as comment
            FROM information_schema.tables
            WHERE TABLE_SCHEMA = ?
            ORDER BY TABLE_NAME
            "#
        };
        
        let rows = sqlx::query(query)
            .bind(&self.db_name)
            .fetch_all(&self.pool)
            .await?;
            
        let mut tables = Vec::new();
        for row in rows {
            tables.push(TableInfo {
                name: row.try_get("table_name")?,
                schema: row.try_get("table_schema").ok(),
                table_type: row.try_get("table_type")?,
                row_count: row.try_get("row_count").ok().flatten(),
                size_bytes: row.try_get("size_bytes").ok().flatten(),
                comment: row.try_get("comment").ok().flatten(),
            });
        }
        
        Ok(tables)
    }
    
    async fn describe_table(&self, table_name: &str) -> Result<TableDescription> {
        // Get table info
        let table_row = sqlx::query(
            r#"
            SELECT 
                TABLE_NAME as table_name,
                TABLE_SCHEMA as table_schema,
                CAST(TABLE_TYPE AS CHAR) as table_type,
                TABLE_ROWS as row_count,
                (DATA_LENGTH + INDEX_LENGTH) as size_bytes,
                CAST(TABLE_COMMENT AS CHAR) as comment
            FROM information_schema.tables
            WHERE TABLE_NAME = ? AND TABLE_SCHEMA = ?
            "#
        )
        .bind(table_name)
        .bind(&self.db_name)
        .fetch_optional(&self.pool)
        .await?;
        
        let table_info = match table_row {
            Some(row) => TableInfo {
                name: row.try_get("table_name")?,
                schema: row.try_get("table_schema").ok(),
                table_type: row.try_get("table_type")?,
                row_count: row.try_get("row_count").ok().flatten(),
                size_bytes: row.try_get("size_bytes").ok().flatten(),
                comment: row.try_get("comment").ok().flatten(),
            },
            None => anyhow::bail!("Table '{}' not found", table_name),
        };
        
        // Get column information
        let column_rows = sqlx::query(
            r#"
            SELECT 
                c.COLUMN_NAME as column_name,
                CAST(c.DATA_TYPE AS CHAR) as data_type,
                CAST(c.COLUMN_TYPE AS CHAR) as column_type,
                c.IS_NULLABLE as is_nullable,
                c.COLUMN_DEFAULT as column_default,
                c.CHARACTER_MAXIMUM_LENGTH as character_maximum_length,
                c.NUMERIC_PRECISION as numeric_precision,
                c.NUMERIC_SCALE as numeric_scale,
                CAST(c.COLUMN_COMMENT AS CHAR) as column_comment,
                CAST(c.COLUMN_KEY AS CHAR) as column_key,
                CAST(c.EXTRA AS CHAR) as extra
            FROM information_schema.columns c
            WHERE c.TABLE_NAME = ? AND c.TABLE_SCHEMA = ?
            ORDER BY c.ORDINAL_POSITION
            "#
        )
        .bind(table_name)
        .bind(&self.db_name)
        .fetch_all(&self.pool)
        .await?;
        
        // Get foreign key information
        let fk_rows = sqlx::query(
            r#"
            SELECT 
                kcu.COLUMN_NAME as column_name,
                kcu.REFERENCED_TABLE_NAME as referenced_table_name,
                kcu.REFERENCED_COLUMN_NAME as referenced_column_name,
                rc.DELETE_RULE as on_delete,
                rc.UPDATE_RULE as on_update
            FROM information_schema.key_column_usage kcu
            LEFT JOIN information_schema.REFERENTIAL_CONSTRAINTS rc
                ON rc.CONSTRAINT_NAME = kcu.CONSTRAINT_NAME
                AND rc.CONSTRAINT_SCHEMA = kcu.TABLE_SCHEMA
            WHERE kcu.TABLE_NAME = ? 
                AND kcu.TABLE_SCHEMA = ?
                AND kcu.REFERENCED_TABLE_NAME IS NOT NULL
            "#
        )
        .bind(table_name)
        .bind(&self.db_name)
        .fetch_all(&self.pool)
        .await?;
        
        // Build foreign key lookup
        let mut foreign_keys = HashMap::new();
        for fk_row in fk_rows {
            let column_name: String = fk_row.try_get("column_name")?;
            let referenced_table: String = fk_row.try_get("referenced_table_name")?;
            let referenced_column: String = fk_row.try_get("referenced_column_name")?;
            let on_delete: Option<String> = fk_row.try_get("on_delete").ok();
            let on_update: Option<String> = fk_row.try_get("on_update").ok();
            foreign_keys.insert(column_name, (referenced_table, referenced_column, on_delete, on_update));
        }
        
        let mut columns = Vec::new();
        for row in column_rows {
            let column_name: String = row.try_get("column_name")?;
            let column_key: String = row.try_get("column_key").unwrap_or_default();
            
            // Check for auto_increment in the EXTRA column
            let extra: Option<String> = row.try_get("extra").ok();
            let is_auto_increment = extra.as_ref().map_or(false, |s| s.contains("auto_increment"));
            
            let is_primary_key = column_key == "PRI";
            let is_unique = column_key == "UNI";
            let is_foreign_key = foreign_keys.contains_key(&column_name);
            
            let (foreign_table, foreign_column, on_delete, on_update) = if is_foreign_key {
                let (table, column, del, upd) = foreign_keys.get(&column_name).unwrap();
                (Some(table.clone()), Some(column.clone()), del.clone(), upd.clone())
            } else {
                (None, None, None, None)
            };
            
            // Mark auto-increment fields with a special default value marker
            let default_value = if is_auto_increment {
                Some("AUTO_INCREMENT".to_string())
            } else {
                row.try_get("column_default").ok()
            };
            
            columns.push(ColumnInfo {
                name: column_name,
                data_type: row.try_get("data_type")?,
                column_type: row.try_get("column_type").ok(),
                postgres_type_name: None, // MySQL doesn't use PostgreSQL type names
                is_nullable: row.try_get::<&str, _>("is_nullable")? == "YES",
                default_value,
                is_primary_key,
                is_unique,
                is_foreign_key,
                foreign_table,
                foreign_column,
                on_delete,
                on_update,
                comment: row.try_get("column_comment").ok(),
                max_length: row.try_get("character_maximum_length").ok(),
                numeric_precision: row.try_get("numeric_precision").ok(),
                numeric_scale: row.try_get("numeric_scale").ok(),
            });
        }
        
        // Get indexes
        let index_rows = sqlx::query(
            r#"
            SELECT 
                s.INDEX_NAME as index_name,
                s.COLUMN_NAME as column_name,
                s.NON_UNIQUE as non_unique,
                s.SEQ_IN_INDEX as seq_in_index
            FROM information_schema.statistics s
            WHERE s.TABLE_NAME = ? AND s.TABLE_SCHEMA = ?
            ORDER BY s.INDEX_NAME, s.SEQ_IN_INDEX
            "#
        )
        .bind(table_name)
        .bind(&self.db_name)
        .fetch_all(&self.pool)
        .await?;
        
        // Group indexes by name
        let mut index_map: HashMap<String, IndexInfo> = HashMap::new();
        for row in index_rows {
            let index_name: String = row.try_get("index_name")?;
            let column_name: String = row.try_get("column_name")?;
            let non_unique: i32 = row.try_get("non_unique")?;
            
            index_map.entry(index_name.clone())
                .or_insert_with(|| IndexInfo {
                    name: index_name.clone(),
                    columns: Vec::new(),
                    is_unique: non_unique == 0,
                    is_primary: index_name == "PRIMARY",
                })
                .columns.push(column_name);
        }
        let indexes: Vec<IndexInfo> = index_map.into_values().collect();
        
        // Get triggers
        let trigger_rows = sqlx::query(
            r#"
            SELECT 
                TRIGGER_NAME as trigger_name,
                EVENT_MANIPULATION as event,
                ACTION_TIMING as timing,
                ACTION_ORIENTATION as for_each,
                ACTION_CONDITION as condition_expr,
                ACTION_STATEMENT as body
            FROM information_schema.TRIGGERS
            WHERE EVENT_OBJECT_TABLE = ? AND EVENT_OBJECT_SCHEMA = ?
            "#
        )
        .bind(table_name)
        .bind(&self.db_name)
        .fetch_all(&self.pool)
        .await?;
        
        let mut triggers = Vec::new();
        for row in trigger_rows {
            triggers.push(TriggerInfo {
                name: row.try_get("trigger_name")?,
                event: row.try_get("event").unwrap_or_default(),
                timing: row.try_get("timing").unwrap_or_default(),
                for_each: row.try_get("for_each").unwrap_or_default(),
                condition: row.try_get("condition_expr").ok(),
                body: row.try_get("body").ok(),
                description: None,
            });
        }
        
        Ok(TableDescription {
            table: table_info,
            columns,
            indexes,
            constraints: Vec::new(), // MySQL doesn't expose check constraints easily
            triggers,
        })
    }
    
    async fn generate_schemas(
        &self,
        output_dir: &PathBuf,
        force: bool,
        filter_tables: &[String],
    ) -> Result<()> {
        // Create output directory if it doesn't exist
        if !output_dir.exists() {
            fs::create_dir_all(output_dir).await?;
        }
        
        let tables = self.list_tables(true).await?;
        println!("📋 Found {} tables", tables.len());
        
        let mut generated_count = 0;
        let mut skipped_count = 0;
        
        for table in tables {
            // Filter tables if specified
            if !filter_tables.is_empty() && !filter_tables.contains(&table.name) {
                continue;
            }
            
            let schema_file = output_dir.join(format!("{}.yaml", table.name));
            
            // Skip if file exists and force is false
            if schema_file.exists() && !force {
                println!("⚠️  Skipping existing schema: {}", schema_file.display());
                skipped_count += 1;
                continue;
            }
            
            // Get detailed table description
            let description = self.describe_table(&table.name).await?;
            
            // Generate YAML schema
            let yaml_content = self.generate_table_schema_yaml(&description)?;
            
            // Write schema file
            fs::write(&schema_file, yaml_content).await?;
            println!("✅ Generated schema: {}", schema_file.display());
            generated_count += 1;
        }
        
        // Generate _meta.yaml
        let meta_file = output_dir.join("_meta.yaml");
        if !meta_file.exists() || force {
            let meta_content = self.generate_meta_yaml().await?;
            fs::write(&meta_file, meta_content).await?;
            println!("✅ Generated metadata: {}", meta_file.display());
        }
        
        println!("📊 Generated {} schemas, skipped {}", generated_count, skipped_count);
        Ok(())
    }
    
    async fn generate_meta_yaml(&self) -> Result<String> {
        let mut yaml = String::new();
        yaml.push_str("# Global schema configuration - Generated from MySQL database\n\n");
        yaml.push_str("version: \"1.0\"\n");
        yaml.push_str("database_type: mysql\n");
        yaml.push_str(&format!("database_name: {}\n", self.db_name));
        yaml.push_str(&format!("description: \"Schema generated from MySQL database '{}'\"\n", self.db_name));
        yaml.push_str("ai_context: \"Generated schemas from existing MySQL database structure with intelligent field mapping\"\n");
        yaml.push_str("\n");
        yaml.push_str("# Global field defaults\n");
        yaml.push_str("field_defaults:\n");
        yaml.push_str("  string:\n");
        yaml.push_str("    max_length: 255\n");
        yaml.push_str("    charset: utf8mb4\n");
        yaml.push_str("    \n");
        yaml.push_str("  timestamp:\n");
        yaml.push_str("    default: now\n");
        yaml.push_str("    on_update: now\n");
        yaml.push_str("    \n");
        yaml.push_str("# Code generation settings\n");
        yaml.push_str("generation:\n");
        yaml.push_str("  base_class: \"BaseModel\"\n");
        yaml.push_str("  use_traits: [HasTimestamps]\n");
        yaml.push_str("\n");
        
        Ok(yaml)
    }
    
    async fn get_database_name(&self) -> Result<String> {
        Ok(self.db_name.clone())
    }
    
    async fn export_data(&self, query: &str, format: &str) -> Result<String> {
        let _rows = sqlx::query(query).fetch_all(&self.pool).await?;
        
        match format {
            "json" => {
                // TODO: Implement JSON export
                Ok("[]".to_string())
            }
            "csv" => {
                // TODO: Implement CSV export
                Ok("".to_string())
            }
            _ => anyhow::bail!("Unsupported export format: {}", format),
        }
    }
}