//! SQLite database introspection implementation

use super::{common::*, DatabaseIntrospector};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::{Map, Value};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{Column, Pool, Row, Sqlite, TypeInfo, ValueRef};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tokio::fs;

pub struct SqliteIntrospector {
    pool: Pool<Sqlite>,
    db_name: String,
}

impl SqliteIntrospector {
    pub async fn new(database_url: &str) -> Result<Self> {
        let options = SqliteConnectOptions::from_str(database_url)?.create_if_missing(true);
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        let db_name = Self::database_name_from_url(database_url);
        Ok(Self { pool, db_name })
    }

    fn database_name_from_url(database_url: &str) -> String {
        let raw = database_url
            .trim_start_matches("sqlite:")
            .trim_start_matches("//");
        if raw.is_empty() || raw == ":memory:" {
            return "sqlite".to_string();
        }

        Path::new(raw)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(raw)
            .to_string()
    }

    fn quote_literal(value: &str) -> String {
        value.replace('\'', "''")
    }

    fn quote_identifier(value: &str) -> String {
        format!("\"{}\"", value.replace('"', "\"\""))
    }

    fn parse_type_metadata(declared_type: &str) -> (String, Option<i32>, Option<i32>, Option<i32>) {
        let normalized = declared_type.trim().to_uppercase();
        let mut max_length = None;
        let mut numeric_precision = None;
        let mut numeric_scale = None;

        if let Some(open) = normalized.find('(') {
            if let Some(close) = normalized[open + 1..].find(')') {
                let args = &normalized[open + 1..open + 1 + close];
                let parts: Vec<&str> = args.split(',').map(|p| p.trim()).collect();
                if parts.len() == 1 {
                    max_length = parts[0].parse().ok();
                    numeric_precision = max_length;
                } else if parts.len() >= 2 {
                    numeric_precision = parts[0].parse().ok();
                    numeric_scale = parts[1].parse().ok();
                }
            }
        }

        let base = normalized
            .split('(')
            .next()
            .unwrap_or("TEXT")
            .trim()
            .to_string();

        (base, max_length, numeric_precision, numeric_scale)
    }

    fn map_to_rust_type(&self, declared_type: &str, is_nullable: bool) -> String {
        let (base, _, _, _) = Self::parse_type_metadata(declared_type);
        let base_type = match base.as_str() {
            "INT" | "INTEGER" | "TINYINT" | "SMALLINT" | "MEDIUMINT" => "i32",
            "BIGINT" => "i64",
            "REAL" | "DOUBLE" | "DOUBLE PRECISION" | "FLOAT" => "f64",
            "NUMERIC" | "DECIMAL" => "Decimal",
            "BOOLEAN" | "BOOL" => "bool",
            "DATE" => "NaiveDate",
            "TIME" => "NaiveTime",
            "DATETIME" | "TIMESTAMP" => "DateTime<Utc>",
            "JSON" => "serde_json::Value",
            "BLOB" => "Vec<u8>",
            _ => "String",
        };

        if is_nullable {
            format!("Option<{}>", base_type)
        } else {
            base_type.to_string()
        }
    }

    fn generate_field_yaml(&self, column: &ColumnInfo) -> String {
        let mut field = String::new();
        let escaped_field_name = escape_yaml_field_name(&column.name);
        field.push_str(&format!("    {}:\n", escaped_field_name));

        let (base, max_length, precision, scale) =
            Self::parse_type_metadata(&column.data_type.to_lowercase());
        let schema_type = match base.as_str() {
            "VARCHAR" | "CHARACTER" | "CHAR" | "NCHAR" | "NVARCHAR" => {
                max_length
                    .map(|len| format!("string({})", len))
                    .unwrap_or_else(|| "string".to_string())
            }
            "TEXT" | "CLOB" => "text".to_string(),
            "INTEGER" | "INT" | "TINYINT" | "SMALLINT" | "MEDIUMINT" => "int".to_string(),
            "BIGINT" => "bigint".to_string(),
            "REAL" | "DOUBLE" | "DOUBLE PRECISION" | "FLOAT" => "float".to_string(),
            "NUMERIC" | "DECIMAL" => match (precision, scale) {
                (Some(p), Some(s)) => format!("decimal({},{})", p, s),
                _ => "decimal".to_string(),
            },
            "BOOLEAN" | "BOOL" => "boolean".to_string(),
            "DATE" => "date".to_string(),
            "TIME" => "time".to_string(),
            "DATETIME" | "TIMESTAMP" => "timestamp".to_string(),
            "JSON" => "json".to_string(),
            "BLOB" => "blob".to_string(),
            _ => "string".to_string(),
        };
        field.push_str(&format!("      type: {}\n", schema_type));

        if let Some(default) = &column.default_value {
            if !default.is_empty() && default != "NULL" {
                field.push_str(&format!("      default: {}\n", default));
            }
        }

        field.push_str(&format!(
            "      lang_type: {}\n",
            self.map_to_rust_type(&column.data_type, column.is_nullable)
        ));

        if column.is_nullable {
            field.push_str("      nullable: true\n");
        } else {
            field.push_str("      required: true\n");
        }

        if column.is_primary_key {
            field.push_str("      primary_key: true\n");
        }

        if column.is_unique {
            field.push_str("      unique: true\n");
        }

        let ai_hint = generate_field_ai_hint(&column.name, &column.data_type, column.is_foreign_key);
        field.push_str(&format!("      ai: \"{}\"\n", ai_hint));

        if column.is_foreign_key {
            if let (Some(table), Some(col)) = (&column.foreign_table, &column.foreign_column) {
                field.push_str(&format!("      foreign_key: {}.{}\n", table, col));
            }
        }

        field.push('\n');
        field
    }

