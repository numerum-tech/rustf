use anyhow::{anyhow, Result};
use handlebars::Handlebars;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;

/// Generate a controller file
pub async fn generate_controller(names: String, crud: bool, routes: bool) -> Result<()> {
    let project_path = std::env::current_dir()?;
    let controller_dir = project_path.join("src").join("controllers");

    // Ensure controllers directory exists
    if !controller_dir.exists() {
        fs::create_dir_all(&controller_dir)?;
    }

    // Load template
    let template_content = include_str!("../../templates/components/controller.rs.template");
    let mut handlebars = Handlebars::new();
    handlebars.register_template_string("controller", template_content)?;

    // Process each controller name
    let controller_names: Vec<&str> = names.split(',').map(|s| s.trim()).collect();

    for name in controller_names {
        let snake_name = to_snake_case(name);
        let controller_path = controller_dir.join(format!("{}.rs", snake_name));

        // Check if file already exists
        if controller_path.exists() {
            println!("⚠️  Controller '{}' already exists, skipping", snake_name);
            continue;
        }

        // Prepare template variables
        let mut vars = HashMap::new();
        vars.insert("controller_name", snake_name.clone());
        vars.insert("controller_title", to_title_case(&snake_name));
        vars.insert("route_prefix", snake_name.clone());
        vars.insert(
            "description",
            format!("{} operations", to_title_case(&snake_name)),
        );

        // Add boolean flags
        if crud {
            vars.insert("crud", "true".to_string());
        }
        if routes {
            vars.insert("routes", "true".to_string());
        }

        // Render template
        let rendered = handlebars.render("controller", &vars)?;

        // Write file
        let mut file = File::create(&controller_path)?;
        file.write_all(rendered.as_bytes())?;

        println!("✅ Created controller: {}", controller_path.display());
    }

    println!("\n📝 Don't forget to:");
    println!("   - Add your controller(s) to auto-discovery or manual registration");
    println!("   - Create corresponding view templates if using view responses");
    if crud {
        println!("   - Implement the TODO sections for database operations");
    }

    Ok(())
}

/// Generate a module/service file
pub async fn generate_module(name: String, shared: bool, with_methods: bool) -> Result<()> {
    let project_path = std::env::current_dir()?;
    let module_dir = project_path.join("src").join("modules");

    // Ensure modules directory exists
    if !module_dir.exists() {
        fs::create_dir_all(&module_dir)?;
    }

    let snake_name = to_snake_case(&name);
    let module_path = module_dir.join(format!("{}.rs", snake_name));

    // Check if file already exists
    if module_path.exists() {
        return Err(anyhow!("Module '{}' already exists", snake_name));
    }

    // Load template
    let template_content = include_str!("../../templates/components/module.rs.template");
    let mut handlebars = Handlebars::new();
    handlebars.register_template_string("module", template_content)?;

    // Prepare template variables
    let mut vars = HashMap::new();
    vars.insert("module_name", snake_name.clone());
    vars.insert("module_title", to_title_case(&snake_name));
    vars.insert("module_struct", to_pascal_case(&snake_name));
    vars.insert(
        "description",
        format!("{} functionality", to_title_case(&snake_name)),
    );

    // Add boolean flags
    if shared {
        vars.insert("shared", "true".to_string());
    }
    if with_methods {
        vars.insert("with_methods", "true".to_string());
    }

    // Render template
    let rendered = handlebars.render("module", &vars)?;

    // Write file
    let mut file = File::create(&module_path)?;
    file.write_all(rendered.as_bytes())?;

    println!("✅ Created module: {}", module_path.display());

    if shared {
        println!("\n📝 Don't forget to:");
        println!("   - Register your module in main.rs using .modules()");
        println!("   - Import and use the helper function in your controllers");
    }

    Ok(())
}

/// Generate a middleware file
pub async fn generate_middleware(
    name: String,
    auth: bool,
    logging: bool,
    priority: i32,
) -> Result<()> {
    let project_path = std::env::current_dir()?;
    let middleware_dir = project_path.join("src").join("middleware");

    // Ensure middleware directory exists
    if !middleware_dir.exists() {
        fs::create_dir_all(&middleware_dir)?;
    }

    let snake_name = to_snake_case(&name);
    let middleware_path = middleware_dir.join(format!("{}.rs", snake_name));

    // Check if file already exists
    if middleware_path.exists() {
        return Err(anyhow!("Middleware '{}' already exists", snake_name));
    }

    // Load template
    let template_content = include_str!("../../templates/components/middleware.rs.template");
    let mut handlebars = Handlebars::new();
    handlebars.register_template_string("middleware", template_content)?;

    // Prepare template variables
    let mut vars = HashMap::new();
    vars.insert("middleware_name", snake_name.clone());
    vars.insert("middleware_title", to_title_case(&snake_name));
    vars.insert("middleware_struct", to_pascal_case(&snake_name));
    vars.insert(
        "description",
        format!(
            "{} middleware for request processing",
            to_title_case(&snake_name)
        ),
    );
    vars.insert("priority", priority.to_string());

    // Add priority context for template
    if priority < 0 {
        vars.insert("priority_negative", "true".to_string());
    } else if priority > 0 {
        vars.insert("priority_positive", "true".to_string());
    }

    // Add boolean flags
    if auth {
        vars.insert("auth", "true".to_string());
    }
    if logging {
        vars.insert("logging", "true".to_string());
    }

    // Render template
    let rendered = handlebars.render("middleware", &vars)?;

    // Write file
    let mut file = File::create(&middleware_path)?;
    file.write_all(rendered.as_bytes())?;

    println!("✅ Created middleware: {}", middleware_path.display());
    println!("\n📝 Don't forget to:");
    println!("   - Add your middleware to auto-discovery or manual registration");
    println!("   - Configure the middleware behavior as needed");

    if auth {
        println!("   - Update the protected_paths for your application");
        println!("   - Ensure session management is configured");
    }

    if logging {
        println!("   - Configure logging levels in your application");
        println!("   - Be careful about logging sensitive data");
    }

    println!(
        "   - Priority {} means: {}",
        priority,
        if priority < 0 {
            "runs early in the chain"
        } else if priority > 0 {
            "runs late in the chain"
        } else {
            "default execution order"
        }
    );

    Ok(())
}

