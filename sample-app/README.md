# Sample App Smoke

A web application built with the RustF framework - an AI-friendly MVC framework for Rust inspired by Total.js.

## 🚀 Quick Start

### Prerequisites
- Rust 1.70+ installed
- Cargo package manager

### Running the Application

```bash
# Install dependencies and run
cargo run

# The server will start at http://127.0.0.1:8000
# Visit the URL to see your application
```

### Development Commands

```bash
# Run in development mode
cargo run

# Build for production
cargo build --release

# Run tests
cargo test

# Format code
cargo fmt

# Check for issues
cargo clippy
```

## 📁 Project Structure

This project follows RustF's convention-over-configuration approach:

### Core Application
- `src/main.rs` - Application entry point with auto-discovery
- `config.toml` - Configuration file (see CONFIGURATION.md for details)

### MVC Components (Auto-Discovered)
- `src/controllers/` - Request handlers and route definitions
- `src/models/` - Data models and business logic
  - `src/models/base/` - Generated models from schemas (don't edit directly)
- `src/middleware/` - Custom middleware components

### Views and Assets
- `views/` - HTML templates (Total.js template engine)
  - `views/layouts/` - Layout templates
- `public/` - Static assets (CSS, JavaScript, images)

### Data and Storage  
- `schemas/` - Data model definitions (YAML format)
- `private/uploads/` - Private file upload storage (gitignored)

## 🤖 AI-Friendly Development

This project is optimized for AI coding assistants:

- **Auto-Discovery**: No manual module declarations needed
- **Convention-based**: Predictable file organization
- **Comprehensive Documentation**: Each directory has detailed READMEs
- **Template Patterns**: Consistent code patterns throughout

### Common AI Queries
- "Add a new controller" → Create files in `src/controllers/`
- "Create a data model" → Define schema in `schemas/`, generate with CLI
- "Add middleware" → Create files in `src/middleware/`
- "Style the application" → Edit files in `public/css/`

## 🛠️ Framework Features

- **Total.js-inspired API**: Familiar `ctx.param_int()`, `ctx.json()`, `ctx.view()` patterns
- **Auto-Discovery**: Automatic registration of controllers, models, and middleware
- **Built-in Middleware**: Session by default; opt-in logging, CORS, rate limiting, CSRF, compression, and method override
- **Session Management**: Flash messages and persistent session data
- **Template Engine**: Total.js-style views with layout support
- **Static File Serving**: Efficient asset delivery
- **Configuration**: File-based, environment, or programmatic configuration

## 📖 Documentation

- Framework documentation: [RustF Book](https://numerum-tech.github.io/rustf/)
- Each directory contains detailed READMEs with AI-friendly guidance
- Check `src/controllers/README.md` for controller patterns
- Check `src/models/README.md` for model development
- Check `schemas/README.md` for data modeling

## 🔧 Configuration

Configuration is loaded in this order:
1. `config.toml` file (if present)
2. Environment variables (`RUSTF_*` prefixes)
3. Default values

Key configuration sections:
- `[server]` - Host, port, SSL settings
- `[views]` - Template engine configuration  
- `[session]` - Session and security settings
- `[database]` - Database connection settings
- `[static_files]` - Asset serving configuration

## 🚀 Deployment

### Development
```bash
cargo run
```

### Production
```bash
# Build optimized binary
cargo build --release

# Run with production config
RUSTF_ENV=production ./target/release/sample_app_smoke
```

## 📝 License

This project is licensed under the MIT License - see the LICENSE file for details.

---

Built with ❤️ using [RustF Framework](https://github.com/numerum-tech/rustf)