    fn extract_view_body(sql: &str) -> Option<String> {
        let bytes = sql.as_bytes();
        for index in 0..bytes.len().saturating_sub(1) {
            if !sql[index..].starts_with("AS") && !sql[index..].starts_with("as") {
                continue;
            }

            let prev = if index == 0 {
                b' '
            } else {
                bytes[index - 1]
            };
            let next = bytes.get(index + 2).copied().unwrap_or(b' ');
            if prev.is_ascii_whitespace() && next.is_ascii_whitespace() {
                let body = sql[index + 2..].trim().trim_end_matches(';').trim();
                if !body.is_empty() {
                    return Some(body.to_string());
                }
            }
        }
        None
    }

    async fn get_view_definition(&self, name: &str) -> Result<Option<String>> {
        let row = sqlx::query(
            "SELECT sql FROM sqlite_master WHERE type = 'view' AND name = ? LIMIT 1",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row
            .and_then(|r| r.try_get::<Option<String>, _>("sql").ok().flatten())
            .and_then(|sql| Self::extract_view_body(&sql)))
    }

    async fn generate_table_schema_yaml(&self, description: &TableDescription) -> Result<String> {
        let mut yaml = String::new();

        yaml.push_str(&format!(
            "# {} entity - Generated from database\n\n",
            description.table.name
        ));

        let model_name = to_pascal_case(&description.table.name);
        yaml.push_str(&format!("{}:\n", model_name));
        yaml.push_str(&format!("  table: {}\n", description.table.name));
        yaml.push_str("  database_type: sqlite\n");
        yaml.push_str(&format!("  database_name: {}\n", self.db_name));

        let element_type = if description.table.table_type.eq_ignore_ascii_case("view") {
            "view"
        } else {
            "table"
        };
        yaml.push_str(&format!("  element_type: {}\n", element_type));
        yaml.push_str("  version: 1\n");

        if element_type == "view" {
            if let Some(body) = self.get_view_definition(&description.table.name).await? {
                yaml.push_str("  view:\n");
                yaml.push_str("    sql: |\n");
                for line in body.lines() {
                    yaml.push_str(&format!("      {}\n", line));
                }
            }
        }

        let ai_context = generate_table_ai_context(&description.table.name);
        yaml.push_str(&format!("  ai_context: \"{}\"\n", ai_context));
        yaml.push_str("  \n  fields:\n");

        for column in &description.columns {
            yaml.push_str(&self.generate_field_yaml(column));
        }

        let relations = generate_relations_yaml(&description.columns);
        if !relations.is_empty() {
            yaml.push_str("  \n  relations:\n");
            yaml.push_str(&relations);
        }

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

        if !description.triggers.is_empty() {
            yaml.push_str("  \n  triggers:\n");
            for trigger in &description.triggers {
                yaml.push_str(&format!("    - name: {}\n", trigger.name));
                yaml.push_str(&format!("      event: {}\n", trigger.event));
                yaml.push_str(&format!("      timing: {}\n", trigger.timing));
                yaml.push_str(&format!("      for_each: {}\n", trigger.for_each));
                yaml.push_str("      ai: \"Database trigger - executes automatically on table changes. Consider application logic implications.\"\n");
                yaml.push('\n');
            }
        }

        yaml.push('\n');
        Ok(yaml)
    }

