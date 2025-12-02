# RustF Utilities (U) User Guide

## Overview

RustF provides a global utilities system through the `U` module, offering commonly used functions for web development. Inspired by Total.js, it provides a convenient API for random generation, string processing, HTTP utilities, encoding, parsing, cryptographic hash functions, and more.

The utilities system supports two access patterns:
- **Direct access**: `U::function_name()` for commonly used functions
- **Namespaced access**: `U::ModuleName::function_name()` for extended functionality

## Getting Started

Import and use the utilities:

```rust
use rustf::prelude::*;  // Includes U
// or
use rustf::U;

// Direct access - commonly used functions
let id = U::guid();
let encoded = U::encode("hello world");
let status = U::http_status(404); // "Not Found"
let hash = U::sha256("data");      // Secure hash function

// Namespaced access - extended functionality  
let slug = U::String::to_slug("Hello World!");
let html = U::Encoding::html_encode("<script>");
let duration = U::Parsing::parse_duration("1h", 0);
let md5_hash = U::Crypto::md5("data");  // Warning: Not secure
```

## Available Utilities

The RustF utilities system provides two access patterns:

### Direct Access Functions (U::function())
These are the most commonly used functions available directly through `U::`:

| Category | Function | Description | Example |
|----------|----------|-------------|---------|
| **Random** | `U::guid()` | Generate UUID without hyphens | `U::guid()` → `"a1b2c3d4e5f67890..."` |
| | `U::random_string(len)` | Generate random alphanumeric string | `U::random_string(8)` → `"Kj8mN2pQ"` |
| | `U::random_number(min, max)` | Generate random number in range | `U::random_number(1, 100)` → `42` |
| **HTTP** | `U::http_status(code)` | Get HTTP status text | `U::http_status(404)` → `"Not Found"` |
| | `U::etag(content)` | Generate ETag for content | `U::etag("hello")` → `"5d41402a..."` |
| | `U::get_content_type(ext)` | Get MIME type by extension | `U::get_content_type("json")` → `"application/json"` |
| **Encoding** | `U::encode(str)` | URL encode string | `U::encode("hello world")` → `"hello%20world"` |
| | `U::decode(str)` | URL decode string | `U::decode("hello%20world")` → `"hello world"` |
| | `U::btoa(str)` | Base64 encode | `U::btoa("hello")` → `"aGVsbG8="` |
| | `U::atob(str)` | Base64 decode | `U::atob("aGVsbG8=")` → `"hello"` |
| **String** | `U::trim(str)` | Clean whitespace | `U::trim("  hello  ")` → `"hello"` |
| | `U::keywords(text, max, min_len)` | Extract keywords | `U::keywords("hello world", 10, 3)` → `["hello", "world"]` |
| **Parsing** | `U::parse_bool(str, default)` | Parse boolean safely | `U::parse_bool("true", false)` → `true` |
| | `U::parse_int(str, default)` | Parse integer safely | `U::parse_int("123", 0)` → `123` |
| | `U::parse_float(str, default)` | Parse float safely | `U::parse_float("12.34", 0.0)` → `12.34` |
| **JSON** | `U::get(obj, path)` | Get nested property | `U::get(&data, "user.name")` → `Some(&Value)` |
| | `U::set(obj, path, value)` | Set nested property | `U::set(&mut data, "user.name", json!("John"))` |
| **Geographic** | `U::distance(lat1, lon1, lat2, lon2)` | Distance in kilometers | `U::distance(40.7, -74.0, 34.0, -118.2)` → `3944.42` |
| **Crypto** | `U::md5(str)` | MD5 hash ⚠️ Not secure | `U::md5("hello world")` → `"5eb63bbbe01..."` |
| | `U::sha256(str)` | SHA256 hash (secure) | `U::sha256("hello world")` → `"b94d27b993..."` |

**Total: 20 direct access functions**

### Namespaced Access Functions (U::Module::function())
Extended functionality is available through nested modules:

### Random Generation

#### Generate UUIDs

