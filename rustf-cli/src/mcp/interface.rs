// Uniform MCP endpoint interface for consistent response handling and parameter processing
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value, Map};
use jsonrpc_core::{Params, Error as JsonRpcError};
use std::path::PathBuf;

/// Standard MCP response format
#[derive(Debug, Serialize, Deserialize)]
pub struct McpResponse {
    /// Status of the operation
    pub status: McpStatus,
    /// Optional error information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpError>,
    /// Response data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    /// Optional metadata about the operation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<McpMetadata>,
}

/// MCP operation status
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum McpStatus {
    Success,
    Error,
    Pending,
    Warning,
}

/// MCP error information
#[derive(Debug, Serialize, Deserialize)]
pub struct McpError {
    /// Error code for programmatic handling
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Optional detailed error information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

/// MCP operation metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct McpMetadata {
    /// Operation execution time in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_time_ms: Option<u64>,
    /// Project path used for the operation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_path: Option<String>,
    /// Database type (for database operations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database_type: Option<String>,
    /// Connection name used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection: Option<String>,
    /// Additional contextual information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Map<String, Value>>,
}

impl McpResponse {
    /// Create a successful response with data
    pub fn success(data: Value) -> Self {
        Self {
            status: McpStatus::Success,
            error: None,
            data: Some(data),
            metadata: None,
        }
    }

    /// Create a successful response with data and metadata
    pub fn success_with_metadata(data: Value, metadata: McpMetadata) -> Self {
        Self {
            status: McpStatus::Success,
            error: None,
            data: Some(data),
            metadata: Some(metadata),
        }
    }

    /// Create an error response
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status: McpStatus::Error,
            error: Some(McpError {
                code: code.into(),
                message: message.into(),
                details: None,
            }),
            data: None,
            metadata: None,
        }
    }

    /// Create an error response with details
    pub fn error_with_details(
        code: impl Into<String>, 
        message: impl Into<String>, 
        details: Value
    ) -> Self {
        Self {
            status: McpStatus::Error,
            error: Some(McpError {
                code: code.into(),
                message: message.into(),
                details: Some(details),
            }),
            data: None,
            metadata: None,
        }
    }

    /// Create a pending response for async operations
    pub fn pending(message: impl Into<String>) -> Self {
        Self {
            status: McpStatus::Pending,
            error: None,
            data: Some(json!({"message": message.into()})),
            metadata: None,
        }
    }

    /// Create a warning response
    pub fn warning(message: impl Into<String>, data: Option<Value>) -> Self {
        Self {
            status: McpStatus::Warning,
            error: Some(McpError {
                code: "WARNING".to_string(),
                message: message.into(),
                details: None,
            }),
            data,
            metadata: None,
        }
    }

    /// Convert to JSON Value for JSON-RPC response
    pub fn to_json_value(self) -> Value {
        serde_json::to_value(self).unwrap_or_else(|_| {
            json!({
                "status": "error",
                "error": {
                    "code": "SERIALIZATION_ERROR",
                    "message": "Failed to serialize response"
                }
            })
        })
    }
}

/// Standard parameter extraction utilities
pub struct McpParams;

impl McpParams {
    /// Parse JSON-RPC parameters into a map
    pub fn parse(params: Params) -> Result<Map<String, Value>, JsonRpcError> {
        params.parse::<Map<String, Value>>()
            .map_err(|_| JsonRpcError::invalid_params("Invalid parameters format"))
    }

    /// Extract required string parameter
    pub fn get_required_string(
        params: &Map<String, Value>, 
        key: &str
    ) -> Result<String, JsonRpcError> {
        params
            .get(key)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| JsonRpcError::invalid_params(&format!("Missing required parameter: {}", key)))
    }

    /// Extract optional string parameter
    pub fn get_optional_string(
        params: &Map<String, Value>, 
        key: &str
    ) -> Option<String> {
        params
            .get(key)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// Extract optional boolean parameter
    pub fn get_optional_bool(
        params: &Map<String, Value>, 
        key: &str
    ) -> Option<bool> {
        params
            .get(key)
            .and_then(|v| v.as_bool())
    }

    /// Extract optional array of strings parameter
    pub fn get_optional_string_array(
        params: &Map<String, Value>, 
        key: &str
    ) -> Option<Vec<String>> {
        params
            .get(key)
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
    }

    /// Extract project path with validation
    pub fn get_project_path(
        params: &Map<String, Value>
    ) -> Result<PathBuf, JsonRpcError> {
        let path_str = Self::get_optional_string(params, "project_path")
            .unwrap_or_else(|| std::env::current_dir()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string());
        
        let path = PathBuf::from(path_str);
        if !path.exists() {
            return Err(JsonRpcError::invalid_params("Project path does not exist"));
        }
        
        Ok(path)
    }
}

/// Execution context for MCP operations
pub struct McpContext {
    pub project_path: PathBuf,
    pub start_time: std::time::Instant,
}

