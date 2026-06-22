use clap::Subcommand;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub enum NewCommand {
    /// Generate controller(s) with routing
    Controller {
        /// Add sample CRUD operations
        #[arg(long)]
        crud: bool,

        /// Controller name(s) - comma-separated for multiple
        #[arg(short, long)]
        names: String,

        /// Add sample routes
        #[arg(long)]
        routes: bool,
    },

    /// Generate an event handler
    Event {
        /// Add custom event samples
        #[arg(long)]
        custom: bool,

        /// Add lifecycle event samples
        #[arg(long)]
        lifecycle: bool,

        /// Event handler name
        #[arg(short, long)]
        name: String,
    },

    /// Generate a middleware component
    Middleware {
        /// Middleware name
        #[arg(short, long)]
        name: String,

        /// Include authentication example
        #[arg(long)]
        auth: bool,

        /// Include logging example
        #[arg(long)]
        logging: bool,

        /// Execution priority (lower = earlier, default: 0)
        #[arg(short, long, default_value = "0")]
        priority: i32,
    },

    /// Generate a utility module or shared service
    Module {
        /// Module name
        #[arg(short, long)]
        name: String,

        /// Generate as a shared service (SharedModule singleton)
        /// Default: generate as simple utility module
        #[arg(long)]
        shared: bool,

        /// Add sample methods
        #[arg(long)]
        with_methods: bool,
    },

    /// Create a new RustF project with AI-friendly structure and documentation
    Project {
        /// Project name (will be converted to snake_case for directory)
        project_name: String,

        /// Overwrite existing non-empty directory
        #[arg(short, long)]
        force: bool,

        /// Target directory path (defaults to current directory)
        #[arg(long)]
        path: Option<PathBuf>,
    },

    /// Generate a background worker for async task execution
    Worker {
        /// Worker name (will be converted to kebab-case for registration)
        #[arg(short, long)]
        name: String,
    },

    /// Generate a full CRUD scaffold enforcing the
    /// Base Model -> Model -> Module -> Controller layering rule.
    ///
    /// Scaffolds the HTTP + business layers around an EXISTING model:
    /// a thin controller, a business-logic module (only caller of the
    /// model), four views (index/show/new/edit), and an integration-test
    /// stub. Requires src/models/<name>.rs to already exist (run
    /// `schema generate models` first); it never creates or edits the model.
    Crud {
        /// Feature name (plural form, e.g. "posts", "users", "articles").
        /// Used as-is for routes, views dir, and controller/module filenames.
        #[arg(short, long)]
        name: String,
    },
}
