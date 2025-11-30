use super::ast::{BinaryOperator, Expression, Helper, Node, Template, UnaryOperator};
use super::translation::TranslationSystem;
use crate::error::Result;
use crate::security::HtmlEscaper;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Loop control flow state
#[derive(Debug, Clone, PartialEq)]
enum LoopControl {
    None,
    Break,
    Continue,
}

/// Context for template rendering
pub struct RenderContext {
    /// Template data (model)
    pub data: Value,

    /// Global repository data (APP/MAIN - shared across all requests)
    pub global_repository: Value,

    /// Context repository data (repository/R - per-request)
    pub repository: Value,

    /// Session data
    pub session: Value,

    /// Query parameters
    pub query: Value,

    /// User data
    pub user: Value,

    /// Configuration values
    pub config: HashMap<String, String>,

    /// Framework configuration (CONF global)
    pub conf: Value,

    /// Current request URL path
    pub url: String,

    /// Current hostname (e.g., "https://example.com")
    pub hostname: String,

    /// Loop context (for nested loops)
    loop_stack: Vec<LoopContext>,

    /// Local variables (from helpers, loops, etc.)
    locals: HashMap<String, Value>,

    /// Defined sections
    sections: HashMap<String, Vec<Node>>,

    /// Defined helpers
    helpers: HashMap<String, Helper>,

    /// Built-in functions
    functions: HashMap<String, Box<dyn Fn(&[Value]) -> Value + Send + Sync>>,

    /// Translation system
    translator: Option<TranslationSystem>,
}

#[derive(Clone)]
struct LoopContext {
    item_name: String,
    current_index: usize,
    items: Vec<Value>,
}

impl Clone for RenderContext {
    fn clone(&self) -> Self {
        let mut new_context = Self {
            data: self.data.clone(),
            global_repository: self.global_repository.clone(),
            repository: self.repository.clone(),
            session: self.session.clone(),
            query: self.query.clone(),
            user: self.user.clone(),
            config: self.config.clone(),
            conf: self.conf.clone(),
            functions: HashMap::new(), // We'll re-register functions below
            translator: self.translator.clone(),
            url: self.url.clone(),
            hostname: self.hostname.clone(),
            loop_stack: self.loop_stack.clone(),
            locals: self.locals.clone(),
            sections: self.sections.clone(),
            helpers: self.helpers.clone(),
        };

        // Re-register built-in functions for the cloned context
        Self::register_builtin_functions(&mut new_context.functions);
        // Context-aware functions will be registered separately since we can't access self here

        new_context
    }
}

impl RenderContext {
    /// Create a new render context with default values
    pub fn new(data: Value) -> Self {
        let mut functions: HashMap<String, Box<dyn Fn(&[Value]) -> Value + Send + Sync>> =
            HashMap::new();

        // Register static built-in functions
        Self::register_builtin_functions(&mut functions);

        let mut context = Self {
            data,
            global_repository: Value::Object(serde_json::Map::new()),
            repository: Value::Object(serde_json::Map::new()),
            session: Value::Object(serde_json::Map::new()),
            query: Value::Object(serde_json::Map::new()),
            user: Value::Null,
            config: HashMap::new(),
            conf: Value::Object(serde_json::Map::new()),
            url: "/".to_string(),
            hostname: "localhost".to_string(),
            loop_stack: Vec::new(),
            locals: HashMap::new(),
            sections: HashMap::new(),
            helpers: HashMap::new(),
            functions,
            translator: None,
        };

        // Register context-aware functions
        let url = context.url.clone();
        let _hostname = context.hostname.clone();
        context.functions.insert(
            "url".to_string(),
            Box::new(move |args| {
                if !args.is_empty() {
                    // If hostname is provided as argument, combine it with URL
                    if let Value::String(host) = &args[0] {
                        Value::String(format!("{}{}", host, url))
                    } else {
                        Value::String(url.clone())
                    }
                } else {
                    // Return just the URL path
                    Value::String(url.clone())
                }
            }),
        );

        context
    }

    /// Set global repository data (APP/MAIN - shared across all requests)
    pub fn with_global_repository(mut self, global_repository: Value) -> Self {
        self.global_repository = global_repository;
        self
    }

    /// Set context repository data (repository/R - per-request)
    pub fn with_repository(mut self, repository: Value) -> Self {
        self.repository = repository;
        self
    }

    /// Set session data
    pub fn with_session(mut self, session: Value) -> Self {
        self.session = session;
        self
    }

    /// Set query parameters
    pub fn with_query(mut self, query: Value) -> Self {
        self.query = query;
        self
    }

    /// Set user data
    pub fn with_user(mut self, user: Value) -> Self {
        self.user = user;
        self
    }

    /// Add configuration values
    pub fn with_config(mut self, config: HashMap<String, String>) -> Self {
        self.config = config;
        self
    }

    /// Set framework configuration (CONF global)
    pub fn with_conf(mut self, conf: Value) -> Self {
        self.conf = conf;
        self
    }

    /// Set current URL
    pub fn with_url(mut self, url: String) -> Self {
        self.url = url.clone();

        // Re-register url function with new URL
        let _hostname = self.hostname.clone();
        self.functions.insert(
            "url".to_string(),
            Box::new(move |args| {
                if !args.is_empty() {
                    // If hostname is provided as argument, combine it with URL
                    if let Value::String(host) = &args[0] {
                        Value::String(format!("{}{}", host, url))
                    } else {
                        Value::String(url.clone())
                    }
                } else {
                    // Return just the URL path
                    Value::String(url.clone())
                }
            }),
        );

        self
    }

    /// Set hostname
    pub fn with_hostname(mut self, hostname: String) -> Self {
        self.hostname = hostname.clone();

        // Re-register url function with new hostname
        let url = self.url.clone();
        self.functions.insert(
            "url".to_string(),
            Box::new(move |args| {
                if !args.is_empty() {
                    // If hostname is provided as argument, combine it with URL
                    if let Value::String(host) = &args[0] {
                        Value::String(format!("{}{}", host, url))
                    } else {
                        Value::String(url.clone())
                    }
                } else {
                    // Return just the URL path
                    Value::String(url.clone())
                }
            }),
        );

        self
    }

    /// Set translation system
    pub fn with_translator(mut self, translator: TranslationSystem) -> Self {
        self.translator = Some(translator);
        self
    }

    /// Set sections (for layout rendering with child-defined sections)
    pub fn with_sections(mut self, sections: HashMap<String, Vec<Node>>) -> Self {
        self.sections = sections;
        self
    }

    /// Get current loop index
    fn get_loop_index(&self) -> Option<usize> {
        self.loop_stack.last().map(|ctx| ctx.current_index)
    }