    async fn fetch_row_count(&self, table_name: &str) -> Option<i64> {
        let query = format!(
            "SELECT COUNT(*) AS row_count FROM {}",
            Self::quote_identifier(table_name)
        );
        sqlx::query(&query)
            .fetch_one(&self.pool)
            .await
            .ok()
            .and_then(|row| row.try_get("row_count").ok())
    }

    fn sqlite_value_to_json(row: &sqlx::sqlite::SqliteRow, index: usize) -> Result<Value> {
        let raw = row.try_get_raw(index)?;
        if raw.is_null() {
            return Ok(Value::Null);
        }

        let type_name = raw.type_info().name().to_uppercase();
        if type_name.contains("INT") {
            if let Ok(value) = row.try_get::<i64, _>(index) {
                return Ok(Value::from(value));
            }
        }
        if type_name.contains("REAL") || type_name.contains("FLOA") || type_name.contains("DOUB") {
            if let Ok(value) = row.try_get::<f64, _>(index) {
                return Ok(Value::from(value));
            }
        }
        if type_name.contains("BLOB") {
            if let Ok(value) = row.try_get::<Vec<u8>, _>(index) {
                let hex = value.iter().map(|b| format!("{:02x}", b)).collect::<String>();
                return Ok(Value::String(hex));
            }
        }
        if let Ok(value) = row.try_get::<String, _>(index) {
            return Ok(Value::String(value));
        }
        if let Ok(value) = row.try_get::<f64, _>(index) {
            return Ok(Value::from(value));
        }
        if let Ok(value) = row.try_get::<i64, _>(index) {
            return Ok(Value::from(value));
        }

        Ok(Value::Null)
    }

    fn escape_csv_field(value: &str) -> String {
        if value.contains([',', '"', '\n']) {
            format!("\"{}\"", value.replace('"', "\"\""))
        } else {
            value.to_string()
        }
    }
}

