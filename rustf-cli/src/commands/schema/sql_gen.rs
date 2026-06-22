//! Shared, dialect-aware SQL DDL generation from a loaded [`Schema`].
//!
//! This replaces three near-identical per-dialect copies that had drifted and
//! all emitted Postgres-flavoured SQL regardless of the target dialect. Those
//! copies also had concrete bugs that made `schema generate migrations`
//! unusable:
//!   * native introspected type names (`tinyint`, `mediumint`, …) were unknown
//!     to the type map and silently became `TEXT`;
//!   * enum `CHECK` constraints referenced a literal `column_name` instead of
//!     the real column;
//!   * `foreign_key` / `auto` constraints were ignored;
//!   * column/table order was taken straight from a `HashMap`, so output was
//!     non-deterministic and the primary key landed in the middle of the table.
//!
//! One generator, parameterised by [`Dialect`], fixes all of the above in a
//! single place.

use rustf_schema::types::{
    AutoGenerate, Field, FieldType, ForeignKeyAction, Table, TypeParam,
};
use rustf_schema::Schema;

/// Target SQL dialect for DDL generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dialect {
    Postgres,
    MySql,
    Sqlite,
}

/// Render a full set of `CREATE TABLE` statements (plus foreign-key
/// `ALTER TABLE`s) for every table in `schema`, targeting `dialect`.
pub fn generate_sql_schema(schema: &Schema, dialect: Dialect) -> anyhow::Result<String> {
    let mut sql = String::new();

    sql.push_str("-- Generated SQL schema\n");
    if let Some(meta) = &schema.meta {
        sql.push_str(&format!(
            "-- Database: {} v{}\n",
            meta.database_name, meta.version
        ));
        if let Some(desc) = &meta.description {
            sql.push_str(&format!("-- {}\n", desc));
        }
    }
    sql.push_str(&format!("-- Dialect: {}\n", dialect.name()));
    sql.push_str("-- DO NOT EDIT - Auto-generated from schema\n\n");

    // Deterministic table order (HashMap iteration is unordered).
    let mut tables: Vec<(&String, &Table)> = schema.tables.iter().collect();
    tables.sort_by(|(a, _), (b, _)| a.cmp(b));

    // Foreign keys are emitted as trailing ALTER TABLE statements so the order
    // in which tables are created never matters (no forward-reference errors).
    let mut foreign_keys = String::new();

    for (_name, table) in &tables {
        render_table(table, dialect, &mut sql, &mut foreign_keys);
    }

    if !foreign_keys.is_empty() {
        sql.push_str("-- Foreign keys\n");
        sql.push_str(&foreign_keys);
    }

    Ok(sql)
}

impl Dialect {
    fn name(&self) -> &'static str {
        match self {
            Dialect::Postgres => "PostgreSQL",
            Dialect::MySql => "MySQL",
            Dialect::Sqlite => "SQLite",
        }
    }
}

/// Emit one `CREATE TABLE`, pushing any foreign-key clauses onto `foreign_keys`.
fn render_table(table: &Table, dialect: Dialect, sql: &mut String, foreign_keys: &mut String) {
    sql.push_str(&format!("-- Table: {}\n", table.table));
    if let Some(desc) = &table.description {
        sql.push_str(&format!("-- {}\n", desc));
    }
    sql.push_str(&format!("CREATE TABLE {} (\n", table.table));

    // Deterministic column order: primary key(s) first, then alphabetical.
    let mut fields: Vec<(&String, &Field)> = table.fields.iter().collect();
    fields.sort_by(|(an, a), (bn, b)| {
        let a_pk = a.constraints.primary_key == Some(true);
        let b_pk = b.constraints.primary_key == Some(true);
        b_pk.cmp(&a_pk).then_with(|| an.cmp(bn))
    });

    let mut defs = Vec::new();
    for (field_name, field) in &fields {
        defs.push(render_column(field_name, field, dialect));

        // Collect foreign keys for a trailing ALTER TABLE.
        if let Some(fk) = &field.constraints.foreign_key {
            foreign_keys.push_str(&render_foreign_key(&table.table, field_name, fk, field, dialect));
        }
    }

    sql.push_str(&defs.join(",\n"));
    sql.push_str("\n);\n\n");
}

