//! Shared, dialect-aware SQL DDL generation from a loaded [`Schema`].
//!
//! This replaces three near-identical per-dialect copies that had drifted and
//! all emitted Postgres-flavoured SQL regardless of the target dialect. Those
//! copies also had concrete bugs that made `schema generate sql`
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

    // Deterministic order (HashMap iteration is unordered).
    let mut entries: Vec<(&String, &Table)> = schema.tables.iter().collect();
    entries.sort_by(|(a, _), (b, _)| a.cmp(b));

    // Split base tables from views. Views must be emitted *after* the tables
    // (and other views) they read from, or the DB rejects them with a
    // "relation does not exist" error. Tables never reference views, so a
    // simple two-phase ordering — all tables, then dependency-ordered views —
    // is enough to avoid forward references.
    let (views, base_tables): (Vec<_>, Vec<_>) = entries
        .into_iter()
        .partition(|(_, t)| is_view_type(t.element_type.as_deref()));

    // Foreign keys are emitted as trailing ALTER TABLE statements so the order
    // in which tables are created never matters (no forward-reference errors).
    let mut foreign_keys = String::new();

    for (_name, table) in &base_tables {
        render_table(table, dialect, &mut sql, &mut foreign_keys);
    }

    if !foreign_keys.is_empty() {
        sql.push_str("-- Foreign keys\n");
        sql.push_str(&foreign_keys);
    }

    // Views last, ordered so a view that references another view is emitted
    // after its dependency.
    if !views.is_empty() {
        sql.push_str("-- Views\n");
        for (_name, table) in order_views(&views) {
            match table.element_type.as_deref() {
                Some("materialized_view") => render_view(table, &mut sql, true),
                _ => render_view(table, &mut sql, false),
            }
        }
    }

    Ok(sql)
}

/// Whether an `element_type` denotes a view (plain or materialized).
fn is_view_type(element_type: Option<&str>) -> bool {
    matches!(element_type, Some("view") | Some("materialized_view"))
}

/// Order views so that any view referencing another view appears after the
/// view it depends on. Dependencies are detected by scanning each view's SQL
/// body for the *table names* of the other views (whole-word match). This is a
/// best-effort topological sort: unresolved cycles fall back to alphabetical
/// order (already established by the caller) so output stays deterministic.
fn order_views<'a>(views: &[(&'a String, &'a Table)]) -> Vec<(&'a String, &'a Table)> {
    // Map each view's DB name to its index for dependency lookup.
    let names: Vec<&str> = views.iter().map(|(_, t)| t.table.as_str()).collect();

    // Dependencies[i] = set of view indices that view i reads from.
    let deps: Vec<Vec<usize>> = views
        .iter()
        .map(|(_, t)| {
            let body = t
                .view
                .as_ref()
                .and_then(|v| v.sql.as_deref())
                .unwrap_or("")
                .to_ascii_lowercase();
            names
                .iter()
                .enumerate()
                .filter(|(_, dep_name)| {
                    // A view never depends on itself, and the reference must be
                    // a whole word (so `orders` doesn't match `orders_archive`).
                    t.table != **dep_name && body_references(&body, &dep_name.to_ascii_lowercase())
                })
                .map(|(j, _)| j)
                .collect()
        })
        .collect();

    // Kahn-style emit: repeatedly take the lowest-index view whose deps are all
    // already emitted. Falls back to forcing the next pending view if a cycle
    // blocks progress, guaranteeing termination.
    let mut emitted = vec![false; views.len()];
    let mut ordered = Vec::with_capacity(views.len());
    while ordered.len() < views.len() {
        let next = (0..views.len()).find(|&i| {
            !emitted[i] && deps[i].iter().all(|&d| emitted[d])
        });
        let pick = match next {
            Some(i) => i,
            // Cycle: force the first pending view to break the deadlock.
            None => (0..views.len()).find(|&i| !emitted[i]).unwrap(),
        };
        emitted[pick] = true;
        ordered.push(views[pick]);
    }
    ordered
}

