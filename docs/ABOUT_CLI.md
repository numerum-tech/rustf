# RustF CLI Complete Guide

**Comprehensive documentation for rustf-cli - The AI-friendly command-line tool for RustF framework**

## Overview

RustF CLI is a powerful command-line tool designed for analyzing, managing, and developing RustF web applications. It provides comprehensive project analysis, database introspection, schema management, code generation, and includes built-in MCP (Model Context Protocol) server support for AI coding assistants.

### Key Features

- 🔍 **Project Analysis** - Deep inspection and validation of RustF applications
- 🤖 **AI Agent Integration** - MCP server with read-only mode for safe remote access
- 🗄️ **Database Tools** - Multi-database support (PostgreSQL, MySQL, SQLite)
- 📋 **Schema Management** - YAML-based schema validation and model generation
- 🚀 **Code Generation** - Create projects, controllers, modules, and events
- 💾 **Automatic Backups** - Safety backups when using `--force` flags
- 📊 **Performance Analysis** - Benchmarking and cache statistics
- 🔒 **Security First** - Read-only mode by default for remote access

## Installation

The RustF CLI is included with the RustF framework:

```bash
# Build from source
cd rustf-cli
cargo build --release

# The binary will be at: target/release/rustf-cli
# Add to PATH for system-wide access
export PATH="$PATH:/path/to/rustf/rustf-cli/target/release"
```

## Global Options

All commands support these global options:

```bash
-P, --project <PATH>     # Specify project directory (defaults to current)
-v, --verbose            # Enable verbose output for debugging
-h, --help              # Show help information
-V, --version           # Show version information
```

## Command Reference

The CLI provides 9 main commands, each with specific functionality:

### 1. `analyze` - Project Component Analysis

Analyze various aspects of your RustF project.

```bash
rustf-cli analyze <SUBCOMMAND>
```

**Subcommands:**

- **`backups`** - List and analyze project backups
  ```bash
  rustf-cli analyze backups [--detailed]
  # Shows backups created by --force operations
  ```

- **`controllers`** - List controllers and their handlers
  ```bash
  rustf-cli analyze controllers [-n <NAME>]
  ```

- **`discover`** - Quick project discovery and overview
  ```bash
  rustf-cli analyze discover [-f <FILTER>]
  ```

- **`middleware`** - Analyze middleware chain and execution order
  ```bash
  rustf-cli analyze middleware [--conflicts]
  ```

- **`models`** - Discover and analyze models
  ```bash
  rustf-cli analyze models [--relationships]
  ```

- **`project`** - Complete project analysis
  ```bash
  rustf-cli analyze project [--detailed] [-f <FORMAT>]
  # FORMAT: table (default), json, yaml
  ```

- **`routes`** - Display route tree with parameters
  ```bash
  rustf-cli analyze routes [--conflicts-only] [--validate]
  ```

- **`views`** - Analyze views and templates
  ```bash
  rustf-cli analyze views [--layout] [-n <NAME>] [--security]
  ```

### 2. `db` - Database Operations

Introspect and manage database schemas.

```bash
rustf-cli db <SUBCOMMAND>
```

**Subcommands:**

- **`describe`** - Describe table structure
  ```bash
  rustf-cli db describe <TABLE_NAME> [--connection <NAME>] [--format <FORMAT>]
  ```

- **`diff-schema`** - Compare database with schema files
  ```bash
  rustf-cli db diff-schema <SCHEMA_FILE> [--connection <NAME>]
  ```

- **`export-data`** - Export table data
  ```bash
  rustf-cli db export-data <TABLE_NAME> [--format json|csv] [--limit <N>] [-o <FILE>]
  ```

- **`generate-schema`** - Generate YAML schemas from database
  ```bash
  rustf-cli db generate-schema [-o <DIR>] [--force] [--tables <TABLE1,TABLE2>]
  # ⚠️ --force creates backups in .rustf/backups/schemas/
  ```

- **`list-tables`** - List all database tables
  ```bash
  rustf-cli db list-tables [--metadata] [--format table|json]
  ```

