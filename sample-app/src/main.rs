use rustf::middleware::builtin::*;
use rustf::prelude::*;
use std::env;

// Auto-discovery using the #[rustf::auto_discover] attribute macro
// The framework automatically generates module declarations at compile time:
// - Controllers from src/controllers/*.rs
// - Models from src/models/*.rs
// - Modules from src/modules/*.rs
// - Middleware from src/middleware/*.rs
// - Definitions from src/definitions/*.rs

#[rustf::auto_discover]
#[tokio::main]
async fn main() -> rustf::Result<()> {
    // Initialize logging
    env_logger::init();

    println!("🚀 Starting Sample App...");

    // Load configuration (try file first, then environment, then defaults)
    let app = if std::path::Path::new("config.toml").exists() {
        println!("📝 Loading configuration from config.toml");
        RustF::from_file("config.toml")?
    } else if env::var("RUSTF_HOST").is_ok() || env::var("RUSTF_PORT").is_ok() {
        println!("🌍 Loading configuration from environment variables");
        RustF::from_env()?
    } else {
        println!("⚙️ Using default configuration");
        RustF::new()
    };

    // Configure the application with auto-discovery and built-in middleware
    let app = app
        .controllers(auto_controllers!()) // Auto-discover controllers
        .models(auto_models!()) // Auto-register models
        // NOTE: Modules are no longer auto-registered via .modules_from()
        // They are discovered via auto_modules!() for IDE support, but you must
        // explicitly register them using MODULE::register() with a unique name.
        // This allows multiple instances of the same type with different configurations.
        .definitions_from(auto_definitions!()) // Auto-discover definitions
        .static_files("/static", "public/") // Serve static files
        // Register built-in middleware using middleware_from
        .middleware_from(|registry| {
            registry.register_dual("logging", LoggingMiddleware::new());
            registry.register_dual(
                "cors",
                CorsMiddleware::new()
                    .allow_origin("*")
                    .allow_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"]),
            );
            registry.register_inbound("rate_limit", RateLimitMiddleware::new(100, 60));
        })
        // Auto-discover custom middleware
        .middleware_from(auto_middleware!());

    // Initialize the MODULE system for explicit registration
    MODULE::init()?;

    // ============================================================================
    // TEST: Named Module Registration with Multiple Instances
    // ============================================================================
    // This demonstrates the new design where:
    // 1. Only SharedModule implementers can be registered (type-safe)
    // 2. Multiple instances of the same type can be registered with different names
    // 3. Simple utilities don't need to be singletons - just import and use directly

    println!("\n📋 Module Registration Test:");

    // Register two EmailService instances with different configurations
    let primary_email = modules::email_service::EmailService::new(
        "noreply@app.com".to_string(),
        "Primary Service".to_string(),
        "smtp.primary.com".to_string(),
    );
    MODULE::register("email-primary", primary_email)?;
    println!("✅ Registered: email-primary");

    let backup_email = modules::email_service::EmailService::new(
        "backup@app.com".to_string(),
        "Backup Service".to_string(),
        "smtp.backup.com".to_string(),
    );
    MODULE::register("email-backup", backup_email)?;
    println!("✅ Registered: email-backup");

    // Access registered modules by name
    let primary = MODULE::get("email-primary")?;
    println!("🔗 Retrieved: email-primary");

    let backup = MODULE::get("email-backup")?;
    println!("🔗 Retrieved: email-backup");

    // List all registered modules
    let modules_list = MODULE::list();
    println!("📊 Registered modules: {}", modules_list.len());
    for (name, module_type) in modules_list {
        println!("   - {}: {}", name, module_type);
    }

    // Verify existence
    assert!(MODULE::exists("email-primary"));
    assert!(MODULE::exists("email-backup"));
    assert!(!MODULE::exists("email-nonexistent"));
    println!("✅ Module existence checks passed");

    // Simple utilities don't need to be registered - just use directly
    let text = "hello world";
    let upper = modules::simple_util::StringUtils::to_upper(text);
    println!(
        "✅ StringUtils (no registration needed): {} -> {}",
        text, upper
    );

    println!("\n✨ Named Module Registration Test Complete!\n");

    // Get server configuration
    let config = &app.config;
    let addr = format!("{}:{}", config.server.host, config.server.port);

    println!("🚀 Server starting on http://{}", addr);
    println!("📁 Views directory: {}", config.views.directory);
    println!(
        "📁 Static files: {} -> {}",
        config.static_files.url_prefix, config.static_files.directory
    );
    println!("🛡️  Middleware: logging, cors, rate_limit + custom middleware");
    println!();
    println!("Visit http://{} to see your application!", addr);

    // Start the server
    app.start().await
}
