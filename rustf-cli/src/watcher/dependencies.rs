use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use serde::Serialize;
use anyhow::Result;


/// Tracks dependencies between project files for incremental analysis
#[derive(Debug, Clone, Serialize)]
pub struct DependencyTracker {
    /// Maps file paths to their direct dependencies
    pub dependencies: HashMap<PathBuf, HashSet<PathBuf>>,
    /// Maps file paths to files that depend on them (reverse dependencies)
    pub dependents: HashMap<PathBuf, HashSet<PathBuf>>,
    /// Tracks when each file was last analyzed
    pub last_analyzed: HashMap<PathBuf, chrono::DateTime<chrono::Utc>>,
    /// Cache of dependency analysis results
    pub analysis_cache: HashMap<PathBuf, DependencyInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DependencyInfo {
    pub file_path: PathBuf,
    pub file_type: FileType,
    pub imports: Vec<ImportDeclaration>,
    pub exports: Vec<ExportDeclaration>,
    pub uses_macros: Vec<String>,
    pub mod_declarations: Vec<String>,
    pub checksum: String,
    pub analyzed_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub enum FileType {
    Controller,
    Model,
    Middleware,
    View,
    Config,
    Library,
    Main,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportDeclaration {
    pub path: String,
    pub items: Vec<String>,
    pub is_glob: bool,
    pub is_crate: bool,
    pub resolved_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExportDeclaration {
    pub name: String,
    pub export_type: ExportType,
    pub visibility: Visibility,
}

#[derive(Debug, Clone, Serialize)]
pub enum ExportType {
    Function,
    Struct,
    Enum,
    Trait,
    Module,
    Const,
    Type,
    Macro,
}

#[derive(Debug, Clone, Serialize)]
pub enum Visibility {
    Public,
    Crate,
    Super,
    Private,
}

impl DependencyTracker {
    pub fn new() -> Self {
        Self {
            dependencies: HashMap::new(),
            dependents: HashMap::new(),
            last_analyzed: HashMap::new(),
            analysis_cache: HashMap::new(),
        }
    }

    /// Analyze dependencies for a single file
    pub async fn analyze_file_dependencies(
        &mut self,
        file_path: &Path,
        project_path: &Path,
    ) -> Result<DependencyInfo> {
        let content = tokio::fs::read_to_string(file_path).await?;
        let checksum = format!("{:x}", md5::compute(&content));
        
        // Check if we have a cached analysis that's still valid
        if let Some(cached) = self.analysis_cache.get(file_path) {
            if cached.checksum == checksum {
                return Ok(cached.clone());
            }
        }

        let file_type = Self::determine_file_type(file_path, project_path);
        let imports = Self::extract_imports(&content, project_path)?;
        let exports = Self::extract_exports(&content)?;
        let uses_macros = Self::extract_macro_uses(&content);
        let mod_declarations = Self::extract_mod_declarations(&content);

        let dependency_info = DependencyInfo {
            file_path: file_path.to_path_buf(),
            file_type,
            imports,
            exports,
            uses_macros,
            mod_declarations,
            checksum,
            analyzed_at: chrono::Utc::now(),
        };

        // Update dependency graph
        self.update_dependency_graph(&dependency_info)?;
        
        // Cache the result
        self.analysis_cache.insert(file_path.to_path_buf(), dependency_info.clone());
        self.last_analyzed.insert(file_path.to_path_buf(), chrono::Utc::now());

        Ok(dependency_info)
    }

    /// Update the bidirectional dependency graph
    fn update_dependency_graph(&mut self, info: &DependencyInfo) -> Result<()> {
        let file_path = &info.file_path;
        
        // Clear existing dependencies for this file
        if let Some(old_deps) = self.dependencies.remove(file_path) {
            for dep in old_deps {
                if let Some(dependents) = self.dependents.get_mut(&dep) {
                    dependents.remove(file_path);
                }
            }
        }

        // Add new dependencies
        let mut new_deps = HashSet::new();
        for import in &info.imports {
            if let Some(resolved_path) = &import.resolved_path {
                new_deps.insert(resolved_path.clone());
                
                // Add to reverse dependencies
                self.dependents
                    .entry(resolved_path.clone())
                    .or_insert_with(HashSet::new)
                    .insert(file_path.clone());
            }
        }

        self.dependencies.insert(file_path.clone(), new_deps);
        Ok(())
    }

    /// Get all files that need to be re-analyzed when a file changes
    pub fn get_affected_files(&self, changed_file: &Path) -> HashSet<PathBuf> {
        let mut affected = HashSet::new();
        let mut to_visit = vec![changed_file.to_path_buf()];
        let mut visited = HashSet::new();

        // Breadth-first traversal of dependents
        while let Some(current) = to_visit.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());
            affected.insert(current.clone());

            // Add all files that depend on the current file
            if let Some(dependents) = self.dependents.get(&current) {
                for dependent in dependents {
                    if !visited.contains(dependent) {
                        to_visit.push(dependent.clone());
                    }
                }
            }
        }

        affected.remove(&changed_file.to_path_buf()); // Don't include the original file
        affected
    }

    /// Check if a file needs re-analysis based on its dependencies
    pub fn needs_reanalysis(&self, file_path: &Path) -> bool {
        let file_last_analyzed = self.last_analyzed.get(file_path);
        
        if file_last_analyzed.is_none() {
            return true; // Never analyzed
        }

        let file_time = file_last_analyzed.unwrap();

        // Check if any dependencies were analyzed more recently
        if let Some(deps) = self.dependencies.get(file_path) {
            for dep in deps {
                if let Some(dep_time) = self.last_analyzed.get(dep) {
                    if dep_time > file_time {
                        return true; // Dependency is newer
                    }
                }
            }
        }

        false
    }

    /// Determine file type based on path and content
    fn determine_file_type(file_path: &Path, project_path: &Path) -> FileType {
        if let Ok(relative_path) = file_path.strip_prefix(project_path) {
            let components: Vec<_> = relative_path.components().collect();
            
            if components.len() >= 2 {
                let first = components[0].as_os_str().to_string_lossy();
                let second = components[1].as_os_str().to_string_lossy();
                
                match (first.as_ref(), second.as_ref()) {
                    ("src", "controllers") => return FileType::Controller,
                    ("src", "models") => return FileType::Model,
                    ("src", "middleware") => return FileType::Middleware,
                    ("views", _) => return FileType::View,
                    _ => {}
                }
            }
            
            if relative_path.file_name() == Some(std::ffi::OsStr::new("main.rs")) {
                return FileType::Main;
            }
            
            if relative_path.extension() == Some(std::ffi::OsStr::new("toml")) {
                return FileType::Config;
            }
        }

        FileType::Library
    }

    /// Extract import declarations from Rust source code
    fn extract_imports(content: &str, project_path: &Path) -> Result<Vec<ImportDeclaration>> {
        let mut imports = Vec::new();
        
        // Parse as Rust AST
        let syntax_tree = syn::parse_file(content)?;
        
        for item in &syntax_tree.items {
            if let syn::Item::Use(use_item) = item {
                let import = Self::parse_use_item(use_item, project_path);
                imports.push(import);
            }
        }

        Ok(imports)
    }

    /// Parse a use statement into an ImportDeclaration
    fn parse_use_item(use_item: &syn::ItemUse, project_path: &Path) -> ImportDeclaration {
        let use_tree = &use_item.tree;
        let path_str = quote::quote!(#use_tree).to_string();
        
        // Simple heuristics to extract information
        let is_crate = path_str.starts_with("crate::");
        let is_glob = path_str.contains("*");
        
        // Extract individual items (simplified)
        let items = if is_glob {
            vec!["*".to_string()]
        } else {
            // Extract items from braced imports like `use foo::{bar, baz}`
            if path_str.contains('{') {
                path_str
                    .split('{')
                    .nth(1)
                    .unwrap_or("")
                    .split('}')
                    .next()
                    .unwrap_or("")
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            } else {
                vec![path_str.split("::").last().unwrap_or("").to_string()]
            }
        };

        // Try to resolve the path (simplified implementation)
        let resolved_path = if is_crate {
            Self::resolve_crate_path(&path_str, project_path)
        } else {
            None
        };

        ImportDeclaration {
            path: path_str,
            items,
            is_glob,
            is_crate,
            resolved_path,
        }
    }

    /// Resolve a crate-relative path to an actual file path
    fn resolve_crate_path(path: &str, project_path: &Path) -> Option<PathBuf> {
        // Remove "crate::" prefix and convert module path to file path
        let module_path = path.strip_prefix("crate::")?;
        let file_path = module_path.replace("::", "/");
        
        // Try different possible locations
        let candidates = [
            project_path.join("src").join(format!("{}.rs", file_path)),
            project_path.join("src").join(&file_path).join("mod.rs"),
        ];

        for candidate in &candidates {
            if candidate.exists() {
                return Some(candidate.clone());
            }
        }

        None
    }

    /// Extract export declarations from Rust source code
    fn extract_exports(content: &str) -> Result<Vec<ExportDeclaration>> {
        let mut exports = Vec::new();
        let syntax_tree = syn::parse_file(content)?;

        for item in &syntax_tree.items {
            let (name, export_type, visibility) = match item {
                syn::Item::Fn(func) => {
                    let name = func.sig.ident.to_string();
                    let vis = Self::parse_visibility(&func.vis);
                    (name, ExportType::Function, vis)
                }
                syn::Item::Struct(struct_item) => {
                    let name = struct_item.ident.to_string();
                    let vis = Self::parse_visibility(&struct_item.vis);
                    (name, ExportType::Struct, vis)
                }
                syn::Item::Enum(enum_item) => {
                    let name = enum_item.ident.to_string();
                    let vis = Self::parse_visibility(&enum_item.vis);
                    (name, ExportType::Enum, vis)
                }
                syn::Item::Trait(trait_item) => {
                    let name = trait_item.ident.to_string();
                    let vis = Self::parse_visibility(&trait_item.vis);
                    (name, ExportType::Trait, vis)
                }
                syn::Item::Mod(mod_item) => {
                    let name = mod_item.ident.to_string();
                    let vis = Self::parse_visibility(&mod_item.vis);
                    (name, ExportType::Module, vis)
                }
                syn::Item::Const(const_item) => {
                    let name = const_item.ident.to_string();
                    let vis = Self::parse_visibility(&const_item.vis);
                    (name, ExportType::Const, vis)
                }
                syn::Item::Type(type_item) => {
                    let name = type_item.ident.to_string();
                    let vis = Self::parse_visibility(&type_item.vis);
                    (name, ExportType::Type, vis)
                }
                syn::Item::Macro(macro_item) => {
                    let name = macro_item.ident.as_ref().map(|i| i.to_string()).unwrap_or_default();
                    (name, ExportType::Macro, Visibility::Public)
                }
                _ => continue,
            };

            exports.push(ExportDeclaration {
                name,
                export_type,
                visibility,
            });
        }

        Ok(exports)
    }

    /// Parse visibility from syn::Visibility
    fn parse_visibility(vis: &syn::Visibility) -> Visibility {
        match vis {
            syn::Visibility::Public(_) => Visibility::Public,
            syn::Visibility::Restricted(restricted) => {
                let path = &restricted.path;
                if path.is_ident("crate") {
                    Visibility::Crate
                } else if path.is_ident("super") {
                    Visibility::Super
                } else {
                    Visibility::Private
                }
            }
            syn::Visibility::Inherited => Visibility::Private,
        }
    }

    /// Extract macro usage from source code
    fn extract_macro_uses(content: &str) -> Vec<String> {
        let mut macros = Vec::new();
        
        // Simple regex-based extraction (could be improved with proper AST parsing)
        let macro_regex = regex::Regex::new(r"(\w+)!").unwrap();
        
        for caps in macro_regex.captures_iter(content) {
            if let Some(macro_name) = caps.get(1) {
                let name = macro_name.as_str().to_string();
                if !macros.contains(&name) {
                    macros.push(name);
                }
            }
        }

        macros
    }

    /// Extract module declarations from source code
    fn extract_mod_declarations(content: &str) -> Vec<String> {
        let mut modules = Vec::new();
        
        // Simple regex-based extraction
        let mod_regex = regex::Regex::new(r"mod\s+(\w+)").unwrap();
        
        for caps in mod_regex.captures_iter(content) {
            if let Some(mod_name) = caps.get(1) {
                modules.push(mod_name.as_str().to_string());
            }
        }

        modules
    }

    /// Get dependency statistics
    pub fn get_statistics(&self) -> DependencyStatistics {
        let total_files = self.analysis_cache.len();
        let total_dependencies = self.dependencies.values().map(|deps| deps.len()).sum();
        
        let mut files_by_type = HashMap::new();
        let most_dependent_files: Vec<(PathBuf, usize)>;
        let most_dependencies: Vec<(PathBuf, usize)>;

        for info in self.analysis_cache.values() {
            let type_name = format!("{:?}", info.file_type);
            *files_by_type.entry(type_name).or_insert(0) += 1;
        }

        // Find files with most dependents
        let mut dependents_count: Vec<_> = self.dependents
            .iter()
            .map(|(path, deps)| (path.clone(), deps.len()))
            .collect();
        dependents_count.sort_by(|a, b| b.1.cmp(&a.1));
        most_dependent_files = dependents_count.into_iter()
            .take(10)
            .collect();

        // Find files with most dependencies
        let mut dependencies_count: Vec<_> = self.dependencies
            .iter()
            .map(|(path, deps)| (path.clone(), deps.len()))
            .collect();
        dependencies_count.sort_by(|a, b| b.1.cmp(&a.1));
        most_dependencies = dependencies_count.into_iter()
            .take(10)
            .collect();

        DependencyStatistics {
            total_files,
            total_dependencies,
            files_by_type,
            most_dependent_files,
            most_dependencies,
            cache_hit_rate: 0.0, // TODO: Track cache hits
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DependencyStatistics {
    pub total_files: usize,
    pub total_dependencies: usize,
    pub files_by_type: HashMap<String, usize>,
    pub most_dependent_files: Vec<(PathBuf, usize)>,
    pub most_dependencies: Vec<(PathBuf, usize)>,
    pub cache_hit_rate: f64,
}

impl Default for DependencyTracker {
    fn default() -> Self {
        Self::new()
    }
}