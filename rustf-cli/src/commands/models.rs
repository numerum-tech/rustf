use std::path::PathBuf;
use anyhow::Result;
use std::fs;
use walkdir::WalkDir;
use serde_json;
use rustf_schema::Schema;

pub async fn run(project_path: PathBuf, relationships: bool) -> Result<()> {
    log::info!("Analyzing models in: {}", project_path.display());
    
    println!("🔍 Analyzing RustF project models...");
    
    // 1. Check for models directory
    let models_dir = project_path.join("src/models");
    if !models_dir.exists() {
        println!("⚠️  No src/models directory found");
        return Ok(());
    }
    
    // 2. Scan for model files
    println!("\n📋 Scanning for model files...");
    let mut model_files = Vec::new();
    let mut schema_files = Vec::new();
    
    // Scan models directory
    for entry in WalkDir::new(&models_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("rs") {
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name != "mod.rs" {
                    model_files.push(path.to_path_buf());
                }
            }
        }
    }
    
    // Scan schemas directory
    let schemas_dir = project_path.join("schemas");
    if schemas_dir.exists() {
        for entry in WalkDir::new(&schemas_dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                schema_files.push(path.to_path_buf());
            }
        }
    }
    
    println!("✅ Found {} model file(s)", model_files.len());
    println!("✅ Found {} schema file(s)", schema_files.len());
    
    if model_files.is_empty() && schema_files.is_empty() {
        println!("\n💡 No models or schemas found. Consider:");
        println!("   - Creating schema files in schemas/ directory");
        println!("   - Running 'rustf schema generate' to create models from schemas");
        println!("   - Running 'rustf db generate-schemas' to create schemas from database");
        return Ok(());
    }
    
    // 3. Analyze model files
    if !model_files.is_empty() {
        println!("\n📊 Model Analysis:");
        println!("==================");
        
        for model_file in &model_files {
            if let Some(file_name) = model_file.file_name().and_then(|n| n.to_str()) {
                let model_name = file_name.strip_suffix(".rs").unwrap_or(file_name);
                println!("📄 {}", model_name);
                
                // Try to read and analyze the model file
                if let Ok(content) = fs::read_to_string(model_file) {
                    analyze_model_content(&content, model_name, relationships);
                } else {
                    println!("   ❌ Could not read file");
                }
                println!();
            }
        }
    }
    
    // 4. Analyze schema files
    if !schema_files.is_empty() {
        println!("\n📋 Schema Analysis:");
        println!("===================");
        
        for schema_file in &schema_files {
            if let Some(file_name) = schema_file.file_name().and_then(|n| n.to_str()) {
                let schema_name = file_name.strip_suffix(".yaml").unwrap_or(file_name);
                println!("📄 {}", schema_name);
                
                // Try to read and analyze the schema file
                if let Ok(content) = fs::read_to_string(schema_file) {
                    analyze_schema_content(&content, schema_name, relationships);
                } else {
                    println!("   ❌ Could not read schema file");
                }
                println!();
            }
        }
    }
    
    // 5. Check for model-schema consistency
    if !model_files.is_empty() && !schema_files.is_empty() {
        println!("\n🔄 Model-Schema Consistency:");
        println!("=============================");
        
        let model_names: Vec<String> = model_files.iter()
            .filter_map(|p| p.file_stem().and_then(|s| s.to_str()))
            .map(|s| s.to_string())
            .collect();
            
        let schema_names: Vec<String> = schema_files.iter()
            .filter_map(|p| p.file_stem().and_then(|s| s.to_str()))
            .map(|s| s.to_string())
            .collect();
        
        // Check for models without schemas
        for model_name in &model_names {
            if !schema_names.contains(model_name) {
                println!("⚠️  Model '{}' has no corresponding schema", model_name);
            }
        }
        
        // Check for schemas without models
        for schema_name in &schema_names {
            if !model_names.contains(schema_name) {
                println!("💡 Schema '{}' could generate a model (run: rustf schema generate)", schema_name);
            }
        }
        
        // Check for matching pairs
        let matching_pairs: Vec<_> = model_names.iter()
            .filter(|m| schema_names.contains(m))
            .collect();
        
        if !matching_pairs.is_empty() {
            println!("✅ Found {} model-schema pairs:", matching_pairs.len());
            for pair in matching_pairs {
                println!("   📋 {}", pair);
            }
        }
    }
    
    // 6. Provide recommendations
    println!("\n💡 Recommendations:");
    println!("===================");
    
    if schema_files.is_empty() {
        println!("📝 Consider creating schema files for better code generation");
        println!("   Run: rustf db generate-schemas --database-url <url>");
    }
    
    if model_files.is_empty() && !schema_files.is_empty() {
        println!("🏗️  Generate models from your schemas");
        println!("   Run: rustf schema generate");
    }
    
    if relationships {
        println!("🔗 Relationship analysis is enabled");
        if !schema_files.is_empty() || !model_files.is_empty() {
            println!("   Review the relationship patterns shown above");
        }
    }
    
    println!("\n✅ Model analysis completed!");
    Ok(())
}

