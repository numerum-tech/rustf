use crate::mcp::{McpServer, ServerInstance, get_registry};
use std::path::PathBuf;
use anyhow::Result;
use chrono::Local;

pub async fn run(
    project_path: PathBuf,
    port: u16,
    bind: String,
    watch: bool,
    websocket: bool,
    name: Option<String>,
    allow_writes: bool,
) -> Result<()> {
    let registry = get_registry();
    
    // Check if port is already in use by another instance
    if registry.is_port_in_use(port).await {
        anyhow::bail!("Port {} is already in use by another RustF CLI instance", port);
    }
    
    // Check if name is already in use
    if let Some(ref instance_name) = name {
        if let Some(existing) = registry.find_by_name(instance_name).await {
            anyhow::bail!(
                "Instance named '{}' is already running on port {}",
                instance_name,
                existing.port
            );
        }
    }
    
    // For WebSocket mode, also check if port+1 is available
    if websocket && registry.is_port_in_use(port + 1).await {
        anyhow::bail!(
            "Port {} is required for WebSocket but is already in use",
            port + 1
        );
    }
    
    // Safety check for write mode on public interface
    let is_public_interface = bind != "127.0.0.1" && bind != "localhost";
    if allow_writes && is_public_interface {
        println!();
        println!("⚠️  WARNING: Write operations enabled on public interface!");
        println!("⚠️  Server will accept write commands from {}:{}", bind, port);
        println!("⚠️  This could allow remote modification of your code!");
        println!();
        println!("For safe remote access, use read-only mode (default):");
        println!("  rustf-cli serve --bind {}", bind);
        println!();
        println!("Press Ctrl+C to cancel, or wait 5 seconds to continue...");
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
    
    // Display server mode clearly
    println!();
    if allow_writes {
        println!("🔓 Starting MCP Server with WRITE operations enabled");
        println!("   Available: All CLI commands including generate, new, fix");
    } else {
        println!("🔒 Starting MCP Server in READ-ONLY mode (safe for remote)");
        println!("   Available: analyze, query, validate, db list/describe");
        println!("   Blocked: generate, new, fix, migrate");
        println!("   To enable writes: --allow-writes (use with caution!)");
    }
    println!();
    
    log::info!("Starting RustF MCP Server...");
    if let Some(ref instance_name) = name {
        log::info!("Instance name: {}", instance_name);
    }
    log::info!("Project: {}", project_path.display());
    log::info!("Bind: {}:{}", bind, port);
    log::info!("Mode: {}", if allow_writes { "read-write" } else { "read-only" });
    log::info!("File watching: {}", if watch { "enabled" } else { "disabled" });
    log::info!("WebSocket support: {}", if websocket { "enabled" } else { "disabled" });
    
    // Register this instance
    let instance = ServerInstance::new(
        name.clone(),
        port,
        bind.clone(),
        project_path.clone(),
        watch,
        websocket,
    );
    registry.register(instance).await?;
    
    // Set up cleanup on shutdown
    let cleanup_port = port;
    let cleanup_registry = get_registry();
    
    // Create a channel for shutdown signal
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel::<()>(1);
    
    // Set up signal handlers
    let shutdown_tx_clone = shutdown_tx.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
        let _ = shutdown_tx_clone.send(()).await;
    });
    
    // Start server in a separate task
    let read_only = !allow_writes;  // Invert for clarity
    let server = McpServer::new(project_path, watch, read_only)?;
    let server_task = tokio::spawn(async move {
        if websocket {
            log::info!("Starting MCP server with WebSocket support on {}:{}", bind, port);
            server.start_with_websocket(&bind, port).await
        } else {
            log::info!("Starting MCP server (HTTP only) on {}:{}", bind, port);
            server.start(&bind, port).await
        }
    });
    
    // Wait for either server to finish or shutdown signal
    tokio::select! {
        result = server_task => {
            // Server finished (possibly with error)
            cleanup_registry.unregister(cleanup_port).await?;
            result??
        }
        _ = shutdown_rx.recv() => {
            // Received shutdown signal
            log::info!("Shutting down server...");
            cleanup_registry.unregister(cleanup_port).await?;
            // Note: In a real implementation, we'd need a way to gracefully shutdown the server
            // For now, the process will exit
        }
    }
    
    Ok(())
}