- **`test-connection`** - Test database connectivity
  ```bash
  rustf-cli db test-connection [--connection <NAME>]
  ```

### 3. `export` - Export Project Analysis

Export project analysis in various formats.

```bash
rustf-cli export [-f <FORMAT>] [--include-code] [-o <FILE>]
# FORMAT: json (default), yaml, markdown
```

### 4. `new` - Create New Components

Generate new RustF components with proper structure.

```bash
rustf-cli new <SUBCOMMAND>
```

**Subcommands:**

- **`controller`** - Generate controller(s)
  ```bash
  rustf-cli new controller -n <NAMES> [--crud] [--routes]
  # Example: rustf-cli new controller -n "UserController,PostController" --crud
  ```

- **`event`** - Generate event handler
  ```bash
  rustf-cli new event -n <NAME> [--lifecycle] [--custom]
  ```

- **`middleware`** - Generate middleware component
  ```bash
  rustf-cli new middleware -n <NAME> [--auth] [--logging] [-p <PRIORITY>]
  
  Options:
    -n, --name <NAME>        # Middleware name (required)
    --auth                   # Include authentication example with protected paths
    --logging                # Include request/response logging example
    -p, --priority <PRIORITY> # Execution priority (default: 0)
                            # Negative = runs early, Positive = runs late
  
  Examples:
    # Basic middleware
    rustf-cli new middleware -n rate-limit
    
    # Auth middleware with early execution
    rustf-cli new middleware -n auth --auth --priority=-50
    
    # Logging middleware (runs very early)
    rustf-cli new middleware -n request-logger --logging --priority=-100
  ```

- **`module`** - Generate service/module
  ```bash
  rustf-cli new module -n <NAME> [--shared] [--with-methods]
  ```

- **`project`** - Create new RustF project
  ```bash
  rustf-cli new project <PROJECT_NAME> [--path <DIR>] [--force]
  # ⚠️ --force creates backup of existing project in .rustf/backups/project/
  ```

### 5. `perf` - Performance Analysis

Analyze application performance characteristics.

```bash
rustf-cli perf <SUBCOMMAND>
```

**Subcommands:**

- **`benchmark`** - Run performance benchmarks
  ```bash
  rustf-cli perf benchmark [--iterations <N>]
  ```

- **`cache-stats`** - Display cache statistics
  ```bash
  rustf-cli perf cache-stats
  ```

- **`stream`** - Streaming performance analysis
  ```bash
  rustf-cli perf stream [--memory-limit <MB>] [--chunk-size <KB>]
  ```

### 6. `query` - Query Specific Items

Query specific components or metadata.

```bash
rustf-cli query <ITEM_TYPE> <ITEM_NAME> [-f <FORMAT>]
# ITEM_TYPE: controller, handler, middleware, model, model-metadata, route, view
```

**Example:**
```bash
rustf-cli query model-metadata User --format json
# Returns field hints, validation rules, and type information
```

### 7. `schema` - Schema Management

Manage YAML schemas and generate code.

```bash
rustf-cli schema <SUBCOMMAND>
```

**Subcommands:**

- **`analyze`** - Analyze schema structure
  ```bash
  rustf-cli schema analyze [-s <SCHEMA_PATH>]
  ```

- **`check-consistency`** - Verify schema/code consistency
  ```bash
  rustf-cli schema check-consistency
  ```

- **`generate`** - Generate code from schemas
  ```bash
  rustf-cli schema generate <TARGET> [--schema-path <DIR>] [-o <DIR>] [--force]
  # TARGET: models, migrations, postgres, mysql, sqlite
  # ⚠️ --force creates backups in .rustf/backups/models/
  ```

- **`validate`** - Validate schema files
  ```bash
  rustf-cli schema validate [-p <SCHEMA_PATH>]
  ```

- **`watch`** - Auto-regenerate on schema changes
  ```bash
  rustf-cli schema watch [-s <SCHEMA_PATH>] [-o <OUTPUT>]
  ```

