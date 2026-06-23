use crate::error::{Error, Result};
use crate::http::files::{FileCollection, MultipartParser};
use http_body_util::BodyExt;
use hyper::Request as HyperRequest;
use ipnetwork::IpNetwork;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use simd_json;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::sync::Arc;
use url::Url;

/// Represents form data that can be either a single value or an array of values
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FormValue {
    Single(String),
    Multiple(Vec<String>),
}

impl FormValue {
    /// Get as a single value (returns first element if array)
    pub fn as_string(&self) -> &str {
        match self {
            FormValue::Single(s) => s,
            FormValue::Multiple(v) => v.first().map(|s| s.as_str()).unwrap_or(""),
        }
    }

    /// Get as array (wraps single value in array if needed)
    pub fn as_array(&self) -> Vec<&str> {
        match self {
            FormValue::Single(s) => vec![s.as_str()],
            FormValue::Multiple(v) => v.iter().map(|s| s.as_str()).collect(),
        }
    }

    /// Check if this is an array value
    pub fn is_array(&self) -> bool {
        matches!(self, FormValue::Multiple(_))
    }
}

/// Parsed form data with typed getters (e.g. `get_int`, `get_str`).
///
/// Returned by [`Context::body_form`](crate::Context::body_form). Implements
/// `Deref<Target = HashMap<String, String>>` so you can use `.get("key")` and
/// iteration as with a plain map.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(transparent)]
pub struct FormData(HashMap<String, String>);

impl std::ops::Deref for FormData {
    type Target = HashMap<String, String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FormData {
    /// Create from a parsed form map (used by `Context::body_form`).
    pub fn new(map: HashMap<String, String>) -> Self {
        Self(map)
    }

    /// Get a required string field (errors if missing or empty).
    pub fn get_str(&self, key: &str) -> Result<String> {
        self.0
            .get(key)
            .filter(|s| !s.is_empty())
            .cloned()
            .ok_or_else(|| Error::InvalidInput(format!("Field '{}' is required", key)))
    }

    /// Get a required integer field (errors if missing or invalid).
    pub fn get_int(&self, key: &str) -> Result<i32> {
        self.get_str(key)?
            .parse()
            .map_err(|_| Error::InvalidInput(format!("Field '{}' must be a valid integer", key)))
    }

    /// Get a required boolean field (errors if missing). Accepts "true", "1", "yes", "on", "checked".
    pub fn get_bool(&self, key: &str) -> Result<bool> {
        let value = self.get_str(key)?;
        Ok(matches!(
            value.as_str(),
            "true" | "1" | "yes" | "on" | "checked"
        ))
    }

    /// Get a string field, or a default if missing or empty.
    pub fn get_str_or(&self, key: &str, default: &str) -> String {
        self.0
            .get(key)
            .filter(|s| !s.is_empty())
            .cloned()
            .unwrap_or_else(|| default.to_string())
    }

    /// Get an integer field, or a default if missing or invalid.
    pub fn get_int_or(&self, key: &str, default: i32) -> i32 {
        self.get_str(key)
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(default)
    }

    /// Get a boolean field, or a default if missing. Accepts "true", "1", "yes", "on", "checked".
    pub fn get_bool_or(&self, key: &str, default: bool) -> bool {
        self.get_str(key)
            .map(|s| matches!(s.as_str(), "true" | "1" | "yes" | "on" | "checked"))
            .unwrap_or(default)
    }
}

pub struct Request {
    pub method: String,
    pub uri: String,
    pub headers: HashMap<String, String>,
    pub params: HashMap<String, String>,
    pub query: HashMap<String, String>,
    body_bytes: Vec<u8>,
    files: Option<FileCollection>,
    /// Cached multipart form data (parsed alongside files)
    multipart_form_data: Option<HashMap<String, String>>,
    /// Lazily-populated cache of the parsed `Cookie` header.
    ///
    /// Session, CSRF, and flash-message middleware all read cookies from the
    /// same request; without this cache each would re-tokenise the header
    /// and allocate a fresh `HashMap`.
    cookies_cache: once_cell::sync::OnceCell<HashMap<String, String>>,
    peer_addr: Option<SocketAddr>,
    trusted_proxies: Arc<Vec<IpNetwork>>,
}

impl Default for Request {
    fn default() -> Self {
        Self {
            method: String::new(),
            uri: String::new(),
            headers: HashMap::new(),
            params: HashMap::new(),
            query: HashMap::new(),
            body_bytes: Vec::new(),
            files: None,
            multipart_form_data: None,
            cookies_cache: once_cell::sync::OnceCell::new(),
            peer_addr: None,
            trusted_proxies: Arc::new(Vec::new()),
        }
    }
}

impl Request {
    /// Create a new Request for testing purposes
    /// This is only intended for testing and should not be used in production code
    #[doc(hidden)]
    pub fn new(method: &str, uri: &str, _version: &str) -> Self {
        Request {
            method: method.to_string(),
            uri: uri.to_string(),
            headers: HashMap::new(),
            params: HashMap::new(),
            query: HashMap::new(),
            body_bytes: Vec::new(),
            files: None,
            multipart_form_data: None,
            cookies_cache: once_cell::sync::OnceCell::new(),
            peer_addr: None,
            trusted_proxies: Arc::new(Vec::new()),
        }
    }

    /// Set body for testing purposes
    #[doc(hidden)]
    #[cfg(test)]
    pub fn set_body(&mut self, body: Vec<u8>) {
        self.body_bytes = body;
    }

    /// Build a `Request` from any hyper body. Production passes
    /// `hyper::body::Incoming` (network); tests/benches pass an in-memory body
    /// like `http_body_util::Full<Bytes>` — both collect the same way.
    pub async fn from_hyper<B>(req: HyperRequest<B>) -> Result<Self>
    where
        B: hyper::body::Body,
        B::Error: std::fmt::Display,
    {
        Self::from_hyper_with_connection(req, None, Arc::new(Vec::new())).await
    }