pub async fn find_available_port(start_port: u16) -> Option<u16> {
    let registry = get_registry();
    registry.find_available_port(start_port, 100).await
}

pub async fn list() -> Result<()> {
    let registry = get_registry();
    let instances = registry.list_instances().await;
    
    if instances.is_empty() {
        println!("No RustF MCP servers are currently running.");
        return Ok(());
    }
    
    println!("🚀 Running RustF MCP Server Instances:\n");
    println!("{:<20} {:<10} {:<15} {:<10} {:<10} {:<30} {:<20}",
        "Name", "Port", "Bind", "PID", "WebSocket", "Project", "Started");
    println!("{}", "-".repeat(125));
    
    for instance in instances {
        let name = instance.name.as_deref().unwrap_or("(unnamed)");
        let ws_info = if instance.websocket_enabled {
            format!("Yes ({})", instance.websocket_port.unwrap_or(0))
        } else {
            "No".to_string()
        };
        let started = instance.started_at.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S");
        
        println!("{:<20} {:<10} {:<15} {:<10} {:<10} {:<30} {:<20}",
            name,
            instance.port,
            instance.bind_address,
            instance.pid,
            ws_info,
            instance.project_path.display(),
            started
        );
    }
    
    println!("\n💡 Use 'rustf-cli serve-stop <port>' to stop a server instance.");
    
    Ok(())
}

pub async fn stop(port: u16) -> Result<()> {
    let registry = get_registry();
    
    // Check if instance exists
    match registry.get_instance(port).await {
        Some(instance) => {
            // Try to stop the process
            #[cfg(unix)]
            {
                use std::process::Command;
                let pid = instance.pid;
                
                // Send SIGTERM for graceful shutdown
                match Command::new("kill")
                    .arg("-TERM")
                    .arg(pid.to_string())
                    .output()
                {
                    Ok(output) => {
                        if output.status.success() {
                            println!("✅ Sent shutdown signal to server on port {} (PID: {})", port, pid);
                            
                            // Wait a moment for the process to clean up
                            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                            
                            // Check if it's still running
                            if !instance.is_alive() {
                                println!("✅ Server stopped successfully.");
                            } else {
                                // Force kill if still running
                                let _ = Command::new("kill")
                                    .arg("-KILL")
                                    .arg(pid.to_string())
                                    .output();
                                println!("⚠️  Server was forcefully terminated.");
                            }
                            
                            // Remove from registry
                            registry.unregister(port).await?;
                        } else {
                            anyhow::bail!("Failed to stop server: {}", String::from_utf8_lossy(&output.stderr));
                        }
                    }
                    Err(e) => {
                        anyhow::bail!("Failed to stop server: {}", e);
                    }
                }
            }
            
            #[cfg(windows)]
            {
                // On Windows, use taskkill
                use std::process::Command;
                let pid = instance.pid;
                
                match Command::new("taskkill")
                    .args(&["/PID", &pid.to_string(), "/F"])
                    .output()
                {
                    Ok(output) => {
                        if output.status.success() {
                            println!("✅ Server on port {} (PID: {}) stopped successfully.", port, pid);
                            registry.unregister(port).await?;
                        } else {
                            anyhow::bail!("Failed to stop server: {}", String::from_utf8_lossy(&output.stderr));
                        }
                    }
                    Err(e) => {
                        anyhow::bail!("Failed to stop server: {}", e);
                    }
                }
            }
            
            #[cfg(not(any(unix, windows)))]
            {
                anyhow::bail!("Stopping servers is not supported on this platform");
            }
        }
        None => {
            // Check if it's a stale entry
            if registry.is_port_in_use(port).await {
                registry.unregister(port).await?;
                println!("✅ Removed stale server entry for port {}", port);
            } else {
                anyhow::bail!("No server found running on port {}", port);
            }
        }
    }
    
    Ok(())
}