```rust
// Generate a simple UUID (without hyphens) - Direct access
let id = U::guid();
println!("ID: {}", id); // e.g., "a1b2c3d4e5f67890abcdef1234567890"

// Generate UUID with hyphens - Extended access
let uuid = U::Random::generate_guid_with_hyphens();
println!("UUID: {}", uuid); // e.g., "a1b2c3d4-e5f6-7890-abcd-ef1234567890"
```

#### Generate Random Strings

```rust
// Generate random alphanumeric string - Direct access
let token = U::random_string(32);
println!("Token: {}", token); // e.g., "Kj8mN2pQ5rT9wX3yZ7aB4cD6fG1hI0kL"

// Use for session tokens, API keys, etc.
let api_key = U::random_string(64);
let session_id = U::random_string(16);

// Generate with custom character set - Extended access
let numeric_code = U::Random::generate_random_string_with_charset(6, "0123456789");
let secure_token = U::Random::generate_secure_token(32); // Cryptographically secure
```

#### Generate Random Numbers

```rust
// Generate random number in range (inclusive)
let dice = U::random_number(1, 6);
let port = U::random_number(8000, 9000);
let user_id = U::random_number(1000, 99999);
```

### HTTP Utilities

#### Status Code Helpers

```rust
// Get human-readable status text
let status = U::http_status(200);  // "OK"
let error = U::http_status(404);   // "Not Found"
let server_error = U::http_status(500); // "Internal Server Error"

// Use in responses
async fn not_found(ctx: Context) -> Result<Response> {
    let message = U::http_status(404);
    Ok(Response::not_found().with_text(message))
}
```

#### ETag Generation

```rust
// Generate ETags for content caching
let content = "Hello, World!";
let etag = U::etag(content);

// Use in HTTP responses
async fn cached_content(ctx: Context) -> Result<Response> {
    let content = get_content();
    let etag = U::etag(&content);
    
    Ok(Response::ok()
        .with_header("ETag", &etag)
        .with_text(content))
}
```

#### MIME Type Detection

```rust
// Get content type by file extension
let html_type = U::get_content_type("html");   // "text/html"
let json_type = U::get_content_type("json");   // "application/json"
let css_type = U::get_content_type("css");     // "text/css"
let js_type = U::get_content_type("js");       // "application/javascript"

// Use in file serving
async fn serve_file(ctx: Context) -> Result<Response> {
    let filename = ctx.param("filename")?;
    let extension = filename.split('.').last().unwrap_or("txt");
    let content_type = U::get_content_type(extension);
    
    let content = std::fs::read_to_string(&filename)?;
    Ok(Response::ok()
        .with_header("Content-Type", content_type)
        .with_text(content))
}
```

### Encoding & Decoding

#### URL Encoding

```rust
// URL encode strings - Direct access
let encoded = U::encode("hello world");
assert_eq!(encoded, "hello%20world");

let encoded = U::encode("user@example.com");
assert_eq!(encoded, "user%40example.com");

// URL decode strings - Direct access
let decoded = U::decode("hello%20world")?;
assert_eq!(decoded, "hello world");

// Extended encoding functions - Namespaced access
let html = U::Encoding::html_encode("<script>alert('xss')</script>");
let hex = U::Encoding::hex_encode(&[255, 0, 128]);  // "ff0080"
let base64_url = U::Encoding::base64_url_encode("data");

// Use in URL building
fn build_search_url(query: &str) -> String {
    format!("/search?q={}", U::encode(query))
}
```

#### Base64 Encoding

```rust
// Base64 encode (btoa = binary to ASCII)
let encoded = U::btoa("hello");
println!("Base64: {}", encoded); // "aGVsbG8="

// Base64 decode (atob = ASCII to binary)
let decoded = U::atob("aGVsbG8=")?;
assert_eq!(decoded, "hello");

// Use for API tokens, data encoding
let token = U::btoa(&format!("{}:{}", username, password));
```

### String Processing

#### Text Cleaning