#[async_trait]
impl DatabaseIntrospector for SqliteIntrospector {
    async fn list_tables(&self, metadata: bool) -> Result<Vec<TableInfo>> {
        let rows = sqlx::query(
            r#"
            SELECT name, type
            FROM sqlite_master
            WHERE type IN ('table', 'view')
              AND name NOT LIKE 'sqlite_%'
            ORDER BY name
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut tables = Vec::new();
        for row in rows {
            let name: String = row.try_get("name")?;
            let table_type: String = row.try_get("type")?;
            let row_count = if metadata && table_type == "table" {
                self.fetch_row_count(&name).await
            } else {
                None
            };

            tables.push(TableInfo {
                name,
                schema: Some("main".to_string()),
                table_type,
                row_count,
                size_bytes: None,
                comment: None,
            });
        }

        Ok(tables)
    }

    async fn describe_table(&self, table_name: &str) -> Result<TableDescription> {
        let table_row = sqlx::query(
            r#"
            SELECT name, type
            FROM sqlite_master
            WHERE type IN ('table', 'view') AND name = ?
            LIMIT 1
            "#,
        )
        .bind(table_name)
        .fetch_optional(&self.pool)
        .await?;

        let table_info = match table_row {
            Some(row) => {
                let table_type: String = row.try_get("type")?;
                TableInfo {
                    name: row.try_get("name")?,
                    schema: Some("main".to_string()),
                    table_type: table_type.clone(),
                    row_count: if table_type == "table" {
                        self.fetch_row_count(table_name).await
                    } else {
                        None
                    },
                    size_bytes: None,
                    comment: None,
                }
            }
            None => anyhow::bail!("Table '{}' not found", table_name),
        };

        let pragma_table = Self::quote_literal(table_name);
        let column_rows = sqlx::query(&format!("PRAGMA table_info('{}')", pragma_table))
            .fetch_all(&self.pool)
            .await?;

        let fk_rows = sqlx::query(&format!("PRAGMA foreign_key_list('{}')", pragma_table))
            .fetch_all(&self.pool)
            .await?;

        let mut foreign_keys = HashMap::new();
        let mut constraints = Vec::new();
        for row in fk_rows {
            let from: String = row.try_get("from")?;
            let table: String = row.try_get("table")?;
            let to: String = row.try_get("to")?;
            let on_delete: Option<String> = row.try_get("on_delete").ok();
            let on_update: Option<String> = row.try_get("on_update").ok();
            foreign_keys.insert(
                from.clone(),
                (table.clone(), to.clone(), on_delete.clone(), on_update.clone()),
            );
            constraints.push(ConstraintInfo {
                name: format!("fk_{}_{}", table_name, from),
                constraint_type: "FOREIGN KEY".to_string(),
                columns: vec![from],
                foreign_table: Some(table),
                foreign_columns: Some(vec![to]),
                check_expression: None,
                on_delete,
                on_update,
            });
        }

        let index_rows = sqlx::query(&format!("PRAGMA index_list('{}')", pragma_table))
            .fetch_all(&self.pool)
            .await?;

        let mut indexes = Vec::new();
        let mut unique_single_columns = HashSet::new();
        for row in index_rows {
            let index_name: String = row.try_get("name")?;
            let is_unique = row.try_get::<i64, _>("unique").unwrap_or(0) == 1;
            let is_primary = row
                .try_get::<String, _>("origin")
                .map(|origin| origin == "pk")
                .unwrap_or(false);

            let index_info_rows = sqlx::query(&format!(
                "PRAGMA index_info('{}')",
                Self::quote_literal(&index_name)
            ))
            .fetch_all(&self.pool)
            .await?;

            let mut columns = Vec::new();
            for info_row in index_info_rows {
                if let Ok(name) = info_row.try_get::<String, _>("name") {
                    columns.push(name);
                }
            }

            if is_unique && columns.len() == 1 && !is_primary {
                unique_single_columns.insert(columns[0].clone());
            }

            indexes.push(IndexInfo {
                name: index_name,
                columns,
                is_unique,
                is_primary,
            });
        }

        let trigger_rows = sqlx::query(
            "SELECT name, sql FROM sqlite_master WHERE type = 'trigger' AND tbl_name = ? ORDER BY name",
        )
        .bind(table_name)
        .fetch_all(&self.pool)
        .await?;

        let mut triggers = Vec::new();
        for row in trigger_rows {
            let name: String = row.try_get("name")?;
            let body: Option<String> = row.try_get("sql").ok();
            let upper = body.as_deref().unwrap_or_default().to_uppercase();
            let timing = if upper.contains(" BEFORE ") {
                "BEFORE"
            } else {
                "AFTER"
            };
            let event = if upper.contains(" INSERT ") {
                "INSERT"
            } else if upper.contains(" UPDATE ") {
                "UPDATE"
            } else if upper.contains(" DELETE ") {
                "DELETE"
            } else {
                ""
            };

            triggers.push(TriggerInfo {
                name,
                event: event.to_string(),
                timing: timing.to_string(),
                for_each: "ROW".to_string(),
                condition: None,
                body,
                description: None,
            });
        }

        let mut columns = Vec::new();
        for row in column_rows {
            let column_name: String = row.try_get("name")?;
            let declared_type = row
                .try_get::<Option<String>, _>("type")
                .ok()
                .flatten()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "TEXT".to_string());
            let (_, max_length, numeric_precision, numeric_scale) =
                Self::parse_type_metadata(&declared_type);

            let pk_position = row.try_get::<i64, _>("pk").unwrap_or(0);
            let is_primary_key = pk_position > 0;
            let is_foreign_key = foreign_keys.contains_key(&column_name);
            let is_unique = unique_single_columns.contains(&column_name);

            let (foreign_table, foreign_column, on_delete, on_update) = if is_foreign_key {
                let (table, column, del, upd) = foreign_keys.get(&column_name).unwrap();
                (
                    Some(table.clone()),
                    Some(column.clone()),
                    del.clone(),
                    upd.clone(),
                )
            } else {
                (None, None, None, None)
            };

            columns.push(ColumnInfo {
                name: column_name,
                data_type: declared_type,
                column_type: None,
                postgres_type_name: None,
                is_nullable: row.try_get::<i64, _>("notnull").unwrap_or(0) == 0,
                default_value: row.try_get("dflt_value").ok(),
                is_primary_key,
                is_unique,
                is_foreign_key,
                foreign_table,
                foreign_column,
                on_delete,
                on_update,
                comment: None,
                max_length,
                numeric_precision,
                numeric_scale,
            });
        }

        Ok(TableDescription {
            table: table_info,
            columns,
            indexes,
            constraints,
            triggers,
        })
    }

