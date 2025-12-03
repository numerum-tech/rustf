//! MySQL-specific schema management implementation for RustF CLI

use crate::analyzer::OutputFormat;
use rust_embed::RustEmbed;
use rustf_schema::{Schema, SchemaError};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

#[derive(RustEmbed)]
#[folder = "templates/"]
struct Templates;

/// Escape Rust reserved keywords by adding r# prefix
fn escape_rust_keyword(field_name: &str) -> String {
    const RUST_KEYWORDS: &[&str] = &[
        "type", "match", "if", "else", "while", "for", "loop", "fn", "let", "mut", "const",
        "static", "struct", "enum", "trait", "impl", "mod", "use", "pub", "return", "break",
        "continue", "true", "false", "self", "Self", "super", "crate", "in", "as", "where",
        "async", "await", "dyn", "move", "ref", "macro", "union", "unsafe", "extern", "yield",
        "try", "catch", "typeof",
    ];

    if RUST_KEYWORDS.contains(&field_name) {
        format!("r#{}", field_name)
    } else {
        field_name.to_string()
    }
}

/// Check if a Rust type implements the Copy trait
/// Copy types can be safely returned by value from getters
fn is_copy_type(type_str: &str) -> bool {
    matches!(
        type_str,
        "i8" | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "f32"
            | "f64"
            | "bool"
            | "chrono::DateTime<chrono::Utc>"
            | "DateTime<Utc>"
            | "chrono::NaiveDate"
            | "NaiveDate"
            | "chrono::NaiveTime"
            | "NaiveTime"
    )
}

/// Extract the inner type from Option<T>
/// Example: "Option<i32>" -> "i32"
fn extract_inner_type(option_type: &str) -> &str {
    if let Some(inner) = option_type.strip_prefix("Option<") {
        if let Some(without_bracket) = inner.strip_suffix(">") {
            return without_bracket;
        }
    }
    option_type
}

// Import the command structures from mod.rs
use crate::commands::schema::{GenerateTarget, SchemaAction, SchemaCommand};

/// Execute schema command for MySQL
pub async fn execute_schema_command(cmd: SchemaCommand) -> anyhow::Result<()> {
    match cmd.action {
        SchemaAction::Validate {
            path,
            check_generated,
        } => {
            validate_schema(&path, check_generated).await?;
        }
        SchemaAction::Analyze { path, format } => {
            analyze_schema(&path, format).await?;
        }
        SchemaAction::Generate { target } => {
            generate_code(target).await?;
        }
        SchemaAction::Watch {
            path,
            auto_generate,
        } => {
            watch_schema(&path, auto_generate).await?;
        }
        SchemaAction::CheckConsistency {
            schema_path,
            models_path,
        } => {
            check_consistency(&schema_path, &models_path).await?;
        }
    }
    Ok(())
}

/// Validate schema files
async fn validate_schema(path: &Path, check_generated: bool) -> Result<(), SchemaError> {
    println!("🔍 Validating schema in {:?}...", path);

    if !path.exists() {
        return Err(SchemaError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Schema directory not found: {:?}", path),
        )));
    }

    let schema = Schema::load_from_directory(path).await?;

    println!("✅ Schema loaded successfully!");
    if let Some(meta) = &schema.meta {
        println!("📊 Database: {} (v{})", meta.database_name, meta.version);
    }
    println!("📋 Tables: {}", schema.tables.len());

    println!("🔍 Validating schema consistency...");

    // Use comprehensive validation to collect all errors
    use rustf_schema::SchemaValidator;
    let validation_result = SchemaValidator::validate_comprehensive(&schema)?;

    if validation_result.has_errors() {
        println!(
            "❌ Schema validation failed with {} error(s):",
            validation_result.errors.len()
        );
        for (i, error) in validation_result.errors.iter().enumerate() {
            println!("  {}. {}", i + 1, error);
        }

        if validation_result.has_warnings() {
            println!("⚠️  {} warning(s):", validation_result.warnings.len());
            for (i, warning) in validation_result.warnings.iter().enumerate() {
                println!("  {}. {}", i + 1, warning);
            }
        }

        return validation_result.into_result();
    } else if validation_result.has_warnings() {
        println!(
            "⚠️  Schema validation passed with {} warning(s):",
            validation_result.warnings.len()
        );
        for (i, warning) in validation_result.warnings.iter().enumerate() {
            println!("  {}. {}", i + 1, warning);
        }
        println!("✅ Schema validation completed!");
    } else {
        println!("✅ Schema validation passed!");
    }

    if check_generated {
        println!("🔍 Checking generated code consistency...");
        let models_path = Path::new("src/models");
        if models_path.exists() {
            let checksums = extract_generated_checksums(models_path).await?;
            schema.validate_consistency(&checksums)?;
            println!("✅ Generated code consistency check passed!");
        } else {
            println!("⚠️  No generated models found at {:?}", models_path);
        }
    }

    println!("📈 Summary:");
    println!("  • {} tables defined", schema.tables.len());

    let total_fields: usize = schema.tables.values().map(|t| t.fields.len()).sum();
    println!("  • {} total fields", total_fields);

    let mut relation_count = 0;
    for table in schema.tables.values() {
        if let Some(belongs_to) = &table.relations.belongs_to {
            relation_count += belongs_to.len();
        }
        if let Some(has_many) = &table.relations.has_many {
            relation_count += has_many.len();
        }
        if let Some(has_one) = &table.relations.has_one {
            relation_count += has_one.len();
        }
        if let Some(many_to_many) = &table.relations.many_to_many {
            relation_count += many_to_many.len();
        }
    }
    println!("  • {} relationships", relation_count);

    Ok(())
}

/// Analyze schema structure
async fn analyze_schema(path: &Path, format: OutputFormat) -> Result<(), SchemaError> {
    println!("🔍 Analyzing schema structure...");

    let schema = Schema::load_from_directory(path).await?;

    match format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&schema)?;
            println!("{}", json);
        }
        OutputFormat::Yaml => {
            let yaml = serde_yaml::to_string(&schema)?;
            println!("{}", yaml);
        }
        OutputFormat::Table => {
            print_schema_table(&schema);
        }
        OutputFormat::Markdown => {
            print_schema_markdown(&schema);
        }
    }

    Ok(())
}

/// Generate code from schema
async fn generate_code(target: GenerateTarget) -> anyhow::Result<()> {
    match target {
        GenerateTarget::Models {
            schema_path,
            output,
            force,
            tables,
            exclude,
        } => {
            generate_models(&schema_path, &output, force, tables, exclude).await?;
        }
        GenerateTarget::Migrations {
            schema_path,
            output,
        } => {
            generate_migrations(&schema_path, &output).await?;
        }
    }
    Ok(())
}

/// Generate Rust model structs following RustF framework principles
async fn generate_models(
    schema_path: &Path,
    output_path: &Path,
    force: bool,
    tables_filter: Option<Vec<String>>,
    exclude_filter: Option<Vec<String>>,
) -> anyhow::Result<()> {
    // Create backup if forcing overwrite of existing models
    if force && output_path.exists() && !crate::utils::backup::is_empty_directory(output_path)? {
        use crate::utils::backup::BackupManager;
        let backup_manager = BackupManager::new()?;
        backup_manager.backup_directory(output_path, "models")?;
    }

    println!("🚀 Generating RustF models with base/wrapper pattern...");
    println!("📂 Schema: {:?}", schema_path);
    println!("📁 Output: {:?}", output_path);

    // Check if this is likely a RustF project with models
    let cargo_toml_path = output_path
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("Cargo.toml"))
        .filter(|p| p.exists());

    if let Some(cargo_path) = cargo_toml_path {
        let cargo_content = fs::read_to_string(&cargo_path).await.unwrap_or_default();

        // Check if essential database dependencies are missing
        if !cargo_content.contains("sqlx") {
            println!("\n⚠️  WARNING: Database dependencies not detected in Cargo.toml");
            println!("   Generated models will require these dependencies to compile.");
            println!("   See instructions at the end of generation for details.\n");
        }
    }

    let schema = Schema::load_from_directory(schema_path).await?;

    // Create directory structure: models/base/ (no mod.rs files!)
    let base_path = output_path.join("base");
    if !output_path.exists() {
        fs::create_dir_all(output_path).await?;
        println!("📁 Created models directory: {:?}", output_path);
    }
    if !base_path.exists() {
        fs::create_dir_all(&base_path).await?;
        println!("📁 Created base directory: {:?}", base_path);
    }

    let mut generated_files = 0;
    let mut wrapper_files = 0;
    let schema_checksum = schema.checksum();
    let generation_time = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    // Multi-database support: detect type per table, with fallbacks
    fn detect_database_type(
        table: &rustf_schema::Table,
        meta: Option<&rustf_schema::SchemaMeta>,
    ) -> &'static str {
        // 1. First: Check table's own database_type (supports multi-database)
        if let Some(ref db_type) = table.database_type {
            return match db_type.as_str() {
                "mysql" => "MySql",
                "postgres" | "postgresql" => "Postgres",
                "sqlite" => "Sqlite",
                _ => "Postgres", // Default fallback
            };
        }

        // 2. Fallback: Check _meta.yaml for database information
        if let Some(meta) = meta {
            // Temporarily use description for type detection until schema is updated
            if let Some(desc) = &meta.description {
                if desc.contains("MySQL") || desc.contains("mysql") {
                    return "MySql";
                } else if desc.contains("SQLite") || desc.contains("sqlite") {
                    return "Sqlite";
                } else if desc.contains("PostgreSQL") || desc.contains("postgres") {
                    return "Postgres";
                }
            }
        }

        // 5. Final fallback
        "Postgres"
    }

    // Filter tables based on --tables and --exclude options
    let tables_to_generate: Vec<_> = schema
        .tables
        .iter()
        .filter(|(_, table)| {
            // Check if table should be included (using database table name)
            if let Some(ref include_list) = tables_filter {
                if !include_list
                    .iter()
                    .any(|t| t.eq_ignore_ascii_case(&table.table))
                {
                    return false;
                }
            }
            // Check if table should be excluded (using database table name)
            if let Some(ref exclude_list) = exclude_filter {
                if exclude_list
                    .iter()
                    .any(|t| t.eq_ignore_ascii_case(&table.table))
                {
                    return false;
                }
            }
            true
        })
        .collect();

    if tables_to_generate.is_empty() {
        println!("⚠️  No tables matched the filter criteria");
        return Ok(());
    }

    println!(
        "📊 Generating models for {} table(s)",
        tables_to_generate.len()
    );

    for (_table_name, table) in tables_to_generate {
        // Detect database type per table (supports multi-database scenarios)
        let pool_type = detect_database_type(table, schema.meta.as_ref());

        // 1. Generate base model: base/{table}.inc.rs (always overwritten)
        let base_model_file = base_path.join(format!("{}.inc.rs", &table.table));
        let base_code = generate_base_model_include_code(
            table,
            &schema,
            &schema_checksum,
            &generation_time,
            pool_type,
        )?;
        fs::write(&base_model_file, base_code).await?;
        println!(
            "✅ Generated base include file: {}",
            base_model_file.display()
        );
        generated_files += 1;

        // 2. Generate wrapper model: {table}.rs (only if doesn't exist)
        let wrapper_model_file = output_path.join(format!("{}.rs", &table.table));
        if !wrapper_model_file.exists() || force {
            let wrapper_code = generate_wrapper_model_include_code(table, pool_type)?;
            fs::write(&wrapper_model_file, wrapper_code).await?;
            println!(
                "✅ Generated wrapper model: {}",
                wrapper_model_file.display()
            );
            wrapper_files += 1;
        } else {
            println!(
                "⚠️  Preserving existing business logic: {}",
                wrapper_model_file.display()
            );
        }
    }

    // 3. Generate MODELS_README.md (always overwritten for latest guidelines)
    let readme_file = output_path.join("MODELS_README.md");
    let readme_content = generate_models_readme(&schema);
    fs::write(&readme_file, readme_content).await?;
    println!("✅ Generated documentation: {}", readme_file.display());

    println!("🎉 Model generation completed!");
    println!(
        "📊 Generated {} base models and {} wrapper models",
        generated_files, wrapper_files
    );
    println!("📝 Base models: Complete CRUD operations (will be overwritten)");
    println!("🔧 Wrapper models: Add your business logic here (preserved)");
    println!("⚠️  Only files in base/ will be overwritten on regeneration");
    println!("📖 Read MODELS_README.md for detailed instructions");

    // Check if we generated models with database dependencies
    if generated_files > 0 {
        println!("\n⚠️  IMPORTANT: Enable required dependencies in your Cargo.toml:");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("# Required for database models:");
        println!("sqlx = {{ version = \"0.8\", features = [\"runtime-tokio-rustls\", \"mysql\", \"postgres\", \"sqlite\", \"chrono\", \"rust_decimal\", \"uuid\"] }}");
        println!("anyhow = \"1.0\"");
        println!("chrono = {{ version = \"0.4\", features = [\"serde\"] }}");
        println!("rust_decimal = {{ version = \"1.32\", features = [\"serde\"] }}");
        println!("uuid = {{ version = \"1.0\", features = [\"v4\", \"serde\"] }}");
        println!("\n💡 TIP: Adjust SQLx features based on your database:");
        println!("   • MySQL: \"mysql\"");
        println!("   • PostgreSQL: \"postgres\"");
        println!("   • SQLite: \"sqlite\"");
        println!("\n🔧 FOR COMPILATION: SQLx requires one of these options:");
        println!("   1. Set DATABASE_URL environment variable:");
        println!("      export DATABASE_URL=\"mysql://user:pass@localhost/dbname\"");
        println!("   2. OR use offline mode (recommended for CI/CD):");
        println!("      cargo sqlx prepare --database-url=\"your_db_url\"");
        println!("      # This creates .sqlx/ directory with query metadata");
        println!("   3. OR disable compile-time checks (add to Cargo.toml):");
        println!("      [env]");
        println!("      SQLX_OFFLINE = \"true\"");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    }

    Ok(())
}