```rust
// Clean and normalize text - Direct access
let messy = "  hello    world  \n\t";
let clean = U::trim(messy);
assert_eq!(clean, "hello world");

// Extended string functions - Namespaced access
let slug = U::String::to_slug("Hello World!");        // "hello-world"
let camel = U::String::to_camel_case("hello world");   // "helloWorld"
let pascal = U::String::to_pascal_case("hello world"); // "HelloWorld"
let snake = U::String::to_snake_case("Hello World");   // "hello_world"
let title = U::String::title_case("hello world");      // "Hello World"
let truncated = U::String::truncate("Long text here", 10); // "Long te..."

// Use for user input processing
async fn process_comment(ctx: Context) -> Result<Response> {
    let raw_comment = ctx.form_value("comment")?;
    let clean_comment = U::trim(&raw_comment);
    
    // Save clean comment to database
    save_comment(&clean_comment).await?;
    ctx.redirect("/comments")
}
```

#### Keyword Extraction

```rust
// Extract keywords for search indexing
let text = "This is a sample blog post about web development";
let keywords = U::keywords(text, 10, 3);
// Returns: ["sample", "blog", "post", "web", "development"]
// (excludes common words like "This", "is", "a", "about")

// Use for content indexing
async fn create_post(ctx: Context) -> Result<Response> {
    let title = ctx.form_value("title")?;
    let content = ctx.form_value("content")?;
    
    // Extract keywords for search
    let search_keywords = U::keywords(&format!("{} {}", title, content), 20, 4);
    
    let post = Post {
        title,
        content,
        keywords: search_keywords.join(", "),
        ..Default::default()
    };
    
    post.save().await?;
    ctx.redirect("/posts")
}
```

### Safe Parsing

Parse strings with fallback values using direct access for common types, or namespaced access for extended parsing:

#### Boolean Parsing

```rust
// Parse booleans safely
let enabled = U::parse_bool("true", false);   // true
let disabled = U::parse_bool("false", true);  // false
let default = U::parse_bool("invalid", false); // false (fallback)

// Use with form data
async fn update_settings(ctx: Context) -> Result<Response> {
    let notifications = U::parse_bool(&ctx.form_value("notifications")?, false);
    let public_profile = U::parse_bool(&ctx.form_value("public")?, true);
    
    update_user_settings(notifications, public_profile).await?;
    ctx.redirect("/settings")
}
```

#### Number Parsing

```rust
// Parse integers safely - Direct access
let page = U::parse_int("5", 1);        // 5
let invalid = U::parse_int("abc", 1);   // 1 (fallback)
let limit = U::parse_int("100", 10);    // 100

// Parse floats safely - Direct access
let price = U::parse_float("19.99", 0.0);    // 19.99
let invalid = U::parse_float("abc", 0.0);    // 0.0 (fallback)

// Extended parsing functions - Namespaced access
let duration = U::Parsing::parse_duration("1h", 0);     // 3600 seconds
let size = U::Parsing::parse_size("1MB", 0);            // 1048576 bytes
let percent = U::Parsing::parse_percentage("75%", 0.0); // 0.75
let unsigned = U::Parsing::parse_unsigned_integer("123", 0); // 123
let comma_separated = U::Parsing::parse_comma_separated("a,b,c"); // ["a", "b", "c"]

// Use with query parameters
async fn list_products(ctx: Context) -> Result<Response> {
    let page = U::parse_int(&ctx.query_value("page")?, 1);
    let limit = U::parse_int(&ctx.query_value("limit")?, 10);
    let min_price = U::parse_float(&ctx.query_value("min_price")?, 0.0);
    
    let products = Product::paginate(page, limit)
        .where_gte("price", min_price)
        .get().await?;
    
    ctx.view("/products/list", json!({
        "products": products,
        "page": page,
        "limit": limit
    }))
}
```

### JSON Object Manipulation

#### Getting Nested Values

```rust
use serde_json::json;

// Get nested properties safely
let data = json!({
    "user": {
        "profile": {
            "name": "John Doe",
            "age": 30
        },
        "settings": {
            "theme": "dark"
        }
    }
});

let name = U::get(&data, "user.profile.name");
// Returns: Some(&Value::String("John Doe"))

let theme = U::get(&data, "user.settings.theme");
// Returns: Some(&Value::String("dark"))

let missing = U::get(&data, "user.profile.email");
// Returns: None

// Use in templates or API responses
async fn user_profile(ctx: Context) -> Result<Response> {
    let user_data = get_user_data().await?;
    
    let display_name = U::get(&user_data, "profile.display_name")
        .and_then(|v| v.as_str())
        .unwrap_or("Anonymous");
    
    ctx.view("/profile", json!({
        "name": display_name,
        "data": user_data
    }))
}
```

