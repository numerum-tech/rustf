use rustf::prelude::*;
use rustf::middleware::builtin::*;
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
        .controllers(auto_controllers!())  // Auto-discover controllers
        .models(auto_models!())           // Auto-register models
        .modules_from(auto_modules!())    // Auto-register modules
        .definitions_from(auto_definitions!()) // Auto-discover definitions
        .static_files("/static", "public/") // Serve static files
        // Register built-in middleware using middleware_from
        .middleware_from(|registry| {
            registry.register_dual("logging", LoggingMiddleware::new());
            registry.register_dual("cors", CorsMiddleware::new()
                .allow_origin("*")
                .allow_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"]));
            registry.register_inbound("rate_limit", RateLimitMiddleware::new(100, 60));
        })
        // Auto-discover custom middleware
        .middleware_from(auto_middleware!());

    // Get server configuration
    let config = &app.config;
    let addr = format!("{}:{}", config.server.host, config.server.port);
    
    println!("🚀 Server starting on http://{}", addr);
    println!("📁 Views directory: {}", config.views.directory);
    println!("📁 Static files: {} -> {}", config.static_files.url_prefix, config.static_files.directory);
    println!("🛡️  Middleware: logging, cors, rate_limit + custom middleware");
    println!();
    println!("Visit http://{} to see your application!", addr);
    
    // Start the server
    app.start().await
}