    async fn generate_schemas(
        &self,
        output_dir: &PathBuf,
        force: bool,
        filter_tables: &[String],
    ) -> Result<()> {
        if !output_dir.exists() {
            fs::create_dir_all(output_dir).await?;
        }

        let tables = self.list_tables(true).await?;
        println!("📋 Found {} tables", tables.len());

        let mut generated_count = 0;
        let mut skipped_count = 0;

        for table in tables {
            if !filter_tables.is_empty() && !filter_tables.contains(&table.name) {
                continue;
            }

            let schema_file = output_dir.join(format!("{}.yaml", table.name));
            if schema_file.exists() && !force {
                println!("⚠️  Skipping existing schema: {}", schema_file.display());
                skipped_count += 1;
                continue;
            }

            let description = self.describe_table(&table.name).await?;
            let yaml_content = self.generate_table_schema_yaml(&description).await?;
            fs::write(&schema_file, yaml_content).await?;
            println!("✅ Generated schema: {}", schema_file.display());
            generated_count += 1;
        }

        let meta_file = output_dir.join("_meta.yaml");
        if !meta_file.exists() || force {
            let meta_content = self.generate_meta_yaml().await?;
            fs::write(&meta_file, meta_content).await?;
            println!("✅ Generated metadata: {}", meta_file.display());
        }

        println!(
            "📊 Generated {} schemas, skipped {}",
            generated_count, skipped_count
        );
        Ok(())
    }

    async fn generate_meta_yaml(&self) -> Result<String> {
        let mut yaml = String::new();
        yaml.push_str("# Global schema configuration - Generated from SQLite database\n\n");
        yaml.push_str("version: \"1.0\"\n");
        yaml.push_str("database_type: sqlite\n");
        yaml.push_str(&format!("database_name: {}\n", self.db_name));
        yaml.push_str(&format!(
            "description: \"Schema generated from SQLite database '{}'\"\n",
            self.db_name
        ));
        yaml.push_str("ai_context: \"Generated schemas from existing SQLite database structure with intelligent field mapping\"\n");
        yaml.push('\n');
        yaml.push_str("# Global field defaults\n");
        yaml.push_str("field_defaults:\n");
        yaml.push_str("  string:\n");
        yaml.push_str("    max_length: 255\n");
        yaml.push_str("    charset: utf8\n");
        yaml.push_str("    \n");
        yaml.push_str("  timestamp:\n");
        yaml.push_str("    default: now\n");
        yaml.push_str("    on_update: now\n");
        yaml.push_str("    \n");
        yaml.push_str("# Code generation settings\n");
        yaml.push_str("generation:\n");
        yaml.push_str("  base_class: \"BaseModel\"\n");
        yaml.push_str("  use_traits: [HasTimestamps]\n");
        yaml.push('\n');
        Ok(yaml)
    }

    async fn get_database_name(&self) -> Result<String> {
        Ok(self.db_name.clone())
    }