### 8. `serve` - MCP Server Management

Start MCP server for AI agent integration.

```bash
rustf-cli serve <SUBCOMMAND>
```

**Subcommands:**

- **`start`** - Start MCP server
  ```bash
  rustf-cli serve start [OPTIONS]
  
  Options:
    --allow-writes       # Enable write operations (default: false - READ-ONLY)
    --auto-port         # Find available port automatically
    --bind <ADDRESS>    # Bind address (default: 127.0.0.1)
    -n, --name <NAME>   # Instance name for multiple servers
    --port <PORT>       # Server port (default: 3000)
    -w, --watch         # Enable file watching
    --websocket         # Enable WebSocket support
  ```

- **`list`** - List running MCP servers
  ```bash
  rustf-cli serve list
  ```

- **`stop`** - Stop MCP server
  ```bash
  rustf-cli serve stop <PORT>
  ```

### 9. `validate` - Project Validation

Validate project structure and conventions.

```bash
rustf-cli validate [--fix] [-w, --watch]
# --fix: Auto-fix issues where possible
# --watch: Continuous validation mode
```

## AI Agent Integration (MCP Server)

The MCP server enables AI agents to interact with your RustF project safely.

### Starting the Server

**Safe Remote Access (Read-Only - Default):**
```bash
# Start read-only server for remote access
rustf-cli serve start --bind 0.0.0.0 --port 8080

🔒 Starting MCP Server in READ-ONLY mode (safe for remote)
   Available: analyze, query, validate, db list/describe
   Blocked: generate, new, fix, migrate
```

**Local Development (With Write Access):**
```bash
# Enable writes for local development only
rustf-cli serve start --allow-writes

🔓 Starting MCP Server with WRITE operations enabled
   Available: All CLI commands including generate, new, fix
```

### MCP Protocol Usage

The server exposes a single `rustf_cli_execute` endpoint that wraps all CLI commands:

```json
// Request format
{
  "method": "rustf_cli_execute",
  "params": {
    "command": "analyze",
    "subcommand": "project",
    "args": ["--format", "json"]
  }
}

// Response format
{
  "status": "success",
  "data": { /* command output */ },
  "metadata": {
    "command_executed": "rustf-cli analyze project --format json",
    "execution_time_ms": 145,
    "read_only_mode": true,
    "command_type": "read_only"
  }
}
```

### Safety Features

1. **Read-Only by Default**: Server starts in read-only mode unless explicitly enabled
2. **Public Interface Warning**: 5-second delay when enabling writes on public interfaces
3. **Command Classification**: Automatic blocking of write operations in read-only mode
4. **Audit Logging**: All blocked write attempts are logged

## Backup System

The CLI automatically creates backups when using `--force` flags to prevent data loss.

### How It Works

When you use `--force` with commands that overwrite files:

1. **Automatic Backup**: Creates timestamped backup in `.rustf/backups/`
2. **Organized Storage**: Backups organized by type (models, schemas, project)
3. **Manual Restore**: Intentionally manual to ensure conscious restoration

### Backup Locations

```
.rustf/
└── backups/
    ├── models/
    │   └── 2024-01-15T10-30-00Z/
    ├── schemas/
    │   └── 2024-01-15T11-00-00Z/
    └── project/
        └── 2024-01-15T12-00-00Z/
```

### Managing Backups

```bash
# List all backups
rustf-cli analyze backups

# Manual restoration (intentionally not automated)
cp -r .rustf/backups/models/2024-01-15T10-30-00Z/* src/models/
```

## Common Workflows

### Setting Up a New Project

```bash
# 1. Create new project
rustf-cli new project my-app

# 2. Configure database in config.toml or .env
export DATABASE_URL="postgresql://user:pass@localhost/myapp"

# 3. Generate schemas from existing database
rustf-cli db generate-schema -o schemas

# 4. Generate models from schemas
rustf-cli schema generate models -o src/models

# 5. Create essential middleware
rustf-cli new middleware -n auth --auth --priority=-50
rustf-cli new middleware -n request-logger --logging --priority=-100

# 6. Start development server
cargo run
```

