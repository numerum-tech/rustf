use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum AnalyzeCommand {
    /// List and analyze project backups
    Backups {
        /// Show detailed backup information
        #[arg(long)]
        detailed: bool,
    },
    
    /// List controllers and their handlers
    Controllers {
        /// Show specific controller details
        #[arg(short, long)]
        name: Option<String>,
    },
    
    /// Quick project discovery and overview
    Discover {
        /// Show only specific components
        #[arg(short, long)]
        filter: Option<String>,
    },
    
    /// Analyze middleware chain and execution order
    Middleware {
        /// Show middleware conflicts
        #[arg(long)]
        conflicts: bool,
    },
    
    /// Discover and analyze models
    Models {
        /// Show model relationships
        #[arg(long)]
        relationships: bool,
    },
    
    /// Analyze complete project structure and generate comprehensive report
    Project {
        /// Include detailed handler analysis
        #[arg(long)]
        detailed: bool,
        
        /// Output format
        #[arg(short, long, default_value = "table")]
        format: String,
    },
    
    /// Display route tree with parameters and conflicts  
    Routes {
        /// Show route conflicts only
        #[arg(long)]
        conflicts_only: bool,
        
        /// Validate all handlers exist
        #[arg(long)]
        validate: bool,
    },
    
    /// Analyze views and templates
    Views {
        /// Show layout hierarchy
        #[arg(long)]
        layout: bool,
        
        /// Show specific view details
        #[arg(short, long)]
        name: Option<String>,
        
        /// Show security analysis
        #[arg(long)]
        security: bool,
    },
}