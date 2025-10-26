use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use anyhow::Result;
use rayon::prelude::*;
use super::metadata::{global_metadata_cache, FileMetadata};

#[derive(Debug)]
pub struct ProjectFiles {
    pub controllers: Vec<PathBuf>,
    pub models: Vec<PathBuf>,
    pub middleware: Vec<PathBuf>,
    pub views: Vec<PathBuf>,
    pub config_files: Vec<PathBuf>,
}

impl ProjectFiles {
    pub fn scan(project_path: &Path) -> Result<Self> {
        let mut files = ProjectFiles {
            controllers: Vec::new(),
            models: Vec::new(),
            middleware: Vec::new(),
            views: Vec::new(),
            config_files: Vec::new(),
        };

        let src_path = project_path.join("src");
        if src_path.exists() {
            files.scan_controllers(&src_path)?;
            files.scan_models(&src_path)?;
            files.scan_middleware(&src_path)?;
        }

        files.scan_views(project_path)?;
        files.scan_config_files(project_path)?;

        Ok(files)
    }
    
    /// Ultra-fast parallel file scanning with metadata caching for large projects
    pub fn scan_parallel(project_path: &Path) -> Result<Self> {
        let src_path = project_path.join("src");
        
        // Use rayon to scan different directories in parallel
        let scan_tasks: Vec<(&str, PathBuf)> = vec![
            ("controllers", src_path.join("controllers")),
            ("models", src_path.join("models")),
            ("middleware", src_path.join("middleware")),
            ("views", project_path.join("views")),
        ];
        
        // Parallel scan of all directories
        let results: Vec<(&str, Vec<PathBuf>)> = scan_tasks
            .par_iter()
            .map(|(dir_type, path)| {
                let files = match *dir_type {
                    "controllers" => Self::scan_rust_files_cached(path),
                    "models" => Self::scan_rust_files_cached(path),
                    "middleware" => Self::scan_rust_files_cached(path),
                    "views" => Self::scan_html_files_cached(path),
                    _ => Vec::new(),
                };
                (*dir_type, files)
            })
            .collect();
        
        // Collect results
        let mut project_files = ProjectFiles {
            controllers: Vec::new(),
            models: Vec::new(),
            middleware: Vec::new(),
            views: Vec::new(),
            config_files: Vec::new(),
        };
        
        for (dir_type, files) in results {
            match dir_type {
                "controllers" => project_files.controllers = files,
                "models" => project_files.models = files,
                "middleware" => project_files.middleware = files,
                "views" => project_files.views = files,
                _ => {}
            }
        }
        
        // Scan config files (usually just a few, so no need for parallelism)
        project_files.scan_config_files(project_path)?;
        
        Ok(project_files)
    }
    
