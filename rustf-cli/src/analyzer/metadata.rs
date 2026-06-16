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
    access_count: u64,
    last_verified: u64,
}

/// Fast file metadata cache with automatic staleness detection
pub struct MetadataCache {
    entries: Arc<RwLock<HashMap<PathBuf, CachedMetadata>>>,
    access_order: Arc<RwLock<Vec<PathBuf>>>,
    max_entries: usize,
    verification_interval: u64, // How often to verify file still exists/unchanged
}

impl Default for MetadataCache {
    fn default() -> Self {
        Self::new(1000, 60) // 1000 entries, verify every minute
    }
}

impl MetadataCache {
    /// Create new metadata cache with specified limits
    pub fn new(max_entries: usize, verification_interval: u64) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            access_order: Arc::new(RwLock::new(Vec::new())),
            max_entries,
            verification_interval,
        }
    }

    /// Get file metadata with caching and automatic verification
    pub fn get_metadata(&self, path: &Path) -> Result<FileMetadata> {
        let now = Self::current_timestamp();
        let path_buf = path.to_path_buf();
        
        // Try to get from cache first
        if let Some(cached) = self.get_cached_metadata(&path_buf, now) {
            return Ok(cached);
        }

        // Cache miss - read from filesystem
        let metadata = self.read_file_metadata(path)?;
        self.cache_metadata(path_buf, metadata.clone(), now);

        Ok(metadata)
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
        MetadataCache::new(2000, 120) // 2000 entries, verify every 2 minutes
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

        let cache = MetadataCache::new(10, 60);

        // First access - cache miss
        let metadata1 = cache.get_metadata(&file_path).unwrap();
        assert_eq!(metadata1.path, file_path);
        assert_eq!(metadata1.size, 12);
        assert!(!metadata1.is_directory);
        assert_eq!(metadata1.extension, Some("txt".to_string()));

        // Second access - cache hit returns equivalent metadata
        let metadata2 = cache.get_metadata(&file_path).unwrap();
        assert_eq!(metadata1.modified_time, metadata2.modified_time);
    }

    #[test]
    fn test_directory_listing() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        // Create test files
        File::create(dir_path.join("file1.rs")).unwrap();
        File::create(dir_path.join("file2.txt")).unwrap();
        fs::create_dir(dir_path.join("subdir")).unwrap();

        let cache = MetadataCache::new(10, 60);
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
}