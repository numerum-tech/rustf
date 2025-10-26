//! AST and analysis result caching for RustF CLI
//! 
//! This module provides high-performance caching of parsed ASTs and analysis results
//! to dramatically improve repeated analysis performance.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::{Arc, RwLock};
use syn::File;
use anyhow::{Result, Context};
use serde::Serialize;
use std::fs;

/// Cached AST entry with validation metadata
#[derive(Debug, Clone)]
pub struct CachedAst {
    pub ast: File,
    pub file_size: u64,
    pub modified_time: u64,
    pub content_hash: u64,
    pub created_at: u64,
}

// SAFETY: syn::File contains only data structures and is safe to Send/Sync
unsafe impl Send for CachedAst {}
unsafe impl Sync for CachedAst {}

/// Cache statistics for monitoring and debugging
#[derive(Debug, Clone, Serialize)]
pub struct CacheStats {
    pub total_requests: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub invalidations: u64,
    pub entries_count: usize,
    pub hit_rate: f64,
}

impl CacheStats {
    pub fn new() -> Self {
        Self {
            total_requests: 0,
            cache_hits: 0,
            cache_misses: 0,
            invalidations: 0,
            entries_count: 0,
            hit_rate: 0.0,
        }
    }
    
    pub fn update_hit_rate(&mut self) {
        if self.total_requests > 0 {
            self.hit_rate = (self.cache_hits as f64 / self.total_requests as f64) * 100.0;
        }
    }
}

/// LRU cache for AST entries with automatic cleanup
#[derive(Debug)]
pub struct AstCache {
    entries: Arc<RwLock<HashMap<PathBuf, CachedAst>>>,
    access_order: Arc<RwLock<Vec<PathBuf>>>,
    stats: Arc<RwLock<CacheStats>>,
    max_entries: usize,
    max_age_seconds: u64,
}