    /// Register built-in functions
    fn register_builtin_functions(
        functions: &mut HashMap<String, Box<dyn Fn(&[Value]) -> Value + Send + Sync>>,
    ) {
        // css function - generates CSS link tag
        functions.insert(
            "css".to_string(),
            Box::new(|args| {
                if args.is_empty() {
                    return Value::String(String::new());
                }
                let url = match &args[0] {
                    Value::String(s) => s.clone(),
                    _ => return Value::String(String::new()),
                };
                Value::String(format!(
                    r#"<link rel="stylesheet" href="{}">"#,
                    HtmlEscaper::escape(&url)
                ))
            }),
        );

        // js function - generates script tag
        functions.insert(
            "js".to_string(),
            Box::new(|args| {
                if args.is_empty() {
                    return Value::String(String::new());
                }
                let url = match &args[0] {
                    Value::String(s) => s.clone(),
                    _ => return Value::String(String::new()),
                };
                Value::String(format!(
                    r#"<script src="{}"></script>"#,
                    HtmlEscaper::escape(&url)
                ))
            }),
        );

        // json function - converts value to JSON string
        functions.insert(
            "json".to_string(),
            Box::new(|args| {
                if args.is_empty() {
                    return Value::String("{}".to_string());
                }
                match serde_json::to_string(&args[0]) {
                    Ok(json) => Value::String(json),
                    Err(_) => Value::String("{}".to_string()),
                }
            }),
        );

        // range function - generates array of numbers
        // Usage: range(10) -> [0,1,2,3,4,5,6,7,8,9]
        // Usage: range(1, 11) -> [1,2,3,4,5,6,7,8,9,10]
        // Usage: range(0, 10, 2) -> [0,2,4,6,8]
        functions.insert(
            "range".to_string(),
            Box::new(|args| {
                eprintln!("DEBUG: range() called with {} args", args.len());
                eprintln!("DEBUG: range() args: {:?}", args);
                let (start, stop, step) = match args.len() {
                    0 => return Value::Array(vec![]),
                    1 => {
                        // range(stop) - from 0 to stop-1
                        let stop = match &args[0] {
                            Value::Number(n) => {
                                // Try as_i64 first, then fall back to as_f64 and convert
                                n.as_i64()
                                    .unwrap_or_else(|| n.as_f64().unwrap_or(0.0) as i64)
                            }
                            _ => {
                                eprintln!("DEBUG: range() arg[0] is not a number: {:?}", args[0]);
                                return Value::Array(vec![]);
                            }
                        };
                        (0, stop, 1)
                    }
                    2 => {
                        // range(start, stop) - from start to stop-1
                        let start = match &args[0] {
                            Value::Number(n) => n
                                .as_i64()
                                .unwrap_or_else(|| n.as_f64().unwrap_or(0.0) as i64),
                            _ => return Value::Array(vec![]),
                        };
                        let stop = match &args[1] {
                            Value::Number(n) => n
                                .as_i64()
                                .unwrap_or_else(|| n.as_f64().unwrap_or(0.0) as i64),
                            _ => return Value::Array(vec![]),
                        };
                        (start, stop, 1)
                    }
                    _ => {
                        // range(start, stop, step) - from start to stop-1 with step
                        let start = match &args[0] {
                            Value::Number(n) => n
                                .as_i64()
                                .unwrap_or_else(|| n.as_f64().unwrap_or(0.0) as i64),
                            _ => return Value::Array(vec![]),
                        };
                        let stop = match &args[1] {
                            Value::Number(n) => n
                                .as_i64()
                                .unwrap_or_else(|| n.as_f64().unwrap_or(0.0) as i64),
                            _ => return Value::Array(vec![]),
                        };
                        let step = match &args[2] {
                            Value::Number(n) => n
                                .as_i64()
                                .unwrap_or_else(|| n.as_f64().unwrap_or(1.0) as i64),
                            _ => 1,
                        };
                        (start, stop, step)
                    }
                };

                // Generate the range
                let mut result = Vec::new();
                if step > 0 {
                    let mut i = start;
                    while i < stop {
                        result.push(Value::Number(serde_json::Number::from(i)));
                        i += step;
                    }
                } else if step < 0 {
                    let mut i = start;
                    while i > stop {
                        result.push(Value::Number(serde_json::Number::from(i)));
                        i += step;
                    }
                }

                eprintln!("DEBUG: range() returning array with {} items", result.len());
                Value::Array(result)
            }),
        );

        // image function - generates img tag
        functions.insert(
            "image".to_string(),
            Box::new(|args| {
                if args.is_empty() {
                    return Value::String(String::new());
                }
                let url = match &args[0] {
                    Value::String(s) => s.clone(),
                    _ => return Value::String(String::new()),
                };

                let mut attrs = String::new();
                if args.len() > 1 {
                    if let Value::Number(width) = &args[1] {
                        attrs.push_str(&format!(r#" width="{}""#, width));
                    }
                }
                if args.len() > 2 {
                    if let Value::Number(height) = &args[2] {
                        attrs.push_str(&format!(r#" height="{}""#, height));
                    }
                }
                if args.len() > 3 {
                    if let Value::String(alt) = &args[3] {
                        attrs.push_str(&format!(r#" alt="{}""#, HtmlEscaper::escape(alt)));
                    }
                }

                Value::String(format!(
                    r#"<img src="{}"{}>"#,
                    HtmlEscaper::escape(&url),
                    attrs
                ))
            }),
        );

        // meta function - generates meta tags
        functions.insert(
            "meta".to_string(),
            Box::new(|args| {
                let mut tags = String::new();

                if !args.is_empty() {
                    if let Value::String(title) = &args[0] {
                        tags.push_str(&format!(
                            r#"<meta property="og:title" content="{}">"#,
                            HtmlEscaper::escape(title)
                        ));
                        tags.push_str(&format!(
                            r#"<meta name="twitter:title" content="{}">"#,
                            HtmlEscaper::escape(title)
                        ));
                    }
                }

                if args.len() > 1 {
                    if let Value::String(desc) = &args[1] {
                        tags.push_str(&format!(
                            r#"<meta name="description" content="{}">"#,
                            HtmlEscaper::escape(desc)
                        ));
                        tags.push_str(&format!(
                            r#"<meta property="og:description" content="{}">"#,
                            HtmlEscaper::escape(desc)
                        ));
                    }
                }

                Value::String(tags)
            }),
        );
    }

    /// Resolve a variable name to its value
    fn resolve_variable(&self, name: &str) -> Value {
        // Check if it's a dotted path
        let parts: Vec<&str> = name.split('.').collect();

        if parts.len() == 1 {
            // Simple variable name

            // Check special variables first
            if name == "index" {
                if let Some(idx) = self.get_loop_index() {
                    return Value::Number(serde_json::Number::from(idx));
                }
            }

            // Check for framework globals
            if name == "CONF" {
                return self.conf.clone();
            }
            if name == "repository" {
                return self.repository.clone();
            }
            if name == "session" {
                return self.session.clone();
            }
            // Handle flash as an alias to session.flash
            if name == "flash" {
                if let Value::Object(session_map) = &self.session {
                    if let Some(flash_value) = session_map.get("flash") {
                        return flash_value.clone();
                    }
                }
                return Value::Null;
            }
            if name == "query" {
                return self.query.clone();
            }
            if name == "user" {
                return self.user.clone();
            }
            if name == "url" {
                return Value::String(self.url.clone());
            }
            if name == "hostname" {
                return Value::String(self.hostname.clone());
            }
            // Model data access (entire model)
            if name == "model" || name == "M" {
                return self.data.clone();
            }
            // Repository access (R is alias for repository)
            if name == "R" {
                return self.repository.clone();
            }
            // APP/MAIN are aliases for global repository
            if name == "APP" || name == "MAIN" {
                return self.global_repository.clone();
            }
            if name == "csrf_token" {
                // Get default CSRF token from session
                if let Value::Object(session_map) = &self.session {
                    if let Some(token_data) = session_map.get("_csrf_token") {
                        // Extract token value from the data object
                        if let Some(token) = token_data.get("token") {
                            return token.clone();
                        }
                    }
                }
                return Value::Null;
            }

            // Handle @{csrf_token.custom_id} - dot notation for custom tokens
            if let Some(token_id) = name.strip_prefix("csrf_token.") {
                // Remove "csrf_token." prefix

                if let Value::Object(session_map) = &self.session {
                    if let Some(token_data) = session_map.get(token_id) {
                        // Extract token value from the data object
                        if let Some(token) = token_data.get("token") {
                            return token.clone();
                        }
                    }
                }
                return Value::Null;
            }
            if name == "root" {
                // Get root from CONF.default_root
                if let Value::Object(conf_map) = &self.conf {
                    if let Some(Value::String(root)) = conf_map.get("default_root") {
                        return Value::String(root.clone());
                    }
                }
                return Value::String("".to_string());
            }

            // Model data should NOT be directly accessible
            // It must be accessed via model.key or M.key
            // This provides clear separation of data sources

            // Check loop stack for item variables
            for loop_ctx in self.loop_stack.iter().rev() {
                if loop_ctx.item_name == name && loop_ctx.current_index < loop_ctx.items.len() {
                    return loop_ctx.items[loop_ctx.current_index].clone();
                }
            }

            // Finally check locals (helper params, etc.)
            if let Some(value) = self.locals.get(name) {
                return value.clone();
            }

            // Fallback to checking model data for top-level properties
            if let Value::Object(map) = &self.data {
                if let Some(value) = map.get(name) {
                    return value.clone();
                }
            }
        } else {
            // Dotted path - need to resolve base first
            let base = parts[0];
            let rest = parts[1..].join(".");

            // Get the base value using the same priority order
            let base_value = {
                // Check for framework globals first
                if base == "CONF" {
                    self.conf.clone()
                } else if base == "repository" {
                    self.repository.clone()
                } else if base == "session" {
                    self.session.clone()
                } else if base == "flash" {
                    // Handle flash.xxx as an alias to session.flash.xxx
                    if let Value::Object(session_map) = &self.session {
                        if let Some(flash_value) = session_map.get("flash") {
                            flash_value.clone()
                        } else {
                            Value::Null
                        }
                    } else {
                        Value::Null
                    }
                } else if base == "query" {
                    self.query.clone()
                } else if base == "user" {
                    self.user.clone()
                } else if base == "url" {
                    Value::String(self.url.clone())
                } else if base == "hostname" {
                    Value::String(self.hostname.clone())
                } else if base == "root" {
                    // Get root from CONF.default_root
                    if let Value::Object(conf_map) = &self.conf {
                        if let Some(Value::String(root)) = conf_map.get("default_root") {
                            Value::String(root.clone())
                        } else {
                            Value::String("".to_string())
                        }
                    } else {
                        Value::String("".to_string())
                    }
                } else if base == "M" || base == "model" {
                    // Handle M.field or model.field access
                    self.data.clone()
                } else if base == "R" {
                    // Handle R.field (repository) access
                    self.repository.clone()
                } else if base == "APP" || base == "MAIN" {
                    // Handle APP.field or MAIN.field (global repository) access
                    self.global_repository.clone()
                } else {
                    // Check loop variables then locals, then data
                    let mut found = None;
                    for loop_ctx in self.loop_stack.iter().rev() {
                        if loop_ctx.item_name == base
                            && loop_ctx.current_index < loop_ctx.items.len()
                        {
                            found = Some(loop_ctx.items[loop_ctx.current_index].clone());
                            break;
                        }
                    }

                    if let Some(v) = found {
                        v
                    } else if let Some(value) = self.locals.get(base) {
                        value.clone()
                    } else if let Value::Object(map) = &self.data {
                        // Fallback to checking data for top-level properties
                        map.get(base).cloned().unwrap_or(Value::Null)
                    } else {
                        Value::Null
                    }
                }
            };

            // Now get the nested value
            if let Some(value) = self.get_nested_value(&base_value, &rest) {
                return value;
            }
        }

        Value::Null
    }

    /// Get nested value from a JSON object
    fn get_nested_value(&self, data: &Value, path: &str) -> Option<Value> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = data.clone();

        for part in parts {
            match &current {
                Value::Object(map) => {
                    current = map.get(part)?.clone();
                }
                Value::Array(arr) => {
                    // Handle array properties like 'length' or numeric indexes
                    match part {
                        "length" | "size" => {
                            return Some(Value::Number(serde_json::Number::from(arr.len())));
                        }
                        _ => {
                            // Try to parse as index
                            if let Ok(index) = part.parse::<usize>() {
                                current = arr.get(index)?.clone();
                            } else {
                                return None;
                            }
                        }
                    }
                }
                Value::String(s) => {
                    // Handle string properties like 'length'
                    match part {
                        "length" | "size" => {
                            return Some(Value::Number(serde_json::Number::from(s.len())));
                        }
                        _ => return None,
                    }
                }
                _ => return None,
            }
        }

        Some(current)
    }

    /// Evaluate an expression to a value
    fn evaluate_expression(&self, expr: &Expression) -> Result<Value> {
        match expr {
            Expression::String(s) => Ok(Value::String(s.clone())),
            Expression::Number(n) => Ok(Value::Number(
                serde_json::Number::from_f64(*n).unwrap_or_else(|| serde_json::Number::from(0)),
            )),
            Expression::Boolean(b) => Ok(Value::Bool(*b)),
            Expression::Null => Ok(Value::Null),

            Expression::Variable(name) => Ok(self.resolve_variable(name)),

            Expression::PropertyAccess { object, property } => {
                let obj_value = self.evaluate_expression(object)?;
                match obj_value {
                    Value::Object(map) => Ok(map.get(property).cloned().unwrap_or(Value::Null)),
                    Value::Array(ref arr) => {
                        // Handle array properties like 'length'
                        match property.as_str() {
                            "length" | "size" => {
                                Ok(Value::Number(serde_json::Number::from(arr.len())))
                            }
                            _ => {
                                // Try to parse as index
                                if let Ok(index) = property.parse::<usize>() {
                                    Ok(arr.get(index).cloned().unwrap_or(Value::Null))
                                } else {
                                    Ok(Value::Null)
                                }
                            }
                        }
                    }
                    Value::String(ref s) => {
                        // Handle string properties
                        match property.as_str() {
                            "length" | "size" => {
                                Ok(Value::Number(serde_json::Number::from(s.len())))
                            }
                            _ => Ok(Value::Null),
                        }
                    }
                    _ => Ok(Value::Null),
                }
            }

            Expression::Array(items) => {
                let values: Result<Vec<Value>> =
                    items.iter().map(|e| self.evaluate_expression(e)).collect();
                Ok(Value::Array(values?))
            }

            Expression::Object(map) => {
                let mut result = serde_json::Map::new();
                for (key, expr) in map {
                    result.insert(key.clone(), self.evaluate_expression(expr)?);
                }
                Ok(Value::Object(result))
            }

            Expression::BinaryOp { left, op, right } => {
                let left_val = self.evaluate_expression(left)?;
                let right_val = self.evaluate_expression(right)?;
                self.evaluate_binary_op(&left_val, op, &right_val)
            }

            Expression::UnaryOp { op, operand } => {
                let operand_val = self.evaluate_expression(operand)?;
                self.evaluate_unary_op(op, &operand_val)
            }

            Expression::FunctionCall { name, args } => {
                // Evaluate built-in functions
                eprintln!("DEBUG: evaluate_expression - FunctionCall: {}", name);
                self.evaluate_function_call(name, args)
            }

            Expression::Ternary {
                condition,
                then_expr,
                else_expr,
            } => {
                let cond_value = self.evaluate_expression(condition)?;
                if self.is_truthy(&cond_value) {
                    self.evaluate_expression(then_expr)
                } else {
                    self.evaluate_expression(else_expr)
                }
            }
        }
    }

    /// Evaluate a binary operation
    fn evaluate_binary_op(
        &self,
        left: &Value,
        op: &BinaryOperator,
        right: &Value,
    ) -> Result<Value> {
        match op {
            BinaryOperator::Equal => Ok(Value::Bool(left == right)),
            BinaryOperator::NotEqual => Ok(Value::Bool(left != right)),

            BinaryOperator::LessThan => {
                if let (Value::Number(l), Value::Number(r)) = (left, right) {
                    Ok(Value::Bool(
                        l.as_f64().unwrap_or(0.0) < r.as_f64().unwrap_or(0.0),
                    ))
                } else {
                    Ok(Value::Bool(false))
                }
            }

            BinaryOperator::LessThanOrEqual => {
                if let (Value::Number(l), Value::Number(r)) = (left, right) {
                    Ok(Value::Bool(
                        l.as_f64().unwrap_or(0.0) <= r.as_f64().unwrap_or(0.0),
                    ))
                } else {
                    Ok(Value::Bool(false))
                }
            }

            BinaryOperator::GreaterThan => {
                if let (Value::Number(l), Value::Number(r)) = (left, right) {
                    Ok(Value::Bool(
                        l.as_f64().unwrap_or(0.0) > r.as_f64().unwrap_or(0.0),
                    ))
                } else {
                    Ok(Value::Bool(false))
                }
            }

            BinaryOperator::GreaterThanOrEqual => {
                if let (Value::Number(l), Value::Number(r)) = (left, right) {
                    Ok(Value::Bool(
                        l.as_f64().unwrap_or(0.0) >= r.as_f64().unwrap_or(0.0),
                    ))
                } else {
                    Ok(Value::Bool(false))
                }
            }

            BinaryOperator::And => {
                let left_bool = self.is_truthy(left);
                let right_bool = self.is_truthy(right);
                Ok(Value::Bool(left_bool && right_bool))
            }

            BinaryOperator::Or => {
                let left_bool = self.is_truthy(left);
                let right_bool = self.is_truthy(right);
                Ok(Value::Bool(left_bool || right_bool))
            }

            BinaryOperator::Add => match (left, right) {
                (Value::Number(l), Value::Number(r)) => {
                    let sum = l.as_f64().unwrap_or(0.0) + r.as_f64().unwrap_or(0.0);
                    Ok(Value::Number(
                        serde_json::Number::from_f64(sum)
                            .unwrap_or_else(|| serde_json::Number::from(0)),
                    ))
                }
                (Value::String(l), Value::String(r)) => Ok(Value::String(format!("{}{}", l, r))),
                _ => Ok(Value::Null),
            },

            BinaryOperator::Subtract => {
                if let (Value::Number(l), Value::Number(r)) = (left, right) {
                    let diff = l.as_f64().unwrap_or(0.0) - r.as_f64().unwrap_or(0.0);
                    Ok(Value::Number(
                        serde_json::Number::from_f64(diff)
                            .unwrap_or_else(|| serde_json::Number::from(0)),
                    ))
                } else {
                    Ok(Value::Null)
                }
            }

            BinaryOperator::Multiply => {
                if let (Value::Number(l), Value::Number(r)) = (left, right) {
                    let product = l.as_f64().unwrap_or(0.0) * r.as_f64().unwrap_or(0.0);
                    Ok(Value::Number(
                        serde_json::Number::from_f64(product)
                            .unwrap_or_else(|| serde_json::Number::from(0)),
                    ))
                } else {
                    Ok(Value::Null)
                }
            }

            BinaryOperator::Divide => {
                if let (Value::Number(l), Value::Number(r)) = (left, right) {
                    let divisor = r.as_f64().unwrap_or(1.0);
                    if divisor == 0.0 {
                        Ok(Value::Null)
                    } else {
                        let quotient = l.as_f64().unwrap_or(0.0) / divisor;
                        Ok(Value::Number(
                            serde_json::Number::from_f64(quotient)
                                .unwrap_or_else(|| serde_json::Number::from(0)),
                        ))
                    }
                } else {
                    Ok(Value::Null)
                }
            }

            BinaryOperator::Modulo => {
                if let (Value::Number(l), Value::Number(r)) = (left, right) {
                    let divisor = r.as_f64().unwrap_or(1.0);
                    if divisor == 0.0 {
                        Ok(Value::Null)
                    } else {
                        let remainder = l.as_f64().unwrap_or(0.0) % divisor;
                        Ok(Value::Number(
                            serde_json::Number::from_f64(remainder)
                                .unwrap_or_else(|| serde_json::Number::from(0)),
                        ))
                    }
                } else {
                    Ok(Value::Null)
                }
            }
        }
    }

    /// Evaluate a unary operation
    fn evaluate_unary_op(&self, op: &UnaryOperator, operand: &Value) -> Result<Value> {
        match op {
            UnaryOperator::Not => Ok(Value::Bool(!self.is_truthy(operand))),

            UnaryOperator::Minus => {
                if let Value::Number(n) = operand {
                    let negated = -(n.as_f64().unwrap_or(0.0));
                    Ok(Value::Number(
                        serde_json::Number::from_f64(negated)
                            .unwrap_or_else(|| serde_json::Number::from(0)),
                    ))
                } else {
                    Ok(Value::Null)
                }
            }
        }
    }

    /// Evaluate built-in function calls
    fn evaluate_function_call(&self, name: &str, args: &[Expression]) -> Result<Value> {
        // First check if it's a registered built-in function
        if let Some(func) = self.functions.get(name) {
            // Evaluate arguments
            let arg_values: Result<Vec<Value>> =
                args.iter().map(|e| self.evaluate_expression(e)).collect();
            let arg_values = arg_values?;

            // Call the function
            return Ok(func(&arg_values));
        }

        // Handle inline functions that don't need registration
        match name {
            "len" | "length" => {
                if args.len() != 1 {
                    return Ok(Value::Null);
                }
                let arg = self.evaluate_expression(&args[0])?;
                match arg {
                    Value::String(s) => Ok(Value::Number(serde_json::Number::from(s.len()))),
                    Value::Array(arr) => Ok(Value::Number(serde_json::Number::from(arr.len()))),
                    Value::Object(map) => Ok(Value::Number(serde_json::Number::from(map.len()))),
                    _ => Ok(Value::Number(serde_json::Number::from(0))),
                }
            }

            "upper" | "toUpperCase" => {
                if args.len() != 1 {
                    return Ok(Value::Null);
                }
                let arg = self.evaluate_expression(&args[0])?;
                if let Value::String(s) = arg {
                    Ok(Value::String(s.to_uppercase()))
                } else {
                    Ok(arg)
                }
            }

            "lower" | "toLowerCase" => {
                if args.len() != 1 {
                    return Ok(Value::Null);
                }
                let arg = self.evaluate_expression(&args[0])?;
                if let Value::String(s) = arg {
                    Ok(Value::String(s.to_lowercase()))
                } else {
                    Ok(arg)
                }
            }

            "csrf" => {
                // @{csrf("token_id")} - returns hidden input with custom token
                // Get token ID from first argument or use default
                let token_id = if !args.is_empty() {
                    let arg = self.evaluate_expression(&args[0])?;
                    arg.as_str().unwrap_or("_csrf_token").to_string()
                } else {
                    "_csrf_token".to_string()
                };

                // Get token from session
                let token = if let Value::Object(session_map) = &self.session {
                    session_map
                        .get(&token_id)
                        .and_then(|data| data.get("token"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                } else {
                    None
                };

                if let Some(token) = token {
                    // Return as HTML string value
                    Ok(Value::String(format!(
                        "<input type=\"hidden\" name=\"{}\" value=\"{}\">",
                        token_id,
                        HtmlEscaper::escape_attribute(&token)
                    )))
                } else {
                    Ok(Value::String(String::new()))
                }
            }

            _ => {
                // Unknown function - return null
                Ok(Value::Null)
            }
        }
    }

    /// Check if a value is truthy
    fn is_truthy(&self, value: &Value) -> bool {
        match value {
            Value::Bool(b) => *b,
            Value::Null => false,
            Value::Number(n) => n.as_f64().unwrap_or(0.0) != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Array(arr) => !arr.is_empty(),
            Value::Object(map) => !map.is_empty(),
        }
    }

    /// Resolve a value from an expression for partials
    pub fn resolve_value_from_expression(&self, expr: &Expression) -> Value {
        self.evaluate_expression(expr).unwrap_or(Value::Null)
    }

    /// Convert a value to string for output
    fn value_to_string(&self, value: &Value, escape: bool) -> String {
        let raw = match value {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => String::new(),
            _ => value.to_string(),
        };

        if escape {
            HtmlEscaper::escape(&raw)
        } else {
            raw
        }
    }
}

/// Template loader function type
pub type TemplateLoader = Box<dyn Fn(&str) -> Result<Template> + Send + Sync>;

/// Template renderer
pub struct Renderer {
    context: RenderContext,
    /// Path to templates directory for loading partials
    template_path: Option<String>,
    /// Template loader function for loading partials
    template_loader: Option<Arc<TemplateLoader>>,
}

impl Renderer {
    /// Create a new renderer with the given context
    pub fn new(context: RenderContext) -> Self {
        Self {
            context,
            template_path: None,
            template_loader: None,
        }
    }

    /// Set the template path for loading partials
    pub fn with_template_path(mut self, path: String) -> Self {
        self.template_path = Some(path);
        self
    }

    /// Set the template loader function for loading partials
    pub fn with_template_loader(mut self, loader: Arc<TemplateLoader>) -> Self {
        self.template_loader = Some(loader);
        self
    }

    /// Render a template to string
    pub fn render(&mut self, template: &Template) -> Result<String> {
        // Merge sections: keep existing sections (e.g., from child views),
        // but add new ones from this template (layout's own sections)
        // Child sections take precedence over layout sections with same name
        for (name, nodes) in &template.sections {
            self.context
                .sections
                .entry(name.clone())
                .or_insert_with(|| nodes.clone());
        }

        // Set helpers from template (helpers are template-scoped)
        self.context.helpers = template.helpers.clone();

        // Render the main nodes
        self.render_nodes(&template.nodes)
    }

    /// Render a list of nodes
    fn render_nodes(&mut self, nodes: &[Node]) -> Result<String> {
        // Pre-allocate capacity: estimate ~512 bytes per node on average
        // This reduces reallocations during string building
        let estimated_capacity = nodes.len() * 512;
        let mut output = String::with_capacity(estimated_capacity.min(64 * 1024)); // Cap at 64KB

        for node in nodes {
            output.push_str(&self.render_node(node)?);
        }

        Ok(output)
    }

    /// Render nodes with loop control flow detection
    fn render_nodes_with_control(&mut self, nodes: &[Node]) -> Result<(String, LoopControl)> {
        // Pre-allocate capacity: estimate ~512 bytes per node on average
        let estimated_capacity = nodes.len() * 512;
        let mut output = String::with_capacity(estimated_capacity.min(64 * 1024)); // Cap at 64KB

        for node in nodes {
            match node {
                Node::Break => {
                    return Ok((output, LoopControl::Break));
                }
                Node::Continue => {
                    return Ok((output, LoopControl::Continue));
                }
                _ => {
                    output.push_str(&self.render_node(node)?);
                }
            }
        }

        Ok((output, LoopControl::None))
    }

    /// Render a single node
    fn render_node(&mut self, node: &Node) -> Result<String> {
        match node {
            Node::Text(text) => Ok(text.clone()),

            Node::Variable { name, raw, expression } => {
                let value = if let Some(expr) = expression {
                    // Evaluate the expression
                    self.context.evaluate_expression(expr)?
                } else {
                    // Resolve as variable name
                    self.context.resolve_variable(name)
                };
                
                // Don't escape certain system variables that contain safe paths/URLs
                let should_escape = if !raw {
                    match name.as_str() {
                        "root" | "url" | "hostname" => false,
                        _ => true,
                    }
                } else {
                    false
                };
                Ok(self.context.value_to_string(&value, should_escape))
            }

            Node::Conditional {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
            } => {
                let cond_value = self.context.evaluate_expression(condition)?;

                if self.context.is_truthy(&cond_value) {
                    self.render_nodes(then_branch)
                } else {
                    // Check else-if branches
                    for (else_if_cond, else_if_body) in else_if_branches {
                        let else_if_value = self.context.evaluate_expression(else_if_cond)?;
                        if self.context.is_truthy(&else_if_value) {
                            return self.render_nodes(else_if_body);
                        }
                    }

                    // Render else branch if present
                    if let Some(else_nodes) = else_branch {
                        self.render_nodes(else_nodes)
                    } else {
                        Ok(String::new())
                    }
                }
            }

            Node::Loop {
                item_name,
                collection,
                body,
            } => {
                let collection_value = self.context.evaluate_expression(collection)?;

                if let Value::Array(items) = collection_value {
                    // Pre-allocate capacity: estimate ~256 bytes per item
                    let estimated_capacity = items.len() * 256;
                    let mut output = String::with_capacity(estimated_capacity.min(64 * 1024)); // Cap at 64KB

                    // Create loop context
                    let loop_ctx = LoopContext {
                        item_name: item_name.clone(),
                        current_index: 0,
                        items: items.clone(),
                    };

                    self.context.loop_stack.push(loop_ctx);

                    for i in 0..items.len() {
                        // Update current index
                        if let Some(ctx) = self.context.loop_stack.last_mut() {
                            ctx.current_index = i;
                        }

                        // Render body and check for break/continue
                        let body_result = self.render_nodes_with_control(body);
                        match body_result {
                            Ok((content, LoopControl::None)) => {
                                output.push_str(&content);
                            }
                            Ok((content, LoopControl::Continue)) => {
                                output.push_str(&content);
                                continue;
                            }
                            Ok((content, LoopControl::Break)) => {
                                output.push_str(&content);
                                break;
                            }
                            Err(e) => {
                                self.context.loop_stack.pop();
                                return Err(e);
                            }
                        }
                    }

                    // Pop loop context
                    self.context.loop_stack.pop();

                    Ok(output)
                } else if let Value::Object(map) = collection_value {
                    let mut output = String::new();

                    // For objects, iterate over key-value pairs
                    let items: Vec<Value> = map
                        .iter()
                        .map(|(k, v)| {
                            let mut pair = serde_json::Map::new();
                            pair.insert("key".to_string(), Value::String(k.clone()));
                            pair.insert("value".to_string(), v.clone());
                            Value::Object(pair)
                        })
                        .collect();

                    let loop_ctx = LoopContext {
                        item_name: item_name.clone(),
                        current_index: 0,
                        items,
                    };

                    self.context.loop_stack.push(loop_ctx);

                    let item_count = self.context.loop_stack.last().unwrap().items.len();
                    for i in 0..item_count {
                        if let Some(ctx) = self.context.loop_stack.last_mut() {
                            ctx.current_index = i;
                        }

                        // Render body and check for break/continue
                        let body_result = self.render_nodes_with_control(body);
                        match body_result {
                            Ok((content, LoopControl::None)) => {
                                output.push_str(&content);
                            }
                            Ok((content, LoopControl::Continue)) => {
                                output.push_str(&content);
                                continue;
                            }
                            Ok((content, LoopControl::Break)) => {
                                output.push_str(&content);
                                break;
                            }
                            Err(e) => {
                                self.context.loop_stack.pop();
                                return Err(e);
                            }
                        }
                    }

                    self.context.loop_stack.pop();

                    Ok(output)
                } else {
                    // Log warning about non-iterable collection
                    eprintln!("WARNING: Loop collection '@{{foreach {} in ...}}' is not iterable. Type: {}, Value: {:?}",
                              item_name,
                              match &collection_value {
                                  Value::Null => "null",
                                  Value::Bool(_) => "boolean",
                                  Value::Number(_) => "number",
                                  Value::String(_) => "string",
                                  _ => "unknown"
                              },
                              collection_value);
                    Ok(String::new())
                }
            }

            Node::Break => {
                // Break is handled in render_nodes_with_control
                // If we reach here outside a loop, it's a no-op
                Ok(String::new())
            }

            Node::Continue => {
                // Continue is handled in render_nodes_with_control
                // If we reach here outside a loop, it's a no-op
                Ok(String::new())
            }

            Node::Index => {
                if let Some(idx) = self.context.get_loop_index() {
                    Ok(idx.to_string())
                } else {
                    Ok(String::new())
                }
            }

            Node::SectionCall(name) => {
                let section_nodes = self.context.sections.get(name).cloned();
                if let Some(nodes) = section_nodes {
                    self.render_nodes(&nodes)
                } else {
                    Ok(String::new())
                }
            }

            Node::SectionDef { .. } => {
                // Section definitions are extracted during parsing
                Ok(String::new())
            }

            Node::HelperCall { name, args } => {
                let helper = self.context.helpers.get(name).cloned();
                if let Some(helper) = helper {
                    // Evaluate arguments
                    let arg_values: Result<Vec<Value>> = args
                        .iter()
                        .map(|e| self.context.evaluate_expression(e))
                        .collect();
                    let arg_values = arg_values?;

                    // Set up local variables for helper parameters
                    let saved_locals = self.context.locals.clone();

                    for (i, param) in helper.params.iter().enumerate() {
                        if i < arg_values.len() {
                            self.context
                                .locals
                                .insert(param.clone(), arg_values[i].clone());
                        }
                    }

                    // Render helper body
                    let result = self.render_nodes(&helper.body)?;

                    // Restore locals
                    self.context.locals = saved_locals;

                    Ok(result)
                } else {
                    Ok(String::new())
                }
            }

            Node::HelperDef { .. } => {
                // Helper definitions are extracted during parsing
                Ok(String::new())
            }

            Node::View { name, model } => {
                // Try to use template loader first (uses cache)
                if let Some(loader) = &self.template_loader {
                    match loader(name) {
                        Ok(mut partial_template) => {
                            // Extract sections and helpers
                            partial_template.extract_sections();
                            partial_template.extract_helpers();

                            // Create a new renderer for the partial with the same or updated context
                            let partial_context = if let Some(model_expr) = model {
                                // If a model expression is provided, create context with that as the data
                                let model_value =
                                    self.context.resolve_value_from_expression(model_expr);
                                RenderContext::new(model_value)
                                    .with_global_repository(self.context.global_repository.clone())
                                    .with_repository(self.context.repository.clone())
                                    .with_session(self.context.session.clone())
                                    .with_query(self.context.query.clone())
                                    .with_user(self.context.user.clone())
                                    .with_config(self.context.config.clone())
                                    .with_conf(self.context.conf.clone())
                                    .with_url(self.context.url.clone())
                                    .with_hostname(self.context.hostname.clone())
                            } else {
                                // No model specified, use the same context
                                self.context.clone()
                            };

                            let mut partial_renderer = Renderer::new(partial_context);

                            // Pass along the template loader to support nested partials
                            if let Some(loader) = &self.template_loader {
                                partial_renderer =
                                    partial_renderer.with_template_loader(Arc::clone(loader));
                            }

                            // Also pass along template path as fallback
                            if let Some(path) = &self.template_path {
                                partial_renderer =
                                    partial_renderer.with_template_path(path.clone());
                            }

                            // Render the partial
                            return partial_renderer.render(&partial_template);
                        }
                        Err(_) => {
                            // Fallback to file system loading
                        }
                    }
                }

                // Fallback to direct file system loading
                if let Some(base_path) = &self.template_path {
                    let partial_path =
                        std::path::Path::new(base_path).join(format!("{}.html", name));

                    // Try to load the partial template
                    if let Ok(content) = std::fs::read_to_string(&partial_path) {
                        // Parse and render the partial
                        if let Ok(mut parser) = super::parser::Parser::new(&content) {
                            if let Ok(template) = parser.parse() {
                                // Create a new renderer for the partial with the same or updated context
                                let partial_context = if let Some(model_expr) = model {
                                    // If a model expression is provided, create context with that as the data
                                    let model_value =
                                        self.context.resolve_value_from_expression(model_expr);
                                    RenderContext::new(model_value)
                                        .with_global_repository(
                                            self.context.global_repository.clone(),
                                        )
                                        .with_repository(self.context.repository.clone())
                                        .with_session(self.context.session.clone())
                                        .with_query(self.context.query.clone())
                                        .with_user(self.context.user.clone())
                                        .with_config(self.context.config.clone())
                                        .with_conf(self.context.conf.clone())
                                        .with_url(self.context.url.clone())
                                        .with_hostname(self.context.hostname.clone())
                                } else {
                                    // No model specified, use the same context
                                    self.context.clone()
                                };

                                let mut partial_renderer = Renderer::new(partial_context)
                                    .with_template_path(base_path.clone());

                                // Pass along the template loader if available
                                if let Some(loader) = &self.template_loader {
                                    partial_renderer =
                                        partial_renderer.with_template_loader(Arc::clone(loader));
                                }

                                // Extract sections and helpers
                                let mut partial_template = template;
                                partial_template.extract_sections();
                                partial_template.extract_helpers();

                                // Render the partial
                                return partial_renderer.render(&partial_template);
                            }
                        }
                    }

                    // If loading failed, return error comment
                    Ok(format!("<!-- Error loading view '{}' -->", name))
                } else {
                    // No template path set, can't load partials
                    Ok(format!(
                        "<!-- Cannot load view '{}' - no template path -->",
                        name
                    ))
                }
            }

            Node::Import(files) => {
                let mut output = String::new();
                for file in files {
                    if file.ends_with(".css") {
                        output.push_str(&format!(
                            "<link rel=\"stylesheet\" href=\"/css/{}\">\n",
                            file
                        ));
                    } else if file.ends_with(".js") {
                        output.push_str(&format!("<script src=\"/js/{}\"></script>\n", file));
                    }
                }
                Ok(output)
            }

            Node::Meta {
                title,
                description,
                keywords,
            } => {
                let mut output = String::new();

                if let Some(t) = title {
                    output.push_str(&format!("<title>{}</title>\n", HtmlEscaper::escape(t)));
                }
                if let Some(d) = description {
                    output.push_str(&format!(
                        "<meta name=\"description\" content=\"{}\">\n",
                        HtmlEscaper::escape_attribute(d)
                    ));
                }
                if let Some(k) = keywords {
                    output.push_str(&format!(
                        "<meta name=\"keywords\" content=\"{}\">\n",
                        HtmlEscaper::escape_attribute(k)
                    ));
                }

                Ok(output)
            }

            Node::Body => {
                // @{body} is the standard Total.js placeholder for layout content
                // The rendered template content is stored in the "content" field when layouts are applied
                if let Value::Object(map) = &self.context.data {
                    if let Some(Value::String(content)) = map.get("content") {
                        Ok(content.clone())
                    } else {
                        Ok(String::new())
                    }
                } else {
                    Ok(String::new())
                }
            }

            Node::Head => {
                // @{head} is for additional head content
                // This would be populated from the context or sections
                if let Some(head_section) = self.context.sections.get("head") {
                    let head_nodes = head_section.clone();
                    self.render_nodes(&head_nodes)
                } else {
                    Ok(String::new())
                }
            }

            Node::Content => {
                // @{content} is our extension to Total.js for backward compatibility
                // Standard Total.js uses @{body} for layout content insertion
                // Both work identically - they retrieve the rendered template from the "content" field
                if let Value::Object(map) = &self.context.data {
                    if let Some(Value::String(content)) = map.get("content") {
                        Ok(content.clone())
                    } else {
                        Ok(String::new())
                    }
                } else {
                    Ok(String::new())
                }
            }

            Node::Csrf => {
                // @{csrf} - renders hidden input with default token
                let token_id = "_csrf_token";
                let token = if let Value::Object(session_map) = &self.context.session {
                    session_map
                        .get(token_id)
                        .and_then(|data| data.get("token"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                } else {
                    None
                };

                if let Some(token) = token {
                    Ok(format!(
                        "<input type=\"hidden\" name=\"{}\" value=\"{}\">",
                        token_id,
                        HtmlEscaper::escape_attribute(&token)
                    ))
                } else {
                    Ok(String::new())
                }
            }

            Node::Translate { text, is_key } => {
                if let Some(translator) = &self.context.translator {
                    if *is_key {
                        // Translate by key
                        Ok(translator.translate_key(text))
                    } else {
                        // Translate text
                        Ok(translator.translate_text(text))
                    }
                } else {
                    // No translator available, return as-is
                    if *is_key {
                        Ok(format!("[#{}]", text))
                    } else {
                        Ok(text.clone())
                    }
                }
            }

            Node::Config(key) => Ok(self.context.config.get(key).cloned().unwrap_or_default()),

            Node::Repository(key) => Ok(self
                .context
                .get_nested_value(&self.context.repository, key)
                .map(|v| self.context.value_to_string(&v, true))
                .unwrap_or_default()),

            Node::Session(key) => Ok(self
                .context
                .get_nested_value(&self.context.session, key)
                .map(|v| self.context.value_to_string(&v, true))
                .unwrap_or_default()),

            Node::Query(key) => Ok(self
                .context
                .get_nested_value(&self.context.query, key)
                .map(|v| self.context.value_to_string(&v, true))
                .unwrap_or_default()),

            Node::User(prop) => {
                if let Some(p) = prop {
                    Ok(self
                        .context
                        .get_nested_value(&self.context.user, p)
                        .map(|v| self.context.value_to_string(&v, true))
                        .unwrap_or_default())
                } else {
                    Ok(self.context.value_to_string(&self.context.user, true))
                }
            }

            Node::App(key) => {
                // Access global repository via APP.key
                Ok(self
                    .context
                    .get_nested_value(&self.context.global_repository, key)
                    .map(|v| self.context.value_to_string(&v, true))
                    .unwrap_or_default())
            }

            Node::Main(key) => {
                // Access global repository via MAIN.key (alias for APP)
                Ok(self
                    .context
                    .get_nested_value(&self.context.global_repository, key)
                    .map(|v| self.context.value_to_string(&v, true))
                    .unwrap_or_default())
            }

            Node::R(key) => {
                // Access context repository via R.key (alias for repository)
                Ok(self
                    .context
                    .get_nested_value(&self.context.repository, key)
                    .map(|v| self.context.value_to_string(&v, true))
                    .unwrap_or_default())
            }

            Node::Model(key) => {
                // Access model data via model.key
                Ok(self
                    .context
                    .get_nested_value(&self.context.data, key)
                    .map(|v| self.context.value_to_string(&v, true))
                    .unwrap_or_default())
            }

            Node::M(key) => {
                // Access model data via M.key (alias for model/data)
                Ok(self
                    .context
                    .get_nested_value(&self.context.data, key)
                    .map(|v| self.context.value_to_string(&v, true))
                    .unwrap_or_default())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_render_text() {
        let template = Template {
            nodes: vec![Node::Text("Hello World".to_string())],
            sections: HashMap::new(),
            helpers: HashMap::new(),
        };

        let context = RenderContext::new(json!({}));
        let mut renderer = Renderer::new(context);
        let result = renderer.render(&template).unwrap();

        assert_eq!(result, "Hello World");
    }

    #[test]
    fn test_render_variable() {
        let template = Template {
            nodes: vec![
                Node::Text("Hello ".to_string()),
                Node::Variable {
                    name: "M.name".to_string(),
                    raw: false,
                    expression: None,
                },
                Node::Text("!".to_string()),
            ],
            sections: HashMap::new(),
            helpers: HashMap::new(),
        };

        let context = RenderContext::new(json!({ "name": "Alice" }));
        let mut renderer = Renderer::new(context);
        let result = renderer.render(&template).unwrap();

        assert_eq!(result, "Hello Alice!");
    }

    #[test]
    fn test_render_conditional() {
        let template = Template {
            nodes: vec![Node::Conditional {
                condition: Expression::from_path("M.isActive"),
                then_branch: vec![Node::Text("Active".to_string())],
                else_if_branches: vec![],
                else_branch: Some(vec![Node::Text("Inactive".to_string())]),
            }],
            sections: HashMap::new(),
            helpers: HashMap::new(),
        };

        // Test true condition
        let context = RenderContext::new(json!({ "isActive": true }));
        let mut renderer = Renderer::new(context);
        let result = renderer.render(&template).unwrap();
        assert_eq!(result, "Active");

        // Test false condition
        let context = RenderContext::new(json!({ "isActive": false }));
        let mut renderer = Renderer::new(context);
        let result = renderer.render(&template).unwrap();
        assert_eq!(result, "Inactive");
    }

    #[test]
    fn test_render_loop() {
        let template = Template {
            nodes: vec![Node::Loop {
                item_name: "item".to_string(),
                collection: Expression::from_path("M.items"),
                body: vec![
                    Node::Variable {
                        name: "item".to_string(),
                        raw: false,
                        expression: None,
                    },
                    Node::Text(" ".to_string()),
                ],
            }],
            sections: HashMap::new(),
            helpers: HashMap::new(),
        };

        let context = RenderContext::new(json!({
            "items": ["A", "B", "C"]
        }));
        let mut renderer = Renderer::new(context);
        let result = renderer.render(&template).unwrap();

        assert_eq!(result, "A B C ");
    }

    #[test]
    fn test_html_escaping() {
        let template = Template {
            nodes: vec![
                Node::Variable {
                    name: "M.html".to_string(),
                    raw: false,
                    expression: None,
                },
                Node::Text(" ".to_string()),
                Node::Variable {
                    name: "M.html".to_string(),
                    raw: true,
                    expression: None,
                },
            ],
            sections: HashMap::new(),
            helpers: HashMap::new(),
        };

        let context = RenderContext::new(json!({
            "html": "<script>alert('xss')</script>"
        }));
        let mut renderer = Renderer::new(context);
        let result = renderer.render(&template).unwrap();

        assert!(result.contains("&lt;script&gt;"));
        assert!(result.contains("<script>"));
    }

    #[test]
    fn test_range_function() {
        // Test range(stop)
        let template = Template {
            nodes: vec![Node::Loop {
                item_name: "num".to_string(),
                collection: Expression::FunctionCall {
                    name: "range".to_string(),
                    args: vec![Expression::Number(5.0)],
                },
                body: vec![
                    Node::Variable {
                        name: "num".to_string(),
                        raw: false,
                        expression: None,
                    },
                    Node::Text(",".to_string()),
                ],
            }],
            sections: HashMap::new(),
            helpers: HashMap::new(),
        };

        let context = RenderContext::new(json!({}));
        let mut renderer = Renderer::new(context);
        let result = renderer.render(&template).unwrap();
        assert_eq!(result, "0,1,2,3,4,");

        // Test range(start, stop)
        let template2 = Template {
            nodes: vec![Node::Loop {
                item_name: "num".to_string(),
                collection: Expression::FunctionCall {
                    name: "range".to_string(),
                    args: vec![Expression::Number(1.0), Expression::Number(6.0)],
                },
                body: vec![
                    Node::Variable {
                        name: "num".to_string(),
                        raw: false,
                        expression: None,
                    },
                    Node::Text(" ".to_string()),
                ],
            }],
            sections: HashMap::new(),
            helpers: HashMap::new(),
        };

        let context2 = RenderContext::new(json!({}));
        let mut renderer2 = Renderer::new(context2);
        let result2 = renderer2.render(&template2).unwrap();
        assert_eq!(result2, "1 2 3 4 5 ");

        // Test range(start, stop, step)
        let template3 = Template {
            nodes: vec![Node::Loop {
                item_name: "num".to_string(),
                collection: Expression::FunctionCall {
                    name: "range".to_string(),
                    args: vec![
                        Expression::Number(0.0),
                        Expression::Number(10.0),
                        Expression::Number(2.0),
                    ],
                },
                body: vec![
                    Node::Variable {
                        name: "num".to_string(),
                        raw: false,
                        expression: None,
                    },
                    Node::Text("-".to_string()),
                ],
            }],
            sections: HashMap::new(),
            helpers: HashMap::new(),
        };

        let context3 = RenderContext::new(json!({}));
        let mut renderer3 = Renderer::new(context3);
        let result3 = renderer3.render(&template3).unwrap();
        assert_eq!(result3, "0-2-4-6-8-");
    }

    #[test]
    fn test_ternary_operator() {
        // Test simple ternary with true condition
        let template = Template {
            nodes: vec![Node::Variable {
                name: "true ? 'yes' : 'no'".to_string(),
                raw: false,
                expression: Some(Expression::Ternary {
                    condition: Box::new(Expression::Boolean(true)),
                    then_expr: Box::new(Expression::String("yes".to_string())),
                    else_expr: Box::new(Expression::String("no".to_string())),
                }),
            }],
            sections: HashMap::new(),
            helpers: HashMap::new(),
        };

        let context = RenderContext::new(json!({}));
        let mut renderer = Renderer::new(context);
        let result = renderer.render(&template).unwrap();
        assert_eq!(result, "yes");

        // Test simple ternary with false condition
        let template2 = Template {
            nodes: vec![Node::Variable {
                name: "false ? 'yes' : 'no'".to_string(),
                raw: false,
                expression: Some(Expression::Ternary {
                    condition: Box::new(Expression::Boolean(false)),
                    then_expr: Box::new(Expression::String("yes".to_string())),
                    else_expr: Box::new(Expression::String("no".to_string())),
                }),
            }],
            sections: HashMap::new(),
            helpers: HashMap::new(),
        };

        let context2 = RenderContext::new(json!({}));
        let mut renderer2 = Renderer::new(context2);
        let result2 = renderer2.render(&template2).unwrap();
        assert_eq!(result2, "no");
    }

    #[test]
    fn test_ternary_with_variables() {
        // Test ternary with variable condition
        let template = Template {
            nodes: vec![Node::Variable {
                name: "M.isActive ? M.name : 'Guest'".to_string(),
                raw: false,
                expression: Some(Expression::Ternary {
                    condition: Box::new(Expression::PropertyAccess {
                        object: Box::new(Expression::Variable("M".to_string())),
                        property: "isActive".to_string(),
                    }),
                    then_expr: Box::new(Expression::PropertyAccess {
                        object: Box::new(Expression::Variable("M".to_string())),
                        property: "name".to_string(),
                    }),
                    else_expr: Box::new(Expression::String("Guest".to_string())),
                }),
            }],
            sections: HashMap::new(),
            helpers: HashMap::new(),
        };

        // Test with active user
        let context = RenderContext::new(json!({
            "isActive": true,
            "name": "Alice"
        }));
        let mut renderer = Renderer::new(context);
        let result = renderer.render(&template).unwrap();
        assert_eq!(result, "Alice");

        // Test with inactive user
        let context2 = RenderContext::new(json!({
            "isActive": false,
            "name": "Bob"
        }));
        let mut renderer2 = Renderer::new(context2);
        let result2 = renderer2.render(&template).unwrap();
        assert_eq!(result2, "Guest");
    }

    #[test]
    fn test_ternary_with_expressions() {
        // Test ternary with comparison expression
        let template = Template {
            nodes: vec![Node::Variable {
                name: "(M.a > M.b) ? 'greater' : 'lesser'".to_string(),
                raw: false,
                expression: Some(Expression::Ternary {
                    condition: Box::new(Expression::BinaryOp {
                        left: Box::new(Expression::PropertyAccess {
                            object: Box::new(Expression::Variable("M".to_string())),
                            property: "a".to_string(),
                        }),
                        op: BinaryOperator::GreaterThan,
                        right: Box::new(Expression::PropertyAccess {
                            object: Box::new(Expression::Variable("M".to_string())),
                            property: "b".to_string(),
                        }),
                    }),
                    then_expr: Box::new(Expression::String("greater".to_string())),
                    else_expr: Box::new(Expression::String("lesser".to_string())),
                }),
            }],
            sections: HashMap::new(),
            helpers: HashMap::new(),
        };

        // Test with a > b
        let context = RenderContext::new(json!({
            "a": 10,
            "b": 5
        }));
        let mut renderer = Renderer::new(context);
        let result = renderer.render(&template).unwrap();
        assert_eq!(result, "greater");

        // Test with a < b
        let context2 = RenderContext::new(json!({
            "a": 3,
            "b": 7
        }));
        let mut renderer2 = Renderer::new(context2);
        let result2 = renderer2.render(&template).unwrap();
        assert_eq!(result2, "lesser");
    }
}