/// Render a single column definition (without trailing comma).
fn render_column(name: &str, field: &Field, dialect: Dialect) -> String {
    let is_pk = field.constraints.primary_key == Some(true);
    let is_auto = matches!(
        &field.constraints.auto,
        Some(AutoGenerate::Boolean(true)) | Some(AutoGenerate::Type(_))
    );

    // SQLite auto-increment is a special case: it must be exactly
    // `INTEGER PRIMARY KEY AUTOINCREMENT`, with the PK declared inline.
    if dialect == Dialect::Sqlite && is_pk && is_auto {
        return format!("    {} INTEGER PRIMARY KEY AUTOINCREMENT", name);
    }

    let sql_type = sql_type(field, dialect, is_auto);
    let mut def = format!("    {} {}", name, sql_type);

    // NOT NULL: only when explicitly required or explicitly non-nullable.
    // (A primary key is implicitly NOT NULL.)
    let not_null = field.constraints.required == Some(true)
        || field.constraints.nullable == Some(false)
        || is_pk;
    if not_null {
        def.push_str(" NOT NULL");
    }

    if is_auto && dialect == Dialect::MySql {
        def.push_str(" AUTO_INCREMENT");
    }

    if is_pk {
        def.push_str(" PRIMARY KEY");
    }

    if field.constraints.unique == Some(true) && !is_pk {
        def.push_str(" UNIQUE");
    }

    if let Some(default) = &field.constraints.default {
        if let Some(rendered) = render_default(default, dialect) {
            def.push_str(&format!(" DEFAULT {}", rendered));
        }
    }

    // Enum becomes an inline CHECK on the real column for dialects without a
    // native enum type (Postgres, SQLite). MySQL uses the native ENUM type
    // produced by `sql_type`, so it needs no CHECK.
    if let FieldType::Enum { values, .. } = &field.field_type {
        if dialect != Dialect::MySql {
            let list = values
                .iter()
                .map(|v| format!("'{}'", v))
                .collect::<Vec<_>>()
                .join(", ");
            def.push_str(&format!(" CHECK ({} IN ({}))", name, list));
        }
    }

    def
}

/// Render a trailing `ALTER TABLE … ADD … FOREIGN KEY` statement.
/// `fk` is `"table.column"` (or just `"table"`, defaulting the column to `id`).
fn render_foreign_key(
    table: &str,
    column: &str,
    fk: &str,
    field: &Field,
    dialect: Dialect,
) -> String {
    let (ref_table, ref_col) = match fk.split_once('.') {
        Some((t, c)) => (t, c),
        None => (fk, "id"),
    };

    let mut stmt = format!(
        "ALTER TABLE {} ADD FOREIGN KEY ({}) REFERENCES {} ({})",
        table, column, ref_table, ref_col
    );

    if let Some(action) = &field.constraints.on_delete {
        stmt.push_str(&format!(" ON DELETE {}", fk_action(action)));
    }
    if let Some(action) = &field.constraints.on_update {
        stmt.push_str(&format!(" ON UPDATE {}", fk_action(action)));
    }
    stmt.push_str(";\n");

    // SQLite cannot ALTER TABLE to add a foreign key; surface that instead of
    // emitting invalid SQL.
    if dialect == Dialect::Sqlite {
        return format!(
            "-- NOTE: SQLite cannot add FKs via ALTER TABLE; declare inline instead.\n-- {}",
            stmt
        );
    }
    stmt
}

fn fk_action(action: &ForeignKeyAction) -> &'static str {
    match action {
        ForeignKeyAction::Cascade => "CASCADE",
        ForeignKeyAction::Restrict => "RESTRICT",
        ForeignKeyAction::SetNull => "SET NULL",
        ForeignKeyAction::SetDefault => "SET DEFAULT",
        ForeignKeyAction::NoAction => "NO ACTION",
    }
}

/// Render a DEFAULT literal for the target dialect, or `None` to omit it.
fn render_default(value: &serde_json::Value, dialect: Dialect) -> Option<String> {
    match value {
        serde_json::Value::String(s) => Some(format!("'{}'", s.replace('\'', "''"))),
        serde_json::Value::Number(n) => Some(n.to_string()),
        serde_json::Value::Bool(b) => Some(match dialect {
            // MySQL has no real boolean; store 1/0.
            Dialect::MySql => if *b { "1".into() } else { "0".into() },
            _ => b.to_string(),
        }),
        _ => None,
    }
}