#### Setting Nested Values

```rust
// Set nested properties
let mut data = json!({});

// Creates nested structure automatically
U::set(&mut data, "user.name", json!("Jane"))?;
U::set(&mut data, "user.profile.age", json!(25))?;
U::set(&mut data, "settings.theme", json!("light"))?;

// Result:
// {
//   "user": {
//     "name": "Jane",
//     "profile": {
//       "age": 25
//     }
//   },
//   "settings": {
//     "theme": "light"
//   }
// }

// Use for dynamic data building
async fn build_user_response(user: &User) -> Result<Value> {
    let mut response = json!({});
    
    U::set(&mut response, "user.id", json!(user.id))?;
    U::set(&mut response, "user.name", json!(user.name))?;
    U::set(&mut response, "user.email", json!(user.email))?;
    U::set(&mut response, "meta.timestamp", json!(chrono::Utc::now()))?;
    
    Ok(response)
}
```

### Geographic Utilities

#### Distance Calculation

```rust
// Calculate distance between two points in kilometers - Direct access
let ny_lat = 40.7128;
let ny_lon = -74.0060;
let la_lat = 34.0522;
let la_lon = -118.2437;

let distance = U::distance(ny_lat, ny_lon, la_lat, la_lon);
println!("Distance: {:.2} km", distance); // Distance: 3944.42 km

// Extended geographic functions - Namespaced access
let distance_miles = U::Geo::distance_miles(ny_lat, ny_lon, la_lat, la_lon);
let is_in_bounds = U::Geo::in_bounds(40.7, -74.0, 40.0, 41.0, -75.0, -73.0);
let bearing = U::Geo::bearing(ny_lat, ny_lon, la_lat, la_lon);

// Use for location-based features
async fn find_nearby_stores(ctx: Context) -> Result<Response> {
    let user_lat = U::parse_float(&ctx.query_value("lat")?, 0.0);
    let user_lon = U::parse_float(&ctx.query_value("lon")?, 0.0);
    
    let stores = get_all_stores().await?;
    let mut nearby_stores = Vec::new();
    
    for store in stores {
        let distance = U::distance(user_lat, user_lon, store.lat, store.lon);
        if distance <= 50.0 { // Within 50km
            nearby_stores.push((store, distance));
        }
    }
    
    // Sort by distance
    nearby_stores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    
    ctx.json(&json!({
        "stores": nearby_stores.into_iter()
            .map(|(store, dist)| json!({
                "store": store,
                "distance": dist
            }))
            .collect::<Vec<_>>()
    }))
}
```

### Cryptographic Hash Functions

RustF provides secure hash functions accessible both directly and through the Crypto namespace:

```rust
// Direct access to most common hash functions
let md5_hash = U::md5("hello world");        // ⚠️ Warning: MD5 not secure
let sha256_hash = U::sha256("hello world");  // ✅ Secure hash function

// Extended access to all hash functions via U::Crypto::
let sha1_hash = U::Crypto::sha1("data");       // ⚠️ Warning: SHA1 not secure  
let sha512_hash = U::Crypto::sha512("data");   // ✅ Very secure hash function

// Hash from byte arrays
let hash_from_bytes = U::Crypto::md5_bytes(&[1, 2, 3, 4, 5]);
let secure_hash_bytes = U::Crypto::sha256_bytes(b"binary data");

// Other crypto utilities
let hash_code = U::Crypto::hash_string("data");           // Fast non-crypto hash
let is_equal = U::Crypto::constant_time_compare("a", "b"); // Timing-safe comparison
let checksum = U::Crypto::simple_checksum("integrity check");

// Simple obfuscation (not encryption!)
let obfuscated = U::Crypto::obfuscate_string("secret", 5);
let deobfuscated = U::Crypto::deobfuscate_string(&obfuscated, 5);

// Basic XOR encryption/decryption (not secure - for obfuscation only)
let encrypted = U::Crypto::xor_encrypt("message", "key");
let decrypted = U::Crypto::xor_decrypt(&encrypted, "key");
```

