use std::path::PathBuf;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use anyhow::Result;

use super::lru_cache::ThreadSafeLruCache;
use super::{ProjectAnalysis, ControllerInfo, RouteInfo};

/// Cache key for analysis results
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum AnalysisCacheKey {
    /// Complete project analysis
    ProjectComplete { project_path: PathBuf, include_views: bool },
    /// Controller-specific analysis
    Controller { file_path: PathBuf },
    /// Route analysis for a specific controller
    Routes { controller_path: PathBuf },
    /// Middleware analysis
    Middleware { file_path: PathBuf },
    /// Model analysis
    Model { file_path: PathBuf },
    /// View analysis
    View { file_path: PathBuf },
    /// Custom analysis type
    Custom { cache_key: String },
}

/// Cached analysis result with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedAnalysis {
    pub key: String,
    pub result: AnalysisResult,
    pub file_checksums: HashMap<PathBuf, String>,
    pub cached_at: DateTime<Utc>,
    pub access_count: u64,
    pub last_access: DateTime<Utc>,
}

/// Analysis result variants that can be cached
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnalysisResult {
    ProjectAnalysis(Box<ProjectAnalysis>),
    Controllers(Vec<ControllerInfo>),
    Routes(Vec<RouteInfo>),
    FileContent(String),
    Json(serde_json::Value),
}

impl AnalysisResult {
    pub fn as_project_analysis(&self) -> Option<&ProjectAnalysis> {
        match self {
            AnalysisResult::ProjectAnalysis(analysis) => Some(analysis),
            _ => None,
        }
    }

    pub fn as_controllers(&self) -> Option<&[ControllerInfo]> {
        match self {
            AnalysisResult::Controllers(controllers) => Some(controllers),
            _ => None,
        }
    }

    pub fn as_routes(&self) -> Option<&[RouteInfo]> {
        match self {
            AnalysisResult::Routes(routes) => Some(routes),
            _ => None,
        }
    }

    pub fn as_json(&self) -> Option<&serde_json::Value> {
        match self {
            AnalysisResult::Json(json) => Some(json),
            _ => None,
        }
    }
}

/// High-performance analysis result cache with intelligent invalidation
pub struct AnalysisCache {
    lru_cache: ThreadSafeLruCache<AnalysisCacheKey, CachedAnalysis>,
    file_watcher: HashMap<PathBuf, DateTime<Utc>>, // Track file modification times
    cache_stats: CacheStats,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub invalidations: u64,
    pub total_requests: u64,
    pub cache_size: usize,
    pub hit_rate: f64,
}

impl AnalysisCache {
    pub fn new(capacity: usize, ttl_seconds: i64) -> Self {
        Self {
            lru_cache: ThreadSafeLruCache::new(capacity, ttl_seconds),
            file_watcher: HashMap::new(),
            cache_stats: CacheStats::default(),
        }
    }

    /// Get cached analysis result if valid
    pub async fn get(&mut self, key: &AnalysisCacheKey) -> Option<AnalysisResult> {
        self.cache_stats.total_requests += 1;

        if let Some(mut cached) = self.lru_cache.get(key) {
            // Check if any tracked files have been modified
            if self.is_cache_valid(&cached).await {
                cached.access_count += 1;
                cached.last_access = Utc::now();
                
                // Update the cache with new access info
                self.lru_cache.put(key.clone(), cached.clone());
                
                self.cache_stats.hits += 1;
                self.update_hit_rate();
                
                Some(cached.result)
            } else {
                // Cache is invalid, remove it
                self.lru_cache.remove(key);
                self.cache_stats.invalidations += 1;
                self.cache_stats.misses += 1;
                self.update_hit_rate();
                None
            }
        } else {
            self.cache_stats.misses += 1;
            self.update_hit_rate();
            None
        }
    }