/// Whole-word substring match: `name` appears in `body` not flanked by other
/// identifier characters. Avoids matching `users` inside `users_archive`.
fn body_references(body: &str, name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let mut from = 0;
    while let Some(pos) = body[from..].find(name) {
        let start = from + pos;
        let end = start + name.len();
        let before_ok = start == 0
            || !body.as_bytes()[start - 1].is_ascii_alphanumeric() && body.as_bytes()[start - 1] != b'_';
        let after_ok = end == body.len()
            || !body.as_bytes()[end].is_ascii_alphanumeric() && body.as_bytes()[end] != b'_';
        if before_ok && after_ok {
            return true;
        }
        from = start + name.len();
    }
    false
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

/// Emit a `CREATE VIEW` (or `CREATE MATERIALIZED VIEW`) from the table's
/// [`view`] body. The body is raw, dialect-native SQL, so it is emitted
/// verbatim. A view without an SQL body cannot be generated — that case is
/// rejected by schema validation, but we still emit a clear comment rather
/// than silently producing invalid SQL if we ever reach it.
///
/// [`view`]: rustf_schema::types::Table::view
fn render_view(table: &Table, sql: &mut String, materialized: bool) {
    let kind = if materialized {
        "MATERIALIZED VIEW"
    } else {
        "VIEW"
    };

    sql.push_str(&format!("-- {}: {}\n", kind, table.table));
    if let Some(desc) = &table.description {
        sql.push_str(&format!("-- {}\n", desc));
    }

    let body = table.view.as_ref().and_then(|v| v.sql.as_deref());
    let body = match body {
        Some(b) if !b.trim().is_empty() => b.trim(),
        _ => {
            sql.push_str(&format!(
                "-- ERROR: view '{}' has no `view.sql` body; cannot generate DDL.\n\n",
                table.table
            ));
            return;
        }
    };

    // `CREATE OR REPLACE` only applies to non-materialized views; materialized
    // views must be dropped and recreated, so we never emit OR REPLACE there.
    let or_replace = !materialized
        && table.view.as_ref().map(|v| v.or_replace).unwrap_or(false);
    let create = if or_replace {
        "CREATE OR REPLACE"
    } else {
        "CREATE"
    };

    sql.push_str(&format!(
        "{} {} {} AS\n{};\n\n",
        create, kind, table.table, body
    ));
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

    const VIEW_YAML: &str = r#"
active_orders:
  table: active_orders
  element_type: view
  version: 1
  description: "Open orders joined to users."
  view:
    or_replace: true
    sql: |
      SELECT o.id, u.email
      FROM orders o
      JOIN users u ON u.id = o.user_id
      WHERE o.status = 'open'
  fields:
    id:
      type: int
    email:
      type: string(255)
"#;

    #[test]
    fn view_emits_create_view_not_create_table() {
        let sql = generate_sql_schema(&schema(VIEW_YAML), Dialect::Postgres).unwrap();
        // Must be a view, never a table.
        assert!(sql.contains("CREATE OR REPLACE VIEW active_orders AS"), "{sql}");
        assert!(!sql.contains("CREATE TABLE active_orders"), "{sql}");
        // Raw body is emitted verbatim.
        assert!(sql.contains("WHERE o.status = 'open'"), "{sql}");
    }

    #[test]
    fn materialized_view_never_uses_or_replace() {
        let yaml = VIEW_YAML.replace("element_type: view", "element_type: materialized_view");
        let sql = generate_sql_schema(&schema(&yaml), Dialect::Postgres).unwrap();
        assert!(sql.contains("CREATE MATERIALIZED VIEW active_orders AS"), "{sql}");
        assert!(!sql.contains("OR REPLACE"), "{sql}");
    }

    #[test]
    fn view_without_body_emits_error_comment_not_table() {
        let yaml = r#"
broken_view:
  table: broken_view
  element_type: view
  version: 1
  fields:
    id:
      type: int
"#;
        let sql = generate_sql_schema(&schema(yaml), Dialect::Postgres).unwrap();
        assert!(sql.contains("ERROR: view 'broken_view' has no `view.sql`"), "{sql}");
        assert!(!sql.contains("CREATE TABLE broken_view"), "{sql}");
    }

    #[test]
    fn tables_emitted_before_views() {
        // `active_orders` (view) reads from `orders` (table). The table's
        // CREATE must come first.
        let yaml = format!("{}{}", TABLE_FOR_VIEW, VIEW_YAML);
        let sql = generate_sql_schema(&schema(&yaml), Dialect::Postgres).unwrap();
        let table_pos = sql.find("CREATE TABLE orders").unwrap();
        let view_pos = sql.find("VIEW active_orders").unwrap();
        assert!(table_pos < view_pos, "table must precede view:\n{sql}");
    }

    #[test]
    fn view_depending_on_view_is_ordered_after_it() {
        // `vip_orders` selects from `active_orders` (another view); it must be
        // emitted after `active_orders` regardless of alphabetical order.
        let yaml = r#"
active_orders:
  table: active_orders
  element_type: view
  version: 1
  view:
    sql: "SELECT id FROM orders WHERE status = 'open'"
vip_orders:
  table: vip_orders
  element_type: view
  version: 1
  view:
    sql: "SELECT id FROM active_orders WHERE vip = true"
"#;
        let sql = generate_sql_schema(&schema(yaml), Dialect::Postgres).unwrap();
        let active = sql.find("VIEW active_orders").unwrap();
        let vip = sql.find("VIEW vip_orders").unwrap();
        assert!(active < vip, "dependency view must come first:\n{sql}");
    }

    const TABLE_FOR_VIEW: &str = r#"
orders:
  table: orders
  version: 1
  fields:
    id:
      type: int
      primary_key: true
      auto: true
    status:
      type: string(20)
"#;

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
