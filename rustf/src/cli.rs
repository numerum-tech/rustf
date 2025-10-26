//! Command-line interface support for RustF applications
//!
//! This module provides optional CLI argument parsing to support common
//! configuration flags like `--config` for specifying config file paths.
//!
//! # Features
//! - `--config <path>` - Specify configuration file location
//! - `--views <directory>` - Override views directory (filesystem storage only)
//! - `--help` - Show help information
//! - Backward compatibility - CLI parsing is opt-in
//! - Multi-instance safe - No global environment variables

use crate::error::{Error, Result};
use std::env;
use std::path::PathBuf;

#[cfg(feature = "cli")]
use clap::{Arg, Command};

/// CLI arguments parsed from command line
#[derive(Debug, Clone)]
#[derive(Default)]
pub struct CliArgs {
    /// Path to configuration file
    pub config_file: Option<PathBuf>,
    /// Path to views directory (only effective with filesystem storage)
    pub views_directory: Option<PathBuf>,
    /// Whether help was requested
    pub help: bool,
}


impl CliArgs {
    /// Parse command line arguments using clap (requires 'cli' feature)
    #[cfg(feature = "cli")]
    pub fn parse() -> Result<Self> {
        let app = Command::new("rustf-app")
            .version(env!("CARGO_PKG_VERSION"))
            .about("RustF web application")
            .arg(
                Arg::new("config")
                    .long("config")
                    .short('c')
                    .value_name("FILE")
                    .help("Configuration file path")
                    .value_parser(clap::value_parser!(PathBuf)),
            )
            .arg(
                Arg::new("views")
                    .long("views")
                    .short('v')
                    .value_name("DIR")
                    .help("Views directory path (only effective with filesystem storage)")
                    .value_parser(clap::value_parser!(PathBuf)),
            );

        let matches = app.try_get_matches().map_err(|e| {
            Error::internal(format!("Failed to parse command line arguments: {}", e))
        })?;

        Ok(CliArgs {
            config_file: matches.get_one::<PathBuf>("config").cloned(),
            views_directory: matches.get_one::<PathBuf>("views").cloned(),
            help: false,
        })
    }

    /// Parse command line arguments using manual parsing (lightweight fallback)
    #[cfg(not(feature = "cli"))]
    pub fn parse() -> Result<Self> {
        Self::parse_simple()
    }

    /// Simple argument parser without clap dependency
    pub fn parse_simple() -> Result<Self> {
        let args: Vec<String> = env::args().collect();
        let mut config_file = None;
        let mut views_directory = None;
        let mut help = false;

        let mut i = 1; // Skip program name
        while i < args.len() {
            match args[i].as_str() {
                "--config" | "-c" => {
                    if i + 1 >= args.len() {
                        return Err(Error::internal("--config flag requires a value"));
                    }
                    config_file = Some(PathBuf::from(&args[i + 1]));
                    i += 2;
                }
                "--views" | "-v" => {
                    if i + 1 >= args.len() {
                        return Err(Error::internal("--views flag requires a value"));
                    }
                    views_directory = Some(PathBuf::from(&args[i + 1]));
                    i += 2;
                }
                "--help" | "-h" => {
                    help = true;
                    i += 1;
                }
                arg if arg.starts_with("--config=") => {
                    let path = arg.strip_prefix("--config=").unwrap();
                    config_file = Some(PathBuf::from(path));
                    i += 1;
                }
                arg if arg.starts_with("-c=") => {
                    let path = arg.strip_prefix("-c=").unwrap();
                    config_file = Some(PathBuf::from(path));
                    i += 1;
                }
                arg if arg.starts_with("--views=") => {
                    let path = arg.strip_prefix("--views=").unwrap();
                    views_directory = Some(PathBuf::from(path));
                    i += 1;
                }
                arg if arg.starts_with("-v=") => {
                    let path = arg.strip_prefix("-v=").unwrap();
                    views_directory = Some(PathBuf::from(path));
                    i += 1;
                }
                _ => {
                    // Ignore unknown arguments for now
                    i += 1;
                }
            }
        }

        if help {
            Self::print_help();
            std::process::exit(0);
        }

        Ok(CliArgs {
            config_file,
            views_directory,
            help,
        })
    }

    /// Print help information
    fn print_help() {
        println!("RustF Web Application");
        println!();
        println!("USAGE:");
        println!(
            "    {} [OPTIONS]",
            env::args()
                .next()
                .unwrap_or_else(|| "rustf-app".to_string())
        );
        println!();
        println!("OPTIONS:");
        println!("    -c, --config <FILE>    Configuration file path");
        println!("    -v, --views <DIR>      Views directory path (filesystem storage only)");
        println!("    -h, --help             Print help information");
        println!();
        println!("EXAMPLES:");
        println!(
            "    {} --config /etc/myapp/config.toml",
            env::args()
                .next()
                .unwrap_or_else(|| "rustf-app".to_string())
        );
        println!(
            "    {} --config ../shared-config.toml --views ./custom-templates",
            env::args()
                .next()
                .unwrap_or_else(|| "rustf-app".to_string())
        );
        println!(
            "    {} --views /shared/templates",
            env::args()
                .next()
                .unwrap_or_else(|| "rustf-app".to_string())
        );
    }

    /// Get the configuration file path to use
    ///
    /// Returns the CLI-specified path if available, otherwise None
    pub fn config_path(&self) -> Option<&PathBuf> {
        self.config_file.as_ref()
    }

    /// Get the views directory path to use
    ///
    /// Returns the CLI-specified path if available, otherwise None
    /// Note: Only effective with filesystem storage
    pub fn views_path(&self) -> Option<&PathBuf> {
        self.views_directory.as_ref()
    }

    /// Check if this is a help request
    pub fn is_help(&self) -> bool {
        self.help
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_parser_basic() {
        // Test with no arguments
        std::env::set_var("RUSTF_TEST_ARGS", "");

        // We can't easily test the arg parser since it uses env::args()
        // But we can test the CliArgs structure
        let args = CliArgs::default();
        assert!(args.config_path().is_none());
        assert!(!args.is_help());
    }

    #[test]
    fn test_cli_args_creation() {
        let args = CliArgs {
            config_file: Some(PathBuf::from("/test/config.toml")),
            views_directory: Some(PathBuf::from("/test/views")),
            help: false,
        };

        assert_eq!(
            args.config_path(),
            Some(&PathBuf::from("/test/config.toml"))
        );
        assert_eq!(args.views_path(), Some(&PathBuf::from("/test/views")));
        assert!(!args.is_help());
    }
}