    /// Put analysis result in cache with file tracking
    pub async fn put(
        &mut self, 
        key: AnalysisCacheKey, 
        result: AnalysisResult,
        tracked_files: Vec<PathBuf>
    ) -> Result<()> {
        // Calculate checksums for tracked files
        let mut file_checksums = HashMap::new();
        for file_path in tracked_files {
            if let Ok(content) = tokio::fs::read(&file_path).await {
                let checksum = format!("{:x}", md5::compute(&content));
                file_checksums.insert(file_path.clone(), checksum);
                
                // Track file modification time
                if let Ok(metadata) = tokio::fs::metadata(&file_path).await {
                    if let Ok(modified) = metadata.modified() {
                        let datetime: DateTime<Utc> = modified.into();
                        self.file_watcher.insert(file_path, datetime);
                    }
                }
            }
        }

        let cached = CachedAnalysis {
            key: format!("{:?}", key),
            result,
            file_checksums,
            cached_at: Utc::now(),
            access_count: 0,
            last_access: Utc::now(),
        };

        self.lru_cache.put(key, cached);
        self.cache_stats.cache_size = self.lru_cache.len();
        
        Ok(())
    }

    /// Check if cached result is still valid based on file modifications
    async fn is_cache_valid(&self, cached: &CachedAnalysis) -> bool {
        for (file_path, expected_checksum) in &cached.file_checksums {
            // Check if file still exists
            if !file_path.exists() {
                return false;
            }

            // Check if file was modified after cache time
            if let Ok(metadata) = tokio::fs::metadata(file_path).await {
                if let Ok(modified) = metadata.modified() {
                    let datetime: DateTime<Utc> = modified.into();
                    if datetime > cached.cached_at {
                        return false;
                    }
                }
            }

            // Check file checksum for content changes
            if let Ok(content) = tokio::fs::read(file_path).await {
                let current_checksum = format!("{:x}", md5::compute(&content));
                if &current_checksum != expected_checksum {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }

    /// Invalidate cache entries for specific files
    pub fn invalidate_by_file(&mut self, file_path: &PathBuf) {
        let keys_to_remove: Vec<_> = self.get_keys_for_file(file_path);
        
        for key in keys_to_remove {
            self.lru_cache.remove(&key);
            self.cache_stats.invalidations += 1;
        }
        
        self.cache_stats.cache_size = self.lru_cache.len();
    }

    /// Get all cache keys that depend on a specific file
    fn get_keys_for_file(&self, _file_path: &PathBuf) -> Vec<AnalysisCacheKey> {
        // This is a simplified implementation
        // In practice, you'd want to track which cache entries depend on which files
        Vec::new()
    }

    /// Clear all cached entries
    pub fn clear(&mut self) {
        self.lru_cache.clear();
        self.file_watcher.clear();
        self.cache_stats = CacheStats::default();
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> CacheStats {
        let mut stats = self.cache_stats.clone();
        stats.cache_size = self.lru_cache.len();
        stats
    }

    /// Clean up expired entries
    pub fn cleanup_expired(&mut self) -> usize {
        let removed = self.lru_cache.cleanup_expired();
        self.cache_stats.cache_size = self.lru_cache.len();
        removed
    }

    /// Update hit rate calculation
    fn update_hit_rate(&mut self) {
        if self.cache_stats.total_requests > 0 {
            self.cache_stats.hit_rate = self.cache_stats.hits as f64 / self.cache_stats.total_requests as f64;
        }
    }

    /// Get cache utilization metrics
    pub fn get_utilization_metrics(&self) -> CacheUtilizationMetrics {
        let lru_stats = self.lru_cache.get_statistics();
        
        CacheUtilizationMetrics {
            capacity: lru_stats.capacity,
            current_size: lru_stats.current_size,
            utilization_percentage: (lru_stats.current_size as f64 / lru_stats.capacity as f64) * 100.0,
            average_age_seconds: lru_stats.average_age_seconds,
            oldest_entry_age_seconds: lru_stats.oldest_entry_age_seconds,
            most_accessed_keys: lru_stats.most_accessed_keys,
            tracked_files_count: self.file_watcher.len(),
        }
    }

    /// Preload cache with common analysis patterns
    pub async fn preload_common_patterns(&mut self, project_path: &PathBuf) -> Result<usize> {
        let loaded_count = 0;
        
        // This would be implemented based on common usage patterns
        // For example, always cache the main project analysis
        let _key = AnalysisCacheKey::ProjectComplete {
            project_path: project_path.clone(),
            include_views: false,
        };
        
        // Implementation would go here...
        
        Ok(loaded_count)
    }
}

#[derive(Debug, Serialize)]
pub struct CacheUtilizationMetrics {
    pub capacity: usize,
    pub current_size: usize,
    pub utilization_percentage: f64,
    pub average_age_seconds: f64,
    pub oldest_entry_age_seconds: f64,
    pub most_accessed_keys: Vec<String>,
    pub tracked_files_count: usize,
}

impl Default for AnalysisCache {
    fn default() -> Self {
        Self::new(500, 1800) // 500 entries, 30 minutes TTL
    }
}

// Global cache instance
use std::sync::Arc;
use tokio::sync::Mutex as AsyncMutex;
use once_cell::sync::Lazy;

static GLOBAL_ANALYSIS_CACHE: Lazy<Arc<AsyncMutex<AnalysisCache>>> = Lazy::new(|| {
    Arc::new(AsyncMutex::new(AnalysisCache::default()))
});

/// Get the global analysis cache instance
pub fn global_analysis_cache() -> Arc<AsyncMutex<AnalysisCache>> {
    GLOBAL_ANALYSIS_CACHE.clone()
}

/// Convenience functions for common cache operations
pub async fn cache_project_analysis(
    project_path: PathBuf,
    include_views: bool,
    analysis: ProjectAnalysis,
    tracked_files: Vec<PathBuf>,
) -> Result<()> {
    let key = AnalysisCacheKey::ProjectComplete { project_path, include_views };
    let result = AnalysisResult::ProjectAnalysis(Box::new(analysis));
    
    {
        let cache_arc = global_analysis_cache();
        let mut cache = cache_arc.lock().await;
        cache.put(key, result, tracked_files).await?;
    }
    
    Ok(())
}

pub async fn get_cached_project_analysis(
    project_path: &PathBuf,
    include_views: bool,
) -> Option<ProjectAnalysis> {
    let key = AnalysisCacheKey::ProjectComplete {
        project_path: project_path.clone(),
        include_views,
    };
    
    {
        let cache_arc = global_analysis_cache();
        let mut cache = cache_arc.lock().await;
        if let Some(result) = cache.get(&key).await {
            return result.as_project_analysis().cloned();
        }
    }
    
    None
}

pub async fn invalidate_file_cache(file_path: &PathBuf) {
    let cache_arc = global_analysis_cache();
    let mut cache = cache_arc.lock().await;
    cache.invalidate_by_file(file_path);
}

pub async fn get_cache_statistics() -> Option<CacheStats> {
    let cache_arc = global_analysis_cache();
    let cache = cache_arc.lock().await;
    Some(cache.get_stats())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs::write;

    #[tokio::test]
    async fn test_cache_basic_operations() {
        let mut cache = AnalysisCache::new(10, 60);
        
        let key = AnalysisCacheKey::Custom { cache_key: "test".to_string() };
        let result = AnalysisResult::Json(serde_json::json!({"test": "data"}));
        
        // Cache miss initially
        assert!(cache.get(&key).await.is_none());
        
        // Put and get
        cache.put(key.clone(), result, vec![]).await.unwrap();
        assert!(cache.get(&key).await.is_some());
    }

    #[tokio::test]
    async fn test_file_based_invalidation() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.rs");
        write(&file_path, "fn test() {}").unwrap();
        
        let mut cache = AnalysisCache::new(10, 60);
        let key = AnalysisCacheKey::Controller { file_path: file_path.clone() };
        let result = AnalysisResult::Json(serde_json::json!({"function": "test"}));
        
        // Cache the result
        cache.put(key.clone(), result, vec![file_path.clone()]).await.unwrap();
        assert!(cache.get(&key).await.is_some());
        
        // Modify the file
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        write(&file_path, "fn test() { println!(); }").unwrap();
        
        // Cache should be invalid now
        assert!(cache.get(&key).await.is_none());
    }
}