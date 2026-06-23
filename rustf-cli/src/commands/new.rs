use anyhow::{anyhow, Result};
use rand::Rng;
use rust_embed::RustEmbed;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(RustEmbed)]
#[folder = "templates/"]
struct Templates;

/// Create a new RustF project with AI-friendly structure
pub async fn run(project_name: String, target_path: Option<PathBuf>, force: bool) -> Result<()> {
    // Validate and normalize project name
    let normalized_name = normalize_project_name(&project_name)?;
    let project_title = project_name_to_title(&project_name);

    // Determine target directory
    let base_path = target_path.unwrap_or_else(|| std::env::current_dir().unwrap());
    let project_path = base_path.join(&normalized_name);

    // Check if directory exists and handle accordingly
    if project_path.exists() {
        if !force {
            if is_directory_empty(&project_path)? {
                println!("📁 Directory exists but is empty, proceeding...");
            } else {
                return Err(anyhow!(
                    "Directory '{}' already exists and is not empty. Use --force to overwrite.",
                    project_path.display()
                ));
            }
        } else {
            // Create backup before overwriting
            if !is_directory_empty(&project_path)? {
                use crate::utils::backup::BackupManager;
                let backup_manager = BackupManager::new()?;
                backup_manager.backup_directory(&project_path, "project")?;
            }
            println!(
                "⚠️  Overwriting existing directory: {}",
                project_path.display()
            );
        }
    }

    println!("🚀 Creating new RustF project: {}", project_title);
    println!("📁 Project directory: {}", project_path.display());

    // Create project structure
    create_project_structure(&project_path)?;

    // Generate template variables
    let variables = create_template_variables(&normalized_name, &project_title);

    // Generate files from templates
    generate_project_files(&project_path, &variables)?;

    println!("✅ Project '{}' created successfully!", project_title);
    println!();
    println!("📋 Next steps:");
    println!("   cd {}", normalized_name);
    println!("   cargo run");
    println!();
    println!("🤖 AI-friendly features included:");
    println!("   • Auto-discovery for controllers, models, middleware, and definitions");
    println!("   • Definitions system for customizing framework behavior");
    println!("   • Comprehensive README files in each directory");
    println!("   • Schema-based model generation support");
    println!("   • Template engine with layout support");
    println!("   • Built-in middleware and security features");

    Ok(())
}

fn normalize_project_name(name: &str) -> Result<String> {
    // Validate project name
    if name.trim().is_empty() {
        return Err(anyhow!("Project name cannot be empty"));
    }

    // Convert to snake_case and validate
    let normalized = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else if c.is_whitespace() || c == '-' {
                '_'
            } else {
                '_'
            }
        })
        .collect::<String>()
        // Remove duplicate underscores
        .split('_')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("_");

    // Ensure it starts with a letter
    if !normalized
        .chars()
        .next()
        .unwrap_or('_')
        .is_ascii_alphabetic()
    {
        return Err(anyhow!("Project name must start with a letter"));
    }

    // Ensure it's not too long
    if normalized.len() > 50 {
        return Err(anyhow!("Project name is too long (max 50 characters)"));
    }

    Ok(normalized)
}

