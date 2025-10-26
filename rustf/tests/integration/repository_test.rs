use rustf::prelude::*;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a simple controller to test repository functionality
    let mut app = RustF::new();
    
    // Set global repository data (APP/MAIN)
    app.set_global_repository(json!({
        "app_name": "TestApp",
        "version": "1.0.0"
    }));
    
    // Create a route that uses repository data
    app.get("/test", |mut ctx: Context| async move {
        // Set context repository data
        ctx.repository_set("user", "john_doe")
           .repository_set("theme", "dark");
        
        // Render a view that uses both repositories
        ctx.view("test_repo", json!({
            "page_title": "Test Page"
        }))
    });
    
    println!("Repository system test completed successfully!");
    Ok(())
}