    /// Build a `Request` from Hyper plus connection metadata.
    pub async fn from_hyper_with_connection<B>(
        req: HyperRequest<B>,
        peer_addr: Option<SocketAddr>,
        trusted_proxies: Arc<Vec<IpNetwork>>,
    ) -> Result<Self>
    where
        B: hyper::body::Body,
        B::Error: std::fmt::Display,
    {
        let method = req.method().to_string();
        let uri = req.uri().to_string();

        // Extract headers
        let mut headers = HashMap::new();
        for (name, value) in req.headers() {
            if let Ok(value_str) = value.to_str() {
                headers.insert(name.to_string(), value_str.to_string());
            }
        }

        // Extract query parameters
        let query = Self::parse_query(req.uri().query().unwrap_or(""));

        let content_length = req
            .headers()
            .get("content-length")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(0);
        let has_transfer_encoding = req.headers().contains_key("transfer-encoding");
        let body_hint = req.body().size_hint();
        let has_body = content_length > 0 || has_transfer_encoding || body_hint.lower() > 0;

        // Read body (hyper 1.x: collect the body into bytes). Generic over the
        // body type, so map its error into our own rather than relying on the
        // `From<hyper::Error>` impl (which only covers `Incoming`).
        let body_bytes = if has_body {
            req.into_body()
                .collect()
                .await
                .map_err(|e| Error::Internal(format!("failed to read request body: {e}")))?
                .to_bytes()
                .to_vec()
        } else {
            Vec::new()
        };

        Ok(Request {
            method,
            uri,
            headers,
            params: HashMap::new(), // Will be filled by router
            query,
            body_bytes,
            files: None,               // Will be parsed on demand
            multipart_form_data: None, // Will be parsed on demand
            cookies_cache: once_cell::sync::OnceCell::new(),
            peer_addr,
            trusted_proxies,
        })
    }

    pub fn body_as_json<T: DeserializeOwned>(&self) -> Result<T> {
        // Use simd-json for faster parsing (2-3x faster than serde_json)
        let mut body_bytes = self.body_bytes.clone();
        simd_json::from_slice(&mut body_bytes)
            .map_err(|e| Error::internal(format!("Failed to parse JSON: {}", e)))
    }

    /// Parse request body as form (application/x-www-form-urlencoded or multipart/form-data).
    /// For multipart requests, parses on first call and uses cached fields so fields like `oldid` are present.
    pub fn body_as_form(&mut self) -> Result<HashMap<String, String>> {
        // Use cached multipart form fields if already parsed (e.g. by files())
        if let Some(ref form) = self.multipart_form_data {
            return Ok(form.clone());
        }
        // Parse multipart on first use so body_form() sees all fields
        if self
            .headers
            .get("content-type")
            .map(|v| v.starts_with("multipart/form-data"))
            == Some(true)
        {
            self.parse_files()?;
            if let Some(ref form) = self.multipart_form_data {
                return Ok(form.clone());
            }
        }
        let body_str = String::from_utf8_lossy(&self.body_bytes);
        Ok(Self::parse_query(&body_str))
    }

    /// Parse form data with support for arrays (field[] syntax)
    pub fn body_as_form_data(&mut self) -> Result<HashMap<String, FormValue>> {
        if let Some(ref form) = self.multipart_form_data {
            return Ok(form
                .iter()
                .map(|(key, value)| (key.clone(), FormValue::Single(value.clone())))
                .collect());
        }

        if self
            .headers
            .get("content-type")
            .map(|v| v.starts_with("multipart/form-data"))
            == Some(true)
        {
            self.parse_files()?;
            if let Some(ref form) = self.multipart_form_data {
                return Ok(form
                    .iter()
                    .map(|(key, value)| (key.clone(), FormValue::Single(value.clone())))
                    .collect());
            }
        }

        // Optimize: Check if body is valid UTF-8 to avoid unnecessary allocation
        let body_str = match std::str::from_utf8(&self.body_bytes) {
            Ok(s) => s,
            Err(_) => {
                return Ok(Self::parse_query_with_arrays(&String::from_utf8_lossy(
                    &self.body_bytes,
                )))
            }
        };
        Ok(Self::parse_query_with_arrays(body_str))
    }

    pub fn body_as_string(&self) -> String {
        String::from_utf8_lossy(&self.body_bytes).to_string()
    }

    fn parse_query(query: &str) -> HashMap<String, String> {
        let mut result = HashMap::new();
        for pair in query.split('&') {
            if pair.is_empty() {
                continue;
            }

            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            if key.is_empty() {
                continue;
            }

            if let (Some(decoded_key), Some(decoded_value)) =
                (urlencoding::decode(key), urlencoding::decode(value))
            {
                if !decoded_key.is_empty() {
                    result.insert(decoded_key.to_string(), decoded_value.to_string());
                }
            }
        }
        result
    }

    /// Parse query/form data with support for arrays
    /// Optimized single-pass parsing to avoid double iteration
    fn parse_query_with_arrays(query: &str) -> HashMap<String, FormValue> {
        let mut result: HashMap<String, FormValue> = HashMap::new();

        for pair in query.split('&') {
            if pair.is_empty() {
                continue;
            }

            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            if key.is_empty() {
                continue;
            }

            // Use safe decoding that filters malicious input
            if let (Some(decoded_key), Some(decoded_value)) =
                (urlencoding::decode(key), urlencoding::decode(value))
            {
                if decoded_key.is_empty() {
                    continue;
                }

                // Check if key ends with [] (array notation) and extract actual key
                let (actual_key, is_array) = if decoded_key.ends_with("[]") {
                    let key_len = decoded_key.len();
                    (decoded_key[..key_len - 2].to_string(), true)
                } else {
                    (decoded_key.to_string(), false)
                };

                if actual_key.is_empty() {
                    continue;
                }

                // Single-pass: directly create FormValue instead of intermediate Vec
                let decoded_value_str = decoded_value.to_string();

                match result.entry(actual_key) {
                    std::collections::hash_map::Entry::Vacant(e) => {
                        if is_array {
                            // Array notation - always create Multiple even for first value
                            e.insert(FormValue::Multiple(vec![decoded_value_str]));
                        } else {
                            // Single value
                            e.insert(FormValue::Single(decoded_value_str));
                        }
                    }
                    std::collections::hash_map::Entry::Occupied(mut e) => {
                        // Key already exists - convert to array if needed
                        match e.get_mut() {
                            FormValue::Single(existing) => {
                                // Convert single to multiple
                                let existing_value = std::mem::take(existing);
                                *e.get_mut() =
                                    FormValue::Multiple(vec![existing_value, decoded_value_str]);
                            }
                            FormValue::Multiple(vec) => {
                                // Add to existing array
                                vec.push(decoded_value_str);
                            }
                        }
                    }
                }
            }
            // Skip pairs with invalid encoding
        }

        result
    }

    // Client information helper methods

    /// Get client IP address (supports X-Forwarded-For and X-Real-IP)
    pub fn client_ip(&self) -> String {
        if self.should_trust_forwarded_headers() {
            if let Some(forwarded) = self.headers.get("x-forwarded-for") {
                let forwarded_ips: Vec<IpAddr> = forwarded
                    .split(',')
                    .filter_map(|ip| ip.trim().parse::<IpAddr>().ok())
                    .collect();
                if !forwarded_ips.is_empty() {
                    if let Some(ip) = forwarded_ips
                        .iter()
                        .rev()
                        .find(|ip| !self.is_trusted_proxy_ip(**ip))
                    {
                        return ip.to_string();
                    }

                    return forwarded_ips[0].to_string();
                }
            }

            if let Some(real_ip) = self.headers.get("x-real-ip") {
                if let Ok(real_ip) = real_ip.parse::<IpAddr>() {
                    return real_ip.to_string();
                }
            }
        }

        self.peer_addr
            .map(|addr| addr.ip().to_string())
            .unwrap_or_else(|| "127.0.0.1".to_string())
    }

