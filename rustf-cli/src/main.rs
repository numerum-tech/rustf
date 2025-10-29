use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Clone, ValueEnum)]
enum ItemType {
    Controller,
    Handler,
    Middleware,
    Model,
    ModelMetadata,
    Route,
    View,
}

mod analysis;
mod analyzer;
mod commands;
mod export;
mod mcp;
mod routes;
mod utils;
mod watcher;

use commands::*;

#[derive(Parser)]
#[command(name = "rustf-cli")]
#[command(about = "CLI tool for analyzing RustF projects with MCP server support for AI agents")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Project directory (defaults to current directory)
    #[arg(short = 'P', long, global = true)]
    project: Option<PathBuf>,

    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze project components
    Analyze {
        #[command(subcommand)]
        command: commands::analyze_cmd::AnalyzeCommand,
    },

    /// Database operations (introspection, schema generation)
    Db(commands::db::DbCommand),

    /// Export project analysis in various formats
    Export {
        /// Output format (json, yaml, markdown)
        #[arg(short, long, default_value = "json")]
        format: String,

        /// Include source code snippets
        #[arg(long)]
        include_code: bool,

        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Create new RustF components (project, controller, module, event)
    New {
        #[command(subcommand)]
        command: commands::new_cmd::NewCommand,
    },

    /// Performance analysis and benchmarking
    Perf {
        #[command(subcommand)]
        command: commands::perf_cmd::PerfCommand,
    },

    /// Query specific items or metadata
    Query {
        /// Item type to query (route, handler, view, middleware, model, controller, model-metadata)
        #[arg(value_enum)]
        item_type: ItemType,

        /// Name or path of the item to query
        item_name: String,

        /// Output format (for model-metadata queries)
        #[arg(short, long, default_value = "json")]
        format: Option<String>,
    },

    /// Schema management (validate, analyze, generate code)
    Schema(commands::schema::SchemaCommand),

    /// Translation management (scan views, update resources)
    Translations {
        #[command(subcommand)]
        command: commands::translations::TranslationsCommand,
    },

    /// MCP server management
    Serve {
        #[command(subcommand)]
        command: commands::serve_cmd::ServeCommand,
    },

    /// Validate project structure and conventions
    Validate {
        /// Fix auto-fixable issues
        #[arg(long)]
        fix: bool,

        /// Watch for changes and validate continuously
        #[arg(short, long)]
        watch: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    if cli.verbose {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    } else {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }

    let project_path = cli
        .project
        .unwrap_or_else(|| std::env::current_dir().unwrap());

    match cli.command {
        Commands::Analyze { command } => {
            use commands::analyze_cmd::AnalyzeCommand;
            match command {
                AnalyzeCommand::Backups { detailed } => {
                    commands::backups::run(project_path, detailed).await
                }
                AnalyzeCommand::Controllers { name } => controllers::run(project_path, name).await,
                AnalyzeCommand::Discover { filter } => discover::run(project_path, filter).await,
                AnalyzeCommand::Middleware { conflicts } => {
                    middleware::run(project_path, conflicts).await
                }
                AnalyzeCommand::Models { relationships } => {
                    models::run(project_path, relationships).await
                }
                AnalyzeCommand::Project { detailed, format } => {
                    analyze::run(project_path, format, detailed).await
                }
                AnalyzeCommand::Routes {
                    conflicts_only,
                    validate,
                } => commands::routes::run(project_path, conflicts_only, validate).await,
                AnalyzeCommand::Views {
                    layout,
                    name,
                    security,
                } => commands::views::run(project_path, name, layout, security).await,
            }
        }
        Commands::Db(db_cmd) => db_cmd.execute(project_path).await,
        Commands::Export {
            format,
            include_code,
            output,
        } => commands::export::run(project_path, format, output, include_code).await,
        Commands::New { command } => {
            use commands::new_cmd::NewCommand;
            use commands::new_component;

            match command {
                NewCommand::Controller {
                    crud,
                    names,
                    routes,
                } => new_component::generate_controller(names, crud, routes).await,
                NewCommand::Event {
                    custom,
                    lifecycle,
                    name,
                } => new_component::generate_event(name, lifecycle, custom).await,
                NewCommand::Middleware {
                    name,
                    auth,
                    logging,
                    priority,
                } => new_component::generate_middleware(name, auth, logging, priority).await,
                NewCommand::Module {
                    name,
                    shared,
                    with_methods,
                } => new_component::generate_module(name, shared, with_methods).await,
                NewCommand::Project {
                    project_name,
                    force,
                    path,
                } => commands::new::run(project_name, path, force).await,
                NewCommand::Worker {
                    name,
                    email,
                    file_processing,
                    cleanup,
                    batch,
                    progress,
                    validation,
                } => {
                    new_component::generate_worker(
                        name,
                        email,
                        file_processing,
                        cleanup,
                        batch,
                        progress,
                        validation,
                    )
                    .await
                }
            }
        }
        Commands::Perf { command } => {
            use commands::perf_cmd::PerfCommand;
            match command {
                PerfCommand::Benchmark { iterations } => {
                    commands::benchmark::run(project_path, iterations).await
                }
                PerfCommand::CacheStats {} => commands::cache_stats::run(project_path).await,
                PerfCommand::Stream {
                    aggressive_cleanup,
                    chunk_size,
                    format,
                    max_concurrent,
                    memory_limit,
                } => {
                    commands::stream::run(
                        project_path,
                        memory_limit,
                        chunk_size,
                        max_concurrent,
                        aggressive_cleanup,
                        format,
                    )
                    .await
                }
            }
        }
        Commands::Query {
            item_type,
            item_name,
            format,
        } => match item_type {
            ItemType::ModelMetadata => {
                let output_format = format.unwrap_or_else(|| "json".to_string());
                models::metadata(project_path, item_name, output_format).await
            }
            _ => {
                commands::query::run(
                    project_path,
                    format!("{:?}", item_type).to_lowercase(),
                    item_name,
                )
                .await
            }
        },
        Commands::Schema(schema_cmd) => schema_cmd.execute().await,
        Commands::Translations { command } => commands::translations::execute(&command),
        Commands::Serve { command } => {
            use commands::serve_cmd::ServeCommand;
            match command {
                ServeCommand::Start {
                    allow_writes,
                    auto_port,
                    bind,
                    name,
                    port,
                    watch,
                    websocket,
                } => {
                    let final_port = if auto_port {
                        match serve::find_available_port(port).await {
                            Some(p) => {
                                log::info!("Auto-selected port: {}", p);
                                p
                            }
                            None => {
                                anyhow::bail!(
                                    "Could not find an available port starting from {}",
                                    port
                                );
                            }
                        }
                    } else {
                        port
                    };
                    serve::run(
                        project_path,
                        final_port,
                        bind,
                        watch,
                        websocket,
                        name,
                        allow_writes,
                    )
                    .await
                }
                ServeCommand::List {} => serve::list().await,
                ServeCommand::Stop { port } => serve::stop(port).await,
            }
        }
        Commands::Validate { fix, watch } => {
            if watch {
                commands::watch::run(project_path).await
            } else {
                validate::run(project_path, fix).await
            }
        }
    }
}
