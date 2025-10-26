//! Request data extraction and manipulation
//!
//! This module provides structures for extracting and working with request data
//! in a portable way that can be passed to service layers without HTTP context.

use crate::error::{Error, Result};
use crate::http::FormValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents the body data of a request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BodyData {
    /// Form data (application/x-www-form-urlencoded or multipart/form-data)
    Form(HashMap<String, FormValue>),
    /// JSON data (application/json)
    Json(serde_json::Value),
    /// Plain text
    Text(String),
    /// Raw bytes
    Raw(Vec<u8>),
    /// Empty body
    Empty,
}

/// Extracted request data that can be passed to service layers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestData {
    /// Query string parameters
    pub query: HashMap<String, String>,
    /// Route parameters
    pub params: HashMap<String, String>,
    /// Request body data
    pub body: BodyData,
    /// Request headers
    pub headers: HashMap<String, String>,
    /// HTTP method
    pub method: String,
    /// Request URI
    pub uri: String,
}

impl RequestData {
    /// Create a new RequestData instance
    pub fn new(
        query: HashMap<String, String>,
        params: HashMap<String, String>,
        body: BodyData,
        headers: HashMap<String, String>,
        method: String,
        uri: String,
    ) -> Self {
        Self {
            query,
            params,
            body,
            headers,
            method,
            uri,
        }
    }

    // Query parameter methods

    /// Get a query parameter (returns error if missing)
    pub fn str_query(&self, key: &str) -> Result<String> {
        self.query
            .get(key)
            .filter(|s| !s.is_empty())
            .cloned()
            .ok_or_else(|| Error::InvalidInput(format!("Query parameter '{}' is required", key)))
    }

    /// Get a query parameter as integer
    pub fn int_query(&self, key: &str) -> Result<i32> {
        self.str_query(key)?.parse().map_err(|_| {
            Error::InvalidInput(format!("Query parameter '{}' must be a valid integer", key))
        })
    }

    /// Get a query parameter as boolean
    pub fn bool_query(&self, key: &str) -> Result<bool> {
        let value = self.str_query(key)?;
        Ok(matches!(value.as_str(), "true" | "1" | "yes" | "on"))
    }

    /// Get a query parameter with default
    pub fn str_query_or(&self, key: &str, default: &str) -> String {
        self.str_query(key).unwrap_or_else(|_| default.to_string())
    }

    /// Get a query parameter as integer with default
    pub fn int_query_or(&self, key: &str, default: i32) -> i32 {
        self.int_query(key).unwrap_or(default)
    }

    /// Get a query parameter as boolean with default
    pub fn bool_query_or(&self, key: &str, default: bool) -> bool {
        self.bool_query(key).unwrap_or(default)
    }

    // Route parameter methods

    /// Get a route parameter (returns error if missing)
    pub fn str_param(&self, key: &str) -> Result<String> {
        self.params
            .get(key)
            .filter(|s| !s.is_empty())
            .cloned()
            .ok_or_else(|| Error::InvalidInput(format!("Route parameter '{}' is required", key)))
    }

    /// Get a route parameter as integer
    pub fn int_param(&self, key: &str) -> Result<i32> {
        self.str_param(key)?.parse().map_err(|_| {
            Error::InvalidInput(format!("Route parameter '{}' must be a valid integer", key))
        })
    }

    /// Get a route parameter with default
    pub fn str_param_or(&self, key: &str, default: &str) -> String {
        self.str_param(key).unwrap_or_else(|_| default.to_string())
    }

    /// Get a route parameter as integer with default
    pub fn int_param_or(&self, key: &str, default: i32) -> i32 {
        self.int_param(key).unwrap_or(default)
    }

    // Body data methods

    /// Get a field from body (form or JSON)
    pub fn str_body(&self, key: &str) -> Result<String> {
        match &self.body {
            BodyData::Form(form) => form
                .get(key)
                .map(|v| v.as_string().to_string())
                .filter(|s| !s.is_empty())
                .ok_or_else(|| Error::InvalidInput(format!("Field '{}' is required", key))),
            BodyData::Json(json) => json
                .get(key)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty())
                .ok_or_else(|| Error::InvalidInput(format!("Field '{}' is required", key))),
            _ => Err(Error::InvalidInput(format!(
                "Field '{}' not found in body",
                key
            ))),
        }
    }

    /// Get a field from body as integer
    pub fn int_body(&self, key: &str) -> Result<i32> {
        match &self.body {
            BodyData::Form(form) => form
                .get(key)
                .ok_or_else(|| Error::InvalidInput(format!("Field '{}' is required", key)))?
                .as_string()
                .parse()
                .map_err(|_| {
                    Error::InvalidInput(format!("Field '{}' must be a valid integer", key))
                }),
            BodyData::Json(json) => json
                .get(key)
                .ok_or_else(|| Error::InvalidInput(format!("Field '{}' is required", key)))?
                .as_i64()
                .map(|n| n as i32)
                .or_else(|| json.get(key)?.as_str()?.parse().ok())
                .ok_or_else(|| {
                    Error::InvalidInput(format!("Field '{}' must be a valid integer", key))
                }),
            _ => Err(Error::InvalidInput(format!(
                "Field '{}' not found in body",
                key
            ))),
        }
    }

    /// Get a field from body as boolean
    pub fn bool_body(&self, key: &str) -> Result<bool> {
        match &self.body {
            BodyData::Form(form) => {
                let value = form
                    .get(key)
                    .ok_or_else(|| Error::InvalidInput(format!("Field '{}' is required", key)))?
                    .as_string();
                Ok(matches!(value, "true" | "1" | "yes" | "on" | "checked"))
            }
            BodyData::Json(json) => json
                .get(key)
                .ok_or_else(|| Error::InvalidInput(format!("Field '{}' is required", key)))?
                .as_bool()
                .or_else(|| {
                    json.get(key)?
                        .as_str()
                        .map(|s| matches!(s, "true" | "1" | "yes" | "on"))
                })
                .ok_or_else(|| Error::InvalidInput(format!("Field '{}' must be a boolean", key))),
            _ => Err(Error::InvalidInput(format!(
                "Field '{}' not found in body",
                key
            ))),
        }
    }

    /// Get a field from body with default
    pub fn str_body_or(&self, key: &str, default: &str) -> String {
        self.str_body(key).unwrap_or_else(|_| default.to_string())
    }

    /// Get a field from body as integer with default
    pub fn int_body_or(&self, key: &str, default: i32) -> i32 {
        self.int_body(key).unwrap_or(default)
    }

    /// Get a field from body as boolean with default
    pub fn bool_body_or(&self, key: &str, default: bool) -> bool {
        self.bool_body(key).unwrap_or(default)
    }

    /// Get all form data as a simple HashMap (for flashing)
    pub fn form_data(&self) -> HashMap<String, String> {
        match &self.body {
            BodyData::Form(form) => form
                .iter()
                .map(|(k, v)| (k.clone(), v.as_string().to_string()))
                .collect(),
            _ => HashMap::new(),
        }
    }

    /// Check if this is a form submission
    pub fn is_form(&self) -> bool {
        matches!(&self.body, BodyData::Form(_))
    }

    /// Check if this is a JSON submission
    pub fn is_json(&self) -> bool {
        matches!(&self.body, BodyData::Json(_))
    }
}