impl McpContext {
    pub fn new(project_path: PathBuf) -> Self {
        Self {
            project_path,
            start_time: std::time::Instant::now(),
        }
    }

    /// Create metadata for the response
    pub fn create_metadata(&self) -> McpMetadata {
        McpMetadata {
            execution_time_ms: Some(self.start_time.elapsed().as_millis() as u64),
            project_path: Some(self.project_path.to_string_lossy().to_string()),
            database_type: None,
            connection: None,
            context: None,
        }
    }

    /// Create metadata with database information
    pub fn create_db_metadata(
        &self, 
        database_type: Option<String>, 
        connection: Option<String>
    ) -> McpMetadata {
        McpMetadata {
            execution_time_ms: Some(self.start_time.elapsed().as_millis() as u64),
            project_path: Some(self.project_path.to_string_lossy().to_string()),
            database_type,
            connection,
            context: None,
        }
    }
}

/// Trait for uniform MCP operation handlers
#[async_trait::async_trait]
pub trait McpHandler {
    /// The name of the MCP tool/method
    fn tool_name(&self) -> &'static str;
    
    /// Execute the MCP operation
    async fn execute(&self, params: Params) -> Result<Value, JsonRpcError>;
    
    /// Get tool description for MCP protocol
    fn description(&self) -> &'static str;
    
    /// Get parameter schema for the tool
    fn parameter_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }
}

/// Macro to implement standard MCP handler boilerplate
#[macro_export]
macro_rules! impl_mcp_handler {
    (
        $handler:ident,
        $tool_name:literal,
        $description:literal,
        $execute_fn:ident
    ) => {
        pub struct $handler;

        #[async_trait::async_trait]
        impl McpHandler for $handler {
            fn tool_name(&self) -> &'static str {
                $tool_name
            }

            async fn execute(&self, params: Params) -> Result<Value, JsonRpcError> {
                $execute_fn(params).await
            }

            fn description(&self) -> &'static str {
                $description
            }
        }
    };
}

/// Helper function to convert anyhow::Error to standardized MCP response
pub fn error_to_mcp_response(error: anyhow::Error) -> Value {
    let error_msg = error.to_string();
    let error_code = if error_msg.contains("connection") {
        "CONNECTION_ERROR"
    } else if error_msg.contains("permission") || error_msg.contains("access") {
        "ACCESS_ERROR"
    } else if error_msg.contains("not found") || error_msg.contains("missing") {
        "NOT_FOUND"
    } else if error_msg.contains("invalid") {
        "INVALID_INPUT"
    } else if error_msg.contains("timeout") {
        "TIMEOUT_ERROR"
    } else {
        "INTERNAL_ERROR"
    };

    McpResponse::error(error_code, error_msg).to_json_value()
}

/// Helper function to convert Result to standardized MCP JSON-RPC response
pub fn result_to_json_rpc<T>(
    result: Result<T>, 
    context: Option<McpContext>
) -> Result<Value, JsonRpcError>
where
    T: Serialize,
{
    match result {
        Ok(data) => {
            let json_data = serde_json::to_value(data)
                .map_err(|_| JsonRpcError::internal_error())?;
            
            let response = if let Some(ctx) = context {
                McpResponse::success_with_metadata(json_data, ctx.create_metadata())
            } else {
                McpResponse::success(json_data)
            };
            
            Ok(response.to_json_value())
        }
        Err(error) => {
            Ok(error_to_mcp_response(error))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_response_success() {
        let data = json!({"test": "value"});
        let response = McpResponse::success(data.clone());
        
        assert!(matches!(response.status, McpStatus::Success));
        assert_eq!(response.data, Some(data));
        assert!(response.error.is_none());
    }

    #[test]
    fn test_mcp_response_error() {
        let response = McpResponse::error("TEST_ERROR", "Test error message");
        
        assert!(matches!(response.status, McpStatus::Error));
        assert!(response.data.is_none());
        assert!(response.error.is_some());
        
        let error = response.error.unwrap();
        assert_eq!(error.code, "TEST_ERROR");
        assert_eq!(error.message, "Test error message");
    }

    #[test]
    fn test_mcp_params_extraction() {
        let mut params_map = Map::new();
        params_map.insert("test_string".to_string(), json!("test_value"));
        params_map.insert("test_bool".to_string(), json!(true));
        params_map.insert("test_array".to_string(), json!(["item1", "item2"]));

        assert_eq!(
            McpParams::get_optional_string(&params_map, "test_string"),
            Some("test_value".to_string())
        );

        assert_eq!(
            McpParams::get_optional_bool(&params_map, "test_bool"),
            Some(true)
        );

        assert_eq!(
            McpParams::get_optional_string_array(&params_map, "test_array"),
            Some(vec!["item1".to_string(), "item2".to_string()])
        );
    }
}