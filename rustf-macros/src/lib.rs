//! RustF Macros - Procedural macros for auto-discovery
//!
//! This crate provides build-time macros to automatically discover and include
//! controllers and models without requiring manual mod.rs files.

use proc_macro::TokenStream;
use quote::quote;
use std::path::PathBuf;
use syn::{parse_macro_input, parse_quote, ItemFn};
use walkdir::WalkDir;

/// Auto-discover and include all controllers from src/controllers/*.rs
///
/// This macro scans the controllers directory at build time and generates
/// the necessary module declarations and route aggregation code.
///
/// # Usage
/// ```rust,ignore
/// use rustf::prelude::*;
///
/// let routes = auto_controllers!();
/// let app = RustF::new().controllers(routes);
/// ```
#[proc_macro]
pub fn auto_controllers(_input: TokenStream) -> TokenStream {
    generate_auto_discovery("controllers", "install").into()
}

/// Auto-discover and include all models from src/models/*.rs
///
/// This macro scans the models directory at build time and generates
/// the necessary module declarations and model registration code.
///
/// # Usage
/// ```rust,ignore
/// use rustf::prelude::*;
///
/// let register_fn = auto_models!();
/// let app = RustF::new().models(register_fn);
/// ```
#[proc_macro]
pub fn auto_models(_input: TokenStream) -> TokenStream {
    generate_auto_discovery("models", "register").into()
}

/// Auto-discover and include all middleware from src/middleware/*.rs
///
/// This macro scans the middleware directory at build time and generates
/// the necessary module declarations and middleware registration code.
///
/// # Usage
/// ```rust,ignore
/// use rustf::prelude::*;
///
/// let register_fn = auto_middleware!();
/// let app = RustF::new().middleware_from(register_fn);
/// ```
#[proc_macro]
pub fn auto_middleware(_input: TokenStream) -> TokenStream {
    generate_auto_discovery("middleware", "install").into()
}

/// Auto-discover and include all definitions from src/definitions/*.rs
///
/// This macro scans the definitions directory at build time and generates
/// the necessary module declarations and registration code.
/// Each file should implement either Helpers or Validators trait.
///
/// # Usage
/// ```rust,ignore
/// let app = RustF::new()
///     .definitions_from(auto_definitions!());
/// ```
#[proc_macro]
pub fn auto_definitions(_input: TokenStream) -> TokenStream {
    generate_auto_discovery("definitions", "install").into()
}

/// Auto-discover and include all modules from src/modules/*.rs
///
/// This macro scans the modules directory at build time and generates
/// the necessary module declarations and module registration code.
/// Following Total.js inspiration, modules contain reusable business logic.
///
/// # Usage
/// ```rust,ignore
/// use rustf::prelude::*;
///
/// let register_fn = auto_modules!();
/// let app = RustF::new().modules_from(register_fn);
/// ```
#[proc_macro]
pub fn auto_modules(_input: TokenStream) -> TokenStream {
    generate_auto_discovery("modules", "install").into()
}

/// Auto-discover and include all event handlers from src/events/*.rs
///
/// This macro scans the events directory at build time and generates
/// the necessary module declarations and event handler registration code.
///
/// # Usage
/// ```rust,ignore
/// use rustf::prelude::*;
///
/// let register_fn = auto_events!();
/// let app = RustF::new().events_from(register_fn);
/// ```
#[proc_macro]
pub fn auto_events(_input: TokenStream) -> TokenStream {
    generate_auto_discovery("events", "install").into()
}

/// Auto-discover and include all workers from src/workers/*.rs
///
/// This macro scans the workers directory at build time and generates
/// the necessary module declarations and worker registration code.
/// Each file should have an async `install()` function that registers workers.
///
/// # Usage
/// ```rust,ignore
/// use rustf::prelude::*;
///
/// let app = RustF::new()
///     .with_workers()
///     .workers_from(auto_workers!());
/// ```
#[proc_macro]
pub fn auto_workers(_input: TokenStream) -> TokenStream {
    generate_worker_discovery().into()
}