/// Generate SQL migrations
async fn generate_migrations(schema_path: &Path, output_path: &Path) -> anyhow::Result<()> {
    println!("🚀 Generating SQL migrations...");

    let schema = Schema::load_from_directory(schema_path).await?;

    // Create output directory if it doesn't exist
    if !output_path.exists() {
        fs::create_dir_all(output_path).await?;
    }

    // Generate initial migration
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let migration_file = output_path.join(format!("{}_initial_schema.sql", timestamp));

    let sql_code = generate_sql_schema(&schema)?;
    fs::write(&migration_file, sql_code).await?;

    println!("✅ Generated {}", migration_file.display());
    println!("🎉 Migration generated successfully!");

    Ok(())
}

/// Watch schema files for changes
async fn watch_schema(path: &Path, auto_generate: bool) -> anyhow::Result<()> {
    println!("👀 Watching schema directory: {:?}", path);

    if auto_generate {
        println!("🔄 Auto-generation enabled");
    }

    // This would use the existing file watcher from the CLI
    // For now, just print a message
    println!("⚠️  Watch mode not yet implemented");
    println!("💡 Use 'rustf-cli watch' for file watching functionality");

    Ok(())
}

/// Check consistency between schema and generated code
async fn check_consistency(schema_path: &Path, models_path: &Path) -> anyhow::Result<()> {
    println!("🔍 Checking consistency between schema and generated code...");
    println!("📂 Schema: {:?}", schema_path);
    println!("📁 Models: {:?}", models_path);

    let schema = Schema::load_from_directory(schema_path).await?;

    if !models_path.exists() {
        println!("❌ Models directory not found: {:?}", models_path);
        println!("💡 Run 'rustf-cli schema generate models' to generate models");
        return Ok(());
    }

    let generated_checksums = extract_generated_checksums(models_path).await?;

    match schema.validate_consistency(&generated_checksums) {
        Ok(()) => {
            println!("✅ Schema and generated code are consistent!");
            println!("📈 {} models validated", generated_checksums.len());
        }
        Err(SchemaError::Consistency(msg)) => {
            println!("❌ Consistency check failed: {}", msg);
            println!("💡 Run 'rustf-cli schema generate models --force' to regenerate models");
            return Err(anyhow::anyhow!("Consistency check failed"));
        }
        Err(e) => return Err(e.into()),
    }

    Ok(())
}

/// Extract checksums from generated model files
async fn extract_generated_checksums(
    models_path: &Path,
) -> Result<HashMap<String, String>, SchemaError> {
    let mut checksums = HashMap::new();

    // Look for generated files in the generated/ subfolder first
    let generated_path = models_path.join("generated");
    if generated_path.exists() {
        let mut entries = fs::read_dir(&generated_path)
            .await
            .map_err(SchemaError::Io)?;

        while let Some(entry) = entries.next_entry().await.map_err(SchemaError::Io)? {
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "rs")
                && path.file_name().map_or(false, |name| name != "mod.rs")
            {
                let content = fs::read_to_string(&path).await.map_err(SchemaError::Io)?;

                // Extract checksum from comment at top of file
                if let Some(checksum_line) = content
                    .lines()
                    .find(|line| line.starts_with("// Schema checksum:"))
                {
                    if let Some(checksum) = checksum_line.strip_prefix("// Schema checksum: ") {
                        // Extract table name from filename (remove _generated suffix)
                        if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
                            let table_name =
                                file_stem.strip_suffix("_generated").unwrap_or(file_stem);
                            // Convert table name to model name (PascalCase)
                            let model_name = table_name
                                .split('_')
                                .map(|part| {
                                    let mut chars = part.chars();
                                    match chars.next() {
                                        None => String::new(),
                                        Some(first) => {
                                            first.to_uppercase().collect::<String>()
                                                + chars.as_str()
                                        }
                                    }
                                })
                                .collect::<String>();

                            checksums.insert(model_name, checksum.trim().to_string());
                        }
                    }
                }
            }
        }
    } else {
        // Fallback to old structure (flat files in models directory)
        let mut entries = fs::read_dir(models_path).await.map_err(SchemaError::Io)?;

        while let Some(entry) = entries.next_entry().await.map_err(SchemaError::Io)? {
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "rs")
                && path.file_name().map_or(false, |name| name != "mod.rs")
            {
                let content = fs::read_to_string(&path).await.map_err(SchemaError::Io)?;

                // Extract checksum from comment at top of file
                if let Some(checksum_line) = content
                    .lines()
                    .find(|line| line.starts_with("// Schema checksum:"))
                {
                    if let Some(checksum) = checksum_line.strip_prefix("// Schema checksum: ") {
                        // Extract table name from filename
                        if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
                            // Convert table name back to model name (simple heuristic)
                            let model_name = file_stem
                                .split('_')
                                .map(|part| {
                                    let mut chars = part.chars();
                                    match chars.next() {
                                        None => String::new(),
                                        Some(first) => {
                                            first.to_uppercase().collect::<String>()
                                                + chars.as_str()
                                        }
                                    }
                                })
                                .collect::<String>();

                            checksums.insert(model_name, checksum.trim().to_string());
                        }
                    }
                }
            }
        }
    }

    Ok(checksums)
}

/// Generate complete model code with full CRUD operations (RustF style)
// removed unused function generate_complete_model_code
/// Generate manual model wrapper code (for business logic)
// removed unused function generate_manual_model_code
/// Generate Rust model code for a table (legacy function for compatibility)
// removed unused function generate_model_code

/// Generate SQL schema
fn generate_sql_schema(schema: &Schema) -> anyhow::Result<String> {
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
    sql.push_str("-- DO NOT EDIT - Auto-generated from schema\n\n");

    // Create tables in dependency order (simplified)
    for (_table_name, table) in &schema.tables {
        sql.push_str(&format!("-- Table: {}\n", table.table));
        if let Some(desc) = &table.description {
            sql.push_str(&format!("-- {}\n", desc));
        }

        sql.push_str(&format!("CREATE TABLE {} (\n", table.table));

        let mut field_definitions = Vec::new();

        for (field_name, field) in &table.fields {
            let sql_type = field_type_to_sql(&field.field_type);
            let mut definition = format!("    {} {}", field_name, sql_type);

            if field.constraints.required == Some(true) || field.constraints.nullable != Some(true)
            {
                definition.push_str(" NOT NULL");
            }

            if field.constraints.primary_key == Some(true) {
                definition.push_str(" PRIMARY KEY");
            }

            if field.constraints.unique == Some(true) && field.constraints.primary_key != Some(true)
            {
                definition.push_str(" UNIQUE");
            }

            if let Some(default) = &field.constraints.default {
                match default {
                    serde_json::Value::String(s) => {
                        definition.push_str(&format!(" DEFAULT '{}'", s))
                    }
                    serde_json::Value::Number(n) => definition.push_str(&format!(" DEFAULT {}", n)),
                    serde_json::Value::Bool(b) => definition.push_str(&format!(" DEFAULT {}", b)),
                    _ => {}
                }
            }

            field_definitions.push(definition);
        }

        sql.push_str(&field_definitions.join(",\n"));
        sql.push_str("\n);\n\n");
    }

    Ok(sql)
}

