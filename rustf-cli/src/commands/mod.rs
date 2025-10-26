// use std::path::PathBuf; // unused

pub mod analyze;
pub mod backups;
pub mod benchmark;
pub mod cache_stats;
pub mod discover;
pub mod routes;
pub mod controllers;
pub mod middleware;
pub mod models;
pub mod export;
pub mod validate;
pub mod serve;
pub mod views;
pub mod query;
pub mod watch;
pub mod schema;
pub mod stream;
pub mod new;
pub mod db;

// New consolidated command modules
pub mod analyze_cmd;
pub mod perf_cmd;
pub mod serve_cmd;
pub mod new_cmd;
pub mod new_component;
pub mod translations;