impl AstCache {
    /// Create new AST cache with specified limits
    pub fn new(max_entries: usize, max_age_seconds: u64) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            access_order: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(CacheStats::new())),
            max_entries,
            max_age_seconds,
        }
    }
    
    /// Get cached AST if valid, otherwise parse and cache
    pub fn get_or_parse(&self, file_path: &Path) -> Result<File> {
        let file_path_buf = file_path.to_path_buf();
        
        // Update stats
        {
            let mut stats = self.stats.write().unwrap();
            stats.total_requests += 1;
        }
        
        // Check if we have a valid cached entry
        if let Some(cached_ast) = self.get_valid_cached_ast(&file_path_buf)? {
            // Cache hit - update access order
            self.update_access_order(&file_path_buf);
            
            let mut stats = self.stats.write().unwrap();
            stats.cache_hits += 1;
            stats.update_hit_rate();
            
            return Ok(cached_ast.ast);
        }
        
        // Cache miss - parse and cache
        let ast = self.parse_and_cache(&file_path_buf)?;
        
        let mut stats = self.stats.write().unwrap();
        stats.cache_misses += 1;
        stats.update_hit_rate();
        
        Ok(ast)
    }
    
    /// Check if cached entry is still valid
    fn get_valid_cached_ast(&self, file_path: &PathBuf) -> Result<Option<CachedAst>> {
        let entries = self.entries.read().unwrap();
        
        if let Some(cached) = entries.get(file_path) {
            // Check if file still exists and get current metadata
            let metadata = match fs::metadata(file_path) {
                Ok(meta) => meta,
                Err(_) => {
                    // File doesn't exist, entry is invalid
                    drop(entries);
                    self.invalidate_entry(file_path);
                    return Ok(None);
                }
            };
            
            let current_modified = metadata
                .modified()
                .unwrap_or(SystemTime::UNIX_EPOCH)
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            
            let current_size = metadata.len();
            
            // Check if entry is still valid
            if cached.modified_time == current_modified && 
               cached.file_size == current_size &&
               !self.is_entry_expired(cached) {
                return Ok(Some(cached.clone()));
            } else {
                // Entry is stale, remove it
                drop(entries);
                self.invalidate_entry(file_path);
            }
        }
        
        Ok(None)
    }
    
    /// Check if cache entry has expired
    fn is_entry_expired(&self, cached: &CachedAst) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        (now - cached.created_at) > self.max_age_seconds
    }
    
    /// Parse file and add to cache
    fn parse_and_cache(&self, file_path: &PathBuf) -> Result<File> {
        // Read and parse file
        let content = fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read file: {}", file_path.display()))?;
        
        let ast: File = syn::parse_file(&content)
            .with_context(|| format!("Failed to parse Rust file: {}", file_path.display()))?;
        
        // Get file metadata
        let metadata = fs::metadata(file_path)
            .with_context(|| format!("Failed to get file metadata: {}", file_path.display()))?;
        
        let modified_time = metadata
            .modified()
            .unwrap_or(SystemTime::UNIX_EPOCH)
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let file_size = metadata.len();
        let content_hash = self.calculate_content_hash(&content);
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Create cached entry
        let cached_ast = CachedAst {
            ast: ast.clone(),
            file_size,
            modified_time,
            content_hash,
            created_at,
        };
        
        // Add to cache
        self.insert_entry(file_path.clone(), cached_ast);
        
        Ok(ast)
    }
    
    /// Insert entry into cache with LRU management
    fn insert_entry(&self, file_path: PathBuf, cached_ast: CachedAst) {
        // Acquire locks
        let mut entries = self.entries.write().unwrap();
        let mut access_order = self.access_order.write().unwrap();
        
        // If at capacity, remove LRU entry
        if entries.len() >= self.max_entries && !entries.contains_key(&file_path) {
            if let Some(lru_path) = access_order.first().cloned() {
                entries.remove(&lru_path);
                access_order.retain(|p| p != &lru_path);
                
                let mut stats = self.stats.write().unwrap();
                stats.invalidations += 1;
            }
        }
        
        // Insert/update entry
        entries.insert(file_path.clone(), cached_ast);
        
        // Update access order (move to end)
        access_order.retain(|p| p != &file_path);
        access_order.push(file_path);
        
        // Update stats
        let mut stats = self.stats.write().unwrap();
        stats.entries_count = entries.len();
    }
    
    /// Update access order for LRU tracking
    fn update_access_order(&self, file_path: &PathBuf) {
        let mut access_order = self.access_order.write().unwrap();
        access_order.retain(|p| p != file_path);
        access_order.push(file_path.clone());
    }
    
    /// Remove specific entry from cache
    fn invalidate_entry(&self, file_path: &PathBuf) {
        let mut entries = self.entries.write().unwrap();
        let mut access_order = self.access_order.write().unwrap();
        
        if entries.remove(file_path).is_some() {
            access_order.retain(|p| p != file_path);
            
            let mut stats = self.stats.write().unwrap();
            stats.invalidations += 1;
            stats.entries_count = entries.len();
        }
    }
    
    /// Calculate simple hash of file content for validation
    fn calculate_content_hash(&self, content: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }
    
    /// Clear all cache entries
    pub fn clear(&self) {
        let mut entries = self.entries.write().unwrap();
        let mut access_order = self.access_order.write().unwrap();
        let mut stats = self.stats.write().unwrap();
        
        entries.clear();
        access_order.clear();
        stats.entries_count = 0;
        stats.invalidations += entries.len() as u64;
    }
    
    /// Get current cache statistics
    pub fn get_stats(&self) -> CacheStats {
        let stats = self.stats.read().unwrap();
        let entries = self.entries.read().unwrap();
        
        let mut current_stats = stats.clone();
        current_stats.entries_count = entries.len();
        current_stats
    }
    
    /// Clean up expired entries
    pub fn cleanup_expired(&self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let mut entries = self.entries.write().unwrap();
        let mut access_order = self.access_order.write().unwrap();
        let mut stats = self.stats.write().unwrap();
        
        let mut expired_paths = Vec::new();
        
        for (path, cached) in entries.iter() {
            if (now - cached.created_at) > self.max_age_seconds {
                expired_paths.push(path.clone());
            }
        }
        
        for path in expired_paths {
            entries.remove(&path);
            access_order.retain(|p| p != &path);
            stats.invalidations += 1;
        }
        
        stats.entries_count = entries.len();
    }
}