    /// Get user agent string
    pub fn user_agent(&self) -> Option<&str> {
        self.headers.get("user-agent").map(|s| s.as_str())
    }

    /// Detect if request is from mobile device
    pub fn is_mobile(&self) -> bool {
        if let Some(ua) = self.user_agent() {
            let ua_lower = ua.to_lowercase();
            ua_lower.contains("mobile")
                || ua_lower.contains("android")
                || ua_lower.contains("iphone")
                || ua_lower.contains("ipad")
                || ua_lower.contains("blackberry")
                || ua_lower.contains("windows phone")
        } else {
            false
        }
    }

    /// Detect if request is from a bot/crawler
    pub fn is_robot(&self) -> bool {
        if let Some(ua) = self.user_agent() {
            let ua_lower = ua.to_lowercase();
            ua_lower.contains("bot")
                || ua_lower.contains("crawler")
                || ua_lower.contains("spider")
                || ua_lower.contains("scraper")
                || ua_lower.contains("googlebot")
                || ua_lower.contains("bingbot")
                || ua_lower.contains("facebookexternalhit")
                || ua_lower.contains("twitterbot")
        } else {
            false
        }
    }

    /// Check if request is HTTPS (via X-Forwarded-Proto or URI scheme)
    pub fn is_secure(&self) -> bool {
        if self.should_trust_forwarded_headers() {
            if let Some(proto) = self.headers.get("x-forwarded-proto") {
                return proto
                    .split(',')
                    .next()
                    .map(|value| value.trim().eq_ignore_ascii_case("https"))
                    .unwrap_or(false);
            }
        }

        // Check URI scheme
        self.uri.starts_with("https://")
    }

    /// Check if request is AJAX/XHR (supports traditional XHR and htmx)
    pub fn is_xhr(&self) -> bool {
        // Check for traditional XHR header (jQuery, Axios, etc.)
        if let Some(xhr_header) = self.headers.get("x-requested-with") {
            if xhr_header.to_lowercase() == "xmlhttprequest" {
                return true;
            }
        }

        // Check for htmx request header
        if let Some(hx_request) = self.headers.get("hx-request") {
            if hx_request.to_lowercase() == "true" {
                return true;
            }
        }

        false
    }

    /// Get preferred language from Accept-Language header
    pub fn language(&self) -> Option<&str> {
        if let Some(accept_lang) = self.headers.get("accept-language") {
            // Parse "en-US,en;q=0.9,fr;q=0.8" and return first language
            accept_lang
                .split(',')
                .next()
                .and_then(|lang| lang.split(';').next())
                .map(|lang| lang.trim())
        } else {
            None
        }
    }

    /// Get HTTP referrer
    pub fn referrer(&self) -> Option<&str> {
        self.headers.get("referer").map(|s| s.as_str())
    }

    // File upload handling methods

    /// Get uploaded files (Total.js: controller.files)
    pub fn files(&mut self) -> Result<&FileCollection> {
        if self.files.is_none() {
            self.parse_files()?;
        }
        Ok(self.files.as_ref().unwrap())
    }

    /// Get a specific uploaded file by field name
    pub fn file(&mut self, field_name: &str) -> Result<Option<&crate::http::files::UploadedFile>> {
        Ok(self.files()?.get(field_name))
    }

    /// Parse multipart form data to extract files
    /// Also caches form data to avoid re-parsing
    fn parse_files(&mut self) -> Result<()> {
        // Check if this is a multipart form
        if let Some(content_type) = self.headers.get("content-type") {
            if content_type.starts_with("multipart/form-data") {
                // Extract boundary
                if let Some(boundary) = self.extract_boundary(content_type) {
                    let (files, form_data) = MultipartParser::parse(&self.body_bytes, &boundary)?;

                    // Cache form data to avoid re-parsing in body_as_form_data
                    self.multipart_form_data = Some(form_data);
                    self.files = Some(files);
                    return Ok(());
                }

                return Err(Error::InvalidInput(
                    "Invalid multipart/form-data Content-Type: missing or malformed boundary"
                        .to_string(),
                ));
            }
        }

        // Not multipart, create empty file collection
        self.files = Some(FileCollection::new());
        self.multipart_form_data = None;
        Ok(())
    }

    /// Extract boundary from Content-Type header
    fn extract_boundary(&self, content_type: &str) -> Option<String> {
        // Parse: multipart/form-data; boundary=----WebKitFormBoundary7MA4YWxkTrZu0gW
        for part in content_type.split(';').skip(1) {
            let part = part.trim();
            if let Some((name, value)) = part.split_once('=') {
                if name.trim().eq_ignore_ascii_case("boundary") {
                    let boundary = value.trim().trim_matches('"');
                    if !boundary.is_empty()
                        && !boundary
                            .chars()
                            .any(|ch| ch.is_control() || ch.is_whitespace())
                    {
                        return Some(boundary.to_string());
                    }
                }
            }
        }
        None
    }

    // Total.js compatibility methods - Phase 1: High Priority Features

    /// Get cookie value by name (Total.js: request.cookie(name)).
    ///
    /// Parses the `Cookie` header once per request and caches the result, so
    /// session + flash + CSRF middleware reading different cookies on the
    /// same request all share one parse.
    pub fn cookie(&self, name: &str) -> Option<String> {
        self.cookies().get(name).cloned()
    }

    /// All cookies on this request, parsed lazily on first access and cached.
    pub fn cookies(&self) -> &HashMap<String, String> {
        self.cookies_cache.get_or_init(|| {
            self.headers
                .get("cookie")
                .map(|h| Self::parse_cookies(h))
                .unwrap_or_default()
        })
    }

    /// Get host from Host header (Total.js: request.host)
    pub fn host(&self) -> Option<&str> {
        self.headers.get("host").map(|s| s.as_str())
    }

    /// Get hostname with optional path (Total.js: request.hostname([path]))
    pub fn hostname(&self, path: Option<&str>) -> String {
        let origin = self
            .configured_origin()
            .unwrap_or_else(|| "http://localhost".to_string());

        if let Some(path) = path {
            let path = if path.starts_with('/') {
                path
            } else {
                &format!("/{}", path)
            };
            format!("{}{}", origin.trim_end_matches('/'), path)
        } else {
            origin
        }
    }

