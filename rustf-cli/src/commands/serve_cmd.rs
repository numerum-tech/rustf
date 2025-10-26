use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum ServeCommand {
    /// Start MCP server for AI agent integration
    Start {
        /// Allow write operations (generate, new, fix). DEFAULT: false (read-only mode)
        #[arg(long, default_value = "false")]
        allow_writes: bool,
        
        /// Automatically find an available port if the specified port is in use
        #[arg(long)]
        auto_port: bool,
        
        /// Bind address
        #[arg(long, default_value = "127.0.0.1")]
        bind: String,
        
        /// Optional name for this server instance
        #[arg(short, long)]
        name: Option<String>,
        
        /// Server port
        #[arg(long, default_value = "3000")]
        port: u16,
        
        /// Enable file watching for live updates
        #[arg(short, long)]
        watch: bool,
        
        /// Enable WebSocket support for real-time notifications
        #[arg(long)]
        websocket: bool,
    },
    
    /// List all running MCP server instances
    List {},
    
    /// Stop a running MCP server instance
    Stop {
        /// Port number of the server to stop
        port: u16,
    },
}