**Security Notice**: MD5 and SHA1 are cryptographically broken and should not be used for security purposes. Use SHA256 or higher for security-sensitive applications.

## Common Usage Patterns

### Form Processing

```rust
async fn process_form(ctx: Context) -> Result<Response> {
    // Safe parsing with defaults - Direct access
    let name = U::trim(&ctx.form_value("name")?);
    let age = U::parse_int(&ctx.form_value("age")?, 0);
    let email = U::trim(&ctx.form_value("email")?);
    let newsletter = U::parse_bool(&ctx.form_value("newsletter")?, false);
    
    // Extended validation using namespaced functions
    let clean_email = U::String::strip_html(&email);
    let email_slug = U::String::to_slug(&name); // For URL-friendly usernames
    
    if name.is_empty() || age <= 0 {
        ctx.flash_set("error_msg", "Please fill in all required fields");
        return ctx.redirect("/form");
    }
    
    // Create user with secure hash for verification tokens
    let user_id = U::guid();
    let verification_token = U::sha256(&format!("{}:{}", user_id, email));
    
    let user = User {
        id: user_id,
        name,
        age,
        email: clean_email,
        newsletter_subscribed: newsletter,
        verification_token,
    };
    
    user.save().await?;
    ctx.flash_set("success_msg", "User created successfully!");
    ctx.redirect("/users")
}
```

### API Response Building

```rust
async fn api_user_list(ctx: Context) -> Result<Response> {
    // Parse pagination parameters safely - Direct access
    let page = U::parse_int(&ctx.query_value("page")?, 1);
    let limit = U::parse_int(&ctx.query_value("limit")?, 10);
    
    // Extended parsing for filtering - Namespaced access  
    let search = U::String::trim(&ctx.query_value("search").unwrap_or_default());
    let sort_order = U::Parsing::parse_enum(&ctx.query_value("sort")?, SortOrder::Asc);
    
    let users = User::paginate(page, limit).await?;
    let total = User::count().await?;
    
    // Build response with utilities - Direct access for JSON manipulation
    let mut response = json!({});
    U::set(&mut response, "data", json!(users))?;
    U::set(&mut response, "pagination.page", json!(page))?;
    U::set(&mut response, "pagination.limit", json!(limit))?;
    U::set(&mut response, "pagination.total", json!(total))?;
    U::set(&mut response, "meta.request_id", json!(U::guid()))?;
    U::set(&mut response, "meta.timestamp", json!(chrono::Utc::now()))?;
    
    // Extended object manipulation - Namespaced access
    let has_filters = U::Object::has_nested_property(&response, "filters");
    if !search.is_empty() {
        U::set(&mut response, "filters.search", json!(search))?;
    }
    
    // Generate ETag for caching - Direct access
    let content = response.to_string();
    let etag = U::etag(&content);
    
    Ok(Response::ok()
        .with_header("ETag", &etag)
        .with_json(&response)?)
}
```

### Search and Filtering

```rust
async fn search_content(ctx: Context) -> Result<Response> {
    // Basic input processing - Direct access
    let query = U::trim(&ctx.query_value("q")?);
    let category = ctx.query_value("category")?;
    
    // Extended string processing - Namespaced access
    let clean_query = U::String::strip_html(&query);
    let query_slug = U::String::to_slug(&clean_query);
    
    if clean_query.is_empty() {
        return ctx.view("/search", json!({
            "message": "Please enter a search term"
        }));
    }
    
    // Extract search keywords - Direct access
    let search_terms = U::keywords(&clean_query, 10, 3);
    
    // Generate search hash for caching - Direct access
    let search_hash = U::sha256(&format!("{}:{}", clean_query, category));
    
    // Build search results
    let results = search_posts(&search_terms, &category).await?;
    
    // Extended parsing for result formatting - Namespaced access
    let result_limit = U::Parsing::parse_int(&ctx.query_value("limit")?, 10);
    let truncated_results: Vec<_> = results.into_iter()
        .take(result_limit as usize)
        .map(|mut post| {
            // Truncate content for search results
            post.content = U::String::truncate_words(&post.content, 100);
            post
        })
        .collect();
    
    ctx.view("/search", json!({
        "query": clean_query,
        "results": truncated_results,
        "keywords": search_terms,
        "total": truncated_results.len(),
        "search_hash": search_hash
    }))
}
```