    /// Get request path from URI (Total.js: request.path)
    pub fn path(&self) -> &str {
        // Extract path from URI, handling both full URLs and paths
        if let Some(path_start) = self.uri.find("://") {
            // Full URL: extract path after domain
            let after_scheme = &self.uri[path_start + 3..];
            if let Some(path_start) = after_scheme.find('/') {
                let path_with_query = &after_scheme[path_start..];
                // Remove query string if present
                if let Some(query_start) = path_with_query.find('?') {
                    &path_with_query[..query_start]
                } else {
                    path_with_query
                }
            } else {
                "/"
            }
        } else {
            // Relative path: remove query string if present
            if let Some(query_start) = self.uri.find('?') {
                &self.uri[..query_start]
            } else {
                &self.uri
            }
        }
    }

    /// Get file extension from path (Total.js: request.extension)
    pub fn extension(&self) -> Option<&str> {
        let path = self.path();
        Path::new(path).extension().and_then(|ext| ext.to_str())
    }

    /// Check if request is authorized (Total.js: request.isAuthorized)
    pub fn is_authorized(&self) -> bool {
        self.headers.get("authorization").is_some()
    }

    /// Get authorization header (Total.js: request.authorization())
    pub fn authorization(&self) -> Option<&str> {
        self.headers.get("authorization").map(|s| s.as_str())
    }

    // Phase 2: Medium Priority Features

    /// Check if request is from a proxy (Total.js: request.isProxy)
    pub fn is_proxy(&self) -> bool {
        self.should_trust_forwarded_headers()
            && (self.headers.contains_key("x-forwarded-for")
                || self.headers.contains_key("x-real-ip")
                || self.headers.contains_key("x-forwarded-proto")
                || self.headers.contains_key("x-forwarded-host")
                || self.headers.contains_key("forwarded"))
    }

    /// Check if request is for a static file (Total.js: request.isStaticFile)
    pub fn is_static_file(&self) -> bool {
        if let Some(ext) = self.extension() {
            // Common static file extensions
            matches!(
                ext.to_lowercase().as_str(),
                "css"
                    | "js"
                    | "jpg"
                    | "jpeg"
                    | "png"
                    | "gif"
                    | "svg"
                    | "webp"
                    | "ico"
                    | "woff"
                    | "woff2"
                    | "ttf"
                    | "eot"
                    | "pdf"
                    | "zip"
                    | "txt"
                    | "xml"
                    | "json"
                    | "csv"
                    | "mp3"
                    | "mp4"
                    | "webm"
                    | "html"
                    | "htm"
                    | "map"
            )
        } else {
            false
        }
    }