    /// Optimized parallel scanning of Rust files in a directory
    fn scan_rust_files(dir_path: &Path) -> Vec<PathBuf> {
        if !dir_path.exists() {
            return Vec::new();
        }
        
        WalkDir::new(dir_path)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .par_bridge() // Convert to parallel iterator
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                    Some(path.to_path_buf())
                } else {
                    None
                }
            })
            .collect()
    }
    
    /// Optimized parallel scanning of HTML files in a directory
    fn scan_html_files(dir_path: &Path) -> Vec<PathBuf> {
        if !dir_path.exists() {
            return Vec::new();
        }
        
        WalkDir::new(dir_path)
            .into_iter()
            .par_bridge() // Convert to parallel iterator
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("html") {
                    Some(path.to_path_buf())
                } else {
                    None
                }
            })
            .collect()
    }
    
    /// Ultra-fast metadata-cached scanning of Rust files in a directory
    fn scan_rust_files_cached(dir_path: &Path) -> Vec<PathBuf> {
        if !dir_path.exists() {
            return Vec::new();
        }
        
        let cache = global_metadata_cache();
        
        // First, get directory contents using cached metadata
        match cache.list_directory(dir_path) {
            Ok(entries) => {
                entries
                    .par_iter()
                    .filter_map(|metadata| {
                        if !metadata.is_directory && 
                           metadata.extension.as_ref() == Some(&"rs".to_string()) {
                            Some(metadata.path.clone())
                        } else {
                            None
                        }
                    })
                    .collect()
            },
            Err(_) => {
                // Fallback to regular scanning if cache fails
                Self::scan_rust_files(dir_path)
            }
        }
    }
    
    /// Ultra-fast metadata-cached scanning of HTML files in a directory
    fn scan_html_files_cached(dir_path: &Path) -> Vec<PathBuf> {
        if !dir_path.exists() {
            return Vec::new();
        }
        
        let cache = global_metadata_cache();
        
        // Use cached directory listing for faster scanning
        match cache.list_directory(dir_path) {
            Ok(entries) => {
                // Recursively scan subdirectories and collect HTML files
                Self::scan_html_files_recursive_cached(&entries, dir_path)
            },
            Err(_) => {
                // Fallback to regular scanning if cache fails
                Self::scan_html_files(dir_path)
            }
        }
    }
    
    /// Recursive HTML file scanning with metadata cache
    fn scan_html_files_recursive_cached(entries: &[FileMetadata], _base_path: &Path) -> Vec<PathBuf> {
        let cache = global_metadata_cache();
        let mut html_files = Vec::new();
        
        // Process files and directories in parallel
        let (files, dirs): (Vec<_>, Vec<_>) = entries
            .par_iter()
            .partition(|metadata| !metadata.is_directory);
        
        // Collect HTML files
        html_files.extend(
            files
                .par_iter()
                .filter_map(|metadata| {
                    if metadata.extension.as_ref() == Some(&"html".to_string()) {
                        Some(metadata.path.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        );
        
        // Recursively scan subdirectories
        for dir_metadata in dirs {
            if let Ok(sub_entries) = cache.list_directory(&dir_metadata.path) {
                html_files.extend(Self::scan_html_files_recursive_cached(&sub_entries, &dir_metadata.path));
            }
        }
        
        html_files
    }
    
    /// Get cached file existence check for performance
    pub fn file_exists(&self, path: &Path) -> bool {
        global_metadata_cache().file_exists(path)
    }
    
    /// Get file metadata using cache
    pub fn get_file_metadata(&self, path: &Path) -> Result<FileMetadata> {
        global_metadata_cache().get_metadata(path)
    }
    
    /// Filter files by extension using cached metadata
    pub fn filter_by_extension(&self, files: &[PathBuf], extension: &str) -> Vec<PathBuf> {
        global_metadata_cache().filter_by_extension(files, extension)
    }

    fn scan_controllers(&mut self, src_path: &Path) -> Result<()> {
        let controllers_path = src_path.join("controllers");
        if controllers_path.exists() {
            for entry in WalkDir::new(&controllers_path)
                .min_depth(1)
                .max_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                    self.controllers.push(path.to_path_buf());
                }
            }
        }
        Ok(())
    }

    fn scan_models(&mut self, src_path: &Path) -> Result<()> {
        let models_path = src_path.join("models");
        if models_path.exists() {
            for entry in WalkDir::new(&models_path)
                .min_depth(1)
                .max_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                    self.models.push(path.to_path_buf());
                }
            }
        }
        Ok(())
    }

    fn scan_middleware(&mut self, src_path: &Path) -> Result<()> {
        let middleware_path = src_path.join("middleware");
        if middleware_path.exists() {
            for entry in WalkDir::new(&middleware_path)
                .min_depth(1)
                .max_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                    self.middleware.push(path.to_path_buf());
                }
            }
        }
        Ok(())
    }

    fn scan_views(&mut self, project_path: &Path) -> Result<()> {
        let views_path = project_path.join("views");
        if views_path.exists() {
            for entry in WalkDir::new(&views_path)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("html") {
                    self.views.push(path.to_path_buf());
                }
            }
        }
        Ok(())
    }

    fn scan_config_files(&mut self, project_path: &Path) -> Result<()> {
        // Scan for common config files
        let config_files = ["config.toml", "Cargo.toml", ".env"];
        for file in &config_files {
            let path = project_path.join(file);
            if path.exists() {
                self.config_files.push(path);
            }
        }
        Ok(())
    }
}