### Analyzing an Existing Project

```bash
# 1. Quick discovery
rustf-cli analyze discover

# 2. Detailed project analysis
rustf-cli analyze project --detailed --format json

# 3. Check for issues
rustf-cli validate

# 4. Analyze routes for conflicts
rustf-cli analyze routes --conflicts-only
```

### Database-First Development

```bash
# 1. Test connection
rustf-cli db test-connection

# 2. List tables
rustf-cli db list-tables --metadata

# 3. Generate schemas from database
rustf-cli db generate-schema --force

# 4. Generate models
rustf-cli schema generate models --force

# 5. Verify consistency
rustf-cli schema check-consistency
```

### Creating Middleware Components

```bash
# 1. Generate authentication middleware
rustf-cli new middleware -n auth --auth --priority=-50

# 2. Generate logging middleware
rustf-cli new middleware -n request-logger --logging --priority=-100

# 3. Generate rate limiting middleware
rustf-cli new middleware -n rate-limit --priority=-75

# 4. Generate custom security headers middleware
rustf-cli new middleware -n security-headers --priority=100

# Note: Priority determines execution order:
# - Negative values run early (e.g., logging, auth)
# - Positive values run late (e.g., response modification)
# - Zero is default (normal priority)
```

### AI Agent Development

```bash
# For remote project analysis (safe)
rustf-cli serve start --bind 0.0.0.0

# For local AI-assisted development
rustf-cli serve start --allow-writes --watch

# AI agent can then connect and execute commands:
# - Analyze project structure
# - Query specific components
# - Generate new code (if writes enabled)
# - Validate changes
```

## Output Formats

Most commands support multiple output formats:

- **table** (default) - Human-readable tables
- **json** - Machine-readable JSON
- **yaml** - YAML format
- **markdown** - Documentation-ready markdown

Example:
```bash
rustf-cli analyze project --format json > analysis.json
rustf-cli db list-tables --format yaml
```

## Environment Variables

The CLI respects these environment variables:

```bash
DATABASE_URL           # Database connection string
RUST_LOG              # Logging level (error, warn, info, debug, trace)
RUSTF_AUTH_TOKEN      # Authentication token for MCP server
```

## Troubleshooting

### Common Issues

1. **Database Connection Failed**
   - Check DATABASE_URL is set correctly
   - Verify database is running
   - Check network connectivity

2. **Schema Generation Errors**
   - Ensure schemas directory exists
   - Check file permissions
   - Validate YAML syntax

3. **MCP Server Port In Use**
   - Use `--auto-port` to find available port
   - Check with `rustf-cli serve list`
   - Stop existing server with `rustf-cli serve stop <PORT>`

4. **Backup Recovery**
   - Backups are in `.rustf/backups/`
   - Use standard file copy commands
   - Check `.rustf/README.md` for instructions

### Debug Mode

Enable verbose output for debugging:
```bash
RUST_LOG=debug rustf-cli -v analyze project
```

## Best Practices

1. **Always Use Backups**: When using `--force`, check `.rustf/backups/` afterward
2. **Read-Only for Remote**: Never use `--allow-writes` on public interfaces
3. **Validate Often**: Run `rustf-cli validate` before commits
4. **Schema-First**: Keep schemas in version control
5. **Use Watch Mode**: `rustf-cli schema watch` for auto-regeneration

## Version History

- **v0.1.0** - Initial release with core functionality
- **v0.2.0** - Added MCP server and backup system
- **v0.3.0** - Component generation and improved safety features
- **v0.3.1** - Added middleware generation to `new` command with auth/logging templates

## License

Part of the RustF framework. See LICENSE file for details.

---

*For framework documentation, see the [RustF Framework Guide](./ABOUT_RUSTF.md)*
*For database tools, see the [Database Tools Guide](./DATABASE_TOOLS_GUIDE.md)*
*For event system, see the [Event System Guide](./EVENT_SYSTEM_GUIDE.md)*