use anyhow::{anyhow, Result};
use handlebars::Handlebars;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

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
///
/// # Arguments
/// * `name` - Module name
/// * `shared` - If true, generate as SharedModule service; if false, generate as simple utility
/// * `with_methods` - If true, include sample methods
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
        vars.insert("is_service", "true".to_string());
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
    println!(
        "   Type: {}",
        if shared {
            "Service (SharedModule)"
        } else {
            "Utility (Simple Helper)"
        }
    );

    if shared {
        println!("\n📝 Service Setup Instructions:");
        println!("   1. Add to your main.rs after MODULE::init():");
        println!(
            "      MODULE::register(\"{}\", {}::new())?;",
            snake_name,
            to_pascal_case(&snake_name)
        );
        println!("   2. Access it in your code:");
        println!("      let service = MODULE::get(\"{}\")?;", snake_name);
    } else {
        println!("\n📝 Utility Usage:");
        println!("   1. Import directly in your code:");
        println!("      use modules::{}::{{}};", snake_name);
        println!("   2. Use the static functions:");
        println!(
            "      let result = {}::helper_function(input);",
            to_pascal_case(&snake_name)
        );
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

/// Generate a CRUD scaffold around an EXISTING database model.
///
/// Emits the HTTP + business layers — controller, business-logic module,
/// four views, and an integration-test stub — all wired to the already-
/// generated model at `src/models/<name>.rs`. Does NOT touch the model
/// file. Per the RustF layering rule (Base Model -> Model -> Module ->
/// Controller), the controller imports only
/// `crate::modules::<name>_service`; that service is the only caller of
/// the model.
///
/// Precondition: `src/models/<name>.rs` must exist. Run
/// `rustf-cli schema generate models` (after defining `schemas/<name>.yaml`)
/// first if it doesn't.
pub async fn generate_crud(name: String) -> Result<()> {
    let project_path = std::env::current_dir()?;

    // Normalise names. Input is treated as plural (e.g. "posts"). The
    // singular is a naive strip of trailing 's'; irregular plurals are a
    // user problem per the plan.
    let plural = to_snake_case(&name);
    if plural.is_empty() {
        return Err(anyhow!("CRUD name cannot be empty"));
    }
    let singular = naive_singular(&plural);
    let pascal_plural = to_pascal_case(&plural);
    let pascal_singular = to_pascal_case(&singular);
    let title_plural = to_title_case(&plural);
    let title_singular = to_title_case(&singular);

    // Target paths
    let controllers_dir = project_path.join("src").join("controllers");
    let modules_dir = project_path.join("src").join("modules");
    let models_dir = project_path.join("src").join("models");
    let views_dir = project_path.join("views").join(&plural);
    let tests_dir = project_path.join("tests");

    // PRECONDITION: the model must already exist. The CRUD scaffolder
    // generates the HTTP/business layer AROUND an existing model; it
    // never emits a model stub.
    let model_path = models_dir.join(format!("{}.rs", plural));
    if !model_path.exists() {
        return Err(anyhow!(
            "Model not found at {}\n\n\
             `rustf-cli new crud` wires controllers/views/services around an EXISTING \
             database model. Generate one first:\n\
             \n    1. Define schemas/{}.yaml\n\
             \n    2. Run: rustf-cli schema generate models\n\
             \n    3. Re-run: rustf-cli new crud --name {}\n",
            model_path.display(),
            plural,
            plural,
        ));
    }

    for dir in [&controllers_dir, &modules_dir, &views_dir, &tests_dir] {
        fs::create_dir_all(dir)?;
    }

    // Fail fast if any target already exists — avoid clobbering.
    let targets = [
        controllers_dir.join(format!("{}.rs", plural)),
        modules_dir.join(format!("{}_service.rs", plural)),
        views_dir.join("index.html"),
        views_dir.join("show.html"),
        views_dir.join("new.html"),
        views_dir.join("edit.html"),
        tests_dir.join(format!("{}_test.rs", plural)),
    ];
    for t in &targets {
        if t.exists() {
            return Err(anyhow!(
                "'{}' already exists — refusing to overwrite",
                t.display()
            ));
        }
    }

    let mut vars = HashMap::new();
    vars.insert("name", plural.clone());
    vars.insert("name_singular", singular.clone());
    vars.insert("pascal_name", pascal_plural.clone());
    vars.insert("pascal_name_singular", pascal_singular.clone());
    vars.insert("title", title_plural.clone());
    vars.insert("title_singular", title_singular.clone());

    let mut handlebars = Handlebars::new();
    handlebars.set_strict_mode(false);

    // (template-key, template-content, output-path)
    let rendering_plan: Vec<(&str, &str, PathBuf)> = vec![
        (
            "crud_controller",
            include_str!("../../templates/components/crud_controller.rs.template"),
            targets[0].clone(),
        ),
        (
            "crud_module",
            include_str!("../../templates/components/crud_module.rs.template"),
            targets[1].clone(),
        ),
        (
            "crud_index",
            include_str!("../../templates/views/crud/index.html.template"),
            targets[2].clone(),
        ),
        (
            "crud_show",
            include_str!("../../templates/views/crud/show.html.template"),
            targets[3].clone(),
        ),
        (
            "crud_new",
            include_str!("../../templates/views/crud/new.html.template"),
            targets[4].clone(),
        ),
        (
            "crud_edit",
            include_str!("../../templates/views/crud/edit.html.template"),
            targets[5].clone(),
        ),
        (
            "crud_test",
            include_str!("../../templates/components/crud_test.rs.template"),
            targets[6].clone(),
        ),
    ];

    for (key, content, out_path) in rendering_plan {
        handlebars.register_template_string(key, content)?;
        let rendered = handlebars.render(key, &vars)?;
        let mut file = File::create(&out_path)?;
        file.write_all(rendered.as_bytes())?;
        println!("✅ {}", out_path.display());
    }

    println!();
    println!("📝 Scaffold layering (enforced in generated code):");
    println!("   HTTP:     src/controllers/{}.rs        (calls {}Service only)", plural, pascal_plural);
    println!("   Business: src/modules/{}_service.rs  (only caller of {})", plural, pascal_singular);
    println!("   Data:     src/models/{}.rs              (PRE-EXISTING — not touched)", plural);
    println!("   Views:    views/{}/{{index,show,new,edit}}.html", plural);
    println!("   Tests:    tests/{}_test.rs             (stubs — uncomment + adjust crate import)", plural);
    println!();
    println!("📝 Next steps:");
    println!("   1. Open src/modules/{}_service.rs and fill in the `create` and `update`", plural);
    println!("      field mappings marked with TODO — they need your schema's real field");
    println!("      names (the generated model's builder + setters are typed per-field).");
    println!("   2. cargo run, then visit http://127.0.0.1:8000/{}", plural);

    Ok(())
}

/// Naive plural -> singular: strip trailing 's'. Users with irregular plurals
/// (child/children, datum/data, etc.) rename the generated files themselves.
fn naive_singular(plural: &str) -> String {
    if plural.ends_with('s') && plural.len() > 1 {
        plural[..plural.len() - 1].to_string()
    } else {
        plural.to_string()
    }
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