fn project_name_to_title(name: &str) -> String {
    name.split(|c: char| c.is_whitespace() || c == '_' || c == '-')
        .filter(|s| !s.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_directory_empty(path: &Path) -> Result<bool> {
    if !path.exists() {
        return Ok(true);
    }

    if !path.is_dir() {
        return Err(anyhow!(
            "Path exists but is not a directory: {}",
            path.display()
        ));
    }

    let entries = fs::read_dir(path)?;
    Ok(entries.count() == 0)
}

fn create_project_structure(project_path: &Path) -> Result<()> {
    let directories = [
        // Main project directory
        "",
        // Source directories
        "src",
        "src/controllers",
        "src/middleware",
        "src/modules",
        "src/models",
        "src/models/base",
        "src/definitions",
        // View directories
        "views",
        "views/layouts",
        // Public asset directories
        "public",
        "public/css",
        "public/js",
        "public/images",
        // Private storage directories
        "private",
        "private/uploads",
        // Schema directory
        "schemas",
    ];

    for dir in &directories {
        let dir_path = if dir.is_empty() {
            project_path.to_path_buf()
        } else {
            project_path.join(dir)
        };

        fs::create_dir_all(&dir_path)?;

        // Create .gitkeep files for empty directories that should be preserved
        if matches!(*dir, "private/uploads" | "src/models/base") {
            let gitkeep_path = dir_path.join(".gitkeep");
            File::create(gitkeep_path)?.write_all(b"")?;
        }
    }

    Ok(())
}

fn create_template_variables(project_name: &str, project_title: &str) -> HashMap<String, String> {
    let mut variables = HashMap::new();

    // Basic project info
    variables.insert("project_name".to_string(), project_name.to_string());
    variables.insert("project_title".to_string(), project_title.to_string());

    // Generate random session secret
    let session_secret = generate_session_secret();
    variables.insert("session_secret".to_string(), session_secret);

    variables
}

fn generate_session_secret() -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();

    (0..64)
        .map(|_| {
            let idx = rng.gen_range(0..CHARS.len());
            CHARS[idx] as char
        })
        .collect()
}

fn generate_project_files(project_path: &Path, variables: &HashMap<String, String>) -> Result<()> {
    // Template mappings: (embedded_path, output_path)
    let file_mappings = [
        // Main project files
        ("project/Cargo.toml.template", "Cargo.toml"),
        ("project/config.toml.template", "config.toml"),
        ("project/config.dev.toml.template", "config.dev.toml"),
        ("project/main.rs.template", "src/main.rs"),
        ("project/README.md.template", "README.md"),
        ("project/gitignore.template", ".gitignore"),
        // View templates
        (
            "views/layouts/default.html.template",
            "views/layouts/default.html",
        ),
        // Directory README files
        (
            "readmes/controllers_README.md.template",
            "src/controllers/README.md",
        ),
        (
            "readmes/middleware_README.md.template",
            "src/middleware/README.md",
        ),
        (
            "readmes/modules_README.md.template",
            "src/modules/README.md",
        ),
        ("readmes/models_README.md.template", "src/models/README.md"),
        (
            "readmes/models_base_README.md.template",
            "src/models/base/README.md",
        ),
        (
            "readmes/definitions_README.md.template",
            "src/definitions/README.md",
        ),
        ("readmes/views_README.md.template", "views/README.md"),
        ("readmes/schemas_README.md.template", "schemas/README.md"),
        (
            "readmes/public_css_README.md.template",
            "public/css/README.md",
        ),
        (
            "readmes/public_js_README.md.template",
            "public/js/README.md",
        ),
        (
            "readmes/public_images_README.md.template",
            "public/images/README.md",
        ),
        ("readmes/uploads_README.md.template", "private/uploads/README.md"),
        // Schema files
        ("schemas/sessions.yaml.template", "schemas/sessions.yaml"),
        // Claude Code skill — teaches AI collaborators the RustF
        // layering rule + code-verified handler/middleware/model/view
        // conventions. Loaded automatically by Claude Code when editing
        // this project.
        (
            "project/claude_skills/rustf/SKILL.md",
            ".claude/skills/rustf/SKILL.md",
        ),
    ];

    for (template_path, output_path) in &file_mappings {
        println!("📝 Creating {}", output_path);

        let template_content = Templates::get(template_path)
            .ok_or_else(|| anyhow!("Template not found: {}", template_path))?;

        let content = std::str::from_utf8(template_content.data.as_ref())?;
        let processed_content = process_template(content, variables)?;

        let output_file_path = project_path.join(output_path);

        // Ensure parent directory exists
        if let Some(parent) = output_file_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&output_file_path, processed_content)?;
    }

    // Create a sample controller
    create_sample_controller(project_path, variables)?;

    // Create a sample definition
    create_sample_definition(project_path, variables)?;

    Ok(())
}

fn process_template(content: &str, variables: &HashMap<String, String>) -> Result<String> {
    let mut processed = content.to_string();

    for (key, value) in variables {
        let placeholder = format!("{{{{{}}}}}", key);
        processed = processed.replace(&placeholder, value);
    }

    Ok(processed)
}

fn create_sample_controller(
    project_path: &Path,
    variables: &HashMap<String, String>,
) -> Result<()> {
    let controller_content = format!(
        r#"use rustf::prelude::*;

pub fn install() -> Vec<Route> {{
    routes![
        GET "/" => index,
        GET "/about" => about,
    ]
}}

async fn index(ctx: &mut Context) -> Result<()> {{
    let data = json!({{
        "title": "Welcome to {}",
        "message": "Your RustF application is running successfully!",
        "features": [
            "🚀 Auto-discovery for controllers, models, and middleware",
            "🎨 Total.js template engine with layout support",
            "🔐 Built-in session management and security features",
            "📊 Schema-based model generation",
            "🛡️ Comprehensive middleware system",
            "🤖 AI-friendly documentation and patterns"
        ]
    }});

    ctx.view("/home/index", data)
}}

async fn about(ctx: &mut Context) -> Result<()> {{
    let data = json!({{
        "title": "About {}",
        "description": "Built with the RustF framework - an AI-friendly MVC framework for Rust."
    }});

    ctx.view("/home/about", data)
}}
"#,
        variables
            .get("project_title")
            .unwrap_or(&"RustF App".to_string()),
        variables
            .get("project_title")
            .unwrap_or(&"RustF App".to_string())
    );

    let controller_path = project_path.join("src/controllers/home.rs");
    fs::write(controller_path, controller_content)?;
    println!("📝 Creating src/controllers/home.rs");

    // Create corresponding view templates
    create_sample_views(project_path, variables)?;

    Ok(())
}