/// Generate an event handler file
pub async fn generate_event(name: String, lifecycle: bool, custom: bool) -> Result<()> {
    let project_path = std::env::current_dir()?;
    let events_dir = project_path.join("src").join("events");

    // Ensure events directory exists
    if !events_dir.exists() {
        fs::create_dir_all(&events_dir)?;
    }

    let snake_name = to_snake_case(&name);
    let event_path = events_dir.join(format!("{}.rs", snake_name));

    // Check if file already exists
    if event_path.exists() {
        return Err(anyhow!("Event handler '{}' already exists", snake_name));
    }

    // Load template
    let template_content = include_str!("../../templates/components/event.rs.template");
    let mut handlebars = Handlebars::new();
    handlebars.register_template_string("event", template_content)?;

    // Prepare template variables
    let mut vars = HashMap::new();
    vars.insert("event_name", snake_name.clone());
    vars.insert("event_title", to_title_case(&snake_name));
    vars.insert(
        "description",
        format!("{} events", to_title_case(&snake_name)),
    );

    // Add boolean flags
    if lifecycle {
        vars.insert("lifecycle", "true".to_string());
    }
    if custom {
        vars.insert("custom", "true".to_string());
    }

    // Render template
    let rendered = handlebars.render("event", &vars)?;

    // Write file
    let mut file = File::create(&event_path)?;
    file.write_all(rendered.as_bytes())?;

    println!("✅ Created event handler: {}", event_path.display());
    println!("\n📝 Don't forget to:");
    println!("   - Add your event handler to auto-discovery with auto_events!()");
    println!("   - Or manually register it with .events_from()");

    if custom {
        println!("   - Emit your custom events from controllers or services");
        println!(
            "   - Example: ctx.emit(\"{}.data.received\", data)?;",
            snake_name
        );
    }

    Ok(())
}

/// Generate a worker file
pub async fn generate_worker(name: String) -> Result<()> {
    let project_path = std::env::current_dir()?;
    let workers_dir = project_path.join("src").join("workers");

    // Ensure workers directory exists
    if !workers_dir.exists() {
        fs::create_dir_all(&workers_dir)?;
    }

    let snake_name = to_snake_case(&name);
    let worker_path = workers_dir.join(format!("{}.rs", snake_name));

    // Check if file already exists
    if worker_path.exists() {
        return Err(anyhow!("Worker '{}' already exists", snake_name));
    }

    // Load template
    let template_content = include_str!("../../templates/components/worker.rs.template");
    let mut handlebars = Handlebars::new();
    handlebars.register_template_string("worker", template_content)?;

    // Prepare template variables
    let mut vars = HashMap::new();

    // Convert name to kebab-case for worker registration
    let kebab_name = snake_name.replace('_', "-");
    vars.insert("worker_name", kebab_name.clone());
    vars.insert("worker_name_underscored", snake_name.clone());
    vars.insert("worker_title", to_title_case(&snake_name));

    // Generate description
    vars.insert(
        "description",
        format!(
            "{} - background worker for async task execution",
            to_title_case(&snake_name)
        ),
    );

    // Render template
    let rendered = handlebars.render("worker", &vars)?;

    // Write file
    let mut file = File::create(&worker_path)?;
    file.write_all(rendered.as_bytes())?;

    println!("✅ Created worker: {}", worker_path.display());
    println!("\n📝 Next steps:");
    println!("   - Workers are auto-discovered from src/workers/ directory");
    println!(
        "   - Execute with: WORKER::run(\"{}\", payload).await",
        kebab_name
    );
    println!(
        "   - For progress updates: WORKER::call(\"{}\", None, payload).await",
        kebab_name
    );
    println!("   - See docs/ABOUT_WORKERS.md for examples (email, batch, cleanup, etc.)");

    Ok(())
}

// Helper functions for name conversion
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut prev_is_upper = false;

    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 && !prev_is_upper {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().unwrap());
            prev_is_upper = true;
        } else if ch == '-' || ch == ' ' {
            result.push('_');
            prev_is_upper = false;
        } else {
            result.push(ch);
            prev_is_upper = false;
        }
    }

    result
}

fn to_pascal_case(s: &str) -> String {
    s.split(|c: char| c == '_' || c == '-' || c == ' ')
        .filter(|s| !s.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first
                    .to_uppercase()
                    .chain(chars.as_str().to_lowercase().chars())
                    .collect(),
            }
        })
        .collect()
}

fn to_title_case(s: &str) -> String {
    s.split(|c: char| c == '_' || c == '-')
        .filter(|s| !s.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first
                    .to_uppercase()
                    .chain(chars.as_str().to_lowercase().chars())
                    .collect(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
