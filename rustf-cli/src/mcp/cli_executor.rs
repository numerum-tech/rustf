//! CLI executor for MCP server - wraps rustf-cli commands

use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

/// Command classification for read-only safety
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CommandType {
    ReadOnly,
    Write,
}

/// CLI executor with read-only mode support
pub struct CliExecutor {
    pub read_only: bool,
    pub project_path: PathBuf,
    pub log_blocked_writes: bool,
}

impl CliExecutor {
    pub fn new(project_path: PathBuf, read_only: bool) -> Self {
        Self {
            read_only,
            project_path,
            log_blocked_writes: true,
        }
    }
    
    /// Execute a rustf-cli command
    pub async fn execute(
        &self,
        command: String,
        subcommand: Option<String>,
        args: Vec<String>,
    ) -> Result<Value> {
        let start_time = Instant::now();
        
        // Build full command for logging
        let full_command = build_command_string(&command, &subcommand, &args);
        
        // Classify command type
        let command_type = classify_command(&command, subcommand.as_deref(), &args);
        
        // Check read-only mode
        if self.read_only && command_type == CommandType::Write {
            if self.log_blocked_writes {
                log::warn!(
                    "BLOCKED write command in read-only mode: {}",
                    full_command
                );
            }
            
            return Ok(json!({
                "status": "error",
                "error": {
                    "code": "READ_ONLY_MODE",
                    "message": format!(
                        "Command '{}' requires write access. Server is running in read-only mode. \
                         To enable writes, restart the server with --allow-writes flag.",
                        full_command
                    ),
                    "blocked_command": full_command,
                },
                "metadata": {
                    "read_only_mode": true,
                    "command_type": "write",
                }
            }));
        }
        
        // Log the command execution
        log::info!("MCP executing: {}", full_command);
        
        // Prepare the command
        let mut cmd = Command::new("rustf-cli");
        cmd.current_dir(&self.project_path);
        
        // Add command and subcommand
        cmd.arg(&command);
        if let Some(ref sub) = subcommand {
            cmd.arg(sub);
        }
        
        // Add arguments
        for arg in &args {
            cmd.arg(arg);
        }
        
        // Execute the command
        let output = match cmd.output() {
            Ok(output) => output,
            Err(e) => {
                log::error!("Failed to execute command: {}", e);
                return Ok(json!({
                    "status": "error",
                    "error": {
                        "code": "EXECUTION_ERROR",
                        "message": format!("Failed to execute command: {}", e),
                    }
                }));
            }
        };
        
        let execution_time_ms = start_time.elapsed().as_millis() as u64;
        
        // Parse stdout as JSON if possible
        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let stderr_str = String::from_utf8_lossy(&output.stderr);
        
        let data = if output.status.success() {
            // Try to parse as JSON first
            if let Ok(json_data) = serde_json::from_str::<Value>(&stdout_str) {
                json_data
            } else {
                // Return as plain text
                json!({
                    "output": stdout_str.to_string(),
                    "format": "text"
                })
            }
        } else {
            json!({
                "output": stdout_str.to_string(),
                "error": stderr_str.to_string(),
                "format": "text"
            })
        };
        
        Ok(json!({
            "status": if output.status.success() { "success" } else { "error" },
            "data": data,
            "metadata": {
                "command_executed": full_command,
                "exit_code": output.status.code().unwrap_or(-1),
                "execution_time_ms": execution_time_ms,
                "read_only_mode": self.read_only,
                "command_type": match command_type {
                    CommandType::ReadOnly => "read_only",
                    CommandType::Write => "write",
                },
            }
        }))
    }
}

/// Classify a command as read-only or write
pub fn classify_command(
    command: &str,
    subcommand: Option<&str>,
    args: &[String],
) -> CommandType {
    match command {
        // Always read-only commands
        "analyze" => CommandType::ReadOnly,
        "query" => CommandType::ReadOnly,
        "export" => CommandType::ReadOnly,
        "perf" => CommandType::ReadOnly,
        
        // Validate is read-only unless --fix is present
        "validate" => {
            if args.iter().any(|arg| arg == "--fix" || arg == "-f") {
                CommandType::Write
            } else {
                CommandType::ReadOnly
            }
        }
        
        // Database commands - depends on subcommand
        "db" => match subcommand {
            Some("list-tables") | Some("describe") | Some("test-connection") | 
            Some("diff-schema") | Some("export-data") => CommandType::ReadOnly,
            Some("generate-schema") => CommandType::Write,
            _ => CommandType::Write, // Safe default
        },
        
        // Schema commands - depends on subcommand
        "schema" => match subcommand {
            Some("validate") | Some("analyze") | Some("diff") => CommandType::ReadOnly,
            Some("generate") | Some("migrate") => CommandType::Write,
            _ => CommandType::Write, // Safe default
        },
        
        // Serve commands - all read-only (listing/stopping servers)
        "serve" => match subcommand {
            Some("list") | Some("stop") => CommandType::ReadOnly,
            Some("start") => CommandType::Write, // Starting a server is a write operation
            _ => CommandType::ReadOnly,
        },
        
        // Always write commands
        "new" => CommandType::Write,
        
        // Unknown commands - assume write for safety
        _ => {
            log::warn!("Unknown command '{}', treating as write operation", command);
            CommandType::Write
        }
    }
}

/// Build a command string for logging/display
fn build_command_string(
    command: &str,
    subcommand: &Option<String>,
    args: &[String],
) -> String {
    let mut parts = vec!["rustf-cli".to_string(), command.to_string()];
    
    if let Some(sub) = subcommand {
        parts.push(sub.clone());
    }
    
    parts.extend(args.iter().cloned());
    
    parts.join(" ")
}

/// Parameters for CLI execution
#[derive(Debug, Deserialize)]
pub struct CliExecuteParams {
    pub command: String,
    pub subcommand: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    pub project_path: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_command_classification() {
        // Read-only commands
        assert_eq!(
            classify_command("analyze", Some("project"), &[]),
            CommandType::ReadOnly
        );
        assert_eq!(
            classify_command("query", Some("handler"), &["UserController::login".to_string()]),
            CommandType::ReadOnly
        );
        assert_eq!(
            classify_command("db", Some("list-tables"), &[]),
            CommandType::ReadOnly
        );
        assert_eq!(
            classify_command("validate", None, &[]),
            CommandType::ReadOnly
        );
        
        // Write commands
        assert_eq!(
            classify_command("new", Some("project"), &["my-app".to_string()]),
            CommandType::Write
        );
        assert_eq!(
            classify_command("schema", Some("generate"), &["models".to_string()]),
            CommandType::Write
        );
        assert_eq!(
            classify_command("validate", None, &["--fix".to_string()]),
            CommandType::Write
        );
        assert_eq!(
            classify_command("db", Some("generate-schema"), &[]),
            CommandType::Write
        );
    }
    
    #[test]
    fn test_build_command_string() {
        assert_eq!(
            build_command_string("analyze", &Some("project".to_string()), &[]),
            "rustf-cli analyze project"
        );
        
        assert_eq!(
            build_command_string(
                "db",
                &Some("describe".to_string()),
                &["--table".to_string(), "users".to_string()]
            ),
            "rustf-cli db describe --table users"
        );
    }
}