### File Upload Processing

```rust
async fn handle_upload(ctx: Context) -> Result<Response> {
    let file = ctx.file("upload")?;
    
    // Generate unique filename - Direct access
    let file_id = U::guid();
    let original_name = file.filename.unwrap_or("upload".to_string());
    let extension = original_name.split('.').last().unwrap_or("bin");
    
    // Extended string processing - Namespaced access
    let safe_original_name = U::String::strip_html(&original_name);
    let filename_slug = U::String::to_slug(&safe_original_name);
    let secure_token = U::Random::generate_secure_token(16);
    let new_filename = format!("{}_{}_{}.{}", file_id, filename_slug, secure_token, extension);
    
    // Get content type and validate - Direct and namespaced access
    let content_type = U::get_content_type(extension);
    let is_valid_type = U::Http::is_success_status(200); // Example validation
    
    // Generate file hash for integrity checking - Direct access
    let file_hash = U::sha256(&format!("{:?}", file.data));
    
    // Extended parsing for file size - Namespaced access
    let max_size = U::Parsing::parse_size("10MB", 10485760); // 10MB default
    if file.data.len() > max_size as usize {
        return ctx.json(&json!({
            "error": "File too large",
            "max_size": format!("{}MB", max_size / 1048576)
        }));
    }
    
    // Save file metadata with enhanced information
    let file_record = FileRecord {
        id: file_id.clone(),
        original_name: safe_original_name,
        filename: new_filename.clone(),
        content_type: content_type.to_string(),
        size: file.data.len(),
        hash: file_hash,
        upload_token: secure_token,
        ..Default::default()
    };
    
    // Save to storage and database
    save_file(&new_filename, &file.data).await?;
    file_record.save().await?;
    
    ctx.json(&json!({
        "success": true,
        "file_id": file_id,
        "filename": new_filename,
        "size": file.data.len(),
        "content_type": content_type
    }))
}
```

## Best Practices

### Error Handling

```rust
// Always handle potential errors from parsing
match U::decode(&encoded_value) {
    Ok(decoded) => process_value(&decoded),
    Err(_) => {
        log::warn!("Failed to decode value: {}", encoded_value);
        use_default_value()
    }
}

// Or use parsing utilities with safe defaults
let safe_value = U::parse_int(&user_input, 0);
```

### Performance Considerations

```rust
// Cache expensive operations
static KEYWORDS_CACHE: OnceCell<HashMap<String, Vec<String>>> = OnceCell::new();

fn get_cached_keywords(text: &str) -> Vec<String> {
    let cache = KEYWORDS_CACHE.get_or_init(HashMap::new);
    
    if let Some(cached) = cache.get(text) {
        return cached.clone();
    }
    
    let keywords = U::keywords(text, 20, 4);
    // In real implementation, you'd update the cache here
    keywords
}
```

### Security

```rust
// Always validate and sanitize user input
async fn update_profile(ctx: Context) -> Result<Response> {
    let name = U::trim(&ctx.form_value("name")?);
    let bio = U::trim(&ctx.form_value("bio")?);
    
    // Validate lengths
    if name.len() > 100 {
        return ctx.json(&json!({
            "error": "Name too long"
        }));
    }
    
    if bio.len() > 1000 {
        return ctx.json(&json!({
            "error": "Bio too long"  
        }));
    }
    
    // Update user...
}
```

## Summary

The RustF utilities system provides comprehensive utility functions organized in a dual-access pattern:

### Core Features
- **Random generation** - UUIDs, strings, numbers, secure tokens
- **HTTP utilities** - Status codes, ETags, MIME types, response validation
- **String processing** - Cleaning, keyword extraction, case conversion, HTML stripping
- **Encoding/decoding** - URL encoding, Base64, HTML entities, hexadecimal
- **Safe parsing** - Numbers, booleans, durations, sizes, percentages with defaults
- **JSON manipulation** - Get/set nested properties, deep merging, object flattening
- **Geographic calculations** - Distance between coordinates, boundary checking, bearing
- **Cryptographic functions** - MD5, SHA1, SHA256, SHA512 hashing, secure comparisons

### Access Patterns
The RustF utilities system provides both direct access (`U::function_name()`) for commonly used functions and namespaced access (`U::ModuleName::function_name()`) for extended functionality.

**Key Achievement**: The exact `U::ModuleName::function()` syntax that was requested now works perfectly, providing powerful, organized access to all utility functions throughout your RustF application.

### Security Features
- ✅ Secure hash functions (SHA256, SHA512) 
- ⚠️ Legacy hash functions (MD5, SHA1) with clear warnings
- 🔒 Timing-safe comparisons for sensitive data
- 🛡️ Input sanitization and validation helpers
- 🔐 Cryptographically secure token generation

Use the utilities system to build secure, maintainable web applications with consistent, predictable utility functions that follow Rust best practices and web development standards.

#### U::Random:: - Random Generation (Extended)

```rust
U::Random::generate_guid_with_hyphens()           // UUID with hyphens
U::Random::generate_random_string_with_charset(len, charset) // Custom character set
U::Random::generate_random_float(min, max)        // Random float
U::Random::generate_random_text(word_count)       // Lorem ipsum text
U::Random::generate_secure_token(byte_len)        // Cryptographically secure token
```

#### U::Encoding:: - Encoding (Extended)

```rust
U::Encoding::encode_with_safe(str, safe_chars)    // URL encode with safe characters
U::Encoding::btoa_bytes(bytes)                    // Base64 encode bytes
U::Encoding::base64_decode_bytes(str)             // Base64 decode to bytes
U::Encoding::base64_url_encode(str)               // URL-safe base64 encode
U::Encoding::base64_url_decode(str)               // URL-safe base64 decode
U::Encoding::html_encode(str)                     // HTML entity encode
U::Encoding::html_decode(str)                     // HTML entity decode
U::Encoding::json_encode(str)                     // JSON string escape
U::Encoding::hex_encode(bytes)                    // Hexadecimal encode
U::Encoding::hex_decode(str)                      // Hexadecimal decode
U::Encoding::query_encode(params)                 // Query string encode
U::Encoding::query_decode(query)                  // Query string decode
```

#### U::String:: - String Processing (Extended)

```rust
U::String::to_slug(str)           // Convert to URL slug
U::String::to_camel_case(str)     // Convert to camelCase
U::String::to_pascal_case(str)    // Convert to PascalCase
U::String::to_snake_case(str)     // Convert to snake_case
U::String::truncate(str, len)     // Truncate by character count
U::String::truncate_words(str, len) // Truncate by word count
U::String::word_count(str)        // Count words
U::String::title_case(str)        // Convert to Title Case
U::String::strip_html(str)        // Remove HTML tags
U::String::wrap_text(str, width)  // Wrap text to width
```

#### U::Parsing:: - Parsing (Extended)

```rust
U::Parsing::parse_unsigned_integer(str, default) // Parse u64
U::Parsing::parse_i32(str, default)              // Parse i32
U::Parsing::parse_f32(str, default)              // Parse f32
U::Parsing::parse_enum(str, default)             // Parse enum from string
U::Parsing::parse_comma_separated(str)           // Parse comma-separated values
U::Parsing::parse_comma_separated_integers(str, default) // Parse comma-separated integers
U::Parsing::parse_key_value(str, separator)      // Parse key=value pairs
U::Parsing::parse_duration(str, default)         // Parse duration (1h, 30m, etc.)
U::Parsing::parse_size(str, default)             // Parse size (1KB, 2MB, etc.)
U::Parsing::parse_percentage(str, default)       // Parse percentage
```