    async fn export_data(&self, query: &str, format: &str) -> Result<String> {
        let rows = sqlx::query(query).fetch_all(&self.pool).await?;
        let headers = rows
            .first()
            .map(|row| {
                row.columns()
                    .iter()
                    .map(|column| column.name().to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let mut json_rows = Vec::new();
        for row in &rows {
            let mut object = Map::new();
            for (index, column) in row.columns().iter().enumerate() {
                object.insert(
                    column.name().to_string(),
                    Self::sqlite_value_to_json(row, index)?,
                );
            }
            json_rows.push(Value::Object(object));
        }

        match format {
            "json" => Ok(serde_json::to_string_pretty(&json_rows)?),
            "csv" => {
                if headers.is_empty() {
                    return Ok(String::new());
                }

                let mut lines = Vec::new();
                lines.push(
                    headers
                        .iter()
                        .map(|header| Self::escape_csv_field(header))
                        .collect::<Vec<_>>()
                        .join(","),
                );

                for row in json_rows {
                    let object = row.as_object().expect("row should be an object");
                    let line = headers
                        .iter()
                        .map(|header| {
                            let value = object.get(header).cloned().unwrap_or(Value::Null);
                            let string_value = match value {
                                Value::Null => String::new(),
                                Value::String(s) => s,
                                other => other.to_string(),
                            };
                            Self::escape_csv_field(&string_value)
                        })
                        .collect::<Vec<_>>()
                        .join(",");
                    lines.push(line);
                }

                Ok(lines.join("\n"))
            }
            _ => anyhow::bail!("Unsupported export format: {}", format),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_sqlite_db() -> Result<(TempDir, SqliteIntrospector)> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.sqlite");
        let database_url = format!("sqlite://{}", db_path.display());

        let options = SqliteConnectOptions::from_str(&database_url)?.create_if_missing(true);
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;

        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await?;
        sqlx::query(
            r#"
            CREATE TABLE users (
                id INTEGER PRIMARY KEY,
                email TEXT NOT NULL UNIQUE,
                age INTEGER,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await?;
        sqlx::query(
            r#"
            CREATE TABLE posts (
                id INTEGER PRIMARY KEY,
                user_id INTEGER NOT NULL,
                title TEXT NOT NULL,
                body TEXT,
                FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE ON UPDATE CASCADE
            )
            "#,
        )
        .execute(&pool)
        .await?;
        sqlx::query("CREATE INDEX idx_posts_user_id ON posts(user_id)")
            .execute(&pool)
            .await?;
        sqlx::query(
            r#"
            CREATE VIEW user_emails AS
            SELECT id, email FROM users
            "#,
        )
        .execute(&pool)
        .await?;
        sqlx::query(
            r#"
            CREATE TRIGGER posts_title_guard
            BEFORE INSERT ON posts
            FOR EACH ROW
            WHEN NEW.title = ''
            BEGIN
                SELECT RAISE(FAIL, 'title required');
            END;
            "#,
        )
        .execute(&pool)
        .await?;
        sqlx::query("INSERT INTO users (email, age) VALUES ('a@example.com', 30), ('b@example.com', 41)")
            .execute(&pool)
            .await?;
        sqlx::query(
            "INSERT INTO posts (user_id, title, body) VALUES (1, 'Hello', 'World'), (2, 'Second', 'Post')",
        )
        .execute(&pool)
        .await?;

        Ok((
            temp_dir,
            SqliteIntrospector {
                pool,
                db_name: "test.sqlite".to_string(),
            },
        ))
    }

    #[tokio::test]
    async fn sqlite_introspector_lists_and_describes_real_tables() -> Result<()> {
        let (_temp_dir, introspector) = create_test_sqlite_db().await?;

        let tables = introspector.list_tables(true).await?;
        assert!(tables.iter().any(|table| table.name == "users" && table.row_count == Some(2)));
        assert!(tables.iter().any(|table| table.name == "posts" && table.row_count == Some(2)));
        assert!(tables.iter().any(|table| table.name == "user_emails" && table.table_type == "view"));

        let description = introspector.describe_table("posts").await?;
        assert_eq!(description.table.name, "posts");
        assert!(description.indexes.iter().any(|index| index.name == "idx_posts_user_id"));
        assert!(description
            .triggers
            .iter()
            .any(|trigger| trigger.name == "posts_title_guard"));

        let user_id = description
            .columns
            .iter()
            .find(|column| column.name == "user_id")
            .expect("user_id column should exist");
        assert!(user_id.is_foreign_key);
        assert_eq!(user_id.foreign_table.as_deref(), Some("users"));
        assert_eq!(user_id.foreign_column.as_deref(), Some("id"));

        Ok(())
    }

    #[tokio::test]
    async fn sqlite_introspector_exports_data_and_generates_yaml() -> Result<()> {
        let (temp_dir, introspector) = create_test_sqlite_db().await?;

        let json = introspector
            .export_data("SELECT id, email FROM users ORDER BY id", "json")
            .await?;
        assert!(json.contains("\"a@example.com\""));
        assert!(json.contains("\"b@example.com\""));

        let csv = introspector
            .export_data("SELECT id, title FROM posts ORDER BY id", "csv")
            .await?;
        assert!(csv.lines().next().unwrap_or_default().contains("id,title"));
        assert!(csv.contains("Hello"));

        let meta = introspector.generate_meta_yaml().await?;
        assert!(meta.contains("database_type: sqlite"));

        let output_dir = temp_dir.path().join("schemas");
        introspector
            .generate_schemas(&output_dir, true, &[])
            .await?;

        let posts_yaml = fs::read_to_string(output_dir.join("posts.yaml")).await?;
        assert!(posts_yaml.contains("foreign_key: users.id"));
        assert!(posts_yaml.contains("database_type: sqlite"));

        let view_yaml = fs::read_to_string(output_dir.join("user_emails.yaml")).await?;
        let normalized_view_yaml = view_yaml.to_lowercase();
        assert!(view_yaml.contains("element_type: view"));
        assert!(normalized_view_yaml.contains("select"));
        assert!(normalized_view_yaml.contains("from users"));

        Ok(())
    }
}
