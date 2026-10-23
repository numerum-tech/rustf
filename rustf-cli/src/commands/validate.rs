use anyhow::Result;
use std::fs;
use std::path::PathBuf;

pub async fn run(project_path: PathBuf, fix: bool) -> Result<()> {
    log::info!("Validating project at: {}", project_path.display());

    let mut issues = Vec::new();
    let mut warnings = Vec::new();

    // 1. Check if it's a valid RustF project
    println!("🔍 Validating RustF project structure...");

    // Check for Cargo.toml
    let cargo_toml = project_path.join("Cargo.toml");
    if !cargo_toml.exists() {
        issues.push("❌ Missing Cargo.toml file".to_string());
    } else {
        // Check if rustf is a dependency
        if let Ok(content) = fs::read_to_string(&cargo_toml) {
            if !content.contains("rustf") {
                warnings.push("⚠️  RustF dependency not found in Cargo.toml".to_string());
            } else {
                println!("✅ Found RustF dependency in Cargo.toml");
            }
        }
    }

    // Check for src directory
    let src_dir = project_path.join("src");
    if !src_dir.exists() {
        issues.push("❌ Missing src/ directory".to_string());
    } else {
        println!("✅ Found src/ directory");

        // Check for main.rs
        let main_rs = src_dir.join("main.rs");
        if !main_rs.exists() {
            warnings.push("⚠️  Missing src/main.rs file".to_string());
        } else {
            println!("✅ Found src/main.rs");
        }
    }

    // 2. Validate project structure conventions
    println!("\n📁 Validating directory structure...");

    let expected_dirs = [
        ("src/controllers", "Controllers directory"),
        ("src/models", "Models directory"),
        ("src/middleware", "Middleware directory"),
        ("views", "Views directory"),
        ("public", "Public assets directory"),
        ("schemas", "Schema definitions directory"),
    ];

    for (dir_path, description) in expected_dirs {
        let full_path = project_path.join(dir_path);
        if full_path.exists() {
            println!("✅ Found {}: {}", description, dir_path);
        } else {
            warnings.push(format!("⚠️  Missing {}: {}", description, dir_path));
        }
    }

    // 3. Validate configuration files
    println!("\n⚙️  Validating configuration...");

    let config_toml = project_path.join("config.toml");
    if config_toml.exists() {
        println!("✅ Found config.toml");

        // Try to parse the config
        if let Ok(content) = fs::read_to_string(&config_toml) {
            match toml::from_str::<toml::Value>(&content) {
                Ok(_) => println!("✅ config.toml is valid TOML"),
                Err(e) => issues.push(format!("❌ Invalid config.toml: {}", e)),
            }
        }
    } else {
        warnings.push("⚠️  No config.toml found (will use defaults)".to_string());
    }

    // Also check for development config (automatically loaded by CLI)
    let dev_config_toml = project_path.join("config.dev.toml");
    if dev_config_toml.exists() {
        println!("✅ Found config.dev.toml");

        // Try to parse the development config
        if let Ok(content) = fs::read_to_string(&dev_config_toml) {
            match toml::from_str::<toml::Value>(&content) {
                Ok(_) => println!("✅ config.dev.toml is valid TOML"),
                Err(e) => issues.push(format!("❌ Invalid config.dev.toml: {}", e)),
            }
        }
    } else {
        println!("ℹ️  No config.dev.toml found (optional, but recommended for CLI usage)");
    }

    // The configuration the application would actually load: merge,
    // "(env)"/"(file)" references, types and validation, not just syntax.
    if config_toml.exists() {
        match rustf::config::AppConfig::load_with_base_dir(&project_path) {
            Ok(config) => println!(
                "✅ Configuration loads ({} environment, references resolved)",
                config.environment.as_str()
            ),
            Err(e) => issues.push(format!("❌ Configuration does not load: {}", e)),
        }
    }

    // Files that are not loaded here — the production overlay, whose variables
    // and secret files are usually not on this machine, and a development
    // overlay without a base config.toml — get their references' tags
    // checked without resolving them.
    let mut tag_checked = vec![project_path.join("config.prod.toml")];
    if !config_toml.exists() {
        tag_checked.push(dev_config_toml.clone());
    }
    for file in tag_checked.iter().filter(|f| f.exists()) {
        let name = file.file_name().unwrap_or_default().to_string_lossy();
        match rustf::config::AppConfig::check_references(file) {
            Ok(()) => println!("✅ {} is valid TOML with valid references", name),
            Err(e) => issues.push(format!("❌ Invalid {}: {}", name, e)),
        }
    }

    // 4. Validate schema files
    println!("\n📋 Validating schema files...");

    let schemas_dir = project_path.join("schemas");
    if schemas_dir.exists() {
        match fs::read_dir(&schemas_dir) {
            Ok(entries) => {
                let mut schema_count = 0;
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                        schema_count += 1;

                        // Try to parse the schema
                        if let Ok(content) = fs::read_to_string(&path) {
                            match serde_yaml::from_str::<serde_yaml::Value>(&content) {
                                Ok(_) => {
                                    if let Some(file_name) =
                                        path.file_name().and_then(|n| n.to_str())
                                    {
                                        println!("✅ Valid schema: {}", file_name);
                                    }
                                }
                                Err(e) => {
                                    if let Some(file_name) =
                                        path.file_name().and_then(|n| n.to_str())
                                    {
                                        issues.push(format!(
                                            "❌ Invalid schema {}: {}",
                                            file_name, e
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }

                if schema_count == 0 {
                    warnings.push("⚠️  No YAML schema files found in schemas/".to_string());
                } else {
                    println!("✅ Found {} schema file(s)", schema_count);
                }
            }
            Err(_) => {
                warnings.push("⚠️  Cannot read schemas directory".to_string());
            }
        }
    }

    // 5. Validate dependencies and build
    println!("\n🔧 Validating build configuration...");

    // Check if the project can be built
    let output = std::process::Command::new("cargo")
        .arg("check")
        .current_dir(&project_path)
        .output();

    match output {
        Ok(result) => {
            if result.status.success() {
                println!("✅ Project compiles successfully");
            } else {
                let stderr = String::from_utf8_lossy(&result.stderr);
                issues.push(format!("❌ Build check failed:\n{}", stderr));
            }
        }
        Err(_) => {
            warnings.push("⚠️  Could not run cargo check (cargo not found?)".to_string());
        }
    }

    // 6. Report results
    println!("\n📊 Validation Summary");
    println!("=====================");

    if issues.is_empty() && warnings.is_empty() {
        println!("🎉 Project validation passed with no issues!");
        return Ok(());
    }

    if !warnings.is_empty() {
        println!("\n⚠️  Warnings ({}):", warnings.len());
        for warning in &warnings {
            println!("   {}", warning);
        }
    }

    if !issues.is_empty() {
        println!("\n❌ Issues ({}):", issues.len());
        for issue in &issues {
            println!("   {}", issue);
        }

        if fix {
            println!("\n🔧 Auto-fix is not yet implemented, but these issues were found:");
            println!("   - Consider running 'rustf new' to create missing structure");
            println!("   - Check the RustF documentation for proper project setup");
        }

        return Err(anyhow::anyhow!("{} validation issues found", issues.len()));
    }

    println!(
        "\n✅ Validation completed with {} warning(s)",
        warnings.len()
    );
    Ok(())
}