impl Default for AstCache {
    fn default() -> Self {
        // Default: 1000 entries, 1 hour expiration
        Self::new(1000, 3600)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::io::Write;
    
    #[test]
    fn test_ast_cache_basic_functionality() {
        let cache = AstCache::new(10, 3600);
        let temp_dir = TempDir::new().unwrap();
        
        // Create a test Rust file
        let file_path = temp_dir.path().join("test.rs");
        let mut file = fs::File::create(&file_path).unwrap();
        writeln!(file, "fn main() {{ println!(\"Hello, world!\"); }}").unwrap();
        
        // First access should be a cache miss
        let ast1 = cache.get_or_parse(&file_path).unwrap();
        let stats1 = cache.get_stats();
        assert_eq!(stats1.cache_misses, 1);
        assert_eq!(stats1.cache_hits, 0);
        
        // Second access should be a cache hit
        let ast2 = cache.get_or_parse(&file_path).unwrap();
        let stats2 = cache.get_stats();
        assert_eq!(stats2.cache_misses, 1);
        assert_eq!(stats2.cache_hits, 1);
        
        // ASTs should be equivalent
        assert_eq!(format!("{:?}", ast1), format!("{:?}", ast2));
    }
    
    #[test]
    fn test_cache_invalidation_on_file_change() {
        let cache = AstCache::new(10, 3600);
        let temp_dir = TempDir::new().unwrap();
        
        // Create a test Rust file
        let file_path = temp_dir.path().join("test.rs");
        let mut file = fs::File::create(&file_path).unwrap();
        writeln!(file, "fn main() {{ println!(\"Hello, world!\"); }}").unwrap();
        
        // First access
        let _ast1 = cache.get_or_parse(&file_path).unwrap();
        
        // Modify the file
        std::thread::sleep(std::time::Duration::from_millis(10)); // Ensure different timestamp
        let mut file = fs::File::create(&file_path).unwrap();
        writeln!(file, "fn main() {{ println!(\"Hello, modified world!\"); }}").unwrap();
        
        // Second access should be a cache miss due to file modification
        let _ast2 = cache.get_or_parse(&file_path).unwrap();
        let stats = cache.get_stats();
        assert_eq!(stats.cache_misses, 2); // Both accesses were misses
    }
    
    #[test]
    fn test_lru_eviction() {
        let cache = AstCache::new(2, 3600); // Very small cache
        let temp_dir = TempDir::new().unwrap();
        
        // Create test files
        let files: Vec<PathBuf> = (0..3).map(|i| {
            let file_path = temp_dir.path().join(format!("test{}.rs", i));
            let mut file = fs::File::create(&file_path).unwrap();
            writeln!(file, "fn main{} () {{ println!(\"Hello {}!\"); }}", i, i).unwrap();
            file_path
        }).collect();
        
        // Fill cache to capacity
        let _ast0 = cache.get_or_parse(&files[0]).unwrap();
        let _ast1 = cache.get_or_parse(&files[1]).unwrap();
        
        // Adding third file should evict first (LRU)
        let _ast2 = cache.get_or_parse(&files[2]).unwrap();
        
        // Accessing first file should be a miss (was evicted)
        let _ast0_again = cache.get_or_parse(&files[0]).unwrap();
        let stats = cache.get_stats();
        
        // Should have 4 misses total (initial 3 + 1 re-access of evicted)
        assert_eq!(stats.cache_misses, 4);
        assert_eq!(stats.entries_count, 2); // Cache size limit
    }
}