/// Generate auto-discovery code for a given directory and function name
fn generate_auto_discovery(dir_name: &str, fn_name: &str) -> proc_macro2::TokenStream {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be set during build");

    let src_dir = PathBuf::from(&manifest_dir).join("src").join(dir_name);

    // Check if we're in the rustf framework itself by looking for Cargo.toml [package] name
    let cargo_toml_path = PathBuf::from(&manifest_dir).join("Cargo.toml");
    let is_rustf_framework = if cargo_toml_path.exists() {
        if let Ok(contents) = std::fs::read_to_string(&cargo_toml_path) {
            contents.contains("name = \"rustf\"") && !contents.contains("name = \"rustf-")
        } else {
            false
        }
    } else {
        false
    };

    // If we're in the rustf framework itself, don't try to auto-discover framework internal modules
    // Only auto-discover in user projects
    let should_skip = is_rustf_framework
        && matches!(
            dir_name,
            "models" | "workers" | "definitions" | "middleware" | "events"
        );

    if should_skip || !src_dir.exists() {
        // If directory doesn't exist or we should skip it, return empty implementation
        // Use correct path prefix based on whether we're in rustf framework or user project
        let path_prefix = if is_rustf_framework {
            quote! { crate }
        } else {
            quote! { rustf }
        };

        return match dir_name {
            "controllers" => quote! {
                {
                    Vec::new()
                }
            },
            "models" => quote! {
                {
                    |_registry: &mut #path_prefix::models::ModelRegistry| {
                        // No models to register
                    }
                }
            },
            "middleware" => quote! {
                {
                    |_registry: &mut #path_prefix::middleware::MiddlewareRegistry| {
                        // No middleware to register
                    }
                }
            },
            "events" => quote! {
                {
                    |_emitter: &mut #path_prefix::events::EventEmitter| {
                        // No event handlers to register
                    }
                }
            },
            "definitions" => quote! {
                {
                    |_defs: &mut #path_prefix::definitions::Definitions| {
                        // No definitions to register
                    }
                }
            },
            _ => quote! { compile_error!("Unknown directory type"); },
        };
    }

    let mut modules = Vec::new();
    let mut function_calls = Vec::new();

    // Walk through all .rs files in the directory with max depth of 3
    for entry in WalkDir::new(&src_dir)
        .min_depth(1)
        .max_depth(3) // Support up to 3 levels of nesting
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // Skip directories (we only want files)
        if path.is_dir() {
            continue;
        }

        // Skip if not a .rs file
        if path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }

        // Skip mod.rs files if they exist
        if path.file_name().and_then(|s| s.to_str()) == Some("mod.rs") {
            continue;
        }

        // Skip .inc.rs files (included code fragments)
        if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
            if file_name.ends_with(".inc.rs") {
                continue;
            }
            // Skip files starting with underscore
            if file_name.starts_with("_") {
                continue;
            }
        }

        // Create the module path relative to the target directory
        let relative_to_target = path
            .strip_prefix(&src_dir)
            .expect("Path should be under target directory");

        // Build nested module path (e.g., api::v1::users for api/v1/users.rs)
        let mut path_components = Vec::new();
        for component in relative_to_target.components() {
            if let std::path::Component::Normal(name) = component {
                if let Some(name_str) = name.to_str() {
                    // Remove .rs extension from the last component
                    if name_str.ends_with(".rs") {
                        path_components.push(name_str.trim_end_matches(".rs"));
                    } else {
                        path_components.push(name_str);
                    }
                }
            }
        }

        // Create the full module path (e.g., "api::v1::users")
        let _module_path = path_components.join("::");

        // Generate unique module identifier for nested paths
        let module_ident_str = path_components.join("_");
        let module_ident = syn::Ident::new(&module_ident_str, proc_macro2::Span::call_site());

        // Create relative path from the src directory (where main.rs is located)
        let src_dir_base = PathBuf::from(&manifest_dir).join("src");
        let relative_path = path
            .strip_prefix(&src_dir_base)
            .expect("Path should be under src directory")
            .to_string_lossy()
            .replace('\\', "/"); // Normalize path separators

        // Generate module declaration with nested structure
        if path_components.len() == 1 {
            // Top-level module (e.g., controllers/home.rs)
            let module_name = path_components[0];
            let module_ident = syn::Ident::new(module_name, proc_macro2::Span::call_site());
            modules.push(quote! {
                #[path = #relative_path]
                pub mod #module_ident;
            });

            // Generate function call using the simple identifier
            match dir_name {
                "controllers" => {
                    let fn_ident = syn::Ident::new(fn_name, proc_macro2::Span::call_site());
                    function_calls.push(quote! {
                        routes.extend(#module_ident::#fn_ident());
                    });
                }
                "models" => {
                    let fn_ident = syn::Ident::new(fn_name, proc_macro2::Span::call_site());
                    function_calls.push(quote! {
                        #module_ident::#fn_ident(registry);
                    });
                }
                "middleware" => {
                    let fn_ident = syn::Ident::new(fn_name, proc_macro2::Span::call_site());
                    function_calls.push(quote! {
                        #module_ident::#fn_ident(registry);
                    });
                }
                "modules" => {
                    // Modules are now discovered but NOT automatically registered
                    // Developers must explicitly call MODULE::register() with a unique name
                    // This allows multiple instances of the same type
                    // (no code generation needed - just module declarations)
                }
                "events" => {
                    let fn_ident = syn::Ident::new(fn_name, proc_macro2::Span::call_site());
                    function_calls.push(quote! {
                        #module_ident::#fn_ident(emitter);
                    });
                }
                "definitions" => {
                    let fn_ident = syn::Ident::new(fn_name, proc_macro2::Span::call_site());
                    function_calls.push(quote! {
                        #module_ident::#fn_ident(defs);
                    });
                }
                "workers" => {
                    let fn_ident = syn::Ident::new(fn_name, proc_macro2::Span::call_site());
                    function_calls.push(quote! {
                        #module_ident::#fn_ident().await?;
                    });
                }
                _ => {}
            }
        } else {
            // Nested module (e.g., controllers/api/users.rs)
            // We need to create nested module structure
            modules.push(quote! {
                #[path = #relative_path]
                pub mod #module_ident;
            });

            // Generate function call with full module path
            match dir_name {
                "controllers" => {
                    let fn_ident = syn::Ident::new(fn_name, proc_macro2::Span::call_site());
                    function_calls.push(quote! {
                        routes.extend(#module_ident::#fn_ident());
                    });
                }
                "models" => {
                    let fn_ident = syn::Ident::new(fn_name, proc_macro2::Span::call_site());
                    function_calls.push(quote! {
                        #module_ident::#fn_ident(registry);
                    });
                }
                "middleware" => {
                    let fn_ident = syn::Ident::new(fn_name, proc_macro2::Span::call_site());
                    function_calls.push(quote! {
                        #module_ident::#fn_ident(registry);
                    });
                }
                "modules" => {
                    // Modules are now discovered but NOT automatically registered
                    // Developers must explicitly call MODULE::register() with a unique name
                    // This allows multiple instances of the same type
                    // (no code generation needed - just module declarations)
                }
                "events" => {
                    let fn_ident = syn::Ident::new(fn_name, proc_macro2::Span::call_site());
                    function_calls.push(quote! {
                        #module_ident::#fn_ident(emitter);
                    });
                }
                "definitions" => {
                    let fn_ident = syn::Ident::new(fn_name, proc_macro2::Span::call_site());
                    function_calls.push(quote! {
                        #module_ident::#fn_ident(defs);
                    });
                }
                _ => {}
            }
        }
    }

    // Generate the final code based on directory type
    // Use correct path prefix based on whether we're in rustf framework or user project
    let path_prefix = if is_rustf_framework {
        quote! { crate }
    } else {
        quote! { rustf }
    };

    match dir_name {
        "controllers" => {
            let controller_count = function_calls.len();
            quote! {
                {
                    // Module declarations
                    #(#modules)*

                    log::info!("Auto-discovery: Loading {} controller(s)", #controller_count);

                    // Function to collect all routes
                    let mut routes = Vec::new();
                    #(#function_calls)*
                    routes
                }
            }
        }
        "models" => {
            let model_count = function_calls.len();
            quote! {
                {
                    // Module declarations
                    #(#modules)*

                    // Function to register all models
                    |registry: &mut #path_prefix::models::ModelRegistry| {
                        log::info!("Auto-discovery: Registering {} model(s)", #model_count);
                        #(#function_calls)*
                    }
                }
            }
        }
        "middleware" => {
            let middleware_count = function_calls.len();
            quote! {
                {
                    // Module declarations
                    #(#modules)*

                    // Function to register all middleware
                    |registry: &mut #path_prefix::middleware::MiddlewareRegistry| {
                        log::info!("Auto-discovery: Registering {} middleware", #middleware_count);
                        #(#function_calls)*
                    }
                }
            }
        }
        "modules" => {
            quote! {
                {
                    // Module declarations (auto-discovery only)
                    #(#modules)*

                    log::info!("Auto-discovery: Found module(s) - explicit MODULE::register() required for registration");
                    // Modules are no longer auto-registered. Developers must explicitly call
                    // MODULE::register("unique-name", module_instance) with a unique name.
                    // This enables multiple instances of the same type with different configs.
                }
            }
        }
        "events" => {
            let event_count = function_calls.len();
            quote! {
                {
                    // Module declarations
                    #(#modules)*

                    // Function to register all event handlers
                    |emitter: &mut #path_prefix::events::EventEmitter| {
                        log::info!("Auto-discovery: Registering {} event handler(s)", #event_count);
                        #(#function_calls)*
                    }
                }
            }
        }
        "definitions" => {
            let definitions_count = function_calls.len();
            quote! {
                {
                    // Module declarations
                    #(#modules)*

                    // Function to register all definitions
                    |defs: &mut #path_prefix::definitions::Definitions| {
                        log::info!("Auto-discovery: Registering {} definition(s)", #definitions_count);
                        #(#function_calls)*
                    }
                }
            }
        }
        _ => quote! { compile_error!("Unknown directory type"); },
    }
}

/// Auto-discover and generate module declarations for the entire application
///
/// This attribute macro scans the filesystem at compile time and generates
/// standard Rust module declarations for modules, controllers, and models.
///
/// # Usage
/// ```rust,ignore
/// use rustf::prelude::*;
///
/// #[rustf::auto_discover]
/// #[tokio::main]
/// async fn main() -> rustf::Result<()> {
///     // Modules are now available via standard Rust module system
///     let user_svc = modules::user_service::UserService::new();
///
///     let app = RustF::new()
///         .controllers(auto_controllers!())
///         .start().await
/// }
/// ```
#[proc_macro_attribute]
pub fn auto_discover(_args: TokenStream, input: TokenStream) -> TokenStream {
    let mut input_fn = parse_macro_input!(input as ItemFn);

    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be set during build");
    let src_dir = PathBuf::from(&manifest_dir).join("src");

    // Generate module declarations for each directory
    let modules_mod = generate_mod_declarations(&src_dir, "modules");
    let controllers_mod = generate_mod_declarations(&src_dir, "controllers");
    let models_mod = generate_mod_declarations(&src_dir, "models");

    // Combine everything
    // Inject auto-discovery hook registration at the start of the function body
    // Note: Modules are no longer auto-registered via hooks. They're discovered via
    // auto_modules!() for IDE support, but developers must call MODULE::register() explicitly.
    let register_stmt = parse_quote! {
        rustf::auto::register_hooks(rustf::auto::AutoDiscoveryHooks {
            controllers: Some(|| auto_controllers!()),
            models: Some(|registry: &mut rustf::models::ModelRegistry| {
                let register = auto_models!();
                register(registry);
            }),
            shared: None,  // Modules are no longer auto-registered. Explicit registration only.
            middleware: Some(|registry: &mut rustf::middleware::MiddlewareRegistry| {
                let register = auto_middleware!();
                register(registry);
            }),
            events: Some(|emitter: &mut rustf::events::EventEmitter| {
                let register = auto_events!();
                register(emitter);
            }),
            definitions: Some(|defs: &mut rustf::definitions::Definitions| {
                let register = auto_definitions!();
                register(defs);
            }),
            workers: Some(auto_workers!()),
        });
    };

    let mut fn_block = (*input_fn.block).clone();
    fn_block.stmts.insert(0, register_stmt);
    input_fn.block = Box::new(fn_block);

    let expanded = quote! {
        // Auto-generated module declarations
        #modules_mod
        #controllers_mod
        #models_mod

        // Original function
        #input_fn
    };

    expanded.into()
}

/// Generate module declarations for a specific directory
fn generate_mod_declarations(src_dir: &PathBuf, dir_name: &str) -> proc_macro2::TokenStream {
    let target_dir = src_dir.join(dir_name);

    if !target_dir.exists() {
        return quote! {};
    }

    let mut module_declarations = Vec::new();
    let mut ide_content = String::new();

    ide_content.push_str("// Auto-generated by RustF #[auto_discover] macro - DO NOT EDIT\n");
    ide_content.push_str("// This file exists ONLY for IDE support - NOT imported by main.rs\n");
    ide_content.push_str(&format!(
        "// {} module declarations for autocomplete\n\n",
        dir_name
    ));

    // Scan for .rs files in the directory with max depth of 3
    for entry in WalkDir::new(&target_dir)
        .min_depth(1)
        .max_depth(3) // Support up to 3 levels of nesting
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // Skip directories
        if path.is_dir() {
            continue;
        }

        // Skip if not a .rs file
        if path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }

        // Skip IDE files, mod.rs files, and .inc.rs files
        if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
            if filename == "mod.rs" || filename.starts_with("_") || filename.ends_with(".inc.rs") {
                continue;
            }
        }

        // Get the relative path from target directory
        let relative_to_target = path
            .strip_prefix(&target_dir)
            .expect("Path should be under target directory");

        // Build nested module path components
        let mut path_components = Vec::new();
        for component in relative_to_target.components() {
            if let std::path::Component::Normal(name) = component {
                if let Some(name_str) = name.to_str() {
                    // Remove .rs extension from the last component
                    if name_str.ends_with(".rs") {
                        path_components.push(name_str.trim_end_matches(".rs"));
                    } else {
                        path_components.push(name_str);
                    }
                }
            }
        }

        if path_components.is_empty() {
            continue;
        }

        // Skip special files for models
        if dir_name == "models" {
            let module_name = path_components.last().unwrap();
            if module_name == &"MODELS_README" || module_name.contains("_generated") {
                continue;
            }
        }

        // Create unique module identifier (e.g., "api_users" for "api/users.rs")
        let module_ident_str = path_components.join("_");
        let module_ident = syn::Ident::new(&module_ident_str, proc_macro2::Span::call_site());

        // Generate module declaration with proper path handling
        if path_components.len() == 1 {
            // Simple top-level module - no path attribute needed
            module_declarations.push(quote! {
                pub mod #module_ident;
            });
        } else {
            // Nested module - use path attribute relative to the module we're in
            let relative_path = relative_to_target.to_string_lossy().replace('\\', "/");
            module_declarations.push(quote! {
                #[path = #relative_path]
                pub mod #module_ident;
            });
        }

        // Create proper relative path for IDE content
        let relative_path = relative_to_target.to_string_lossy().replace('\\', "/");
        ide_content.push_str(&format!("#[path = \"{}/{}\"]\n", dir_name, relative_path));
        ide_content.push_str(&format!("pub mod {};\n", module_ident_str));
    }

    // Handle special case for models with base subdirectory
    if dir_name == "models" {
        let base_dir = target_dir.join("base");
        if base_dir.exists() {
            let mut base_declarations = Vec::new();
            let mut base_content = String::new();

            for entry in WalkDir::new(&base_dir)
                .min_depth(1)
                .max_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();

                if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                    // Skip .inc.rs files (included code fragments)
                    if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                        if file_name.ends_with(".inc.rs") {
                            continue;
                        }
                    }

                    if let Some(module_name) = path.file_stem().and_then(|s| s.to_str()) {
                        let module_ident =
                            syn::Ident::new(module_name, proc_macro2::Span::call_site());
                        base_declarations.push(quote! {
                            pub mod #module_ident;
                        });
                        base_content
                            .push_str(&format!("    #[path = \"base/{}.rs\"]\n", module_name));
                        base_content.push_str(&format!("    pub mod {};\n", module_name));
                    }
                }
            }

            if !base_declarations.is_empty() {
                module_declarations.push(quote! {
                    pub mod base {
                        #(#base_declarations)*
                    }
                });

                // Add base module to IDE content
                ide_content.push_str("\n// Base models (auto-generated, do not edit)\n");
                ide_content.push_str("pub mod base {\n");
                ide_content.push_str(&base_content);
                ide_content.push_str("}\n");
            }
        }
    }

    // Write the IDE support file (NOT imported by main.rs)
    if !module_declarations.is_empty() {
        let ide_file_path = src_dir.join(format!("_{}.rs", dir_name));
        if let Err(e) = std::fs::write(&ide_file_path, &ide_content) {
            // Don't panic on write errors, just continue without the file
            eprintln!(
                "Warning: Failed to write {}: {}",
                ide_file_path.display(),
                e
            );
        }
    }

    if module_declarations.is_empty() {
        return quote! {};
    }

    // Return in-memory module declarations for compilation
    let dir_ident = syn::Ident::new(dir_name, proc_macro2::Span::call_site());
    quote! {
        mod #dir_ident {
            #(#module_declarations)*
        }
    }
}

