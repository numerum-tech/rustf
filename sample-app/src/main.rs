use rustf::prelude::*;

// Auto-discovery using the #[rustf::auto_discover] attribute macro
// The framework automatically generates module declarations at compile time

#[rustf::auto_discover]
#[tokio::main]
async fn main() -> rustf::Result<()> {
    env_logger::init();

    println!("Starting RustF Tasks...");

    let app = RustF::with_args()?;
    let app = app.auto_load_with(&["logging", "csrf"]).with_method_override();

    let config = &app.config;
    let addr = format!("{}:{}", config.server.host, config.server.port);

    println!("Server listening on http://{}", addr);

    app.start().await
}