    /// Get subdomain from host (Total.js: request.subdomain)
    pub fn subdomain(&self) -> Option<String> {
        if let Some(host) = self.host() {
            // Remove port if present
            let host = if let Some(port_pos) = host.find(':') {
                &host[..port_pos]
            } else {
                host
            };

            let parts: Vec<&str> = host.split('.').collect();
            if parts.len() > 2 {
                // Return everything before the last two parts (domain.tld)
                let subdomain_parts = &parts[..parts.len() - 2];
                if !subdomain_parts.is_empty() {
                    Some(subdomain_parts.join("."))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Get path segments as array (Total.js: request.split)
    pub fn split(&self) -> Vec<&str> {
        self.path()
            .trim_start_matches('/')
            .trim_end_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// Generate or retrieve CSRF token (Total.js: request.csrf())
    ///
    /// This generates a cryptographically secure CSRF token for the current request.
    /// In a full implementation, this would typically be stored in the session
    /// and validated against form submissions.
    pub fn csrf(&self) -> String {
        // Check if CSRF token already exists in headers (for validation)
        if let Some(existing_token) = self.headers.get("x-csrf-token") {
            return existing_token.clone();
        }

        // Generate new CSRF token
        Self::generate_csrf_token()
    }

    /// Generate a cryptographically secure CSRF token
    fn generate_csrf_token() -> String {
        use rand::{thread_rng, Rng};

        // Generate 32-byte random token and encode as base64
        let token_bytes: Vec<u8> = (0..32).map(|_| thread_rng().gen::<u8>()).collect();

        Self::base64_encode(&token_bytes)
    }

    // Helper methods

    /// Parse cookies from Cookie header
    fn parse_cookies(cookie_header: &str) -> HashMap<String, String> {
        let mut cookies = HashMap::new();

        for cookie_pair in cookie_header.split(';') {
            let cookie_pair = cookie_pair.trim();
            if let Some((name, value)) = cookie_pair.split_once('=') {
                let name = name.trim().to_string();
                let value = value.trim().to_string();
                cookies.insert(name, value);
            }
        }

        cookies
    }

    fn should_trust_forwarded_headers(&self) -> bool {
        self.peer_addr
            .map(|addr| self.is_trusted_proxy_ip(addr.ip()))
            .unwrap_or(false)
    }

    fn is_trusted_proxy_ip(&self, ip: IpAddr) -> bool {
        self.trusted_proxies
            .iter()
            .any(|network| network.contains(ip))
    }

    fn configured_origin(&self) -> Option<String> {
        if let Some(public_url) = crate::configuration::CONF::get_string("server.public_url") {
            if let Some(origin) = Self::normalize_origin(&public_url) {
                return Some(origin);
            }
        }

        let host = crate::configuration::CONF::get_string("server.host")?;
        let port = crate::configuration::CONF::get::<u16>("server.port")
            .unwrap_or(if self.is_secure() { 443 } else { 80 });
        let scheme = if self.is_secure()
            || crate::configuration::CONF::get_bool("server.ssl_enabled").unwrap_or(false)
        {
            "https"
        } else {
            "http"
        };

        let default_port = match scheme {
            "https" => 443,
            _ => 80,
        };
        let port_suffix = if port == default_port {
            String::new()
        } else {
            format!(":{}", port)
        };

        Some(format!("{}://{}{}", scheme, host, port_suffix))
    }

    fn normalize_origin(input: &str) -> Option<String> {
        let parsed = Url::parse(input).ok()?;
        let scheme = parsed.scheme();
        let host = parsed.host_str()?;
        let default_port = match scheme {
            "http" => Some(80),
            "https" => Some(443),
            _ => None,
        };
        let port_suffix = match parsed.port() {
            Some(port) if Some(port) != default_port => format!(":{}", port),
            _ => String::new(),
        };

        Some(format!("{}://{}{}", scheme, host, port_suffix))
    }

    /// Simple base64 encoding without external dependencies
    fn base64_encode(input: &[u8]) -> String {
        const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut result = String::new();

        for chunk in input.chunks(3) {
            let b1 = chunk[0] as u32;
            let b2 = chunk.get(1).copied().unwrap_or(0) as u32;
            let b3 = chunk.get(2).copied().unwrap_or(0) as u32;

            let combined = (b1 << 16) | (b2 << 8) | b3;

            result.push(CHARS[((combined >> 18) & 63) as usize] as char);
            result.push(CHARS[((combined >> 12) & 63) as usize] as char);
            result.push(if chunk.len() > 1 {
                CHARS[((combined >> 6) & 63) as usize] as char
            } else {
                '='
            });
            result.push(if chunk.len() > 2 {
                CHARS[(combined & 63) as usize] as char
            } else {
                '='
            });
        }

        result
    }

    /// Reset Request state for pool reuse
    ///
    /// Clears all fields without deallocating underlying storage
    /// to maximize reuse efficiency.
    pub fn reset(&mut self) {
        self.method.clear();
        self.uri.clear();
        self.headers.clear();
        self.params.clear();
        self.query.clear();
        self.body_bytes.clear();
        self.files = None;
        self.multipart_form_data = None;
        self.cookies_cache = once_cell::sync::OnceCell::new();

        // Shrink collections if they've grown too large
        // This prevents memory bloat from requests with large payloads
        const MAX_CAPACITY: usize = 1024;

        if self.headers.capacity() > MAX_CAPACITY {
            self.headers = HashMap::new();
        }
        if self.params.capacity() > MAX_CAPACITY {
            self.params = HashMap::new();
        }
        if self.query.capacity() > MAX_CAPACITY {
            self.query = HashMap::new();
        }
        if self.body_bytes.capacity() > MAX_CAPACITY * 1024 {
            // 1MB
            self.body_bytes = Vec::new();
        }
    }
}

// Secure URL encoding/decoding implementation
mod urlencoding {
    use std::borrow::Cow;

    /// Securely decode URL-encoded strings with validation
    pub fn decode(s: &str) -> Option<Cow<'_, str>> {
        decode_safe(s).ok()
    }

    /// Safe URL decoding with comprehensive validation
    pub fn decode_safe(input: &str) -> Result<Cow<'_, str>, UrlDecodeError> {
        // Input validation
        if input.len() > MAX_URL_LENGTH {
            return Err(UrlDecodeError::TooLong);
        }

        let mut result = Vec::new();
        let mut chars = input.char_indices();

        while let Some((i, ch)) = chars.next() {
            match ch {
                '%' => {
                    // Ensure we have at least 2 more characters
                    if i + 2 >= input.len() {
                        return Err(UrlDecodeError::InvalidEncoding);
                    }

                    // Get the next two characters
                    let hex_str = &input[i + 1..i + 3];

                    // Validate hex characters
                    if !hex_str.chars().all(|c| c.is_ascii_hexdigit()) {
                        return Err(UrlDecodeError::InvalidHex);
                    }

                    // Parse hex value
                    let hex_value =
                        u8::from_str_radix(hex_str, 16).map_err(|_| UrlDecodeError::InvalidHex)?;

                    result.push(hex_value);

                    // Skip the next two characters
                    chars.next();
                    chars.next();
                }
                '+' => {
                    // Convert + to space (application/x-www-form-urlencoded)
                    result.push(b' ');
                }
                c if c.is_ascii() && is_safe_url_char(c) => {
                    result.push(c as u8);
                }
                _ => {
                    // Invalid or unsafe character
                    return Err(UrlDecodeError::InvalidCharacter);
                }
            }
        }

        // Convert bytes to string, validating UTF-8
        match String::from_utf8(result) {
            Ok(decoded) => {
                if decoded
                    .chars()
                    .any(|c| c == '\0' || (c.is_control() && !matches!(c, '\n' | '\r' | '\t')))
                {
                    Err(UrlDecodeError::UnsafeByte)
                } else if decoded == input {
                    Ok(Cow::Borrowed(input))
                } else {
                    Ok(Cow::Owned(decoded))
                }
            }
            Err(_) => Err(UrlDecodeError::InvalidUtf8),
        }
    }

    const MAX_URL_LENGTH: usize = 8192; // 8KB limit

    #[derive(Debug)]
    pub enum UrlDecodeError {
        TooLong,
        InvalidEncoding,
        InvalidHex,
        UnsafeByte,
        InvalidCharacter,
        InvalidUtf8,
    }

    /// Check if a URL character is safe to include
    fn is_safe_url_char(c: char) -> bool {
        match c {
            // Unreserved characters
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '.' | '_' | '~' => true,
            // Reserved characters that are safe in query strings
            ':' | '/' | '?' | '#' | '[' | ']' | '@' => true,
            // Sub-delims
            '!' | '$' | '&' | '\'' | '(' | ')' | '*' | '+' | ',' | ';' | '=' => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_path_normalizes_origin_and_absolute_forms() {
        // HTTP/1 origin-form
        let h1 = Request::new("GET", "/api/status?x=1", "HTTP/1.1");
        assert_eq!(h1.path(), "/api/status");

        // HTTP/2 absolute-form (scheme + authority) — must yield the same path
        // so the router matches identically over h2 (regression: was 404 over h2).
        let h2 = Request::new("GET", "http://127.0.0.1:8000/api/status?x=1", "HTTP/2");
        assert_eq!(h2.path(), "/api/status");

        // absolute-form, no query, root path
        let root = Request::new("GET", "http://example.com/", "HTTP/2");
        assert_eq!(root.path(), "/");
    }

    #[test]
    fn test_secure_url_decoding() {
        use super::urlencoding::decode;

        // Test basic decoding
        assert_eq!(decode("hello%20world").unwrap(), "hello world");
        assert_eq!(decode("test+string").unwrap(), "test string");
        assert_eq!(decode("normal_string").unwrap(), "normal_string");
        assert_eq!(decode("caf%C3%A9").unwrap(), "café");

        // Decoding should be syntax-focused, not content-blocking.
        assert_eq!(decode("../etc/passwd").unwrap(), "../etc/passwd");
        assert_eq!(decode("%2e%2e%2fpasswd").unwrap(), "../passwd");
        assert_eq!(decode("%3cscript%3e").unwrap(), "<script>");

        // Null and control characters remain invalid.
        assert!(decode("%00null").is_none());

        // Test length limits
        let long_string = "a".repeat(10000);
        assert!(decode(&long_string).is_none());

        // Test invalid encoding
        assert!(decode("invalid%GG").is_none());
        assert!(decode("incomplete%2").is_none());

        // Test control character filtering
        assert!(decode("%01%02%03").is_none());
    }

    #[test]
    fn test_url_decode_error_types() {
        use super::urlencoding::{decode_safe, UrlDecodeError};

        // Test specific error types
        let long_input = "a".repeat(10000);
        assert!(matches!(
            decode_safe(&long_input),
            Err(UrlDecodeError::TooLong)
        ));

        assert!(matches!(
            decode_safe("caf%C3%A9"),
            Ok(value) if value.as_ref() == "café"
        ));
        assert!(matches!(
            decode_safe("invalid%GG"),
            Err(UrlDecodeError::InvalidHex)
        ));
        assert!(matches!(
            decode_safe("incomplete%2"),
            Err(UrlDecodeError::InvalidEncoding)
        ));

        assert!(matches!(
            decode_safe("%00test"),
            Err(UrlDecodeError::UnsafeByte)
        ));

        // Test unsafe byte with a different control character that's not in malicious patterns
        assert!(matches!(
            decode_safe("%01test"),
            Err(UrlDecodeError::UnsafeByte)
        ));
    }

    #[test]
    fn test_safe_query_parsing() {
        // Syntax-invalid values are skipped; content is preserved for later layers.
        let malicious_query = "key1=../passwd&key2=%00null&key3=normal";
        let parsed = Request::parse_query(malicious_query);

        assert_eq!(parsed.get("key1"), Some(&"../passwd".to_string()));
        assert_eq!(parsed.get("key3"), Some(&"normal".to_string()));
        assert!(!parsed.contains_key("key2"));
    }

    #[test]
    fn test_parse_query_function() {
        let query = Request::parse_query("param=value&other=test&flag");

        assert_eq!(query.get("param"), Some(&"value".to_string()));
        assert_eq!(query.get("other"), Some(&"test".to_string()));
        assert_eq!(query.get("flag"), Some(&"".to_string()));
    }

    #[test]
    fn test_parse_query_skips_invalid_or_empty_keys() {
        let query = Request::parse_query("=bad&%00evil=value&ok=1");

        assert_eq!(query.get("ok"), Some(&"1".to_string()));
        assert!(!query.contains_key(""));
        assert_eq!(query.len(), 1);
    }

    #[test]
    fn test_form_data_getters() {
        let mut map = HashMap::new();
        map.insert("name".to_string(), "Alice".to_string());
        map.insert("age".to_string(), "30".to_string());
        map.insert("active".to_string(), "true".to_string());
        map.insert("empty".to_string(), "".to_string());
        let form = FormData::new(map);

        assert_eq!(form.get_str("name").unwrap(), "Alice");
        assert_eq!(form.get_int("age").unwrap(), 30);
        assert!(form.get_bool("active").unwrap());

        assert!(form.get_str("missing").is_err());
        assert!(form.get_str("empty").is_err());
        assert!(form.get_int("name").is_err());

        assert_eq!(form.get_str_or("name", "default"), "Alice");
        assert_eq!(form.get_str_or("missing", "default"), "default");
        assert_eq!(form.get_str_or("empty", "default"), "default");
        assert_eq!(form.get_int_or("age", 0), 30);
        assert_eq!(form.get_int_or("missing", 42), 42);
        assert_eq!(form.get_bool_or("active", false), true);
        assert_eq!(form.get_bool_or("missing", true), true);
    }

    #[test]
    fn test_form_data_deref() {
        let mut map = HashMap::new();
        map.insert("k".to_string(), "v".to_string());
        let form = FormData::new(map);
        assert_eq!(form.get("k"), Some(&"v".to_string()));
    }

    #[test]
    fn test_form_data_default() {
        let form = FormData::default();
        assert!(form.is_empty());
    }

    #[test]
    fn test_form_arrays() {
        // Test array notation with []
        let query = Request::parse_query_with_arrays("tags[]=rust&tags[]=web&tags[]=framework");

        assert_eq!(query.len(), 1);
        let tags_value = query.get("tags").unwrap();
        assert!(tags_value.is_array());
        assert_eq!(tags_value.as_array(), vec!["rust", "web", "framework"]);

        // Test mixed array and single values
        let query =
            Request::parse_query_with_arrays("name=John&hobbies[]=coding&hobbies[]=reading&age=30");

        assert_eq!(query.len(), 3);

        let name = query.get("name").unwrap();
        assert!(!name.is_array());
        assert_eq!(name.as_string(), "John");

        let hobbies = query.get("hobbies").unwrap();
        assert!(hobbies.is_array());
        assert_eq!(hobbies.as_array(), vec!["coding", "reading"]);

        let age = query.get("age").unwrap();
        assert!(!age.is_array());
        assert_eq!(age.as_string(), "30");
    }

    #[test]
    fn test_form_value_conversions() {
        // Test single value
        let single = FormValue::Single("test".to_string());
        assert_eq!(single.as_string(), "test");
        assert_eq!(single.as_array(), vec!["test"]);
        assert!(!single.is_array());

        // Test multiple values
        let multiple = FormValue::Multiple(vec![
            "one".to_string(),
            "two".to_string(),
            "three".to_string(),
        ]);
        assert_eq!(multiple.as_string(), "one"); // Gets first element
        assert_eq!(multiple.as_array(), vec!["one", "two", "three"]);
        assert!(multiple.is_array());

        // Test empty array
        let empty_multiple = FormValue::Multiple(vec![]);
        assert_eq!(empty_multiple.as_string(), "");
        assert_eq!(empty_multiple.as_array(), Vec::<&str>::new());
        assert!(empty_multiple.is_array());
    }

    #[test]
    fn test_form_parsing_with_encoding() {
        // Test URL encoded values
        let query = Request::parse_query_with_arrays(
            "message=Hello%20World&tags[]=rust%2Blang&tags[]=web%20dev",
        );

        let message = query.get("message").unwrap();
        assert_eq!(message.as_string(), "Hello World");

        let tags = query.get("tags").unwrap();
        assert_eq!(tags.as_array(), vec!["rust+lang", "web dev"]);
    }

    #[test]
    fn test_multipart_form_data_uses_cached_fields() {
        let mut request = Request::default();
        request.headers.insert(
            "content-type".to_string(),
            "multipart/form-data; boundary=boundary123".to_string(),
        );
        request.set_body(
            b"--boundary123\r\n\
Content-Disposition: form-data; name=\"title\"\r\n\r\n\
Hello multipart\r\n\
--boundary123\r\n\
Content-Disposition: form-data; name=\"upload\"; filename=\"test.txt\"\r\n\
Content-Type: text/plain\r\n\r\n\
file contents\r\n\
--boundary123--\r\n"
                .to_vec(),
        );

        let form = request.body_as_form_data().unwrap();
        assert_eq!(
            form.get("title").map(FormValue::as_string),
            Some("Hello multipart")
        );
        assert!(form.get("upload").is_none());
    }

    #[test]
    fn test_extract_boundary_supports_quoted_values() {
        let request = Request::default();
        let boundary = request.extract_boundary("multipart/form-data; boundary=\"abc123\"");

        assert_eq!(boundary.as_deref(), Some("abc123"));
    }

    #[test]
    fn test_parse_files_errors_on_missing_boundary() {
        let mut request = Request::default();
        request.headers.insert(
            "content-type".to_string(),
            "multipart/form-data".to_string(),
        );
        request.set_body(b"--ignored--".to_vec());

        let err = request.files().unwrap_err();
        assert!(matches!(err, Error::InvalidInput(_)));
    }

    #[test]
    fn test_form_parsing_malicious_input() {
        // Query parsing preserves content and only rejects invalid encodings/control chars.
        let query = Request::parse_query("safe=value&bad=../etc/passwd&normal=test");

        assert_eq!(query.get("safe"), Some(&"value".to_string()));
        assert_eq!(query.get("bad"), Some(&"../etc/passwd".to_string()));
        assert_eq!(query.get("normal"), Some(&"test".to_string()));
    }

    #[test]
    fn test_form_arrays_edge_cases() {
        // Test single value that becomes array when more values added
        let query = Request::parse_query_with_arrays("item=first&item=second&item=third");

        let items = query.get("item").unwrap();
        assert!(items.is_array());
        assert_eq!(items.as_array(), vec!["first", "second", "third"]);

        // Test empty brackets
        let query = Request::parse_query_with_arrays("empty[]=&empty[]=&filled[]=value");

        let empty = query.get("empty").unwrap();
        assert!(empty.is_array());
        assert_eq!(empty.as_array(), vec!["", ""]);

        let filled = query.get("filled").unwrap();
        assert_eq!(filled.as_string(), "value");
    }

    // Tests for new Total.js compatibility features

    #[test]
    fn test_cookie_parsing() {
        let mut request = Request::default();
        request.headers.insert(
            "cookie".to_string(),
            "session=abc123; user=john; theme=dark".to_string(),
        );

        assert_eq!(request.cookie("session"), Some("abc123".to_string()));
        assert_eq!(request.cookie("user"), Some("john".to_string()));
        assert_eq!(request.cookie("theme"), Some("dark".to_string()));
        assert_eq!(request.cookie("nonexistent"), None);

        // Test with no cookies
        let empty_request = Request::default();
        assert_eq!(empty_request.cookie("any"), None);
    }

    #[test]
    fn test_cookies_cache_returns_stable_reference() {
        // Two calls to cookies() must return the exact same HashMap —
        // proves the lazy cache isn't re-parsing per call.
        let mut request = Request::default();
        request
            .headers
            .insert("cookie".to_string(), "a=1; b=2; c=3".to_string());

        let first = request.cookies() as *const _;
        let second = request.cookies() as *const _;
        assert_eq!(
            first, second,
            "cookies() must cache — got different HashMap pointers"
        );

        // And multiple cookie(name) calls go through the same cache.
        assert_eq!(request.cookie("a"), Some("1".to_string()));
        assert_eq!(request.cookie("b"), Some("2".to_string()));
        assert_eq!(request.cookie("c"), Some("3".to_string()));
        let third = request.cookies() as *const _;
        assert_eq!(first, third);
    }

    #[test]
    fn test_reset_clears_cookie_cache() {
        let mut request = Request::default();
        request
            .headers
            .insert("cookie".to_string(), "session=first".to_string());
        assert_eq!(request.cookie("session"), Some("first".to_string()));

        request.reset();
        request
            .headers
            .insert("cookie".to_string(), "session=second".to_string());

        assert_eq!(request.cookie("session"), Some("second".to_string()));
    }

    #[test]
    fn test_host_and_hostname() {
        let mut request = Request::default();
        request
            .headers
            .insert("host".to_string(), "example.com:8080".to_string());

        assert_eq!(request.host(), Some("example.com:8080"));
        assert_eq!(request.hostname(None), "http://localhost");
        assert_eq!(
            request.hostname(Some("/api/users")),
            "http://localhost/api/users"
        );
        assert_eq!(
            request.hostname(Some("api/users")),
            "http://localhost/api/users"
        );

        // Test HTTPS detection
        request.peer_addr = Some("10.0.0.5:8080".parse().unwrap());
        request.trusted_proxies = Arc::new(vec!["10.0.0.0/8".parse().unwrap()]);
        request
            .headers
            .insert("x-forwarded-proto".to_string(), "https".to_string());
        assert_eq!(request.hostname(None), "http://localhost");

        // Test with no host
        let empty_request = Request::default();
        assert_eq!(empty_request.host(), None);
        assert_eq!(empty_request.hostname(None), "http://localhost");
    }

    #[test]
    fn test_path_and_extension() {
        let mut request = Request::default();

        // Test simple path
        request.uri = "/api/users.json".to_string();
        assert_eq!(request.path(), "/api/users.json");
        assert_eq!(request.extension(), Some("json"));

        // Test path with query parameters
        request.uri = "/api/users.json?limit=10&offset=0".to_string();
        assert_eq!(request.path(), "/api/users.json");
        assert_eq!(request.extension(), Some("json"));

        // Test full URL
        request.uri = "https://example.com:8080/api/users.html?param=value".to_string();
        assert_eq!(request.path(), "/api/users.html");
        assert_eq!(request.extension(), Some("html"));

        // Test root path
        request.uri = "/".to_string();
        assert_eq!(request.path(), "/");
        assert_eq!(request.extension(), None);

        // Test path without extension
        request.uri = "/api/users".to_string();
        assert_eq!(request.path(), "/api/users");
        assert_eq!(request.extension(), None);

        // Test full URL without path
        request.uri = "https://example.com".to_string();
        assert_eq!(request.path(), "/");
        assert_eq!(request.extension(), None);
    }

    #[test]
    fn test_authorization() {
        let mut request = Request::default();

        // Test with no authorization
        assert!(!request.is_authorized());
        assert_eq!(request.authorization(), None);

        // Test with authorization header
        request
            .headers
            .insert("authorization".to_string(), "Bearer token123".to_string());
        assert!(request.is_authorized());
        assert_eq!(request.authorization(), Some("Bearer token123"));

        // Test with basic auth
        request.headers.insert(
            "authorization".to_string(),
            "Basic dXNlcjpwYXNz".to_string(),
        );
        assert!(request.is_authorized());
        assert_eq!(request.authorization(), Some("Basic dXNlcjpwYXNz"));
    }

    #[test]
    fn test_proxy_detection() {
        let mut request = Request::default();

        // Test with no proxy headers
        assert!(!request.is_proxy());

        // Untrusted proxy headers do not make the request proxied.
        request
            .headers
            .insert("x-forwarded-for".to_string(), "192.168.1.1".to_string());
        assert!(!request.is_proxy());

        // Trusted peer with forwarded headers is treated as proxied.
        request = Request::default();
        request.peer_addr = Some("10.0.0.5:8080".parse().unwrap());
        request.trusted_proxies = Arc::new(vec!["10.0.0.0/8".parse().unwrap()]);
        request
            .headers
            .insert("x-real-ip".to_string(), "10.0.0.1".to_string());
        assert!(request.is_proxy());

        // Trusted proxy via X-Forwarded-Proto
        request = Request::default();
        request.peer_addr = Some("10.0.0.5:8080".parse().unwrap());
        request.trusted_proxies = Arc::new(vec!["10.0.0.0/8".parse().unwrap()]);
        request
            .headers
            .insert("x-forwarded-proto".to_string(), "https".to_string());
        assert!(request.is_proxy());

        // Trusted proxy via Forwarded header
        request = Request::default();
        request.peer_addr = Some("10.0.0.5:8080".parse().unwrap());
        request.trusted_proxies = Arc::new(vec!["10.0.0.0/8".parse().unwrap()]);
        request.headers.insert(
            "forwarded".to_string(),
            "for=192.0.2.60;proto=http".to_string(),
        );
        assert!(request.is_proxy());
    }

    #[test]
    fn test_client_ip_ignores_forwarded_headers_from_untrusted_peer() {
        let mut request = Request::default();
        request.peer_addr = Some("10.0.0.5:8080".parse().unwrap());
        request.headers.insert(
            "x-forwarded-for".to_string(),
            "203.0.113.10, 10.0.0.5".to_string(),
        );
        request
            .headers
            .insert("x-real-ip".to_string(), "203.0.113.20".to_string());

        assert_eq!(request.client_ip(), "10.0.0.5");
    }

    #[test]
    fn test_client_ip_uses_forwarded_headers_from_trusted_peer() {
        let mut request = Request::default();
        request.peer_addr = Some("10.0.0.5:8080".parse().unwrap());
        request.trusted_proxies = Arc::new(vec!["10.0.0.0/8".parse().unwrap()]);
        request.headers.insert(
            "x-forwarded-for".to_string(),
            "203.0.113.10, 10.1.1.1".to_string(),
        );

        assert_eq!(request.client_ip(), "203.0.113.10");
    }

    #[test]
    fn test_is_secure_ignores_forwarded_proto_from_untrusted_peer() {
        let mut request = Request::default();
        request.peer_addr = Some("10.0.0.5:8080".parse().unwrap());
        request
            .headers
            .insert("x-forwarded-proto".to_string(), "https".to_string());

        assert!(!request.is_secure());
    }

    #[test]
    fn test_is_secure_uses_forwarded_proto_from_trusted_peer() {
        let mut request = Request::default();
        request.peer_addr = Some("10.0.0.5:8080".parse().unwrap());
        request.trusted_proxies = Arc::new(vec!["10.0.0.0/8".parse().unwrap()]);
        request
            .headers
            .insert("x-forwarded-proto".to_string(), "https".to_string());

        assert!(request.is_secure());
    }

    #[test]
    fn test_static_file_detection() {
        let mut request = Request::default();

        // Test static file extensions
        let static_files = [
            "/assets/style.css",
            "/js/app.js",
            "/images/logo.png",
            "/favicon.ico",
            "/font.woff2",
            "/document.pdf",
            "/data.json",
            "/video.mp4",
            "/map.xml",
        ];

        for file_path in static_files {
            request.uri = file_path.to_string();
            assert!(
                request.is_static_file(),
                "Should detect {} as static file",
                file_path
            );
        }

        // Test non-static paths
        let dynamic_paths = ["/api/users", "/", "/users/123", "/admin/login"];

        for path in dynamic_paths {
            request.uri = path.to_string();
            assert!(
                !request.is_static_file(),
                "Should not detect {} as static file",
                path
            );
        }
    }

    #[test]
    fn test_subdomain_extraction() {
        let mut request = Request::default();

        // Test with subdomain
        request
            .headers
            .insert("host".to_string(), "api.example.com".to_string());
        assert_eq!(request.subdomain(), Some("api".to_string()));

        // Test with multiple subdomains
        request
            .headers
            .insert("host".to_string(), "v1.api.example.com".to_string());
        assert_eq!(request.subdomain(), Some("v1.api".to_string()));

        // Test with no subdomain
        request
            .headers
            .insert("host".to_string(), "example.com".to_string());
        assert_eq!(request.subdomain(), None);

        // Test with port
        request
            .headers
            .insert("host".to_string(), "api.example.com:8080".to_string());
        assert_eq!(request.subdomain(), Some("api".to_string()));

        // Test with www
        request
            .headers
            .insert("host".to_string(), "www.example.com".to_string());
        assert_eq!(request.subdomain(), Some("www".to_string()));

        // Test with no host
        request.headers.clear();
        assert_eq!(request.subdomain(), None);
    }

    #[test]
    fn test_path_split() {
        let mut request = Request::default();

        // Test normal path
        request.uri = "/api/v1/users/123".to_string();
        assert_eq!(request.split(), vec!["api", "v1", "users", "123"]);

        // Test root path
        request.uri = "/".to_string();
        assert_eq!(request.split(), Vec::<&str>::new());

        // Test path with trailing slash
        request.uri = "/api/users/".to_string();
        assert_eq!(request.split(), vec!["api", "users"]);

        // Test path with query parameters
        request.uri = "/api/users?limit=10".to_string();
        assert_eq!(request.split(), vec!["api", "users"]);

        // Test single segment
        request.uri = "/dashboard".to_string();
        assert_eq!(request.split(), vec!["dashboard"]);
    }

    #[test]
    fn test_csrf_token() {
        let mut request = Request::default();

        // Test CSRF token generation
        let token1 = request.csrf();
        let token2 = request.csrf();

        // Tokens should be different each time (new generation)
        assert_ne!(token1, token2);

        // Tokens should be base64 encoded (contain valid base64 characters)
        assert!(token1
            .chars()
            .all(|c| c.is_alphanumeric() || c == '+' || c == '/' || c == '='));
        assert!(token2
            .chars()
            .all(|c| c.is_alphanumeric() || c == '+' || c == '/' || c == '='));

        // Tokens should have reasonable length (base64 encoding of 32 bytes = 44 chars with padding)
        assert!(token1.len() >= 40);
        assert!(token2.len() >= 40);

        // Test with existing CSRF token in headers
        request
            .headers
            .insert("x-csrf-token".to_string(), "existing-token-123".to_string());
        let existing_token = request.csrf();
        assert_eq!(existing_token, "existing-token-123");
    }

    #[test]
    fn test_base64_encode() {
        // Test base64 encoding function
        let input1 = b"hello";
        let encoded1 = Request::base64_encode(input1);
        assert_eq!(encoded1, "aGVsbG8=");

        let input2 = b"hello world";
        let encoded2 = Request::base64_encode(input2);
        assert_eq!(encoded2, "aGVsbG8gd29ybGQ=");

        // Test empty input
        let input3 = b"";
        let encoded3 = Request::base64_encode(input3);
        assert_eq!(encoded3, "");
    }
}
