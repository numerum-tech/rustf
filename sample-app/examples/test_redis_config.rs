use rustf::config::AppConfig;

fn main() {
    println!("Testing Redis config parsing...");
    
    let config_content = r#"
[server]
host = "127.0.0.1"
port = 8000

[session]
secret = "test-secret-key"
timeout = 3600
cookie_name = "test_session"

[session.storage]
type = "redis"
url = "redis://localhost:6379"
prefix = "test:session:"
pool_size = 10
connection_timeout = 5000
command_timeout = 3000
"#;
    
    match toml::from_str::<AppConfig>(&config_content) {
        Ok(config) => {
            println!("✅ Redis config parsed successfully!");
            println!("   Storage type: {:?}", config.session.storage);
        }
        Err(e) => {
            println!("❌ Failed to parse Redis config:");
            println!("   Error: {}", e);
            std::process::exit(1);
        }
    }
}