fn create_sample_views(project_path: &Path, variables: &HashMap<String, String>) -> Result<()> {
    // Create home directory
    let home_views_dir = project_path.join("views/home");
    fs::create_dir_all(&home_views_dir)?;

    // Load index.html template from embedded files
    let index_template = Templates::get("views/home/index.html.template")
        .ok_or_else(|| anyhow!("Template not found: views/home/index.html.template"))?;
    let index_content = std::str::from_utf8(index_template.data.as_ref())?;
    let processed_index = process_template(index_content, variables)?;

    let index_path = home_views_dir.join("index.html");
    fs::write(index_path, processed_index)?;
    println!("📝 Creating views/home/index.html");

    // Load about.html template from embedded files
    let about_template = Templates::get("views/home/about.html.template")
        .ok_or_else(|| anyhow!("Template not found: views/home/about.html.template"))?;
    let about_content = std::str::from_utf8(about_template.data.as_ref())?;
    let processed_about = process_template(about_content, variables)?;

    let about_path = home_views_dir.join("about.html");
    fs::write(about_path, processed_about)?;
    println!("📝 Creating views/home/about.html");

    Ok(())
}

fn create_sample_definition(
    project_path: &Path,
    _variables: &HashMap<String, String>,
) -> Result<()> {
    let definition_content = format!(
        r#"//! Application definitions
//!
//! Customises framework behaviour through the definitions system:
//! template helpers, validators, and the session-storage factory.
//!
//! NOTE: The definitions system covers helpers + validators + a session-
//! storage factory. It does NOT include a generic interceptor/provider
//! system — if you need one, reach for middleware, events, or shared
//! modules instead.

use rustf::definitions::{{Definitions, Helper, Validator}};
use rustf::prelude::*;
use serde_json::Value;

/// Install function called by auto-discovery.
pub fn install(defs: &mut Definitions) {{
    register_helpers(defs);

    // Uncomment to add more customisations:
    // register_validators(defs);
    // defs.set_session_storage_factory(my_session_factory);
}}

/// Register template helpers (callable from views).
fn register_helpers(defs: &mut Definitions) {{
    // Closure-based helper — goes through HelperRegistry::register_fn,
    // which takes (name, description, closure).
    defs.helpers.register_fn(
        "format_number",
        "Format a number with thousands separators (e.g. 1234 -> 1,234)",
        |args, _ctx| {{
            if let Some(Value::Number(n)) = args.first() {{
                if let Some(num) = n.as_u64() {{
                    let formatted = format!("{{}}", num)
                        .chars()
                        .rev()
                        .enumerate()
                        .map(|(i, c)| {{
                            if i > 0 && i % 3 == 0 {{
                                format!(",{{}}", c)
                            }} else {{
                                c.to_string()
                            }}
                        }})
                        .collect::<String>()
                        .chars()
                        .rev()
                        .collect::<String>();
                    return Ok(Value::String(formatted));
                }}
            }}
            Ok(args.first().cloned().unwrap_or(Value::Null))
        }},
    );

    // App-specific helper.
    defs.helpers.register_fn(
        "app_version",
        "Return the current app version string",
        |_args, _ctx| Ok(Value::String("1.0.0".to_string())),
    );
}}

// Example: Custom validators (uncomment to use). Same register_fn shape:
// (name, description, closure) — goes through ValidatorRegistry::register_fn.
/*
fn register_validators(defs: &mut Definitions) {{
    defs.validators.register_fn(
        "strong_password",
        "Require 8+ chars with upper, lower, and a digit",
        |value, _options| {{
            if let Some(password) = value.as_str() {{
                if password.len() < 8 {{
                    return Err(rustf::Error::validation("Password must be at least 8 characters"));
                }}
                if !password.chars().any(|c| c.is_uppercase()) {{
                    return Err(rustf::Error::validation("Password must contain at least one uppercase letter"));
                }}
                if !password.chars().any(|c| c.is_lowercase()) {{
                    return Err(rustf::Error::validation("Password must contain at least one lowercase letter"));
                }}
                if !password.chars().any(|c| c.is_numeric()) {{
                    return Err(rustf::Error::validation("Password must contain at least one number"));
                }}
            }}
            Ok(())
        }},
    );
}}
*/
"#
    );

    let definition_path = project_path.join("src/definitions/app.rs");
    fs::write(definition_path, definition_content)?;
    println!("📝 Creating src/definitions/app.rs");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_project_name() {
        assert_eq!(normalize_project_name("MyApp").unwrap(), "myapp");
        assert_eq!(normalize_project_name("my-app").unwrap(), "my_app");
        assert_eq!(
            normalize_project_name("My Great App").unwrap(),
            "my_great_app"
        );
        assert_eq!(normalize_project_name("my__app").unwrap(), "my_app");

        assert!(normalize_project_name("").is_err());
        assert!(normalize_project_name("123app").is_err());
    }

    #[test]
    fn test_project_name_to_title() {
        assert_eq!(project_name_to_title("my_app"), "My App");
        assert_eq!(project_name_to_title("my-great-app"), "My Great App");
        assert_eq!(project_name_to_title("MyApp"), "MyApp");
    }

    #[test]
    fn test_generate_session_secret() {
        let secret1 = generate_session_secret();
        let secret2 = generate_session_secret();

        assert_eq!(secret1.len(), 64);
        assert_eq!(secret2.len(), 64);
        assert_ne!(secret1, secret2); // Should be different

        // Should only contain alphanumeric characters
        assert!(secret1.chars().all(|c| c.is_ascii_alphanumeric()));
    }
}
