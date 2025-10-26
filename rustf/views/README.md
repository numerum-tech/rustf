# Framework Views Directory

This directory contains placeholder files to enable the `embedded-views` feature compilation in the RustF framework.

## Purpose

The embedded-views feature requires a `views/` directory to exist at compile time. This directory serves that purpose for the framework itself.

## For Application Developers

When using RustF in your application:

1. Create your own `views/` directory in your application root
2. Place your actual templates there
3. Enable the `embedded-views` feature in your Cargo.toml:
   ```toml
   [dependencies]
   rustf = { version = "0.1", features = ["embedded-views"] }
   ```
4. Use the embedded view engine:
   ```rust
   let view_engine = ViewEngine::totaljs_embedded();
   ```

Your application's templates will be embedded into your application binary at compile time, not the placeholder files from this directory.

## Convention

By convention, RustF looks for templates in the `views/` directory as configured in your application's `views.directory` setting (default: "views").