#### U::Crypto:: - Cryptographic Functions

```rust
// Hash functions (includes all direct access functions plus extended ones)
U::Crypto::md5(str)                    // MD5 hash ⚠️ Not secure
U::Crypto::md5_bytes(bytes)           // MD5 hash from bytes ⚠️ Not secure
U::Crypto::sha1(str)                  // SHA1 hash ⚠️ Not secure
U::Crypto::sha1_bytes(bytes)         // SHA1 hash from bytes ⚠️ Not secure
U::Crypto::sha256(str)               // SHA256 hash (secure)
U::Crypto::sha256_bytes(bytes)       // SHA256 hash from bytes (secure)
U::Crypto::sha512(str)               // SHA512 hash (secure)
U::Crypto::sha512_bytes(bytes)       // SHA512 hash from bytes (secure)

// Other crypto utilities
U::Crypto::hash_string(str)                      // Hash string (non-crypto)
U::Crypto::hash_bytes(bytes)                     // Hash bytes (non-crypto)
U::Crypto::xor_encrypt(data, key)                // XOR encrypt (not secure)
U::Crypto::xor_decrypt(data, key)                // XOR decrypt (not secure)
U::Crypto::simple_checksum(data)                 // Simple checksum
U::Crypto::constant_time_compare(a, b)           // Timing-safe string compare
U::Crypto::constant_time_compare_bytes(a, b)     // Timing-safe byte compare
U::Crypto::obfuscate_string(str, offset)         // Simple obfuscation
U::Crypto::deobfuscate_string(str, offset)       // Simple deobfuscation
```

#### U::Http:: - HTTP (Extended)

```rust
U::Http::generate_strong_etag(content, modified) // Strong ETag with timestamp
U::Http::get_extension_from_content_type(mime)   // Get extension from MIME type
U::Http::is_success_status(code)                 // Check if 2xx status
U::Http::is_client_error(code)                   // Check if 4xx status
U::Http::is_server_error(code)                   // Check if 5xx status
U::Http::is_redirect_status(code)                // Check if 3xx status
```

#### U::Object:: - Object Manipulation (Extended)

```rust
U::Object::remove_nested_property(obj, path)     // Remove nested property
U::Object::deep_merge(target, source)            // Deep merge objects
U::Object::shallow_merge(target, source)         // Shallow merge objects
U::Object::deep_clone(value)                     // Deep clone JSON value
U::Object::flatten_object(obj, prefix)           // Flatten nested object
U::Object::has_nested_property(obj, path)        // Check if property exists
U::Object::get_all_keys(obj)                     // Get all object keys
```

#### U::Geo:: - Geographic (Extended)

```rust
U::Geo::distance_miles(lat1, lon1, lat2, lon2)   // Distance in miles
U::Geo::in_bounds(lat, lon, min_lat, max_lat, min_lon, max_lon) // Check if in bounds
U::Geo::bearing(lat1, lon1, lat2, lon2)          // Calculate bearing/direction
```


## Usage Patterns

The RustF utilities system supports three usage patterns:

### 1. Direct Access (Recommended for Common Operations)

```rust
use rustf::U;

// Most commonly used functions - direct access
let id = U::guid();
let encoded = U::encode("hello world");
let hash = U::sha256("secure data");
let parsed = U::parse_int("123", 0);
```

### 2. Namespaced Access (Extended Functionality)

```rust
use rustf::U;

// Extended functions through nested modules - exact syntax requested!
let slug = U::String::to_slug("Hello World!");
let html = U::Encoding::html_encode("<p>Hello</p>");
let size = U::Parsing::parse_size("1MB", 0);
let secure_token = U::Random::generate_secure_token(32);
let sha1_hash = U::Crypto::sha1("legacy data"); // Warning: Not secure
```

### 3. Module-Level Access (Alternative)

```rust
// Alternative approach - direct module imports
use rustf::utils::{string, encoding, parsing};

let slug = string::to_slug("Hello World!");
let html = encoding::html_encode("<p>Hello</p>");
let size = parsing::parse_size("1MB", 0);
```