# Installation

This guide covers multiple ways to install and set up RustF for development.

## Method 1: Using RustF CLI (Recommended)

The easiest way to create a new RustF project is using the CLI tool:

### Install the RustF CLI

**Prebuilt binary — fastest, no compile** (recommended):

```bash
# install the binstall helper once, if you don't have it:
cargo install cargo-binstall
# then fetch the prebuilt rustf-cli binary from the GitHub Release:
cargo binstall rustf-cli
```

**From crates.io** (compiles from source — needs a few minutes):

```bash
cargo install rustf-cli
```

**From source** (pre-release / local development):

```bash
git clone https://github.com/numerum-tech/rustf.git
cargo install --path rustf/rustf-cli   # repo root is the clone dir 'rustf', crate is 'rustf-cli'
```

> Until `rustf-cli` is published to crates.io, use the **from source** method.
> After publishing, `cargo binstall rustf-cli` is the quickest path.

### Create a New Project

```bash
# Create a new project
rustf-cli new project my-app

# Navigate to your project
cd my-app

# Run the server
cargo run
```

The CLI tool will:
- Set up the correct project structure
- Create necessary directories (controllers, models, views, etc.)
- Generate a basic `main.rs` with auto-discovery
- Create a sample controller
- Set up configuration files

## Method 2: Manual Setup

If you prefer to set up a project manually:

### 1. Create a New Cargo Project

```bash
cargo new my-app
cd my-app
```

### 2. Add Dependencies

Edit `Cargo.toml`:

```toml
[package]
name = "my-app"
version = "0.1.0"
edition = "2021"

[dependencies]
rustf = { path = "../rustf" }  # Or from crates.io when published
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
log = "0.4"
env_logger = "0.10"
```

### 3. Create Project Structure

```bash
mkdir -p src/controllers src/models src/modules src/middleware
mkdir -p views/layouts public/css public/js
mkdir -p schemas uploads
```

### 4. Create Main Application

Create `src/main.rs`:

```rust
use rustf::prelude::*;

#[tokio::main]
async fn main() -> rustf::Result<()> {
    env_logger::init();
    
    let app = RustF::new()
        .controllers(auto_controllers!())
        .middleware_from(auto_middleware!());
    
    println!("🚀 Server at http://127.0.0.1:8000");
    app.start().await
}
```

### 5. Create Your First Controller

Create `src/controllers/home.rs`:

```rust
use rustf::prelude::*;

pub fn install() -> Vec<Route> {
    routes![
        GET "/" => index,
    ]
}

async fn index(ctx: &mut Context) -> Result<()> {
    ctx.html("<h1>Hello, RustF!</h1>")
}
```

### 6. Create Configuration

Create `config.toml`:

```toml
[server]
host = "127.0.0.1"
port = 8000

[views]
directory = "views"
cache_enabled = false
```

## Method 3: Using Git Template

You can also use the sample application as a template:

```bash
# Clone the repository
git clone https://github.com/numerum-tech/rustf.git
cd rustf/sample-app

# Copy to your project
cp -r . /path/to/your/project
cd /path/to/your/project

# Install dependencies and run
cargo run
```

## Cargo Features

RustF ships its SQL drivers and Redis support as **optional, on-by-default**
features. Out of the box you get everything (PostgreSQL, MySQL, SQLite, and
Redis sessions), so most users need no configuration. If you want a leaner
build, opt out and enable only what you use.

| Feature | Default | What it enables |
|---------|:-------:|-----------------|
| `database` | ✅ | Driverless SQL core (query builder, models, migrations) + `decimal` |
| `db-postgres` | ✅ | PostgreSQL driver |
| `db-mysql` | ✅ | MySQL / MariaDB driver |
| `db-sqlite` | ✅ | SQLite driver |
| `redis` | ✅ | Redis-backed session storage (the only built-in cross-instance/cluster store) |
| `embedded-views` | ✅ | Compile templates into the binary |
| `schema` | ✅ | YAML schema validation & codegen |
| `decimal` | ✅ | `DECIMAL`/`NUMERIC` support via `rust_decimal` — **implied by `database`**, so any `db-*` feature pulls it in automatically |
| `cli` | — | `clap`-based CLI argument parsing |

> **Always compiled (not features):** TOML config loading, the auto-discovery
> macros (`auto_controllers!`, …), and UUID support are core to the framework
> and cannot be disabled — there are no `config` / `auto-discovery` / `uuid`
> flags.

Each `db-*` feature pulls in `database` automatically and turns on the matching
`sqlx` driver. A PostgreSQL-only app, for example, can skip compiling the MySQL
and SQLite drivers plus Redis:

```toml
[dependencies]
rustf = { version = "1.0.0-rc1", default-features = false, features = [
    "embedded-views", "schema",
    "db-postgres",   # pulls in `database` + `decimal` automatically
] }
```

> **Note — clustering:** `redis` is the only built-in storage that lets multiple
> RustF instances share session state. If you disable it for a multi-instance
> deployment, provide your own `SessionStorage` implementation for shared
> sessions.

## Verifying Installation

After installation, verify everything works:

```bash
# Build the project
cargo build

# Run tests
cargo test

# Start the server
cargo run
```

You should see:
```
🚀 Server at http://127.0.0.1:8000
```

Visit `http://127.0.0.1:8000` in your browser to see your application.

## Development Dependencies

For development, you may also want to add:

```toml
[dev-dependencies]
tokio-test = "0.4"
tempfile = "3.8"
```

## Troubleshooting

### Common Issues

**Issue: "cannot find crate `rustf`"**
- Solution: Make sure you've added rustf to `Cargo.toml` dependencies
- If using local development: Use `path = "../rustf"` in Cargo.toml
- If using published version: Use `version = "1.0.0-rc1"` from crates.io

**Issue: "proc macro not found"**
- Solution: Make sure `rustf` is a dependency (the auto-discovery macros are
  re-exported from `rustf` and always available — there is no feature to enable)
- Bring them into scope with `use rustf::prelude::*;`

**Issue: "port already in use"**
- Solution: Change the port in `config.toml` or stop the other process
- Default port is 8000

## Next Steps

Now that you have RustF installed:

1. **[Hello World Tutorial](hello-world.md)** - Build your first application
2. **[Project Structure](project-structure.md)** - Understand the framework layout
3. **[Controllers Guide](../guides/controllers.md)** - Learn about routing and controllers