/// Map a field's logical type to a concrete SQL type for `dialect`.
fn sql_type(field: &Field, dialect: Dialect, is_auto: bool) -> String {
    match &field.field_type {
        FieldType::Simple(t) => simple_type(t, dialect, is_auto),
        FieldType::Parameterized { base_type, params } => {
            parameterized_type(base_type, params, dialect)
        }
        FieldType::Enum { values, .. } => match dialect {
            // Native ENUM type on MySQL.
            Dialect::MySql => {
                let list = values
                    .iter()
                    .map(|v| format!("'{}'", v))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("ENUM({})", list)
            }
            // Postgres / SQLite: stored as TEXT, constrained via the inline
            // CHECK added in `render_column` (which knows the column name).
            _ => "TEXT".to_string(),
        },
        FieldType::Json { .. } => json_type(dialect),
    }
}

/// Map a simple (non-parameterised) type name to SQL, honouring both generic
/// names and the native names emitted by database introspection.
fn simple_type(t: &str, dialect: Dialect, is_auto: bool) -> String {
    let t = t.to_ascii_lowercase();

    // Auto-increment integers map to SERIAL-family on Postgres.
    if is_auto && dialect == Dialect::Postgres {
        return match t.as_str() {
            "bigint" | "int8" => "BIGSERIAL".to_string(),
            "smallint" | "int2" | "tinyint" => "SMALLSERIAL".to_string(),
            _ => "SERIAL".to_string(),
        };
    }

    match t.as_str() {
        // Integer family — width preserved on MySQL, widened sensibly elsewhere.
        "tinyint" => pick(dialect, "TINYINT", "SMALLINT", "INTEGER"),
        "smallint" | "int2" => pick(dialect, "SMALLINT", "SMALLINT", "INTEGER"),
        "mediumint" => pick(dialect, "MEDIUMINT", "INTEGER", "INTEGER"),
        "int" | "integer" | "int4" => pick(dialect, "INT", "INTEGER", "INTEGER"),
        "bigint" | "int8" => pick(dialect, "BIGINT", "BIGINT", "INTEGER"),
        "serial" => pick(dialect, "INT", "SERIAL", "INTEGER"),

        // Booleans — MySQL has no native bool.
        "boolean" | "bool" => pick(dialect, "TINYINT(1)", "BOOLEAN", "BOOLEAN"),

        // Floating point / fixed point.
        "decimal" | "numeric" => "DECIMAL".to_string(),
        "float" => pick(dialect, "FLOAT", "REAL", "REAL"),
        "double" | "double precision" => pick(dialect, "DOUBLE", "DOUBLE PRECISION", "REAL"),

        // Text.
        "string" | "text" | "mediumtext" | "longtext" | "tinytext" => "TEXT".to_string(),

        // Temporal.
        "date" => "DATE".to_string(),
        "time" => "TIME".to_string(),
        "datetime" => pick(dialect, "DATETIME", "TIMESTAMP", "TIMESTAMP"),
        "timestamp" => "TIMESTAMP".to_string(),

        // Structured / misc.
        "json" => json_type(dialect),
        "jsonb" => pick(dialect, "JSON", "JSONB", "TEXT"),
        "uuid" => pick(dialect, "CHAR(36)", "UUID", "TEXT"),
        "blob" | "binary" | "bytea" => pick(dialect, "BLOB", "BYTEA", "BLOB"),

        // Unknown: fall back to TEXT but keep the original name as a hint.
        other => {
            let _ = other;
            "TEXT".to_string()
        }
    }
}

