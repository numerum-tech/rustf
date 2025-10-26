use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};

/// File metadata information cached for fast access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub path: PathBuf,
    pub size: u64,
    pub modified_time: u64,
    pub is_directory: bool,
    pub extension: Option<String>,
    pub content_hash: Option<u64>, // Optional for content-based caching
}

/// Cached entry with access tracking
#[derive(Debug, Clone)]
struct CachedMetadata {
    metadata: FileMetadata,
    cached_at: u64,
    access_count: u64,
    last_verified: u64,
}

/// Fast file metadata cache with automatic staleness detection
pub struct MetadataCache {
    entries: Arc<RwLock<HashMap<PathBuf, CachedMetadata>>>,
    access_order: Arc<RwLock<Vec<PathBuf>>>,
    stats: Arc<RwLock<CacheStats>>,
    max_entries: usize,
    max_age_seconds: u64,
    verification_interval: u64, // How often to verify file still exists/unchanged
}

#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub verifications: u64,
    pub invalidations: u64,
    pub entries_count: usize,
    pub hit_rate: f64,
    pub total_requests: u64,
}

impl Default for MetadataCache {
    fn default() -> Self {
        Self::new(1000, 3600, 60) // 1000 entries, 1 hour max age, verify every minute
    }
}

impl MetadataCache {
    /// Create new metadata cache with specified limits
    pub fn new(max_entries: usize, max_age_seconds: u64, verification_interval: u64) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            access_order: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(CacheStats::default())),
            max_entries,
            max_age_seconds,
            verification_interval,
        }
    }

    /// Get file metadata with caching and automatic verification
    pub fn get_metadata(&self, path: &Path) -> Result<FileMetadata> {
        let now = Self::current_timestamp();
        let path_buf = path.to_path_buf();
        
        // Try to get from cache first
        if let Some(cached) = self.get_cached_metadata(&path_buf, now) {
            self.update_stats(true);
            return Ok(cached);
        }

        // Cache miss - read from filesystem
        let metadata = self.read_file_metadata(path)?;
        self.cache_metadata(path_buf, metadata.clone(), now);
        self.update_stats(false);
        
        Ok(metadata)
    }

    /// Get metadata for multiple files in parallel
    pub fn get_metadata_batch(&self, paths: &[PathBuf]) -> Vec<Result<FileMetadata>> {
        use rayon::prelude::*;
        
        paths.par_iter()
            .map(|path| self.get_metadata(path))
            .collect()
    }

    /// Check if file exists without full metadata read (fast path)
    pub fn file_exists(&self, path: &Path) -> bool {
        let path_buf = path.to_path_buf();
        let now = Self::current_timestamp();
        
        // Check cache first
        if let Ok(entries) = self.entries.read() {
            if let Some(cached) = entries.get(&path_buf) {
                // If recently verified, trust the cache
                if now.saturating_sub(cached.last_verified) < self.verification_interval {
                    return !cached.metadata.is_directory || path.exists();
                }
            }
        }
        
        // Fallback to filesystem check
        path.exists()
    }

    /// Get directory contents with caching
    pub fn list_directory(&self, dir_path: &Path) -> Result<Vec<FileMetadata>> {
        if !dir_path.is_dir() {
            return Err(anyhow::anyhow!("Path is not a directory: {}", dir_path.display()));
        }

        let mut results = Vec::new();
        
        for entry in fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();
            
            match self.get_metadata(&path) {
                Ok(metadata) => results.push(metadata),
                Err(e) => {
                    log::warn!("Failed to get metadata for {}: {}", path.display(), e);
                }
            }
        }
        
        // Sort by name for consistent results
        results.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(results)
    }

    /// Filter files by extension with caching
    pub fn filter_by_extension(&self, paths: &[PathBuf], extension: &str) -> Vec<PathBuf> {
        paths.iter()
            .filter(|path| {
                if let Ok(metadata) = self.get_metadata(path) {
                    metadata.extension.as_ref().map_or(false, |ext| ext == extension)
                } else {
                    false
                }
            })
            .cloned()
            .collect()
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> CacheStats {
        if let Ok(stats) = self.stats.read() {
            let mut stats = stats.clone();
            if let Ok(entries) = self.entries.read() {
                stats.entries_count = entries.len();
            }
            stats.total_requests = stats.hits + stats.misses;
            if stats.total_requests > 0 {
                stats.hit_rate = (stats.hits as f64 / stats.total_requests as f64) * 100.0;
            }
            stats
        } else {
            CacheStats::default()
        }
    }

    /// Clear all cached entries
    pub fn clear(&self) {
        if let Ok(mut entries) = self.entries.write() {
            entries.clear();
        }
        if let Ok(mut access_order) = self.access_order.write() {
            access_order.clear();
        }
        if let Ok(mut stats) = self.stats.write() {
            *stats = CacheStats::default();
        }
    }

    /// Remove stale entries based on file system changes
    pub fn cleanup_stale(&self) {
        let now = Self::current_timestamp();
        let mut to_remove = Vec::new();
        
        if let Ok(entries) = self.entries.read() {
            for (path, cached) in entries.iter() {
                // Remove if too old
                if now.saturating_sub(cached.cached_at) > self.max_age_seconds {
                    to_remove.push(path.clone());
                    continue;
                }
                
                // Remove if file no longer exists or has changed
                if let Ok(current_meta) = fs::metadata(path) {
                    let current_modified = current_meta.modified()
                        .unwrap_or(SystemTime::UNIX_EPOCH)
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    
                    if current_modified != cached.metadata.modified_time {
                        to_remove.push(path.clone());
                    }
                } else {
                    // File no longer exists
                    to_remove.push(path.clone());
                }
            }
        }
        
        // Remove stale entries
        if !to_remove.is_empty() {
            if let Ok(mut entries) = self.entries.write() {
                for path in &to_remove {
                    entries.remove(path);
                }
            }
            
            if let Ok(mut access_order) = self.access_order.write() {
                access_order.retain(|p| !to_remove.contains(p));
            }
            
            if let Ok(mut stats) = self.stats.write() {
                stats.invalidations += to_remove.len() as u64;
            }
        }
    }

    // Private helper methods
    
    fn get_cached_metadata(&self, path: &PathBuf, now: u64) -> Option<FileMetadata> {
        if let Ok(mut entries) = self.entries.write() {
            if let Some(cached) = entries.get_mut(path) {
                // Check if verification is needed
                if now.saturating_sub(cached.last_verified) >= self.verification_interval {
                    // Quick verification - just check modification time
                    if let Ok(meta) = fs::metadata(path) {
                        let current_modified = meta.modified()
                            .unwrap_or(SystemTime::UNIX_EPOCH)
                            .duration_since(SystemTime::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();
                        
                        if current_modified != cached.metadata.modified_time {
                            // File has changed, invalidate cache
                            return None;
                        }
                        
                        cached.last_verified = now;
                        cached.access_count += 1;
                        
                        if let Ok(mut stats) = self.stats.write() {
                            stats.verifications += 1;
                        }
                    } else {
                        // File no longer exists
                        return None;
                    }
                }
                
                // Update access tracking
                self.update_access_order(path);
                return Some(cached.metadata.clone());
            }
        }
        None
    }

    fn cache_metadata(&self, path: PathBuf, metadata: FileMetadata, now: u64) {
        // Ensure we don't exceed max entries (LRU eviction)
        self.ensure_capacity();
        
        let cached = CachedMetadata {
            metadata,
            cached_at: now,
            access_count: 1,
            last_verified: now,
        };
        
        if let Ok(mut entries) = self.entries.write() {
            entries.insert(path.clone(), cached);
        }
        
        self.update_access_order(&path);
    }

    fn read_file_metadata(&self, path: &Path) -> Result<FileMetadata> {
        let meta = fs::metadata(path)
            .with_context(|| format!("Failed to read metadata for {}", path.display()))?;
        
        let modified_time = meta.modified()
            .unwrap_or(SystemTime::UNIX_EPOCH)
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_lowercase());
        
        Ok(FileMetadata {
            path: path.to_path_buf(),
            size: meta.len(),
            modified_time,
            is_directory: meta.is_dir(),
            extension,
            content_hash: None, // Could be computed on demand for specific use cases
        })
    }

    fn ensure_capacity(&self) {
        if let Ok(entries) = self.entries.read() {
            if entries.len() >= self.max_entries {
                drop(entries); // Release read lock
                
                // Remove least recently used entries
                if let (Ok(mut entries), Ok(mut access_order)) = 
                    (self.entries.write(), self.access_order.write()) {
                    
                    let to_remove = entries.len().saturating_sub(self.max_entries * 3 / 4); // Remove 25% when full
                    
                    for _ in 0..to_remove {
                        if let Some(lru_path) = access_order.first().cloned() {
                            entries.remove(&lru_path);
                            access_order.retain(|p| p != &lru_path);
                        } else {
                            break;
                        }
                    }
                }
            }
        }
    }

    fn update_access_order(&self, path: &PathBuf) {
        if let Ok(mut access_order) = self.access_order.write() {
            // Remove if already present
            access_order.retain(|p| p != path);
            // Add to end (most recent)
            access_order.push(path.clone());
        }
    }

    fn update_stats(&self, hit: bool) {
        if let Ok(mut stats) = self.stats.write() {
            if hit {
                stats.hits += 1;
            } else {
                stats.misses += 1;
            }
        }
    }

    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

/// Global metadata cache instance for the CLI
static GLOBAL_METADATA_CACHE: std::sync::OnceLock<MetadataCache> = std::sync::OnceLock::new();

/// Get the global metadata cache instance
pub fn global_metadata_cache() -> &'static MetadataCache {
    GLOBAL_METADATA_CACHE.get_or_init(|| {
        MetadataCache::new(2000, 7200, 120) // 2000 entries, 2 hours, verify every 2 minutes
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_metadata_cache_basic() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        // Create test file
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"test content").unwrap();
        
        let cache = MetadataCache::new(10, 3600, 60);
        
        // First access - cache miss
        let metadata1 = cache.get_metadata(&file_path).unwrap();
        assert_eq!(metadata1.path, file_path);
        assert_eq!(metadata1.size, 12);
        assert!(!metadata1.is_directory);
        assert_eq!(metadata1.extension, Some("txt".to_string()));
        
        // Second access - cache hit
        let metadata2 = cache.get_metadata(&file_path).unwrap();
        assert_eq!(metadata1.modified_time, metadata2.modified_time);
        
        let stats = cache.get_stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert!(stats.hit_rate > 0.0);
    }

    #[test]
    fn test_directory_listing() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();
        
        // Create test files
        File::create(dir_path.join("file1.rs")).unwrap();
        File::create(dir_path.join("file2.txt")).unwrap();
        fs::create_dir(dir_path.join("subdir")).unwrap();
        
        let cache = MetadataCache::new(10, 3600, 60);
        let contents = cache.list_directory(dir_path).unwrap();
        
        assert_eq!(contents.len(), 3);
        
        // Verify file types
        let rs_files: Vec<_> = contents.iter()
            .filter(|m| m.extension == Some("rs".to_string()))
            .collect();
        assert_eq!(rs_files.len(), 1);
        
        let directories: Vec<_> = contents.iter()
            .filter(|m| m.is_directory)
            .collect();
        assert_eq!(directories.len(), 1);
    }

    #[test]
    fn test_extension_filtering() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();
        
        // Create test files
        let file1 = dir_path.join("test1.rs");
        let file2 = dir_path.join("test2.txt");
        let file3 = dir_path.join("test3.rs");
        
        File::create(&file1).unwrap();
        File::create(&file2).unwrap();
        File::create(&file3).unwrap();
        
        let cache = MetadataCache::new(10, 3600, 60);
        let all_files = vec![file1, file2, file3];
        
        let rs_files = cache.filter_by_extension(&all_files, "rs");
        assert_eq!(rs_files.len(), 2);
        
        let txt_files = cache.filter_by_extension(&all_files, "txt");
        assert_eq!(txt_files.len(), 1);
    }

    #[test]
    fn test_cache_eviction() {
        let cache = MetadataCache::new(3, 3600, 60); // Small cache for testing
        let temp_dir = TempDir::new().unwrap();
        
        // Create more files than cache capacity
        for i in 0..5 {
            let file_path = temp_dir.path().join(format!("file{}.txt", i));
            File::create(&file_path).unwrap();
            let _ = cache.get_metadata(&file_path);
        }
        
        let stats = cache.get_stats();
        assert!(stats.entries_count <= 3); // Should not exceed max capacity
    }
}