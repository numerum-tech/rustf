use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum PerfCommand {
    /// Run performance benchmarks
    Benchmark {
        /// Number of analysis iterations
        #[arg(short, long, default_value = "5")]
        iterations: usize,
    },
    
    /// Show AST cache performance statistics
    CacheStats {},
    
    /// Streaming analysis for large projects with memory optimization
    Stream {
        /// Enable aggressive memory cleanup
        #[arg(long)]
        aggressive_cleanup: bool,
        
        /// Chunk size for batch processing  
        #[arg(long, default_value = "50")]
        chunk_size: usize,
        
        /// Output format
        #[arg(short, long, default_value = "table")]
        format: String,
        
        /// Maximum concurrent files
        #[arg(long, default_value = "8")]
        max_concurrent: usize,
        
        /// Maximum memory limit in MB
        #[arg(long, default_value = "512")]
        memory_limit: usize,
    },
}