fn analyze_model_content(content: &str, _model_name: &str, show_relationships: bool) {
    let mut has_struct = false;
    let mut has_impl = false;
    let mut has_derives = false;
    let mut has_fields = false;
    let mut field_count = 0;
    let mut relationships = Vec::new();
    
    for line in content.lines() {
        let line = line.trim();
        
        // Check for struct definition
        if line.starts_with("pub struct") || line.starts_with("struct") {
            has_struct = true;
        }
        
        // Check for impl blocks
        if line.starts_with("impl ") {
            has_impl = true;
        }
        
        // Check for derives
        if line.starts_with("#[derive(") {
            has_derives = true;
        }
        
        // Count fields (simplified)
        if has_struct && (line.contains("pub ") || line.contains(":")) && line.contains(":") {
            has_fields = true;
            field_count += 1;
        }
        
        // Look for potential relationships
        if show_relationships {
            if line.contains("Vec<") || line.contains("Option<") {
                if let Some(type_start) = line.find(":") {
                    let type_part = &line[type_start..];
                    if type_part.contains("Vec<") || type_part.contains("Option<") {
                        relationships.push(format!("   🔗 {}", line.trim()));
                    }
                }
            }
        }
    }
    
    // Report findings
    if has_struct {
        println!("   ✅ Contains struct definition");
    } else {
        println!("   ⚠️  No struct definition found");
    }
    
    if has_derives {
        println!("   ✅ Has derive macros");
    }
    
    if has_impl {
        println!("   ✅ Has implementation block(s)");
    }
    
    if has_fields {
        println!("   📊 {} field(s) detected", field_count);
    }
    
    if show_relationships && !relationships.is_empty() {
        println!("   🔗 Potential relationships:");
        for rel in relationships {
            println!("     {}", rel);
        }
    }
    
    // Check for common RustF patterns
    if content.contains("sqlx::") {
        println!("   📊 Uses SQLx for database operations");
    }
    
    if content.contains("serde::") || content.contains("Serialize") || content.contains("Deserialize") {
        println!("   📊 Has serialization support");
    }
    
    if content.contains("FromRow") {
        println!("   📊 Implements SQLx FromRow");
    }
}