/// Convert field type to Rust type
// removed unused function field_type_to_rust
/// Convert field type to SQL type
fn field_type_to_sql(field_type: &rustf_schema::types::FieldType) -> String {
    match field_type {
        rustf_schema::types::FieldType::Simple(t) => {
            match t.as_str() {
                "int" | "integer" => "INTEGER".to_string(),
                "bigint" => "BIGINT".to_string(),
                "serial" => "SERIAL".to_string(),
                "string" | "text" => "TEXT".to_string(),
                "decimal" => "DECIMAL".to_string(),
                "float" => "REAL".to_string(),
                "double" => "DOUBLE PRECISION".to_string(),
                "boolean" | "bool" => "BOOLEAN".to_string(),
                "timestamp" | "datetime" => "TIMESTAMP".to_string(),
                "date" => "DATE".to_string(),
                "time" => "TIME".to_string(),
                "json" => "JSON".to_string(),
                "jsonb" => "JSONB".to_string(),
                "uuid" => "UUID".to_string(),
                "blob" => "BYTEA".to_string(),
                "enum" => "TEXT".to_string(), // Will add CHECK constraint
                _ => "TEXT".to_string(),
            }
        }
        rustf_schema::types::FieldType::Parameterized { base_type, params } => {
            match base_type.as_str() {
                "string" | "varchar" => {
                    if let Some(rustf_schema::types::TypeParam::Number(len)) = params.first() {
                        format!("VARCHAR({})", len)
                    } else {
                        "VARCHAR".to_string()
                    }
                }
                "decimal" => {
                    if params.len() >= 2 {
                        if let (
                            Some(rustf_schema::types::TypeParam::Number(p)),
                            Some(rustf_schema::types::TypeParam::Number(s)),
                        ) = (params.get(0), params.get(1))
                        {
                            format!("DECIMAL({},{})", p, s)
                        } else {
                            "DECIMAL".to_string()
                        }
                    } else {
                        "DECIMAL".to_string()
                    }
                }
                _ => "TEXT".to_string(),
            }
        }
        rustf_schema::types::FieldType::Enum { values, .. } => {
            format!(
                "TEXT CHECK ({} IN ({}))",
                "column_name", // This would need the actual column name
                values
                    .iter()
                    .map(|v| format!("'{}'", v))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
        rustf_schema::types::FieldType::Json { .. } => "JSON".to_string(),
    }
}

/// Print schema in table format
fn print_schema_table(schema: &Schema) {
    if let Some(meta) = &schema.meta {
        println!("📊 Database: {} (v{})", meta.database_name, meta.version);
        if let Some(desc) = &meta.description {
            println!("📝 Description: {}", desc);
        }
        println!();
    }

    for (table_name, table) in &schema.tables {
        println!("📋 Table: {} ({})", table_name, table.table);
        if let Some(desc) = &table.description {
            println!("   Description: {}", desc);
        }

        println!("   Fields:");
        for (field_name, field) in &table.fields {
            let mut flags = Vec::new();
            if field.constraints.primary_key == Some(true) {
                flags.push("PK");
            }
            if field.constraints.unique == Some(true) {
                flags.push("UNIQUE");
            }
            if field.constraints.required == Some(true) {
                flags.push("NOT NULL");
            }

            let flags_str = if flags.is_empty() {
                String::new()
            } else {
                format!(" ({})", flags.join(", "))
            };

            println!(
                "     • {}: {}{}",
                field_name,
                field.field_type.base_type(),
                flags_str
            );

            if let Some(ai_hint) = &field.ai {
                println!("       💡 {}", ai_hint);
            }
        }

        // Relations
        if let Some(belongs_to) = &table.relations.belongs_to {
            if !belongs_to.is_empty() {
                println!("   Belongs to:");
                for (rel_name, rel) in belongs_to {
                    println!(
                        "     👆 {}: {}.{} -> {}.{}",
                        rel_name, table.name, rel.local_field, rel.model, rel.foreign_field
                    );
                }
            }
        }

        if let Some(has_many) = &table.relations.has_many {
            if !has_many.is_empty() {
                println!("   Has many:");
                for (rel_name, rel) in has_many {
                    println!(
                        "     👇 {}: {}.{} <- {}.{}",
                        rel_name, table.name, rel.local_field, rel.model, rel.foreign_field
                    );
                }
            }
        }

        println!();
    }
}

/// Print schema in Markdown format
fn print_schema_markdown(schema: &Schema) {
    println!("# Database Schema");
    println!();

    if let Some(meta) = &schema.meta {
        println!("**Database:** {} v{}", meta.database_name, meta.version);
        if let Some(desc) = &meta.description {
            println!("**Description:** {}", desc);
        }
        println!();
    }

    for (table_name, table) in &schema.tables {
        println!("## {}", table_name);

        if let Some(desc) = &table.description {
            println!("{}", desc);
            println!();
        }

        if let Some(ai_context) = &table.ai_context {
            println!("**AI Context:** {}", ai_context);
            println!();
        }

        println!("**Table:** `{}`", table.table);
        println!();

        // Fields table
        println!("| Field | Type | Constraints | Description |");
        println!("|-------|------|-------------|-------------|");

        for (field_name, field) in &table.fields {
            let mut constraints = Vec::new();
            if field.constraints.primary_key == Some(true) {
                constraints.push("PK");
            }
            if field.constraints.unique == Some(true) {
                constraints.push("UNIQUE");
            }
            if field.constraints.required == Some(true) {
                constraints.push("NOT NULL");
            }

            let constraints_str = constraints.join(", ");
            let description = field.ai.as_deref().unwrap_or("");

            println!(
                "| `{}` | `{}` | {} | {} |",
                field_name,
                field.field_type.base_type(),
                constraints_str,
                description
            );
        }

        println!();

        // Relations

        if let Some(belongs_to) = &table.relations.belongs_to {
            if !belongs_to.is_empty() {
                println!("### Relations");
                println!();

                println!("**Belongs to:**");
                for (rel_name, rel) in belongs_to {
                    println!(
                        "- `{}`: {}.{} → {}.{}",
                        rel_name, table.name, rel.local_field, rel.model, rel.foreign_field
                    );
                }
                println!();
            }
        }

        if let Some(has_many) = &table.relations.has_many {
            if !has_many.is_empty() {
                println!("### Relations");
                println!();

                println!("**Has many:**");
                for (rel_name, rel) in has_many {
                    println!(
                        "- `{}`: {}.{} ← {}.{}",
                        rel_name, table.name, rel.local_field, rel.model, rel.foreign_field
                    );
                }
                println!();
            }
        }

        println!("---");
        println!();
    }
}

/// Process a template by replacing placeholders with values
fn process_template(content: &str, variables: &HashMap<String, String>) -> String {
    let mut processed = content.to_string();

    log::debug!("Processing template with {} variables", variables.len());
    for (key, value) in variables {
        let placeholder = format!("{{{{{}}}}}", key);
        let display_value = if value.len() > 100 {
            format!("{}...", &value[..100])
        } else {
            value.clone()
        };
        log::debug!(
            "Replacing '{}' with '{}' (length: {})",
            placeholder,
            display_value,
            value.len()
        );
        processed = processed.replace(&placeholder, value);
    }

    // Check for unreplaced variables
    let remaining_vars = processed.matches("{{").count();
    if remaining_vars > 0 {
        log::warn!(
            "Template still contains {} unreplaced variables",
            remaining_vars
        );
    }

    processed
}

/// Prepare template variables for base model generation
fn prepare_base_model_variables(
    table: &rustf_schema::Table,
    _schema: &Schema,
    checksum: &str,
    generation_time: &str,
    pool_type: &str,
) -> HashMap<String, String> {
    let mut vars = HashMap::new();

    // Basic metadata
    vars.insert("model_name".to_string(), table.name.clone());
    vars.insert("table_name".to_string(), table.table.clone());
    vars.insert("pool_type".to_string(), pool_type.to_string());
    vars.insert("pool_getter".to_string(), "mysql_pool".to_string()); // MySQL specific pool getter
    vars.insert("checksum".to_string(), checksum.to_string());
    vars.insert("generation_time".to_string(), generation_time.to_string());
    vars.insert("cli_version".to_string(), "0.1.0".to_string());

    // Add table-level documentation
    let mut table_doc = Vec::new();
    log::debug!("Preparing table documentation for '{}'", table.name);
    if let Some(ai_context) = &table.ai_context {
        log::debug!("Found ai_context: '{}'", ai_context);
        table_doc.push(ai_context.clone());
    } else {
        log::debug!("No ai_context found");
    }
    if let Some(description) = &table.description {
        if !description.is_empty() {
            log::debug!("Found description: '{}'", description);
            table_doc.push(format!("Database: {}", description));
        }
    } else {
        log::debug!("No description found");
    }

    let table_documentation = if table_doc.is_empty() {
        "Auto-generated model from schema".to_string()
    } else {
        table_doc.join("\n/// ")
    };
    log::debug!("Final table documentation: '{}'", table_documentation);
    vars.insert("table_documentation".to_string(), table_documentation);

    // Sort fields alphabetically once for consistent ordering throughout generation
    let mut sorted_fields: Vec<_> = table.fields.values().collect();
    sorted_fields.sort_by(|a, b| a.name.cmp(&b.name));
    log::debug!(
        "Sorted fields order for {}: {:?}",
        table.name,
        sorted_fields.iter().map(|f| &f.name).collect::<Vec<_>>()
    );

    // Determine primary key field (detect early for use throughout generation)
    let primary_key = table
        .fields
        .values()
        .find(|f| f.constraints.primary_key.unwrap_or(false))
        .map(|f| f.name.clone())
        .unwrap_or_else(|| "id".to_string());
    vars.insert("primary_key".to_string(), primary_key.clone());

    // Determine ID type and whether this model has a primary key field
    let has_primary_key = table
        .fields
        .values()
        .any(|f| f.constraints.primary_key.unwrap_or(false));
    
    let id_type = if let Some(pk_field) = table
        .fields
        .values()
        .find(|f| f.constraints.primary_key.unwrap_or(false))
    {
        if let Some(ref lang_type) = pk_field.lang_type {
            lang_type.clone()
        } else {
            match pk_field.field_type.base_type() {
                "int" | "integer" => "i32",
                "bigint" => "i64",
                "string" => "String",
                _ => "i32",
            }
            .to_string()
        }
    } else {
        "i32".to_string() // Default type for models without primary key
    };
    vars.insert("id_type".to_string(), id_type.clone());

    // Generate id method implementation based on whether primary key field exists
    let id_method_impl = if has_primary_key {
        // Use the primary key field name dynamically
        let escaped_pk_field = escape_rust_keyword(&primary_key);
        // Clone String IDs to avoid move out of borrowed content
        if id_type == "String" {
            format!("        self.{}.clone()", escaped_pk_field)
        } else {
            format!("        self.{}", escaped_pk_field)
        }
    } else {
        "        panic!(\"This model does not have a primary key field - it is likely a database view or composite entity\")".to_string()
    };
    vars.insert("id_method_impl".to_string(), id_method_impl);

    // Generate imports
    let mut imports = vec![
        "use serde::{Deserialize, Serialize};".to_string(),
        format!("use sqlx::{{Pool, {}}};", pool_type),
        "use anyhow::Result;".to_string(),
    ];

    // Check if we need chrono or other imports
    let needs_datetime = table
        .fields
        .values()
        .any(|f| matches!(f.field_type.base_type(), "timestamp" | "datetime"));
    let needs_date = table
        .fields
        .values()
        .any(|f| f.field_type.base_type() == "date");
    let needs_time = table
        .fields
        .values()
        .any(|f| f.field_type.base_type() == "time");

    if needs_datetime || needs_date || needs_time {
        let mut chrono_imports = vec![];
        if needs_datetime {
            chrono_imports.push("DateTime");
            chrono_imports.push("Utc");
        }
        if needs_date {
            chrono_imports.push("NaiveDate");
        }
        if needs_time {
            chrono_imports.push("NaiveTime");
        }
        imports.push(format!("use chrono::{{{}}};", chrono_imports.join(", ")));
    }

    let needs_decimal = table
        .fields
        .values()
        .any(|f| f.field_type.base_type() == "decimal");

    if needs_decimal {
        imports.push("use rust_decimal::Decimal;".to_string());
    }

    let needs_json = table
        .fields
        .values()
        .any(|f| f.field_type.base_type() == "json");

    if needs_json {
        imports.push("use serde_json;".to_string());
    }

    let needs_uuid = table
        .fields
        .values()
        .any(|f| f.field_type.base_type() == "uuid");

    if needs_uuid {
        imports.push("use uuid::Uuid;".to_string());
    }

    vars.insert("imports".to_string(), imports.join("\n"));

    // NOTE: Field hints and validation rules are now accessed via CLI command:
    // rustf-cli model-metadata <ModelName> --format json
    // This eliminates runtime overhead while providing better AI development experience

    // Generate struct fields with documentation
    let mut struct_fields = Vec::new();
    // Generate struct fields - documentation is handled elsewhere
    for field in &sorted_fields {
        let is_nullable = field.constraints.nullable.unwrap_or(false);

        let rust_type = if let Some(ref lang_type) = field.lang_type {
            lang_type.clone()
        } else {
            match field.field_type.base_type() {
                // MySQL supports both signed and unsigned integers
                "tinyint" => {
                    if is_nullable {
                        "Option<i8>"
                    } else {
                        "i8"
                    }
                }
                "smallint" => {
                    if is_nullable {
                        "Option<i16>"
                    } else {
                        "i16"
                    }
                }
                "mediumint" | "int" | "integer" => {
                    if is_nullable {
                        "Option<i32>"
                    } else {
                        "i32"
                    }
                }
                "bigint" => {
                    if is_nullable {
                        "Option<i64>"
                    } else {
                        "i64"
                    }
                }

                // MySQL unsigned integers
                "unsigned_tinyint" | "tinyint_unsigned" => {
                    if is_nullable {
                        "Option<u8>"
                    } else {
                        "u8"
                    }
                }
                "unsigned_smallint" | "smallint_unsigned" => {
                    if is_nullable {
                        "Option<u16>"
                    } else {
                        "u16"
                    }
                }
                "unsigned_int" | "unsigned_integer" | "int_unsigned" => {
                    if is_nullable {
                        "Option<u32>"
                    } else {
                        "u32"
                    }
                }
                "unsigned_bigint" | "bigint_unsigned" => {
                    if is_nullable {
                        "Option<u64>"
                    } else {
                        "u64"
                    }
                }

                // Floating point types
                "float" => {
                    if is_nullable {
                        "Option<f32>"
                    } else {
                        "f32"
                    }
                }
                "double" | "real" => {
                    if is_nullable {
                        "Option<f64>"
                    } else {
                        "f64"
                    }
                }
                "decimal" | "numeric" => {
                    if is_nullable {
                        "Option<Decimal>"
                    } else {
                        "Decimal"
                    }
                }

                // Text types - MySQL has various text sizes
                "varchar" | "char" | "string" => {
                    if is_nullable {
                        "Option<String>"
                    } else {
                        "String"
                    }
                }
                "text" | "tinytext" | "mediumtext" | "longtext" => {
                    if is_nullable {
                        "Option<String>"
                    } else {
                        "String"
                    }
                }

                // Boolean (MySQL often uses TINYINT(1))
                "boolean" | "bool" => {
                    if is_nullable {
                        "Option<bool>"
                    } else {
                        "bool"
                    }
                }

                // Date/Time types
                "timestamp" | "datetime" => {
                    if is_nullable {
                        "Option<DateTime<Utc>>"
                    } else {
                        "DateTime<Utc>"
                    }
                }
                "date" => {
                    if is_nullable {
                        "Option<NaiveDate>"
                    } else {
                        "NaiveDate"
                    }
                }
                "time" => {
                    if is_nullable {
                        "Option<NaiveTime>"
                    } else {
                        "NaiveTime"
                    }
                }
                "year" => {
                    if is_nullable {
                        "Option<i16>"
                    } else {
                        "i16"
                    }
                } // MySQL YEAR type

                // Semantic types
                "json" => {
                    if is_nullable {
                        "Option<serde_json::Value>"
                    } else {
                        "serde_json::Value"
                    }
                }
                "uuid" => {
                    if is_nullable {
                        "Option<Uuid>"
                    } else {
                        "Uuid"
                    }
                }
                "enum" | "set" => {
                    if is_nullable {
                        "Option<String>"
                    } else {
                        "String"
                    }
                }

                // Binary
                "binary" | "varbinary" | "blob" | "tinyblob" | "mediumblob" | "longblob" => {
                    if is_nullable {
                        "Option<Vec<u8>>"
                    } else {
                        "Vec<u8>"
                    }
                }

                // Default fallback
                _ => {
                    if is_nullable {
                        "Option<String>"
                    } else {
                        "String"
                    }
                }
            }
            .to_string()
        };

        let escaped_field_name = escape_rust_keyword(&field.name);

        // Add field documentation from AI hints and other metadata
        let mut field_doc = Vec::new();

        // Add AI hint if present
        if let Some(ai_hint) = &field.ai {
            field_doc.push(format!("    /// {}", ai_hint));
        }

        // Database comments would be from the original DB schema (not available in Field struct yet)
        // This could be added in the future when we extend the Field structure

        // Add field type info
        field_doc.push(format!(
            "    /// Type: {:?} ({})",
            field.field_type, rust_type
        ));

        // Add validation info
        if field.constraints.required.unwrap_or(false) {
            field_doc.push("    /// Required field".to_string());
        }
        if field.constraints.unique.unwrap_or(false) {
            field_doc.push("    /// Unique constraint".to_string());
        }
        if field.constraints.primary_key.unwrap_or(false) {
            field_doc.push("    /// Primary key".to_string());
        }

        // Add foreign key info
        if let Some(fk) = &field.constraints.foreign_key {
            field_doc.push(format!("    /// Foreign key: {}", fk));
        }

        let field_declaration = if field_doc.is_empty() {
            format!("    pub {}: {},", escaped_field_name, rust_type)
        } else {
            format!(
                "{}\n    pub {}: {},",
                field_doc.join("\n"),
                escaped_field_name,
                rust_type
            )
        };

        struct_fields.push(field_declaration);
    }
    vars.insert("struct_fields".to_string(), struct_fields.join("\n"));
    // Template compatibility - some templates use struct_fields_with_docs
    vars.insert(
        "struct_fields_with_docs".to_string(),
        struct_fields.join("\n"),
    );

    // Generate type constants
    let mut type_constants = Vec::new();
    for field in &sorted_fields {
        let is_nullable = field.constraints.nullable.unwrap_or(false);

        let rust_type = if let Some(ref lang_type) = field.lang_type {
            lang_type.clone()
        } else {
            match field.field_type.base_type() {
                // MySQL supports both signed and unsigned integers
                "tinyint" => {
                    if is_nullable {
                        "Option<i8>"
                    } else {
                        "i8"
                    }
                }
                "smallint" => {
                    if is_nullable {
                        "Option<i16>"
                    } else {
                        "i16"
                    }
                }
                "mediumint" | "int" | "integer" => {
                    if is_nullable {
                        "Option<i32>"
                    } else {
                        "i32"
                    }
                }
                "bigint" => {
                    if is_nullable {
                        "Option<i64>"
                    } else {
                        "i64"
                    }
                }

                // MySQL unsigned integers
                "unsigned_tinyint" | "tinyint_unsigned" => {
                    if is_nullable {
                        "Option<u8>"
                    } else {
                        "u8"
                    }
                }
                "unsigned_smallint" | "smallint_unsigned" => {
                    if is_nullable {
                        "Option<u16>"
                    } else {
                        "u16"
                    }
                }
                "unsigned_int" | "unsigned_integer" | "int_unsigned" => {
                    if is_nullable {
                        "Option<u32>"
                    } else {
                        "u32"
                    }
                }
                "unsigned_bigint" | "bigint_unsigned" => {
                    if is_nullable {
                        "Option<u64>"
                    } else {
                        "u64"
                    }
                }

                // Floating point types
                "float" => {
                    if is_nullable {
                        "Option<f32>"
                    } else {
                        "f32"
                    }
                }
                "double" | "real" => {
                    if is_nullable {
                        "Option<f64>"
                    } else {
                        "f64"
                    }
                }
                "decimal" | "numeric" => {
                    if is_nullable {
                        "Option<Decimal>"
                    } else {
                        "Decimal"
                    }
                }

                // Text types - MySQL has various text sizes
                "varchar" | "char" | "string" => {
                    if is_nullable {
                        "Option<String>"
                    } else {
                        "String"
                    }
                }
                "text" | "tinytext" | "mediumtext" | "longtext" => {
                    if is_nullable {
                        "Option<String>"
                    } else {
                        "String"
                    }
                }

                // Boolean (MySQL often uses TINYINT(1))
                "boolean" | "bool" => {
                    if is_nullable {
                        "Option<bool>"
                    } else {
                        "bool"
                    }
                }

                // Date/Time types
                "timestamp" | "datetime" => {
                    if is_nullable {
                        "Option<DateTime<Utc>>"
                    } else {
                        "DateTime<Utc>"
                    }
                }
                "date" => {
                    if is_nullable {
                        "Option<NaiveDate>"
                    } else {
                        "NaiveDate"
                    }
                }
                "time" => {
                    if is_nullable {
                        "Option<NaiveTime>"
                    } else {
                        "NaiveTime"
                    }
                }
                "year" => {
                    if is_nullable {
                        "Option<i16>"
                    } else {
                        "i16"
                    }
                } // MySQL YEAR type

                // Semantic types
                "json" => {
                    if is_nullable {
                        "Option<serde_json::Value>"
                    } else {
                        "serde_json::Value"
                    }
                }
                "uuid" => {
                    if is_nullable {
                        "Option<Uuid>"
                    } else {
                        "Uuid"
                    }
                }
                "enum" | "set" => {
                    if is_nullable {
                        "Option<String>"
                    } else {
                        "String"
                    }
                }

                // Binary
                "binary" | "varbinary" | "blob" | "tinyblob" | "mediumblob" | "longblob" => {
                    if is_nullable {
                        "Option<Vec<u8>>"
                    } else {
                        "Vec<u8>"
                    }
                }

                // Default fallback
                _ => {
                    if is_nullable {
                        "Option<String>"
                    } else {
                        "String"
                    }
                }
            }
            .to_string()
        };

        let escaped_field_name = escape_rust_keyword(&field.name);
        type_constants.push(format!(
            "    pub type {} = {};",
            escaped_field_name, rust_type
        ));
    }
    vars.insert("type_constants".to_string(), type_constants.join("\n"));

    // Type imports for the Types module
    let mut type_imports = Vec::new();
    if needs_datetime || needs_date || needs_time {
        let mut chrono_imports = vec![];
        if needs_datetime {
            chrono_imports.push("DateTime");
            chrono_imports.push("Utc");
        }
        if needs_date {
            chrono_imports.push("NaiveDate");
        }
        if needs_time {
            chrono_imports.push("NaiveTime");
        }
        type_imports.push(format!(
            "    use chrono::{{{}}};",
            chrono_imports.join(", ")
        ));
    }
    if needs_decimal {
        type_imports.push("    use rust_decimal::Decimal;".to_string());
    }
    if needs_uuid {
        type_imports.push("    use uuid::Uuid;".to_string());
    }
    vars.insert("type_imports".to_string(), type_imports.join("\n"));

    // Generate column constants for type-safe queries
    let mut column_constants = Vec::new();
    for field in &sorted_fields {
        let field_name = &field.name;
        let const_name = field_name.to_uppercase();
        column_constants.push(format!(
            "    pub const {}: &'static str = \"{}\";",
            const_name, field_name
        ));
    }
    vars.insert("column_constants".to_string(), column_constants.join("\n"));

    // Generate builder fields
    let mut builder_fields = Vec::new();
    for field in &sorted_fields {
        let is_nullable = field.constraints.nullable.unwrap_or(false);
        let is_auto = field.constraints.auto.is_some();

        // Skip auto-generated fields (but keep non-auto ID fields like UUIDs)
        if is_auto {
            continue;
        }

        let rust_type = if let Some(ref lang_type) = field.lang_type {
            lang_type.clone()
        } else {
            match field.field_type.base_type() {
                // MySQL supports both signed and unsigned integers
                "tinyint" => {
                    if is_nullable {
                        "Option<i8>"
                    } else {
                        "i8"
                    }
                }
                "smallint" => {
                    if is_nullable {
                        "Option<i16>"
                    } else {
                        "i16"
                    }
                }
                "mediumint" | "int" | "integer" => {
                    if is_nullable {
                        "Option<i32>"
                    } else {
                        "i32"
                    }
                }
                "bigint" => {
                    if is_nullable {
                        "Option<i64>"
                    } else {
                        "i64"
                    }
                }

                // MySQL unsigned integers
                "unsigned_tinyint" | "tinyint_unsigned" => {
                    if is_nullable {
                        "Option<u8>"
                    } else {
                        "u8"
                    }
                }
                "unsigned_smallint" | "smallint_unsigned" => {
                    if is_nullable {
                        "Option<u16>"
                    } else {
                        "u16"
                    }
                }
                "unsigned_int" | "unsigned_integer" | "int_unsigned" => {
                    if is_nullable {
                        "Option<u32>"
                    } else {
                        "u32"
                    }
                }
                "unsigned_bigint" | "bigint_unsigned" => {
                    if is_nullable {
                        "Option<u64>"
                    } else {
                        "u64"
                    }
                }

                // Floating point types
                "float" => {
                    if is_nullable {
                        "Option<f32>"
                    } else {
                        "f32"
                    }
                }
                "double" | "real" => {
                    if is_nullable {
                        "Option<f64>"
                    } else {
                        "f64"
                    }
                }
                "decimal" | "numeric" => {
                    if is_nullable {
                        "Option<Decimal>"
                    } else {
                        "Decimal"
                    }
                }

                // Text types - MySQL has various text sizes
                "varchar" | "char" | "string" => {
                    if is_nullable {
                        "Option<String>"
                    } else {
                        "String"
                    }
                }
                "text" | "tinytext" | "mediumtext" | "longtext" => {
                    if is_nullable {
                        "Option<String>"
                    } else {
                        "String"
                    }
                }

                // Boolean (MySQL often uses TINYINT(1))
                "boolean" | "bool" => {
                    if is_nullable {
                        "Option<bool>"
                    } else {
                        "bool"
                    }
                }

                // Date/Time types
                "timestamp" | "datetime" => {
                    if is_nullable {
                        "Option<DateTime<Utc>>"
                    } else {
                        "DateTime<Utc>"
                    }
                }
                "date" => {
                    if is_nullable {
                        "Option<NaiveDate>"
                    } else {
                        "NaiveDate"
                    }
                }
                "time" => {
                    if is_nullable {
                        "Option<NaiveTime>"
                    } else {
                        "NaiveTime"
                    }
                }
                "year" => {
                    if is_nullable {
                        "Option<i16>"
                    } else {
                        "i16"
                    }
                } // MySQL YEAR type

                // Semantic types
                "json" => {
                    if is_nullable {
                        "Option<serde_json::Value>"
                    } else {
                        "serde_json::Value"
                    }
                }
                "uuid" => {
                    if is_nullable {
                        "Option<Uuid>"
                    } else {
                        "Uuid"
                    }
                }
                "enum" | "set" => {
                    if is_nullable {
                        "Option<String>"
                    } else {
                        "String"
                    }
                }

                // Binary
                "binary" | "varbinary" | "blob" | "tinyblob" | "mediumblob" | "longblob" => {
                    if is_nullable {
                        "Option<Vec<u8>>"
                    } else {
                        "Vec<u8>"
                    }
                }

                // Default fallback
                _ => {
                    if is_nullable {
                        "Option<String>"
                    } else {
                        "String"
                    }
                }
            }
            .to_string()
        };

        let escaped_field_name = escape_rust_keyword(&field.name);
        let _is_required = field.constraints.required.unwrap_or(false) && !is_nullable;

        // For builder, wrap all fields in Option to allow incremental building
        builder_fields.push(format!(
            "    {}: Option<{}>,",
            escaped_field_name, rust_type
        ));
    }
    vars.insert("builder_fields".to_string(), builder_fields.join("\n"));

    // Generate builder defaults
    let mut builder_defaults = Vec::new();
    for field in &sorted_fields {
        let is_auto = field.constraints.auto.is_some();
        if is_auto {
            continue;
        }
        let escaped_field_name = escape_rust_keyword(&field.name);
        builder_defaults.push(format!("            {}: None,", escaped_field_name));
    }
    vars.insert("builder_defaults".to_string(), builder_defaults.join("\n"));

    // Generate builder methods
    let mut builder_methods = Vec::new();
    for field in &sorted_fields {
        let is_auto = field.constraints.auto.is_some();
        if is_auto {
            continue;
        }

        let is_nullable = field.constraints.nullable.unwrap_or(false);
        let rust_type = if let Some(ref lang_type) = field.lang_type {
            lang_type.clone()
        } else {
            match field.field_type.base_type() {
                // MySQL supports both signed and unsigned integers
                "tinyint" => {
                    if is_nullable {
                        "Option<i8>"
                    } else {
                        "i8"
                    }
                }
                "smallint" => {
                    if is_nullable {
                        "Option<i16>"
                    } else {
                        "i16"
                    }
                }
                "mediumint" | "int" | "integer" => {
                    if is_nullable {
                        "Option<i32>"
                    } else {
                        "i32"
                    }
                }
                "bigint" => {
                    if is_nullable {
                        "Option<i64>"
                    } else {
                        "i64"
                    }
                }

                // MySQL unsigned integers
                "unsigned_tinyint" | "tinyint_unsigned" => {
                    if is_nullable {
                        "Option<u8>"
                    } else {
                        "u8"
                    }
                }
                "unsigned_smallint" | "smallint_unsigned" => {
                    if is_nullable {
                        "Option<u16>"
                    } else {
                        "u16"
                    }
                }
                "unsigned_int" | "unsigned_integer" | "int_unsigned" => {
                    if is_nullable {
                        "Option<u32>"
                    } else {
                        "u32"
                    }
                }
                "unsigned_bigint" | "bigint_unsigned" => {
                    if is_nullable {
                        "Option<u64>"
                    } else {
                        "u64"
                    }
                }

                // Floating point types
                "float" => {
                    if is_nullable {
                        "Option<f32>"
                    } else {
                        "f32"
                    }
                }
                "double" | "real" => {
                    if is_nullable {
                        "Option<f64>"
                    } else {
                        "f64"
                    }
                }
                "decimal" | "numeric" => {
                    if is_nullable {
                        "Option<Decimal>"
                    } else {
                        "Decimal"
                    }
                }

                // Text types - MySQL has various text sizes
                "varchar" | "char" | "string" => {
                    if is_nullable {
                        "Option<String>"
                    } else {
                        "String"
                    }
                }
                "text" | "tinytext" | "mediumtext" | "longtext" => {
                    if is_nullable {
                        "Option<String>"
                    } else {
                        "String"
                    }
                }

                // Boolean (MySQL often uses TINYINT(1))
                "boolean" | "bool" => {
                    if is_nullable {
                        "Option<bool>"
                    } else {
                        "bool"
                    }
                }

                // Date/Time types
                "timestamp" | "datetime" => {
                    if is_nullable {
                        "Option<DateTime<Utc>>"
                    } else {
                        "DateTime<Utc>"
                    }
                }
                "date" => {
                    if is_nullable {
                        "Option<NaiveDate>"
                    } else {
                        "NaiveDate"
                    }
                }
                "time" => {
                    if is_nullable {
                        "Option<NaiveTime>"
                    } else {
                        "NaiveTime"
                    }
                }
                "year" => {
                    if is_nullable {
                        "Option<i16>"
                    } else {
                        "i16"
                    }
                } // MySQL YEAR type

                // Semantic types
                "json" => {
                    if is_nullable {
                        "Option<serde_json::Value>"
                    } else {
                        "serde_json::Value"
                    }
                }
                "uuid" => {
                    if is_nullable {
                        "Option<Uuid>"
                    } else {
                        "Uuid"
                    }
                }
                "enum" | "set" => {
                    if is_nullable {
                        "Option<String>"
                    } else {
                        "String"
                    }
                }

                // Binary
                "binary" | "varbinary" | "blob" | "tinyblob" | "mediumblob" | "longblob" => {
                    if is_nullable {
                        "Option<Vec<u8>>"
                    } else {
                        "Vec<u8>"
                    }
                }

                // Default fallback
                _ => {
                    if is_nullable {
                        "Option<String>"
                    } else {
                        "String"
                    }
                }
            }
            .to_string()
        };

        let escaped_field_name = escape_rust_keyword(&field.name);
        let field_doc = field.ai.as_deref().unwrap_or("Set this field");

        // For String types in non-Option fields, use impl Into<String> for convenience
        // For Option fields, handle Option<String> specially to accept Option<impl Into<String>>
        let (param_type, value_expr) = if rust_type == "Option<String>" {
            // Optional String field - accept Option<impl Into<String>>
            ("Option<impl Into<String>>", "value.map(|v| v.into())")
        } else if rust_type.starts_with("Option") {
            // Other optional fields - accept the Option<T> type directly
            (rust_type.as_str(), "value")
        } else if rust_type.contains("String") {
            // Non-nullable String field
            ("impl Into<String>", "value.into()")
        } else {
            // Non-nullable non-String field
            (rust_type.as_str(), "value")
        };

        builder_methods.push(format!(
            "    /// {}\n    pub fn {}(mut self, value: {}) -> Self {{\n        self.{} = Some({});\n        self\n    }}",
            field_doc, escaped_field_name, param_type, escaped_field_name, value_expr
        ));
    }
    vars.insert("builder_methods".to_string(), builder_methods.join("\n\n"));

    // Generate builder validation helper method
    let mut required_fields_checks = Vec::new();
    let mut required_fields_names = Vec::new();
    for field in &sorted_fields {
        let is_auto = field.constraints.auto.is_some();
        // Skip auto fields AND UUID primary key fields (which are auto-generated)
        let is_uuid_pk = field.constraints.primary_key.unwrap_or(false) && field.field_type.base_type() == "uuid";
        if is_auto || is_uuid_pk {
            continue;
        }
        let is_nullable = field.constraints.nullable.unwrap_or(false);
        let is_required = field.constraints.required.unwrap_or(false) && !is_nullable;

        if is_required {
            let escaped_field_name = escape_rust_keyword(&field.name);
            required_fields_checks.push(format!(
                "        if self.{}.is_none() {{\n            missing.push(\"{}\");\n        }}",
                escaped_field_name, field.name
            ));
            required_fields_names.push(&field.name);
        }
    }

    // Generate validate() method
    let validate_method = if required_fields_names.is_empty() {
        format!(
            "    /// Validate the builder has all required fields\n    \
             /// Returns Ok(()) if valid, or Err with list of missing fields\n    \
             pub fn validate(&self) -> Result<(), Vec<&'static str>> {{\n        \
             // No required fields to validate\n        \
             Ok(())\n    \
             }}"
        )
    } else {
        format!(
            "    /// Validate the builder has all required fields\n    \
             /// Returns Ok(()) if valid, or Err with list of missing fields\n    \
             pub fn validate(&self) -> Result<(), Vec<&'static str>> {{\n        \
             let mut missing = Vec::new();\n        \n{}\n        \n        \
             if missing.is_empty() {{\n            \
             Ok(())\n        \
             }} else {{\n            \
             Err(missing)\n        \
             }}\n    \
             }}",
            required_fields_checks.join("\n")
        )
    };
    vars.insert("builder_validate_method".to_string(), validate_method);

    // Generate builder validation for build() method
    let builder_validation = if required_fields_names.is_empty() {
        "        // No required fields to validate".to_string()
    } else {
        format!(
            "        // Validate all required fields are present\n        \
             if let Err(missing) = self.validate() {{\n            \
             return Err(format!(\"Missing required fields: {{}}\", missing.join(\", \")));\n        \
             }}"
        )
    };
    vars.insert("builder_validation".to_string(), builder_validation);

    // Generate builder build implementation
    let mut builder_build = Vec::new();
    for field in &sorted_fields {
        let is_auto = field.constraints.auto.is_some();
        let escaped_field_name = escape_rust_keyword(&field.name);

        if field.constraints.primary_key.unwrap_or(false) {
            // Handle primary key field specially - usually auto-generated
            // Check if it's a UUID field that needs special handling
            let is_uuid = field.field_type.base_type() == "uuid";
            if is_uuid {
                builder_build.push(format!(
                    "            {}: Uuid::new_v4(),",
                    escaped_field_name
                ));
            } else if is_auto {
                builder_build.push(format!(
                    "            {}: Default::default(), // Auto-generated",
                    escaped_field_name
                ));
            } else {
                // For non-UUID, non-auto ID fields, use the value from the builder
                builder_build.push(format!(
                    "            {}: self.{}.unwrap(),",
                    escaped_field_name, escaped_field_name
                ));
            }
        } else if is_auto {
            // Skip other auto fields in builder
            builder_build.push(format!(
                "            {}: Default::default(), // Auto-generated",
                escaped_field_name
            ));
        } else {
            let is_nullable = field.constraints.nullable.unwrap_or(false);
            let is_required = field.constraints.required.unwrap_or(false) && !is_nullable;

            if is_required {
                builder_build.push(format!(
                    "            {}: self.{}.unwrap(),",
                    escaped_field_name, escaped_field_name
                ));
            } else if is_nullable {
                // For nullable fields, builder has Option<Option<T>>, flatten to Option<T>
                builder_build.push(format!(
                    "            {}: self.{}.flatten(),",
                    escaped_field_name, escaped_field_name
                ));
            } else {
                builder_build.push(format!(
                    "            {}: self.{}.unwrap_or_default(),",
                    escaped_field_name, escaped_field_name
                ));
            }
        }
    }
    // Add changed_fields and null_fields initialization
    builder_build.push("            changed_fields: HashSet::new(),".to_string());
    builder_build.push("            null_fields: HashSet::new(),".to_string());
    vars.insert("builder_build".to_string(), builder_build.join("\n"));

    // Generate create method
    // Sort fields by name to ensure consistent parameter order
    let mut non_auto_fields: Vec<_> = table
        .fields
        .values()
        .filter(|f| !f.constraints.auto.is_some() && f.name != "id")
        .collect();
    non_auto_fields.sort_by(|a, b| a.name.cmp(&b.name));

    // Generate insert field mapping for create_internal method
    let mut insert_field_mappings = Vec::new();
    for field in &sorted_fields {
        // Skip auto-increment ID fields
        if field.constraints.primary_key.unwrap_or(false) && field.constraints.auto.is_some() {
            continue;
        }
        let escaped_field_name = escape_rust_keyword(&field.name);
        let field_name = &field.name;
        insert_field_mappings.push(format!(
            "        insert_data.insert(\"{}\".to_string(), SqlValue::from(model.{}));",
            field_name, escaped_field_name
        ));
    }
    vars.insert(
        "insert_field_mapping".to_string(),
        insert_field_mappings.join("\n"),
    );

    // Determine database backend
    let database_backend = match pool_type {
        "MySql" => "DatabaseBackend::MySQL",
        "Postgres" => "DatabaseBackend::Postgres",
        "Sqlite" => "DatabaseBackend::SQLite",
        _ => "DatabaseBackend::MySQL", // Default fallback
    };
    vars.insert("database_backend".to_string(), database_backend.to_string());

    // Generate row conversion code for different database backends
    let mut postgres_conversion = Vec::new();
    let mut mysql_conversion = Vec::new();
    let mut sqlite_conversion = Vec::new();

    for field in &sorted_fields {
        let escaped_field_name = escape_rust_keyword(&field.name);
        let field_name = &field.name;

        // PostgreSQL conversion
        postgres_conversion.push(format!(
            "                json_obj.insert(\"{}\".to_string(), \n                    serde_json::to_value(row.try_get::<_, _>(\"{}\")).unwrap_or(serde_json::Value::Null));",
            field_name, field_name
        ));

        // MySQL conversion
        mysql_conversion.push(format!(
            "                json_obj.insert(\"{}\".to_string(), \n                    serde_json::to_value(row.try_get::<_, _>(\"{}\")).unwrap_or(serde_json::Value::Null));",
            field_name, field_name
        ));

        // SQLite conversion
        sqlite_conversion.push(format!(
            "                json_obj.insert(\"{}\".to_string(), \n                    serde_json::to_value(row.try_get::<_, _>(\"{}\")).unwrap_or(serde_json::Value::Null));",
            field_name, field_name
        ));
    }

    vars.insert(
        "postgres_row_conversion".to_string(),
        postgres_conversion.join("\n"),
    );
    vars.insert(
        "mysql_row_conversion".to_string(),
        mysql_conversion.join("\n"),
    );
    vars.insert(
        "sqlite_row_conversion".to_string(),
        sqlite_conversion.join("\n"),
    );

    // Generate FromRow implementations for each database type
    let mut postgres_from_row = Vec::new();
    let mut mysql_from_row = Vec::new();
    let mut sqlite_from_row = Vec::new();

    for field in &sorted_fields {
        let escaped_field_name = escape_rust_keyword(&field.name);
        let field_name = &field.name;

        // PostgreSQL FromRow
        postgres_from_row.push(format!(
            "            {}: row.try_get(\"{}\")?,",
            escaped_field_name, field_name
        ));

        // MySQL FromRow
        mysql_from_row.push(format!(
            "            {}: row.try_get(\"{}\")?,",
            escaped_field_name, field_name
        ));

        // SQLite FromRow
        sqlite_from_row.push(format!(
            "            {}: row.try_get(\"{}\")?,",
            escaped_field_name, field_name
        ));
    }

    vars.insert(
        "postgres_from_row".to_string(),
        postgres_from_row.join("\n"),
    );
    vars.insert("mysql_from_row".to_string(), mysql_from_row.join("\n"));
    vars.insert("sqlite_from_row".to_string(), sqlite_from_row.join("\n"));

    // Generate conditional SQLite FromRow implementation
    let sqlite_from_row_impl = if needs_decimal {
        // Skip SQLite support for models with Decimal fields since rust_decimal doesn't support SQLite
        "// SQLite FromRow implementation skipped: Table contains Decimal fields which are not supported by SQLite".to_string()
    } else {
        format!(
            "impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for {}Base {{\n    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {{\n        use sqlx::Row;\n        Ok(Self {{\n{}\n            changed_fields: HashSet::new(),\n        }})\n    }}\n}}",
            table.name,
            sqlite_from_row.join("\n")
        )
    };
    vars.insert(
        "sqlite_from_row_impl".to_string(),
        sqlite_from_row_impl.clone(),
    );

    // Build complete MySQL FromRow block (for MySQL-focused generation)
    let mysql_fromrow_block = format!(
        "impl sqlx::FromRow<'_, sqlx::mysql::MySqlRow> for {} {{\n    fn from_row(row: &sqlx::mysql::MySqlRow) -> sqlx::Result<Self> {{\n        use sqlx::Row;\n        Ok(Self {{\n{}\n            changed_fields: HashSet::new(),\n            null_fields: HashSet::new(),\n        }})\n    }}\n}}",
        table.name,
        mysql_from_row.join("\n")
    );
    vars.insert("mysql_fromrow_block".to_string(), mysql_fromrow_block);

    // For MySQL-only generation, PostgreSQL and SQLite blocks are empty
    vars.insert("postgres_fromrow_block".to_string(), String::new());
    vars.insert("sqlite_fromrow_block".to_string(), String::new());

    // Generate conditional SQLite select support
    let sqlite_select_support = if needs_decimal {
        "                return Err(anyhow::anyhow!(\"SQLite is not supported for this model: contains Decimal fields\"));"
    } else {
        "                let results = sqlx::query_as::<_, Self>(sql)\n                    .fetch_all(pool)\n                    .await?;\n                Ok(results)"
    };
    vars.insert(
        "sqlite_select_support".to_string(),
        sqlite_select_support.to_string(),
    );

    let sqlite_select_one_support = if needs_decimal {
        "                return Err(anyhow::anyhow!(\"SQLite is not supported for this model: contains Decimal fields\"));"
    } else {
        "                let result = sqlx::query_as::<_, Self>(sql)\n                    .fetch_optional(pool)\n                    .await?;\n                Ok(result)"
    };
    vars.insert(
        "sqlite_select_one_support".to_string(),
        sqlite_select_one_support.to_string(),
    );

    // Generate field setters with change tracking
    let mut field_setters = Vec::new();
    for field in &sorted_fields {
        // Skip ID field - usually immutable
        if field.constraints.primary_key.unwrap_or(false) {
            continue;
        }

        let is_nullable = field.constraints.nullable.unwrap_or(false);
        let rust_type = if let Some(ref lang_type) = field.lang_type {
            lang_type.clone()
        } else {
            match field.field_type.base_type() {
                // MySQL supports both signed and unsigned integers
                "tinyint" => {
                    if is_nullable {
                        "Option<i8>"
                    } else {
                        "i8"
                    }
                }
                "smallint" => {
                    if is_nullable {
                        "Option<i16>"
                    } else {
                        "i16"
                    }
                }
                "mediumint" | "int" | "integer" => {
                    if is_nullable {
                        "Option<i32>"
                    } else {
                        "i32"
                    }
                }
                "bigint" => {
                    if is_nullable {
                        "Option<i64>"
                    } else {
                        "i64"
                    }
                }

                // MySQL unsigned integers
                "unsigned_tinyint" | "tinyint_unsigned" => {
                    if is_nullable {
                        "Option<u8>"
                    } else {
                        "u8"
                    }
                }
                "unsigned_smallint" | "smallint_unsigned" => {
                    if is_nullable {
                        "Option<u16>"
                    } else {
                        "u16"
                    }
                }
                "unsigned_int" | "unsigned_integer" | "int_unsigned" => {
                    if is_nullable {
                        "Option<u32>"
                    } else {
                        "u32"
                    }
                }
                "unsigned_bigint" | "bigint_unsigned" => {
                    if is_nullable {
                        "Option<u64>"
                    } else {
                        "u64"
                    }
                }

                // Floating point types
                "float" => {
                    if is_nullable {
                        "Option<f32>"
                    } else {
                        "f32"
                    }
                }
                "double" | "real" => {
                    if is_nullable {
                        "Option<f64>"
                    } else {
                        "f64"
                    }
                }
                "decimal" | "numeric" => {
                    if is_nullable {
                        "Option<Decimal>"
                    } else {
                        "Decimal"
                    }
                }

                // Text types - MySQL has various text sizes
                "varchar" | "char" | "string" => {
                    if is_nullable {
                        "Option<String>"
                    } else {
                        "String"
                    }
                }
                "text" | "tinytext" | "mediumtext" | "longtext" => {
                    if is_nullable {
                        "Option<String>"
                    } else {
                        "String"
                    }
                }

                // Boolean (MySQL often uses TINYINT(1))
                "boolean" | "bool" => {
                    if is_nullable {
                        "Option<bool>"
                    } else {
                        "bool"
                    }
                }

                // Date/Time types
                "timestamp" | "datetime" => {
                    if is_nullable {
                        "Option<DateTime<Utc>>"
                    } else {
                        "DateTime<Utc>"
                    }
                }
                "date" => {
                    if is_nullable {
                        "Option<NaiveDate>"
                    } else {
                        "NaiveDate"
                    }
                }
                "time" => {
                    if is_nullable {
                        "Option<NaiveTime>"
                    } else {
                        "NaiveTime"
                    }
                }
                "year" => {
                    if is_nullable {
                        "Option<i16>"
                    } else {
                        "i16"
                    }
                } // MySQL YEAR type

                // Semantic types
                "json" => {
                    if is_nullable {
                        "Option<serde_json::Value>"
                    } else {
                        "serde_json::Value"
                    }
                }
                "uuid" => {
                    if is_nullable {
                        "Option<Uuid>"
                    } else {
                        "Uuid"
                    }
                }
                "enum" | "set" => {
                    if is_nullable {
                        "Option<String>"
                    } else {
                        "String"
                    }
                }

                // Binary
                "binary" | "varbinary" | "blob" | "tinyblob" | "mediumblob" | "longblob" => {
                    if is_nullable {
                        "Option<Vec<u8>>"
                    } else {
                        "Vec<u8>"
                    }
                }

                // Default fallback
                _ => {
                    if is_nullable {
                        "Option<String>"
                    } else {
                        "String"
                    }
                }
            }
            .to_string()
        };

        let escaped_field_name = escape_rust_keyword(&field.name);
        let default_doc = format!("Set the {} field", field.name);
        let field_doc = field.ai.as_deref().unwrap_or(&default_doc);

        // For String types, use Into trait for flexibility
        let param_type = if rust_type.contains("String") && !rust_type.starts_with("Option") {
            "impl Into<String>".to_string()
        } else if rust_type == "Option<String>" {
            "impl Into<String>".to_string()
        } else {
            rust_type.clone()
        };

        // Generate setter method
        if rust_type.starts_with("Option<String>") {
            // For Option<String>, accept Option<impl Into<String>> to allow setting to None
            field_setters.push(format!(
                "    /// {}\n    pub fn set_{}(&mut self, value: Option<impl Into<String>>) {{\n        self.{} = value.map(|v| v.into());\n        self.mark_changed(\"{}\", self.{}.is_none());\n    }}",
                field_doc, escaped_field_name, escaped_field_name, field.name, escaped_field_name
            ));
        } else if rust_type.starts_with("Option") {
            // For other Option types, accept Option<T> to allow setting to None
            // Handle nested generics like Option<DateTime<Utc>>
            let inner_type = if rust_type.starts_with("Option<") && rust_type.ends_with(">") {
                &rust_type[7..rust_type.len() - 1]
            } else {
                rust_type.as_str()
            };
            field_setters.push(format!(
                "    /// {}\n    pub fn set_{}(&mut self, value: Option<{}>) {{\n        self.{} = value;\n        self.mark_changed(\"{}\", self.{}.is_none());\n    }}",
                field_doc, escaped_field_name, inner_type, escaped_field_name, field.name, escaped_field_name
            ));
        } else if rust_type.contains("String") {
            field_setters.push(format!(
                "    /// {}\n    pub fn set_{}(&mut self, value: {}) {{\n        self.{} = value.into();\n        self.mark_changed(\"{}\", false);\n    }}",
                field_doc, escaped_field_name, param_type, escaped_field_name, field.name
            ));
        } else {
            field_setters.push(format!(
                "    /// {}\n    pub fn set_{}(&mut self, value: {}) {{\n        self.{} = value;\n        self.mark_changed(\"{}\", false);\n    }}",
                field_doc, escaped_field_name, param_type, escaped_field_name, field.name
            ));
        }
    }
    vars.insert("field_setters".to_string(), field_setters.join("\n\n"));

    // Generate field getters
    let mut field_getters = Vec::new();
    for field in &sorted_fields {
        let is_nullable = field.constraints.nullable.unwrap_or(false);
        let rust_type = if let Some(ref lang_type) = field.lang_type {
            lang_type.clone()
        } else {
            match field.field_type.base_type() {
                // MySQL supports both signed and unsigned integers
                "tinyint" => {
                    if is_nullable {
                        "Option<i8>"
                    } else {
                        "i8"
                    }
                }
                "smallint" => {
                    if is_nullable {
                        "Option<i16>"
                    } else {
                        "i16"
                    }
                }
                "mediumint" | "int" | "integer" => {
                    if is_nullable {
                        "Option<i32>"
                    } else {
                        "i32"
                    }
                }
                "bigint" => {
                    if is_nullable {
                        "Option<i64>"
                    } else {
                        "i64"
                    }
                }

                // MySQL unsigned integers
                "unsigned_tinyint" | "tinyint_unsigned" => {
                    if is_nullable {
                        "Option<u8>"
                    } else {
                        "u8"
                    }
                }
                "unsigned_smallint" | "smallint_unsigned" => {
                    if is_nullable {
                        "Option<u16>"
                    } else {
                        "u16"
                    }
                }
                "unsigned_int" | "unsigned_integer" | "int_unsigned" => {
                    if is_nullable {
                        "Option<u32>"
                    } else {
                        "u32"
                    }
                }
                "unsigned_bigint" | "bigint_unsigned" => {
                    if is_nullable {
                        "Option<u64>"
                    } else {
                        "u64"
                    }
                }

                // Floating point types
                "float" => {
                    if is_nullable {
                        "Option<f32>"
                    } else {
                        "f32"
                    }
                }
                "double" | "real" => {
                    if is_nullable {
                        "Option<f64>"
                    } else {
                        "f64"
                    }
                }
                "decimal" | "numeric" => {
                    if is_nullable {
                        "Option<Decimal>"
                    } else {
                        "Decimal"
                    }
                }

                // Text types - MySQL has various text sizes
                "varchar" | "char" | "string" => {
                    if is_nullable {
                        "Option<String>"
                    } else {
                        "String"
                    }
                }
                "text" | "tinytext" | "mediumtext" | "longtext" => {
                    if is_nullable {
                        "Option<String>"
                    } else {
                        "String"
                    }
                }

                // Boolean (MySQL often uses TINYINT(1))
                "boolean" | "bool" => {
                    if is_nullable {
                        "Option<bool>"
                    } else {
                        "bool"
                    }
                }

                // Date/Time types
                "timestamp" | "datetime" => {
                    if is_nullable {
                        "Option<DateTime<Utc>>"
                    } else {
                        "DateTime<Utc>"
                    }
                }
                "date" => {
                    if is_nullable {
                        "Option<NaiveDate>"
                    } else {
                        "NaiveDate"
                    }
                }
                "time" => {
                    if is_nullable {
                        "Option<NaiveTime>"
                    } else {
                        "NaiveTime"
                    }
                }
                "year" => {
                    if is_nullable {
                        "Option<i16>"
                    } else {
                        "i16"
                    }
                } // MySQL YEAR type

                // Semantic types
                "json" => {
                    if is_nullable {
                        "Option<serde_json::Value>"
                    } else {
                        "serde_json::Value"
                    }
                }
                "uuid" => {
                    if is_nullable {
                        "Option<Uuid>"
                    } else {
                        "Uuid"
                    }
                }
                "enum" | "set" => {
                    if is_nullable {
                        "Option<String>"
                    } else {
                        "String"
                    }
                }

                // Binary
                "binary" | "varbinary" | "blob" | "tinyblob" | "mediumblob" | "longblob" => {
                    if is_nullable {
                        "Option<Vec<u8>>"
                    } else {
                        "Vec<u8>"
                    }
                }

                // Default fallback
                _ => {
                    if is_nullable {
                        "Option<String>"
                    } else {
                        "String"
                    }
                }
            }
            .to_string()
        };

        let escaped_field_name = escape_rust_keyword(&field.name);

        // Generate getter method documentation
        let field_doc = if let Some(ai_hint) = &field.ai {
            format!(
                "Get the {} field\n    /// \n    /// {}",
                field.name, ai_hint
            )
        } else {
            format!("Get the {} field", field.name)
        };

        // Generate getter method based on type
        if rust_type.contains("String") && !rust_type.starts_with("Option") {
            // For non-optional String, return &str to avoid cloning
            field_getters.push(format!(
                "    /// {}\n    pub fn {}(&self) -> &str {{\n        &self.{}\n    }}",
                field_doc, escaped_field_name, escaped_field_name
            ));
        } else if rust_type.starts_with("Option<String>") {
            // For Option<String>, return Option<&str>
            field_getters.push(format!(
                "    /// {}\n    pub fn {}(&self) -> Option<&str> {{\n        self.{}.as_deref()\n    }}",
                field_doc, escaped_field_name, escaped_field_name
            ));
        } else if rust_type.starts_with("Option<") {
            // For Option<T>, check if T is Copy
            let inner_type = extract_inner_type(&rust_type);
            if is_copy_type(inner_type) {
                // For Option<Copy>, return by value (cheap copy)
                // This makes unwrap_or() work naturally without .copied()
                field_getters.push(format!(
                    "    /// {}\n    pub fn {}(&self) -> {} {{\n        self.{}\n    }}",
                    field_doc, escaped_field_name, rust_type, escaped_field_name
                ));
            } else {
                // For Option<NonCopy>, return reference
                field_getters.push(format!(
                    "    /// {}\n    pub fn {}(&self) -> &{} {{\n        &self.{}\n    }}",
                    field_doc, escaped_field_name, rust_type, escaped_field_name
                ));
            }
        } else if rust_type.contains("Vec") || rust_type.contains("serde_json::Value") {
            // For Vec and JSON values, return reference
            field_getters.push(format!(
                "    /// {}\n    pub fn {}(&self) -> &{} {{\n        &self.{}\n    }}",
                field_doc, escaped_field_name, rust_type, escaped_field_name
            ));
        } else {
            // For primitive types, return by value (Copy trait)
            field_getters.push(format!(
                "    /// {}\n    pub fn {}(&self) -> {} {{\n        self.{}\n    }}",
                field_doc, escaped_field_name, rust_type, escaped_field_name
            ));
        }
    }
    vars.insert("field_getters".to_string(), field_getters.join("\n\n"));

    // Generate get_field_value implementation
    let mut get_field_cases = Vec::new();
    for field in &sorted_fields {
        let escaped_field_name = escape_rust_keyword(&field.name);
        let field_name = &field.name;

        // Check if this is an enum field
        let is_enum = matches!(
            field.field_type,
            rustf_schema::types::FieldType::Enum { .. }
        );
        if is_enum {
            // For enum fields, check if it's optional
            let is_nullable = field.constraints.nullable.unwrap_or(false);
            if is_nullable {
                // For optional enum fields, map the Option
                get_field_cases.push(format!(
                    "            \"{}\" => Ok(self.{}.clone().map(SqlValue::Enum).unwrap_or(SqlValue::Null)),",
                    field_name, escaped_field_name
                ));
            } else {
                // For required enum fields, use SqlValue::Enum directly
                get_field_cases.push(format!(
                    "            \"{}\" => Ok(SqlValue::Enum(self.{}.clone())),",
                    field_name, escaped_field_name
                ));
            }
        } else {
            // Regular fields use SqlValue::from
            get_field_cases.push(format!(
                "            \"{}\" => Ok(SqlValue::from(self.{}.clone())),",
                field_name, escaped_field_name
            ));
        }
    }

    let get_field_value_impl = format!(
        "        match field_name {{\n{}\n            _ => Err(anyhow::anyhow!(\"Unknown field: {{}}\", field_name)),\n        }}",
        get_field_cases.join("\n")
    );
    vars.insert("get_field_value_impl".to_string(), get_field_value_impl);

    // For the include template, we need different variable names
    vars.insert(
        "get_field_value_match".to_string(),
        get_field_cases.join("\n"),
    );

    // Determine primary key field (default to "id")
    let primary_key = table
        .fields
        .values()
        .find(|f| f.constraints.primary_key.unwrap_or(false))
        .map(|f| f.name.clone())
        .unwrap_or_else(|| "id".to_string());
    vars.insert("primary_key".to_string(), primary_key);

    // Generate FromRow implementations for each database type
    let mut from_row_pg = Vec::new();
    let mut from_row_mysql = Vec::new();
    let mut from_row_sqlite = Vec::new();

    for field in &sorted_fields {
        let escaped_field_name = escape_rust_keyword(&field.name);
        let field_name = &field.name;

        // Generate field extraction for each database type
        from_row_pg.push(format!(
            "            {}: row.try_get(\"{}\")?,",
            escaped_field_name, field_name
        ));
        from_row_mysql.push(format!(
            "            {}: row.try_get(\"{}\")?,",
            escaped_field_name, field_name
        ));
        from_row_sqlite.push(format!(
            "            {}: row.try_get(\"{}\")?,",
            escaped_field_name, field_name
        ));
    }

    vars.insert("from_row_pg".to_string(), from_row_pg.join("\n"));
    vars.insert("from_row_mysql".to_string(), from_row_mysql.join("\n"));
    vars.insert("from_row_sqlite".to_string(), from_row_sqlite.join("\n"));

    // Check if model has decimal fields (SQLite doesn't support Decimal type)
    let needs_decimal = table
        .fields
        .values()
        .any(|f| f.field_type.base_type() == "decimal");

    // Generate conditional SQLite FromRow implementation
    let sqlite_from_row_impl = if needs_decimal {
        // Skip SQLite support for models with Decimal fields since rust_decimal doesn't support SQLite
        "// SQLite FromRow implementation skipped: Table contains Decimal fields which are not supported by SQLite".to_string()
    } else {
        format!(
            "impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for {} {{\n    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {{\n        use sqlx::Row;\n        Ok(Self {{\n{}\n            changed_fields: HashSet::new(),\n        }})\n    }}\n}}",
            table.name,  // This is correct - the struct is named after table.name without Base suffix
            from_row_sqlite.join("\n")
        )
    };
    vars.insert(
        "sqlite_from_row_impl".to_string(),
        sqlite_from_row_impl.clone(),
    );

    // Build complete MySQL FromRow block (for MySQL-focused generation)
    let mysql_fromrow_block = format!(
        "impl sqlx::FromRow<'_, sqlx::mysql::MySqlRow> for {} {{\n    fn from_row(row: &sqlx::mysql::MySqlRow) -> sqlx::Result<Self> {{\n        use sqlx::Row;\n        Ok(Self {{\n{}\n            changed_fields: HashSet::new(),\n            null_fields: HashSet::new(),\n        }})\n    }}\n}}",
        table.name,
        from_row_mysql.join("\n")
    );
    vars.insert("mysql_fromrow_block".to_string(), mysql_fromrow_block);

    // For MySQL-only generation, PostgreSQL and SQLite blocks are empty
    vars.insert("postgres_fromrow_block".to_string(), String::new());
    vars.insert("sqlite_fromrow_block".to_string(), String::new());

    // Generate conditional SQLite select support
    let sqlite_select_support = if needs_decimal {
        "                return Err(anyhow::anyhow!(\"SQLite is not supported for this model: contains Decimal fields\"));"
    } else {
        "                let results = sqlx::query_as::<_, Self>(sql)\n                    .fetch_all(pool)\n                    .await?;\n                Ok(results)"
    };
    vars.insert(
        "sqlite_select_support".to_string(),
        sqlite_select_support.to_string(),
    );

    let sqlite_select_one_support = if needs_decimal {
        "                return Err(anyhow::anyhow!(\"SQLite is not supported for this model: contains Decimal fields\"));"
    } else {
        "                let result = sqlx::query_as::<_, Self>(sql)\n                    .fetch_optional(pool)\n                    .await?;\n                Ok(result)"
    };
    vars.insert(
        "sqlite_select_one_support".to_string(),
        sqlite_select_one_support.to_string(),
    );

    // Collect enum fields and their values
    let mut enum_fields = Vec::new();
    let mut enum_constants = Vec::new();
    let mut enum_converters = Vec::new();

    for field in &sorted_fields {
        if let rustf_schema::types::FieldType::Enum { values, .. } = &field.field_type {
            enum_fields.push(field.name.clone());

            // Generate constants for this enum field
            for value in values {
                let const_name = format!(
                    "{}_{}",
                    field.name.to_uppercase(),
                    value.replace('-', "_").replace(' ', "_")
                );
                enum_constants.push(format!(
                    "    /// {} value for {} field\n    pub const {}: &'static str = \"{}\";",
                    value, field.name, const_name, value
                ));
            }

            // Generate field-specific enum converter method (pass-through for MySQL)
            // MySQL doesn't need type casting, but we provide the method for API consistency
            enum_converters.push(format!(
                "    /// Convert a value to MySQL enum format for {} field\n    /// \n    /// # Example\n    /// ```\n    /// let value = {}::as_{}_enum(\"ACTIVE\");\n    /// // Returns: \"ACTIVE\" (pass-through for MySQL)\n    /// ```\n    pub fn as_{}_enum(value: &str) -> String {{\n        value.to_string()\n    }}",
                field.name, table.name, field.name, field.name
            ));
        }
    }

    // Generate enum constants as part of the model impl block
    // This makes them more discoverable for AI agents
    if !enum_constants.is_empty() {
        vars.insert(
            "enum_constants_in_impl".to_string(),
            enum_constants.join("\n"),
        );
    } else {
        vars.insert("enum_constants_in_impl".to_string(), String::new());
    }

    // Add enum converter methods to template
    if !enum_converters.is_empty() {
        vars.insert("enum_converters".to_string(), enum_converters.join("\n\n"));
    } else {
        vars.insert("enum_converters".to_string(), String::new());
    }

    // Generate list of enum fields
    let enum_fields_list =
        if !enum_fields.is_empty() {
            format!(
            "    /// List of fields that are enums\n    pub const ENUM_FIELDS: &[&str] = &[{}];",
            enum_fields.iter().map(|f| format!("\"{}\"", f)).collect::<Vec<_>>().join(", ")
        )
        } else {
            "    /// List of fields that are enums\n    pub const ENUM_FIELDS: &[&str] = &[];"
                .to_string()
        };
    vars.insert("enum_fields_list".to_string(), enum_fields_list);

    // Generate insert field values mapping
    let mut insert_field_values = Vec::new();
    for field in &sorted_fields {
        let escaped_field_name = escape_rust_keyword(&field.name);
        let field_name = &field.name;

        // Check if field has a default value or is auto-generated
        let is_auto = field.constraints.auto.is_some();
        if !is_auto {
            // Check if this is an enum field
            let value_expr = if matches!(
                &field.field_type,
                rustf_schema::types::FieldType::Enum { .. }
            ) {
                // For enum fields, check if it's optional
                let is_nullable = field.constraints.nullable.unwrap_or(false);
                if is_nullable {
                    // For optional enum fields, map the Option
                    format!(
                        "model.{}.clone().map(SqlValue::Enum).unwrap_or(SqlValue::Null)",
                        escaped_field_name
                    )
                } else {
                    // For required enum fields, use SqlValue::Enum directly
                    format!("SqlValue::Enum(model.{}.clone())", escaped_field_name)
                }
            } else {
                // For other fields, use SqlValue::from
                format!("SqlValue::from(model.{})", escaped_field_name)
            };

            insert_field_values.push(format!(
                "        insert_data.insert(\"{}\".to_string(), {});",
                field_name, value_expr
            ));
        }
    }
    vars.insert(
        "insert_field_values".to_string(),
        insert_field_values.join("\n"),
    );

    vars
}

/// Prepare template variables for wrapper model generation
fn prepare_wrapper_model_variables(
    table: &rustf_schema::Table,
    pool_type: &str,
) -> HashMap<String, String> {
    let mut vars = HashMap::new();

    // Basic metadata
    vars.insert("model_name".to_string(), table.name.clone());
    vars.insert("table_name".to_string(), table.table.clone());
    vars.insert("pool_type".to_string(), pool_type.to_string());

    // Sort fields alphabetically once for consistent ordering
    let mut sorted_fields: Vec<_> = table.fields.values().collect();
    sorted_fields.sort_by(|a, b| a.name.cmp(&b.name));
    log::debug!(
        "Sorted fields order for wrapper {}: {:?}",
        table.name,
        sorted_fields.iter().map(|f| &f.name).collect::<Vec<_>>()
    );

    // Generate the correct AnyDatabase variant with cloned pool
    let any_database_variant = match pool_type {
        "MySql" => "AnyDatabase::MySQL(pool.clone())",
        "Postgres" => "AnyDatabase::Postgres(pool.clone())",
        "Sqlite" => "AnyDatabase::SQLite(pool.clone())",
        _ => "AnyDatabase::MySQL(pool.clone())", // Default fallback
    };
    vars.insert(
        "any_database_variant".to_string(),
        any_database_variant.to_string(),
    );

    // Determine primary key field (detect early for use throughout generation)
    let primary_key = table
        .fields
        .values()
        .find(|f| f.constraints.primary_key.unwrap_or(false))
        .map(|f| f.name.clone())
        .unwrap_or_else(|| "id".to_string());
    vars.insert("primary_key".to_string(), primary_key.clone());

    // Determine ID type and whether this model has a primary key field
    let has_primary_key = table
        .fields
        .values()
        .any(|f| f.constraints.primary_key.unwrap_or(false));
    
    let id_type = if let Some(pk_field) = table
        .fields
        .values()
        .find(|f| f.constraints.primary_key.unwrap_or(false))
    {
        if let Some(ref lang_type) = pk_field.lang_type {
            lang_type.clone()
        } else {
            match pk_field.field_type.base_type() {
                "int" | "integer" => "i32",
                "bigint" => "i64",
                "string" => "String",
                _ => "i32",
            }
            .to_string()
        }
    } else {
        "i32".to_string() // Default type for models without primary key
    };
    vars.insert("id_type".to_string(), id_type.clone());

    // Generate id method implementation for wrapper
    let id_method_impl = if has_primary_key {
        // Use the primary key field name dynamically
        let escaped_pk_field = escape_rust_keyword(&primary_key);
        // Clone String IDs to avoid move out of borrowed content
        if id_type == "String" {
            format!(" self.base.{}.clone() ", escaped_pk_field)
        } else {
            format!(" self.base.{} ", escaped_pk_field)
        }
    } else {
        " panic!(\"This model does not have a primary key field - it is likely a database view or composite entity\") ".to_string()
    };
    vars.insert("id_method_impl".to_string(), id_method_impl);

    // Check if we need additional imports for the wrapper based on field types
    let mut wrapper_imports = Vec::new();

    let needs_datetime = table
        .fields
        .values()
        .any(|f| matches!(f.field_type.base_type(), "timestamp" | "datetime"));
    let needs_date = table
        .fields
        .values()
        .any(|f| f.field_type.base_type() == "date");
    let needs_time = table
        .fields
        .values()
        .any(|f| f.field_type.base_type() == "time");

    if needs_datetime || needs_date || needs_time {
        let mut chrono_imports = vec![];
        if needs_datetime {
            chrono_imports.push("DateTime");
            chrono_imports.push("Utc");
        }
        if needs_date {
            chrono_imports.push("NaiveDate");
        }
        if needs_time {
            chrono_imports.push("NaiveTime");
        }
        wrapper_imports.push(format!("use chrono::{{{}}};", chrono_imports.join(", ")));
    }

    let needs_decimal = table
        .fields
        .values()
        .any(|f| f.field_type.base_type() == "decimal");

    if needs_decimal {
        wrapper_imports.push("use rust_decimal::Decimal;".to_string());
    }

    let needs_json = table
        .fields
        .values()
        .any(|f| f.field_type.base_type() == "json");

    if needs_json {
        wrapper_imports.push("use serde_json;".to_string());
    }

    let needs_uuid = table
        .fields
        .values()
        .any(|f| f.field_type.base_type() == "uuid");

    if needs_uuid {
        wrapper_imports.push("use uuid::Uuid;".to_string());
    }

    let wrapper_imports_str = if !wrapper_imports.is_empty() {
        format!("\n{}", wrapper_imports.join("\n"))
    } else {
        String::new()
    };

    vars.insert("wrapper_imports".to_string(), wrapper_imports_str);

    // No builder setter delegations needed - Deref handles it automatically!
    vars.insert("builder_setter_delegations".to_string(), String::new());

    // No longer need wrapper setters - Deref handles delegation automatically!
    vars.insert("wrapper_setters".to_string(), String::new());

    // No longer need field accessors - Deref handles delegation automatically!
    let field_accessors = Vec::<String>::new();

    for field in &sorted_fields {
        if field.constraints.primary_key.unwrap_or(false) {
            continue; // ID is handled separately
        }

        let escaped_field_name = escape_rust_keyword(&field.name);
        let field_doc = field
            .ai
            .as_ref()
            .unwrap_or(&format!("Get the {} field", field.name))
            .clone();

        // Determine the return type
        let is_nullable = field.constraints.nullable.unwrap_or(false);
        let return_type = if let Some(ref lang_type) = field.lang_type {
            lang_type.clone()
        } else {
            match field.field_type.base_type() {
                // MySQL precise integer types for FromRow
                "tinyint" => {
                    if is_nullable {
                        "Option<i8>"
                    } else {
                        "i8"
                    }
                }
                "smallint" => {
                    if is_nullable {
                        "Option<i16>"
                    } else {
                        "i16"
                    }
                }
                "mediumint" | "int" | "integer" => {
                    if is_nullable {
                        "Option<i32>"
                    } else {
                        "i32"
                    }
                }
                "bigint" => {
                    if is_nullable {
                        "Option<i64>"
                    } else {
                        "i64"
                    }
                }
                "text" | "varchar" | "string" | "enum" => {
                    if is_nullable {
                        "Option<String>"
                    } else {
                        "String"
                    }
                }
                "boolean" | "bool" => {
                    if is_nullable {
                        "Option<bool>"
                    } else {
                        "bool"
                    }
                }
                "timestamp" | "datetime" => {
                    if is_nullable {
                        "Option<DateTime<Utc>>"
                    } else {
                        "DateTime<Utc>"
                    }
                }
                "date" => {
                    if is_nullable {
                        "Option<NaiveDate>"
                    } else {
                        "NaiveDate"
                    }
                }
                "time" => {
                    if is_nullable {
                        "Option<NaiveTime>"
                    } else {
                        "NaiveTime"
                    }
                }
                "decimal" => {
                    if is_nullable {
                        "Option<Decimal>"
                    } else {
                        "Decimal"
                    }
                }
                "json" => {
                    if is_nullable {
                        "Option<serde_json::Value>"
                    } else {
                        "serde_json::Value"
                    }
                }
                "uuid" => {
                    if is_nullable {
                        "Option<Uuid>"
                    } else {
                        "Uuid"
                    }
                }
                "real" | "float" | "double" => {
                    if is_nullable {
                        "Option<f64>"
                    } else {
                        "f64"
                    }
                }
                _ => {
                    if is_nullable {
                        "Option<String>"
                    } else {
                        "String"
                    }
                }
            }
            .to_string()
        };

        // For String types, we need to clone
        let needs_clone = return_type.contains("String");
        let accessor_impl = if needs_clone {
            format!("self.base.{}.clone()", escaped_field_name)
        } else {
            format!("self.base.{}", escaped_field_name)
        };

        // Skip generating field accessors - Deref handles this
    }

    // Empty field_accessors since Deref handles delegation
    vars.insert("field_accessors".to_string(), String::new());

    // Check if we should use macros for cleaner code
    let use_macros = true; // Always use macros for new generation
    vars.insert("use_macros".to_string(), use_macros.to_string());

    // No longer generating create_method_wrapper - builder pattern is the way

    vars
}

/// Generate base model include file using embedded template
fn generate_base_model_include_code(
    table: &rustf_schema::Table,
    schema: &Schema,
    checksum: &str,
    generation_time: &str,
    pool_type: &str,
) -> anyhow::Result<String> {
    // Get the include template
    let template = Templates::get("models/base_model_include.rs.template")
        .or_else(|| Templates::get("models/base_model.rs.template")) // Fallback to old template
        .ok_or_else(|| anyhow::anyhow!("Base model template not found"))?;

    let template_content = std::str::from_utf8(template.data.as_ref())?;

    // Prepare variables
    let variables =
        prepare_base_model_variables(table, schema, checksum, generation_time, pool_type);

    // Process template
    let processed = process_template(template_content, &variables);

    Ok(processed)
}

/// Generate wrapper model include code using embedded template
fn generate_wrapper_model_include_code(
    table: &rustf_schema::Table,
    pool_type: &str,
) -> anyhow::Result<String> {
    // Get the include template
    let template = Templates::get("models/wrapper_model_include.rs.template")
        .or_else(|| Templates::get("models/wrapper_model.rs.template")) // Fallback to old template
        .ok_or_else(|| anyhow::anyhow!("Wrapper model template not found"))?;

    let template_content = std::str::from_utf8(template.data.as_ref())?;

    // Prepare variables
    let variables = prepare_wrapper_model_variables(table, pool_type);

    // Process template
    let processed = process_template(template_content, &variables);

    Ok(processed)
}

/// Generate MODELS_README.md using embedded template
fn generate_models_readme_from_template(
    generation_time: &str,
    pool_type: &str,
) -> anyhow::Result<String> {
    // Get the template
    let template = Templates::get("readmes/models_README.md.template")
        .ok_or_else(|| anyhow::anyhow!("Models README template not found"))?;

    let template_content = std::str::from_utf8(template.data.as_ref())?;

    // Prepare variables
    let mut variables = HashMap::new();
    variables.insert("generation_time".to_string(), generation_time.to_string());
    variables.insert("cli_version".to_string(), "0.1.0".to_string());
    variables.insert("pool_type".to_string(), pool_type.to_string());

    // Process template
    let processed = process_template(template_content, &variables);

    Ok(processed)
}

/// Generate MODELS_README.md with comprehensive instructions (fallback)
fn generate_models_readme(_schema: &Schema) -> String {
    let generation_time = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    // Try template first, fallback to inline if template fails
    match generate_models_readme_from_template(&generation_time, "MySql") {
        Ok(content) => content,
        Err(_) => {
            // Fallback inline version (simplified)
            format!(
                r#"# 📋 RustF Models Guide

## 🏗️ Directory Structure

```
src/models/
├── base/           # 🚫 NEVER EDIT - Generated code only
│   ├── user_base.rs    # ⚠️  Overwritten on each generation
│   └── product_base.rs # ⚠️  Overwritten on each generation
├── user.rs         # ✅ EDIT HERE - Business logic
├── product.rs      # ✅ EDIT HERE - Business logic
└── MODELS_README.md    # 📖 This file
```

## 🤖 Instructions for AI Agents

### ❌ NEVER DO:
- Edit files in `base/`
- Modify files with "AUTOMATICALLY GENERATED" header
- Import UserBase, ProductBase directly in the application
- Add business logic to base models
- Remove the `register()` function from wrappers

### ✅ ALWAYS DO:
- Edit wrapper models (user.rs, product.rs, etc.)
- Add business methods to wrapper structs
- Use composition: `struct User {{ base: UserBase }}`
- Preserve the `register()` function in each model
- Guide users to wrapper models for customization
- Use `self.base.field` to access DB fields
- Use `self.base.create()`, `.update()`, `.delete()` for DB operations

## 🔄 Development workflow

1. **Initial generation**: `rustf-cli schema generate models`
2. **Development**: Edit `user.rs` to add business logic
3. **Schema evolution**: Modify `schemas/users.yaml`
4. **Regeneration**: `rustf-cli schema generate models` (only `base/` is overwritten)
5. **Auto-discovery**: `auto_models!()` automatically finds wrappers

## 📊 File types

| Type | Location | Editable | Description |
|------|----------|----------|-------------|
| Base | `base/*_base.rs` | ❌ No | Generated CRUD models |
| Wrapper | `*.rs` | ✅ Yes | Business logic |
| Schema | `schemas/*.yaml` | ✅ Yes | DB definition |

## 🚨 Important rules

- **Never import directly** base models in the application
- **Always use** wrappers (User, Product, etc.)
- **Preserve** the `register()` function for auto_models!()
- **Composition** rather than inheritance: `User {{ base: UserBase }}`

---

### 📚 Generated at {} by RustF CLI v0.1.0
"#,
                generation_time
            )
        }
    }
}

/// Simple compilation validation for generated models
///
/// This creates a temporary Rust file and runs `cargo check` to ensure
/// the generated code compiles without errors.
#[allow(dead_code)]
fn validate_generated_model_compilation(model_name: &str, code: &str) -> anyhow::Result<()> {
    use std::fs;
    use std::process::Command;
    use tempfile::NamedTempFile;

    // Create a temporary Rust file
    let temp_file = NamedTempFile::new()?;
    let temp_path = temp_file.path();

    // Write the generated code to temp file
    fs::write(temp_path, code)?;

    // Run cargo check on the file (basic syntax validation)
    let output = Command::new("rustc")
        .arg("--edition")
        .arg("2021")
        .arg("--crate-type")
        .arg("lib")
        .arg("--check-cfg")
        .arg("cfg()")
        .arg(temp_path)
        .output();

    match output {
        Ok(result) => {
            if !result.status.success() {
                let stderr = String::from_utf8_lossy(&result.stderr);
                eprintln!(
                    "⚠️  Compilation validation failed for model '{}':",
                    model_name
                );
                eprintln!("{}", stderr);

                // Don't fail the generation, just warn
                println!("🔍 Generated code saved to: {:?} for debugging", temp_path);
                return Err(anyhow::anyhow!(
                    "Model '{}' has compilation errors. Check the generated code.",
                    model_name
                ));
            }
            println!(
                "✅ Compilation validation passed for model '{}'",
                model_name
            );
            Ok(())
        }
        Err(e) => {
            // rustc not available, skip validation
            println!(
                "⚠️  Skipping compilation validation (rustc not found): {}",
                e
            );
            Ok(())
        }
    }
}