/// Map a parameterised type (`string(150)`, `decimal(10,2)`, …) to SQL.
fn parameterized_type(base: &str, params: &[TypeParam], dialect: Dialect) -> String {
    match base.to_ascii_lowercase().as_str() {
        "string" | "varchar" => match params.first() {
            Some(TypeParam::Number(len)) => format!("VARCHAR({})", len),
            _ => "VARCHAR(255)".to_string(),
        },
        "char" => match params.first() {
            Some(TypeParam::Number(len)) => format!("CHAR({})", len),
            _ => "CHAR".to_string(),
        },
        "decimal" | "numeric" => match (params.first(), params.get(1)) {
            (Some(TypeParam::Number(p)), Some(TypeParam::Number(s))) => {
                format!("DECIMAL({},{})", p, s)
            }
            (Some(TypeParam::Number(p)), None) => format!("DECIMAL({})", p),
            _ => "DECIMAL".to_string(),
        },
        // Width-carrying integer types from MySQL introspection, e.g. int(11).
        "tinyint" | "smallint" | "mediumint" | "int" | "integer" | "bigint" => {
            simple_type(base, dialect, false)
        }
        _ => "TEXT".to_string(),
    }
}

fn json_type(dialect: Dialect) -> String {
    pick(dialect, "JSON", "JSONB", "TEXT")
}

/// Pick a type string by dialect: `(mysql, postgres, sqlite)`.
fn pick(dialect: Dialect, mysql: &str, postgres: &str, sqlite: &str) -> String {
    match dialect {
        Dialect::MySql => mysql,
        Dialect::Postgres => postgres,
        Dialect::Sqlite => sqlite,
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustf_schema::SchemaParser;

    fn schema(yaml: &str) -> Schema {
        Schema {
            tables: SchemaParser::parse_yaml(yaml).expect("parse yaml"),
            meta: None,
        }
    }

    const YAML: &str = r#"
accounts:
  table: accounts
  version: 1
  fields:
    id:
      type: mediumint
      primary_key: true
      auto: true
      required: true
    status:
      type: enum
      values: [active, closed]
      required: true
    owner_id:
      type: mediumint
      foreign_key: accounts.id
    verified:
      type: tinyint
"#;

    #[test]
    fn mysql_uses_native_types_and_enum() {
        let sql = generate_sql_schema(&schema(YAML), Dialect::MySql).unwrap();
        // native integer width preserved + auto-increment
        assert!(sql.contains("id MEDIUMINT NOT NULL AUTO_INCREMENT PRIMARY KEY"), "{sql}");
        // tinyint must NOT degrade to TEXT
        assert!(sql.contains("verified TINYINT"), "{sql}");
        // native ENUM type, not a CHECK
        assert!(sql.contains("status ENUM('active', 'closed')"), "{sql}");
        // real foreign key emitted
        assert!(sql.contains("ALTER TABLE accounts ADD FOREIGN KEY (owner_id) REFERENCES accounts (id)"), "{sql}");
        // the old bug: a literal column_name must never appear
        assert!(!sql.contains("column_name"), "{sql}");
    }

    #[test]
    fn postgres_enum_check_uses_real_column_name() {
        let sql = generate_sql_schema(&schema(YAML), Dialect::Postgres).unwrap();
        assert!(sql.contains("id SERIAL"), "{sql}");
        assert!(sql.contains("verified SMALLINT"), "{sql}");
        // CHECK references the actual column, not a literal placeholder
        assert!(sql.contains("status TEXT NOT NULL CHECK (status IN ('active', 'closed'))"), "{sql}");
        assert!(!sql.contains("column_name"), "{sql}");
    }

    #[test]
    fn sqlite_autoincrement_and_no_alter_fk() {
        let sql = generate_sql_schema(&schema(YAML), Dialect::Sqlite).unwrap();
        assert!(sql.contains("id INTEGER PRIMARY KEY AUTOINCREMENT"), "{sql}");
        assert!(sql.contains("verified INTEGER"), "{sql}");
        // SQLite can't ALTER-add FKs: must be commented out, not emitted as runnable SQL
        assert!(sql.contains("-- NOTE: SQLite cannot add FKs"), "{sql}");
    }

    #[test]
    fn columns_are_deterministically_ordered_pk_first() {
        let sql = generate_sql_schema(&schema(YAML), Dialect::MySql).unwrap();
        let id = sql.find("id MEDIUMINT").unwrap();
        let owner = sql.find("owner_id").unwrap();
        let status = sql.find("status ").unwrap();
        // primary key first, then alphabetical (owner_id before status)
        assert!(id < owner && owner < status, "{sql}");
    }
}