/// Utility macro to include a directory of Rust files as modules
///
/// This is a more generic version that can be used for custom directory structures.
/// Note: This is a placeholder for future extensibility.
#[proc_macro]
pub fn include_dir_as_modules(_input: TokenStream) -> TokenStream {
    // For now, this is a placeholder - could be implemented later if needed
    quote! {
        compile_error!("include_dir_as_modules! is not yet implemented. Use auto_controllers! or auto_models! instead.");
    }.into()
}

/// Generate worker auto-discovery code
/// Returns an async closure compatible with workers_from()
fn generate_worker_discovery() -> proc_macro2::TokenStream {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be set during build");

    let src_dir = PathBuf::from(&manifest_dir).join("src").join("workers");

    // Check if we're in the rustf framework itself by looking for Cargo.toml [package] name
    let cargo_toml_path = PathBuf::from(&manifest_dir).join("Cargo.toml");
    let is_rustf_framework = if cargo_toml_path.exists() {
        if let Ok(contents) = std::fs::read_to_string(&cargo_toml_path) {
            contents.contains("name = \"rustf\"") && !contents.contains("name = \"rustf-")
        } else {
            false
        }
    } else {
        false
    };

    // If we're in the rustf framework itself, don't try to auto-discover workers
    // (workers/ is an internal module, not user-defined workers)
    let path_prefix = if is_rustf_framework {
        quote! { crate }
    } else {
        quote! { rustf }
    };

    if is_rustf_framework || !src_dir.exists() {
        // Return empty implementation
        return quote! {
            {
                |_manager: std::sync::Arc<#path_prefix::workers::WorkerManager>| {
                    Box::pin(async move {
                        // No workers to register
                        Ok(())
                    })
                }
            }
        };
    }

    let mut modules = Vec::new();
    let mut function_calls = Vec::new();

    // Walk through all .rs files in the workers directory
    for entry in WalkDir::new(&src_dir)
        .min_depth(1)
        .max_depth(3)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // Skip directories
        if path.is_dir() {
            continue;
        }

        // Skip if not a .rs file
        if path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }

        // Skip mod.rs files
        if path.file_name().and_then(|s| s.to_str()) == Some("mod.rs") {
            continue;
        }

        // Skip .inc.rs files and files starting with underscore
        if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
            if file_name.ends_with(".inc.rs") || file_name.starts_with("_") {
                continue;
            }
        }

        // Create the module path relative to the workers directory
        let relative_to_workers = path
            .strip_prefix(&src_dir)
            .expect("Path should be under workers directory");

        // Build nested module path
        let mut path_components = Vec::new();
        for component in relative_to_workers.components() {
            if let std::path::Component::Normal(name) = component {
                if let Some(name_str) = name.to_str() {
                    if name_str.ends_with(".rs") {
                        path_components.push(name_str.trim_end_matches(".rs"));
                    } else {
                        path_components.push(name_str);
                    }
                }
            }
        }

        if path_components.is_empty() {
            continue;
        }

        // Generate unique module identifier
        let module_ident_str = path_components.join("_");
        let module_ident = syn::Ident::new(&module_ident_str, proc_macro2::Span::call_site());

        // Create relative path from src directory
        let src_dir_base = PathBuf::from(&manifest_dir).join("src");
        let relative_path = path
            .strip_prefix(&src_dir_base)
            .expect("Path should be under src directory")
            .to_string_lossy()
            .replace('\\', "/");

        // Generate module declaration
        modules.push(quote! {
            #[path = #relative_path]
            pub mod #module_ident;
        });

        // Generate async install call
        function_calls.push(quote! {
            #module_ident::install().await?;
        });
    }

    let worker_count = function_calls.len();

    quote! {
        {
            // Module declarations
            #(#modules)*

            // Return async closure for workers_from()
            |_manager: std::sync::Arc<#path_prefix::workers::WorkerManager>| {
                Box::pin(async move {
                    log::info!("Auto-discovery: Loading {} worker(s)", #worker_count);
                    #(#function_calls)*
                    Ok(())
                })
            }
        }
    }
}
