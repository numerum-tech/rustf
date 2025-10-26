# RustF CLI - AI-Friendly RustF Framework Analysis Tool

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A comprehensive command-line tool designed specifically for AI coding agents to analyze, understand, and interact with RustF framework projects. Features include AST-based code analysis, real-time file watching, and Model Context Protocol (MCP) server integration.

## 🚀 Quick Start

```bash
# Install from source
git clone <repository-url>
cd rustf-cli
cargo build --release

# Analyze a RustF project
./target/release/rustf-cli analyze --project /path/to/rustf-project

# Start MCP server for AI agent integration
./target/release/rustf-cli serve --port 3000 --watch

# Watch files for real-time changes
./target/release/rustf-cli watch --project /path/to/rustf-project
```

## 📋 Features

### 🔍 Project Analysis
- **Complete Project Scanning**: Automatically discovers controllers, routes, handlers, middleware, models, and views
- **AST-Based Analysis**: Deep code analysis using Rust's `syn` crate for accurate parsing
- **Route Tree Generation**: Builds comprehensive routing tables with conflict detection
- **Handler Analysis**: Analyzes function signatures, Context API usage, and complexity metrics
- **View Template Analysis**: Parses HTML templates, detects security issues, extracts variables
- **Cross-Reference Analysis**: Maps relationships between components (views ↔ controllers, routes ↔ handlers)

### 🔄 Real-Time Monitoring
- **File Watcher**: Monitors source files, templates, and configuration changes
- **Component Impact Analysis**: Identifies which framework components are affected by file changes
- **Event Aggregation**: Summarizes changes over time windows
- **Real-Time Notifications**: Instant updates for connected AI agents

### 🤖 AI Agent Integration
- **MCP Server**: JSON-RPC 2.0 compliant Model Context Protocol server
- **RESTful API**: HTTP endpoints for project analysis and querying
- **WebSocket Support**: Real-time updates and bidirectional communication
- **Structured Data Export**: JSON, YAML, and Markdown formats optimized for AI consumption

## 🛠️ Installation

### Prerequisites
- Rust 1.70+ with Cargo
- Git (for cloning the repository)

### From Source
```bash
git clone <repository-url>
cd rustf-cli
cargo build --release
```

### Development Build
```bash
cargo build
./target/debug/rustf-cli --help
```

## 📖 Usage

### Command Overview

```bash
rustf-cli [OPTIONS] <COMMAND>

Commands:
  analyze      Complete project analysis with detailed reporting
  discover     Quick project overview and component discovery
  routes       Route tree analysis with conflict detection
  controllers  Controller and handler analysis
  middleware   Middleware chain analysis
  models       Model discovery and analysis
  views        Template analysis with security checking
  query        Query specific components by name
  export       Export analysis data in various formats
  validate     Project structure validation
  serve        Start MCP server for AI agents
  watch        Real-time file monitoring
```

### Basic Analysis

```bash
# Complete project analysis
rustf-cli analyze

# Quick discovery
rustf-cli discover

# Route analysis with conflict detection
rustf-cli routes --validate --conflicts-only

# Controller analysis
rustf-cli controllers --name home

# Query specific components
rustf-cli query handler home::index
rustf-cli query route "GET /"
rustf-cli query view login
```

### Export and Integration

```bash
# Export to JSON for AI consumption
rustf-cli export --format json --output analysis.json --include-code

# Export to Markdown documentation
rustf-cli export --format markdown --output docs.md

# Export to YAML for configuration
rustf-cli export --format yaml --output config.yaml
```

### MCP Server Mode

```bash
# Start MCP server
rustf-cli serve --port 3000 --bind 127.0.0.1

# Start with file watching
rustf-cli serve --port 3000 --watch

# Start with WebSocket support
rustf-cli serve --port 3000 --websocket --watch
```

### File Watching

```bash
# Watch for file changes
rustf-cli watch

# Watch with verbose output
rustf-cli --verbose watch
```

## 📦 Installation

```bash
# Clone and build
git clone <repository>
cd rustf-cli
cargo build --release

# Install globally
cargo install --path .
```

## 🎯 **Use Cases**

### **For AI Coding Assistants**
- Real-time project understanding
- Route conflict detection
- Missing handler identification
- Code generation assistance

### **For Developers**
- Project validation and linting
- Architecture analysis
- Documentation generation
- CI/CD integration

## 🔧 **Current Status**

### ✅ **Implemented**
- [x] CLI framework with comprehensive commands
- [x] Basic project detection and analysis
- [x] MCP server with JSON-RPC protocol
- [x] WebSocket support for real-time updates
- [x] Multiple export formats (JSON, YAML, table)
- [x] Integration tested with rustf-example project

### 🚧 **In Development**
- [ ] Advanced AST parsing for route extraction
- [ ] Route conflict detection and analysis
- [ ] Handler function signature analysis
- [ ] Middleware chain analysis
- [ ] File watching with real-time notifications

### 🎯 **Planned Features**
- [ ] GitHub Actions integration
- [ ] VSCode extension support
- [ ] Claude Desktop integration examples
- [ ] Comprehensive test suite

## 🤝 **AI Agent Integration**

This tool is the **first framework-specific MCP server** designed for AI agents working with RustF projects. It provides:

- **Deep Framework Knowledge**: Understanding of RustF conventions
- **Real-time Updates**: Live project analysis as code changes
- **Structured Data**: Machine-readable project information
- **Intelligent Suggestions**: Missing handlers, route conflicts, best practices

## 📖 **Example Output**

```bash
$ rustf-cli analyze --project ../rustf-example

=== RustF Project Analysis ===

Project: rustf-example
Framework Version: detected
Controllers: 3
Routes: 8
Middleware: 4
Models: 2
```

## 🔗 **Integration with RustF Framework**

This CLI tool is part of the RustF ecosystem:
- **RustF Core**: The main framework library
- **RustF Macros**: Auto-discovery procedural macros
- **RustF Example**: Reference implementation
- **RustF CLI**: This analysis and development tool

Perfect for AI-assisted development workflows with the RustF framework!