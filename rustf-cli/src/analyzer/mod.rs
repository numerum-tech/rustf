// removed unused imports
use serde::{Serialize, Deserialize};

mod project;
pub mod files;
mod ast;
pub mod cache;
pub mod metadata;
pub mod lookup;
pub mod lru_cache;
pub mod analysis_cache;
pub mod streaming;

pub use project::ProjectAnalyzer;
pub use streaming::{StreamingAnalyzer, StreamingConfigBuilder, MemoryStats};
// pub use files::ProjectFiles; // unused
// pub use ast::{AstAnalyzer, BasicControllerInfo}; // unused
// pub use cache::{AstCache, CacheStats}; // unused
// pub use metadata::{MetadataCache, FileMetadata, global_metadata_cache}; // unused
// pub use lookup::{AnalysisLookup}; // unused
use crate::analysis::views::ViewAnalysis;

/// Output format for analysis results
#[derive(Debug, Clone, clap::ValueEnum)]
pub enum OutputFormat {
    Json,
    Yaml,
    Table,
    Markdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectAnalysis {
    pub project_name: String,
    pub framework_version: String,
    pub controllers: Vec<ControllerInfo>,
    pub routes: Vec<RouteInfo>,
    pub middleware: Vec<MiddlewareInfo>,
    pub models: Vec<ModelInfo>,
    pub views: Vec<ViewAnalysis>,
    pub issues: Vec<Issue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerInfo {
    pub name: String,
    pub file_path: String,
    pub handlers: Vec<HandlerInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandlerInfo {
    pub name: String,
    pub qualified_name: String,  // controller::handler format
    pub routes: Vec<RouteReference>,
    pub complexity: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteReference {
    pub method: String,
    pub path: String,
    pub parameters: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteInfo {
    pub method: String,
    pub path: String,
    pub handler: String,
    pub parameters: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiddlewareInfo {
    pub name: String,
    pub priority: Option<i32>,
    pub middleware_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub severity: String,
    pub message: String,
    pub file_path: Option<String>,
    pub line: Option<u32>,
}