fn analyze_schema_content(content: &str, _schema_name: &str, show_relationships: bool) {
    // Try to parse as YAML
    match serde_yaml::from_str::<serde_yaml::Value>(content) {
        Ok(schema) => {
            if let Some(mapping) = schema.as_mapping() {
                // Count tables/entities
                let table_count = mapping.len();
                println!("   ✅ Valid YAML schema with {} table(s)", table_count);
                
                // Analyze each table
                for (key, value) in mapping {
                    if let (Some(table_name), Some(table_def)) = (key.as_str(), value.as_mapping()) {
                        println!("   📋 Table: {}", table_name);
                        
                        // Count fields
                        if let Some(fields) = table_def.get("fields").and_then(|f| f.as_mapping()) {
                            println!("     📊 {} field(s)", fields.len());
                            
                            // Check for relationships
                            if show_relationships {
                                if let Some(relations) = table_def.get("relations").and_then(|r| r.as_mapping()) {
                                    if !relations.is_empty() {
                                        println!("     🔗 {} relationship(s) defined", relations.len());
                                        
                                        for (rel_type, rel_def) in relations {
                                            if let Some(rel_name) = rel_type.as_str() {
                                                println!("       🔗 {}: {:?}", rel_name, rel_def);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        
                        // Check for indexes
                        if let Some(indexes) = table_def.get("indexes").and_then(|i| i.as_sequence()) {
                            if !indexes.is_empty() {
                                println!("     📇 {} index(es) defined", indexes.len());
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            println!("   ❌ Invalid YAML schema: {}", e);
        }
    }
}

/// Get model metadata for AI agents (field hints, validation rules, schema info)
pub async fn metadata(project_path: PathBuf, model_name: String, format: String) -> Result<()> {
    log::info!("Getting metadata for model '{}' in: {}", model_name, project_path.display());
    
    // Check for schemas directory first
    let schemas_dir = project_path.join("schemas");
    if !schemas_dir.exists() {
        println!("❌ No schemas directory found at {:?}", schemas_dir);
        println!("💡 Model metadata requires schema files to be present");
        println!("💡 Try running: rustf-cli db generate-schema --output schemas");
        return Ok(());
    }
    
    // Load schema
    let schema = match Schema::load_from_directory(&schemas_dir).await {
        Ok(s) => s,
        Err(e) => {
            println!("❌ Failed to load schema: {}", e);
            return Ok(());
        }
    };
    
    // Find the table by model name (convert PascalCase to snake_case)
    let table_name = pascal_to_snake_case(&model_name);
    let table = match schema.tables.get(&table_name) {
        Some(t) => t,
        None => {
            // Try direct lookup if conversion didn't work
            match schema.tables.get(&model_name) {
                Some(t) => t,
                None => {
                    println!("❌ Model '{}' not found in schema", model_name);
                    println!("Available models:");
                    for table_name in schema.tables.keys() {
                        let model_name_converted = snake_to_pascal_case(table_name);
                        println!("  • {} (table: {})", model_name_converted, table_name);
                    }
                    return Ok(());
                }
            }
        }
    };
    
    // Collect field hints for AI agents
    let mut field_hints = Vec::new();
    for field in table.fields.values() {
        if let Some(ai_hint) = &field.ai {
            field_hints.push((field.name.clone(), ai_hint.clone()));
        }
    }
    
    // Collect validation rules
    let mut validation_rules = Vec::new();
    for field in table.fields.values() {
        let mut rules = Vec::new();
        
        if field.constraints.required.unwrap_or(false) {
            rules.push("required");
        }
        if field.constraints.unique.unwrap_or(false) {
            rules.push("unique");
        }
        if field.constraints.primary_key.unwrap_or(false) {
            rules.push("primary_key");
        }
        if let Some(_fk) = &field.constraints.foreign_key {
            rules.push("foreign_key");
        }
        if field.name.contains("email") {
            rules.push("email_format");
        }
        if field.name.contains("password") {
            rules.push("secure_hash");
        }
        
        if !rules.is_empty() {
            validation_rules.push((field.name.clone(), rules));
        }
    }
    
    // Collect field types and constraints
    let mut field_info = Vec::new();
    for field in table.fields.values() {
        let field_meta = serde_json::json!({
            "name": field.name,
            "type": field.field_type,
            "rust_type": field.lang_type.as_ref().unwrap_or(&"String".to_string()),
            "nullable": field.constraints.nullable.unwrap_or(false),
            "required": field.constraints.required.unwrap_or(false),
            "unique": field.constraints.unique.unwrap_or(false),
            "primary_key": field.constraints.primary_key.unwrap_or(false),
            "foreign_key": field.constraints.foreign_key,
            "ai_hint": field.ai
        });
        field_info.push(field_meta);
    }
    
    // Build metadata response
    let metadata = serde_json::json!({
        "model_name": model_name,
        "table_name": table.table,
        "description": table.description,
        "ai_context": table.ai_context,
        "field_hints": field_hints,
        "validation_rules": validation_rules,
        "fields": field_info,
        "relationships": {
            "belongs_to": table.relations.belongs_to,
            "has_many": table.relations.has_many,
            "has_one": table.relations.has_one,
            "many_to_many": table.relations.many_to_many
        }
    });
    
    match format.as_str() {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&metadata)?);
        }
        "yaml" => {
            println!("{}", serde_yaml::to_string(&metadata)?);
        }
        "table" => {
            print_metadata_table(&metadata, &model_name);
        }
        _ => {
            println!("❌ Unsupported format '{}'. Use: json, yaml, table", format);
            return Ok(());
        }
    }
    
    Ok(())
}

/// Convert PascalCase to snake_case
fn pascal_to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    
    while let Some(c) = chars.next() {
        if c.is_uppercase() && !result.is_empty() {
            result.push('_');
        }
        result.push(c.to_lowercase().next().unwrap());
    }
    
    result
}

/// Convert snake_case to PascalCase
fn snake_to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<String>()
}

/// Print metadata in table format for AI agents
fn print_metadata_table(metadata: &serde_json::Value, model_name: &str) {
    println!("🤖 AI Agent Metadata for Model: {}", model_name);
    println!("==========================================");
    
    if let Some(description) = metadata["description"].as_str() {
        println!("📝 Description: {}", description);
    }
    
    if let Some(ai_context) = metadata["ai_context"].as_str() {
        println!("🎯 AI Context: {}", ai_context);
    }
    
    println!("📋 Table: {}", metadata["table_name"].as_str().unwrap_or("unknown"));
    println!();
    
    // Field Hints
    if let Some(field_hints) = metadata["field_hints"].as_array() {
        if !field_hints.is_empty() {
            println!("💡 Field Hints for AI Agents:");
            println!("------------------------------");
            for hint in field_hints {
                if let (Some(field), Some(description)) = (hint[0].as_str(), hint[1].as_str()) {
                    println!("  • {}: {}", field, description);
                }
            }
            println!();
        }
    }
    
    // Validation Rules
    if let Some(validation_rules) = metadata["validation_rules"].as_array() {
        if !validation_rules.is_empty() {
            println!("✅ Validation Rules:");
            println!("--------------------");
            for rule in validation_rules {
                if let (Some(field), Some(rules)) = (rule[0].as_str(), rule[1].as_array()) {
                    let rule_strs: Vec<&str> = rules.iter().filter_map(|r| r.as_str()).collect();
                    println!("  • {}: [{}]", field, rule_strs.join(", "));
                }
            }
            println!();
        }
    }
    
    // Field Information
    if let Some(fields) = metadata["fields"].as_array() {
        println!("📊 Field Details:");
        println!("-----------------");
        for field in fields {
            let name = field["name"].as_str().unwrap_or("unknown");
            let rust_type = field["rust_type"].as_str().unwrap_or("String");
            let required = field["required"].as_bool().unwrap_or(false);
            let unique = field["unique"].as_bool().unwrap_or(false);
            let pk = field["primary_key"].as_bool().unwrap_or(false);
            
            let mut flags = Vec::new();
            if pk { flags.push("PK"); }
            if unique && !pk { flags.push("UNIQUE"); }
            if required { flags.push("REQUIRED"); }
            
            let flags_str = if flags.is_empty() {
                String::new()
            } else {
                format!(" ({})", flags.join(", "))
            };
            
            println!("  • {}: {}{}", name, rust_type, flags_str);
            
            if let Some(ai_hint) = field["ai_hint"].as_str() {
                println!("    💡 {}", ai_hint);
            }
        }
    }
    
    println!("\n🔧 Usage in AI Code Generation:");
    println!("-------------------------------");
    println!("  • Use field hints to understand column purposes");
    println!("  • Apply validation rules for data integrity");
    println!("  • Reference field types for proper Rust code generation");
    println!("  • Follow AI context guidance for business logic");
}