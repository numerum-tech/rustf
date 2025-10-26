use super::*;
use super::cli_executor::{CliExecutor, CliExecuteParams};
use anyhow::Result;
use jsonrpc_core::{IoHandler, Params, Value, Error as JsonRpcError};
use jsonrpc_http_server::ServerBuilder;
use jsonrpc_ws_server::{ServerBuilder as WsServerBuilder};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::broadcast;
use serde_json::json;

pub struct McpServer {
    state: Arc<McpState>,
    io_handler: IoHandler,
    notification_sender: broadcast::Sender<Value>,
    cli_executor: Arc<CliExecutor>,
    read_only: bool,
}

impl McpServer {
    pub fn new(project_path: PathBuf, watch: bool, read_only: bool) -> Result<Self> {
        let state = McpState::new(project_path.clone(), watch)?;
        let state_arc = Arc::new(state);
        let mut io_handler = IoHandler::new();
        
        // Create CLI executor
        let cli_executor = Arc::new(CliExecutor::new(project_path, read_only));
        
        // Create broadcast channel for notifications
        let (notification_sender, _) = broadcast::channel(100);
        
        // Register the single CLI execute handler
        Self::register_cli_handler(&mut io_handler, cli_executor.clone());
        
        Ok(Self {
            state: state_arc,
            io_handler,
            notification_sender,
            cli_executor,
            read_only,
        })
    }
    
    pub async fn start(&self, bind: &str, port: u16) -> Result<()> {
        let addr: SocketAddr = format!("{}:{}", bind, port).parse()?;
        
        let server = ServerBuilder::new(self.io_handler.clone())
            .start_http(&addr)?;
        
        log::info!("MCP Server running on http://{}", addr);
        server.wait();
        
        Ok(())
    }
    
    pub async fn start_with_websocket(&self, bind: &str, port: u16) -> Result<()> {
        let addr: SocketAddr = format!("{}:{}", bind, port).parse()?;
        
        // Start HTTP server
        let http_server = ServerBuilder::new(self.io_handler.clone())
            .start_http(&addr)?;
        
        // Start WebSocket server on port + 1
        let ws_addr: SocketAddr = format!("{}:{}", bind, port + 1).parse()?;
        let ws_server = WsServerBuilder::new(self.io_handler.clone())
            .start(&ws_addr)?;
        
        log::info!("MCP Server running on http://{}", addr);
        log::info!("WebSocket server running on ws://{}", ws_addr);
        
        // Wait for both servers
        tokio::select! {
            _ = async { http_server.wait() } => {},
            _ = async { ws_server.wait() } => {},
        }
        
        Ok(())
    }
    
    fn register_cli_handler(io: &mut IoHandler, cli_executor: Arc<CliExecutor>) {
        // Single CLI execute handler that wraps all CLI commands
        {
            let executor = cli_executor.clone();
            io.add_method("rustf_cli_execute", move |params: Params| {
                let executor = executor.clone();
                async move {
                    // Parse parameters
                    let params: CliExecuteParams = match params.parse() {
                        Ok(p) => p,
                        Err(e) => {
                            return Ok(json!({
                                "status": "error",
                                "error": {
                                    "code": "INVALID_PARAMS",
                                    "message": format!("Invalid parameters: {}", e)
                                }
                            }));
                        }
                    };
                    
                    // Execute the CLI command
                    let result = executor.execute(
                        params.command,
                        params.subcommand,
                        params.args
                    ).await;
                    
                    match result {
                        Ok(result) => Ok(result),
                        Err(e) => {
                            Ok(json!({
                                "status": "error",
                                "error": {
                                    "code": "EXECUTION_ERROR",
                                    "message": format!("Command execution failed: {}", e)
                                }
                            }))
                        }
                    }
                }
            });
        }
        
        // Register a help/discovery endpoint
        {
            let read_only = cli_executor.read_only;
            io.add_method("rustf_help", move |_params: Params| {
                async move {
                    Ok::<Value, JsonRpcError>(json!({
                        "status": "success",
                        "data": {
                            "description": "RustF MCP Server - CLI wrapper for remote access",
                            "methods": [
                                {
                                    "name": "rustf_cli_execute",
                                    "description": "Execute any rustf-cli command",
                                    "params": {
                                        "command": "Main command (analyze, db, schema, etc.)",
                                        "subcommand": "Optional subcommand",
                                        "args": "Array of arguments",
                                        "project_path": "Optional project path"
                                    },
                                    "examples": [
                                        {
                                            "description": "Analyze project",
                                            "params": {
                                                "command": "analyze",
                                                "subcommand": "project",
                                                "args": ["--format", "json"]
                                            }
                                        },
                                        {
                                            "description": "List database tables",
                                            "params": {
                                                "command": "db",
                                                "subcommand": "list-tables",
                                                "args": []
                                            }
                                        }
                                    ]
                                }
                            ],
                            "read_only_mode": read_only,
                            "version": env!("CARGO_PKG_VERSION")
                        }
                    }))
                }
            });
        }
        
        let mode = if cli_executor.read_only { "read-only" } else { "read-write" };
        log::info!("Registered MCP CLI wrapper in {} mode", mode);
    }
}