/// Cache statistics and monitoring utilities
///
/// Provides comprehensive statistics collection and reporting for all cache types:
/// - Hit/miss rates and trends
/// - Memory usage and capacity utilization
/// - Cache entry lifecycle metrics
/// - Performance impact measurements
/// - Export capabilities for monitoring systems
use super::CacheStats;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Comprehensive cache statistics across all cache types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalCacheStats {
    /// Template cache statistics
    pub template_cache: Option<CacheTypeStats>,
    /// Response cache statistics
    pub response_cache: Option<CacheTypeStats>,
    /// Query cache statistics
    pub query_cache: Option<CacheTypeStats>,
    /// Memory cache statistics (generic)
    pub memory_caches: Vec<NamedCacheStats>,
    /// Overall statistics across all caches
    pub overall: OverallCacheStats,
    /// Statistics collection timestamp
    pub collected_at: u64,
}

/// Statistics for a specific cache type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheTypeStats {
    /// Base cache statistics
    pub base: CacheStats,
    /// Cache type specific metrics
    pub specific_metrics: HashMap<String, f64>,
    /// Recent performance trends
    pub trends: CacheTrends,
}

/// Named cache statistics for identification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamedCacheStats {
    /// Cache identifier
    pub name: String,
    /// Cache statistics
    pub stats: CacheTypeStats,
}

/// Overall statistics across all cache types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverallCacheStats {
    /// Total hits across all caches
    pub total_hits: u64,
    /// Total misses across all caches
    pub total_misses: u64,
    /// Total entries across all caches
    pub total_entries: u64,
    /// Total maximum capacity across all caches
    pub total_max_entries: u64,
    /// Overall hit rate
    pub overall_hit_rate: f64,
    /// Overall capacity utilization
    pub overall_utilization: f64,
    /// Estimated memory usage in bytes
    pub estimated_memory_bytes: u64,
}

/// Cache performance trends over time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheTrends {
    /// Hit rate trend (positive = improving, negative = declining)
    pub hit_rate_trend: f64,
    /// Entry count trend
    pub entry_count_trend: f64,
    /// Average response time trend (for applicable caches)
    pub response_time_trend_ms: Option<f64>,
    /// Data points used for trend calculation
    pub data_points: usize,
}

/// Cache statistics collector and manager
pub struct CacheStatsCollector {
    /// Historical data points for trend analysis
    history: Vec<CacheStatsSnapshot>,
    /// Maximum history size
    max_history_size: usize,
    /// Collection interval
    _collection_interval: Duration,
    /// Last collection timestamp
    last_collection: u64,
}

/// Point-in-time cache statistics snapshot
#[derive(Debug, Clone)]
struct CacheStatsSnapshot {
    _timestamp: u64,
    stats: GlobalCacheStats,
}

impl CacheStatsCollector {
    /// Create new cache statistics collector
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            max_history_size: 100, // Keep last 100 snapshots
            _collection_interval: Duration::from_secs(60), // Collect every minute
            last_collection: 0,
        }
    }

    /// Create collector with custom configuration
    pub fn with_config(max_history_size: usize, _collection_interval: Duration) -> Self {
        Self {
            history: Vec::new(),
            max_history_size,
            _collection_interval,
            last_collection: 0,
        }
    }

    /// Collect current cache statistics
    pub fn collect_stats(&mut self, cache_sources: &CacheStatsSources) -> GlobalCacheStats {
        let now = current_timestamp();

        // Collect individual cache statistics
        let template_cache = cache_sources.template_cache.as_ref().map(|cache| {
            let base_stats = cache();
            CacheTypeStats {
                base: base_stats.clone(),
                specific_metrics: HashMap::new(),
                trends: self.calculate_trends("template", &base_stats),
            }
        });

        let response_cache = cache_sources.response_cache.as_ref().map(|cache| {
            let base_stats = cache();
            let mut specific_metrics = HashMap::new();
            specific_metrics.insert("conditional_requests".to_string(), 0.0); // Placeholder

            CacheTypeStats {
                base: base_stats.clone(),
                specific_metrics,
                trends: self.calculate_trends("response", &base_stats),
            }
        });

        let query_cache = cache_sources.query_cache.as_ref().map(|cache| {
            let base_stats = cache();
            let mut specific_metrics = HashMap::new();
            specific_metrics.insert("cached_tables".to_string(), 0.0); // Placeholder

            CacheTypeStats {
                base: base_stats.clone(),
                specific_metrics,
                trends: self.calculate_trends("query", &base_stats),
            }
        });

        // Collect named memory caches
        let memory_caches: Vec<NamedCacheStats> = cache_sources
            .memory_caches
            .iter()
            .map(|(name, cache_fn)| {
                let base_stats = cache_fn();
                NamedCacheStats {
                    name: name.clone(),
                    stats: CacheTypeStats {
                        base: base_stats.clone(),
                        specific_metrics: HashMap::new(),
                        trends: self.calculate_trends(name, &base_stats),
                    },
                }
            })
            .collect();

        // Calculate overall statistics
        let overall = self.calculate_overall_stats(
            &template_cache,
            &response_cache,
            &query_cache,
            &memory_caches,
        );

        let global_stats = GlobalCacheStats {
            template_cache,
            response_cache,
            query_cache,
            memory_caches,
            overall,
            collected_at: now,
        };

        // Add to history for trend analysis
        self.add_to_history(global_stats.clone());
        self.last_collection = now;

        global_stats
    }

    /// Calculate overall statistics across all cache types
    fn calculate_overall_stats(
        &self,
        template_cache: &Option<CacheTypeStats>,
        response_cache: &Option<CacheTypeStats>,
        query_cache: &Option<CacheTypeStats>,
        memory_caches: &[NamedCacheStats],
    ) -> OverallCacheStats {
        let mut total_hits = 0;
        let mut total_misses = 0;
        let mut total_entries = 0;
        let mut total_max_entries = 0;
        let mut estimated_memory_bytes = 0;

        // Aggregate template cache stats
        if let Some(stats) = template_cache {
            total_hits += stats.base.hits;
            total_misses += stats.base.misses;
            total_entries += stats.base.entries;
            total_max_entries += stats.base.max_entries;
            estimated_memory_bytes += self.estimate_cache_memory(&stats.base);
        }

        // Aggregate response cache stats
        if let Some(stats) = response_cache {
            total_hits += stats.base.hits;
            total_misses += stats.base.misses;
            total_entries += stats.base.entries;
            total_max_entries += stats.base.max_entries;
            estimated_memory_bytes += self.estimate_cache_memory(&stats.base);
        }

        // Aggregate query cache stats
        if let Some(stats) = query_cache {
            total_hits += stats.base.hits;
            total_misses += stats.base.misses;
            total_entries += stats.base.entries;
            total_max_entries += stats.base.max_entries;
            estimated_memory_bytes += self.estimate_cache_memory(&stats.base);
        }

        // Aggregate memory cache stats
        for named_cache in memory_caches {
            total_hits += named_cache.stats.base.hits;
            total_misses += named_cache.stats.base.misses;
            total_entries += named_cache.stats.base.entries;
            total_max_entries += named_cache.stats.base.max_entries;
            estimated_memory_bytes += self.estimate_cache_memory(&named_cache.stats.base);
        }

        let overall_hit_rate = if total_hits + total_misses > 0 {
            total_hits as f64 / (total_hits + total_misses) as f64
        } else {
            0.0
        };

        let overall_utilization = if total_max_entries > 0 {
            total_entries as f64 / total_max_entries as f64
        } else {
            0.0
        };

        OverallCacheStats {
            total_hits,
            total_misses,
            total_entries,
            total_max_entries,
            overall_hit_rate,
            overall_utilization,
            estimated_memory_bytes,
        }
    }

    /// Estimate memory usage for a cache (rough approximation)
    fn estimate_cache_memory(&self, stats: &CacheStats) -> u64 {
        // Very rough estimation: assume average 1KB per entry
        // In production, this would be more sophisticated
        stats.entries * 1024
    }

    /// Calculate performance trends for a specific cache
    fn calculate_trends(&self, cache_name: &str, _current_stats: &CacheStats) -> CacheTrends {
        if self.history.len() < 2 {
            return CacheTrends {
                hit_rate_trend: 0.0,
                entry_count_trend: 0.0,
                response_time_trend_ms: None,
                data_points: 0,
            };
        }

        // Get historical hit rates for this cache type
        let historical_hit_rates: Vec<f64> = self
            .history
            .iter()
            .filter_map(|snapshot| self.extract_cache_hit_rate(&snapshot.stats, cache_name))
            .collect();

        let historical_entry_counts: Vec<u64> = self
            .history
            .iter()
            .filter_map(|snapshot| self.extract_cache_entry_count(&snapshot.stats, cache_name))
            .collect();

        let hit_rate_trend = self.calculate_linear_trend(&historical_hit_rates);
        let entry_count_trend = self.calculate_linear_trend(
            &historical_entry_counts
                .iter()
                .map(|&x| x as f64)
                .collect::<Vec<_>>(),
        );

        CacheTrends {
            hit_rate_trend,
            entry_count_trend,
            response_time_trend_ms: None, // NOTE: Response time tracking planned for future release
            data_points: historical_hit_rates.len(),
        }
    }

    /// Extract hit rate for specific cache from global stats
    fn extract_cache_hit_rate(
        &self,
        global_stats: &GlobalCacheStats,
        cache_name: &str,
    ) -> Option<f64> {
        match cache_name {
            "template" => global_stats
                .template_cache
                .as_ref()
                .map(|s| s.base.hit_rate()),
            "response" => global_stats
                .response_cache
                .as_ref()
                .map(|s| s.base.hit_rate()),
            "query" => global_stats.query_cache.as_ref().map(|s| s.base.hit_rate()),
            _ => {
                // Look in memory caches
                global_stats
                    .memory_caches
                    .iter()
                    .find(|mc| mc.name == cache_name)
                    .map(|mc| mc.stats.base.hit_rate())
            }
        }
    }

    /// Extract entry count for specific cache from global stats
    fn extract_cache_entry_count(
        &self,
        global_stats: &GlobalCacheStats,
        cache_name: &str,
    ) -> Option<u64> {
        match cache_name {
            "template" => global_stats.template_cache.as_ref().map(|s| s.base.entries),
            "response" => global_stats.response_cache.as_ref().map(|s| s.base.entries),
            "query" => global_stats.query_cache.as_ref().map(|s| s.base.entries),
            _ => {
                // Look in memory caches
                global_stats
                    .memory_caches
                    .iter()
                    .find(|mc| mc.name == cache_name)
                    .map(|mc| mc.stats.base.entries)
            }
        }
    }

    /// Calculate linear trend (slope) for a series of values
    fn calculate_linear_trend(&self, values: &[f64]) -> f64 {
        if values.len() < 2 {
            return 0.0;
        }

        let n = values.len() as f64;
        let x_sum: f64 = (0..values.len()).map(|i| i as f64).sum();
        let y_sum: f64 = values.iter().sum();
        let xy_sum: f64 = values.iter().enumerate().map(|(i, &y)| i as f64 * y).sum();
        let x_squared_sum: f64 = (0..values.len()).map(|i| (i as f64).powi(2)).sum();

        let denominator = n * x_squared_sum - x_sum.powi(2);
        if denominator.abs() < f64::EPSILON {
            return 0.0;
        }

        (n * xy_sum - x_sum * y_sum) / denominator
    }

    /// Add statistics snapshot to history
    fn add_to_history(&mut self, stats: GlobalCacheStats) {
        let snapshot = CacheStatsSnapshot {
            _timestamp: stats.collected_at,
            stats,
        };

        self.history.push(snapshot);

        // Trim history if it exceeds maximum size
        if self.history.len() > self.max_history_size {
            self.history.remove(0);
        }
    }

    /// Export statistics in various formats
    pub fn export_stats(&self, format: ExportFormat) -> Result<String, Box<dyn std::error::Error>> {
        if self.history.is_empty() {
            return Ok(String::new());
        }

        let latest_stats = &self.history.last().unwrap().stats;

        match format {
            ExportFormat::Json => Ok(serde_json::to_string_pretty(latest_stats)?),
            ExportFormat::Prometheus => Ok(self.to_prometheus_format(latest_stats)),
            ExportFormat::Csv => Ok(self.to_csv_format(latest_stats)),
        }
    }

    /// Convert to Prometheus metrics format
    fn to_prometheus_format(&self, stats: &GlobalCacheStats) -> String {
        let mut output = String::new();

        // Overall metrics
        output.push_str("# HELP rustf_cache_hits_total Total cache hits across all caches\n");
        output.push_str("# TYPE rustf_cache_hits_total counter\n");
        output.push_str(&format!(
            "rustf_cache_hits_total {}\n",
            stats.overall.total_hits
        ));

        output.push_str("# HELP rustf_cache_misses_total Total cache misses across all caches\n");
        output.push_str("# TYPE rustf_cache_misses_total counter\n");
        output.push_str(&format!(
            "rustf_cache_misses_total {}\n",
            stats.overall.total_misses
        ));

        output.push_str("# HELP rustf_cache_hit_rate Overall cache hit rate\n");
        output.push_str("# TYPE rustf_cache_hit_rate gauge\n");
        output.push_str(&format!(
            "rustf_cache_hit_rate {:.4}\n",
            stats.overall.overall_hit_rate
        ));

        output.push_str("# HELP rustf_cache_utilization Overall cache capacity utilization\n");
        output.push_str("# TYPE rustf_cache_utilization gauge\n");
        output.push_str(&format!(
            "rustf_cache_utilization {:.4}\n",
            stats.overall.overall_utilization
        ));

        // Per-cache type metrics
        if let Some(template_stats) = &stats.template_cache {
            self.add_cache_metrics(&mut output, "template", &template_stats.base);
        }

        if let Some(response_stats) = &stats.response_cache {
            self.add_cache_metrics(&mut output, "response", &response_stats.base);
        }

        if let Some(query_stats) = &stats.query_cache {
            self.add_cache_metrics(&mut output, "query", &query_stats.base);
        }

        output
    }

    /// Add cache-specific metrics to Prometheus output
    fn add_cache_metrics(&self, output: &mut String, cache_type: &str, stats: &CacheStats) {
        output.push_str(&format!(
            "rustf_cache_hits_total{{cache_type=\"{}\"}} {}\n",
            cache_type, stats.hits
        ));
        output.push_str(&format!(
            "rustf_cache_misses_total{{cache_type=\"{}\"}} {}\n",
            cache_type, stats.misses
        ));
        output.push_str(&format!(
            "rustf_cache_entries{{cache_type=\"{}\"}} {}\n",
            cache_type, stats.entries
        ));
        output.push_str(&format!(
            "rustf_cache_hit_rate{{cache_type=\"{}\"}} {:.4}\n",
            cache_type,
            stats.hit_rate()
        ));
        output.push_str(&format!(
            "rustf_cache_utilization{{cache_type=\"{}\"}} {:.4}\n",
            cache_type,
            stats.utilization()
        ));
    }

    /// Convert to CSV format
    fn to_csv_format(&self, stats: &GlobalCacheStats) -> String {
        let mut output = String::new();
        output.push_str(
            "timestamp,cache_type,hits,misses,entries,max_entries,hit_rate,utilization\n",
        );

        let timestamp = stats.collected_at;

        if let Some(template_stats) = &stats.template_cache {
            output.push_str(&format!(
                "{},template,{},{},{},{},{:.4},{:.4}\n",
                timestamp,
                template_stats.base.hits,
                template_stats.base.misses,
                template_stats.base.entries,
                template_stats.base.max_entries,
                template_stats.base.hit_rate(),
                template_stats.base.utilization()
            ));
        }

        if let Some(response_stats) = &stats.response_cache {
            output.push_str(&format!(
                "{},response,{},{},{},{},{:.4},{:.4}\n",
                timestamp,
                response_stats.base.hits,
                response_stats.base.misses,
                response_stats.base.entries,
                response_stats.base.max_entries,
                response_stats.base.hit_rate(),
                response_stats.base.utilization()
            ));
        }

        if let Some(query_stats) = &stats.query_cache {
            output.push_str(&format!(
                "{},query,{},{},{},{},{:.4},{:.4}\n",
                timestamp,
                query_stats.base.hits,
                query_stats.base.misses,
                query_stats.base.entries,
                query_stats.base.max_entries,
                query_stats.base.hit_rate(),
                query_stats.base.utilization()
            ));
        }

        output
    }
}

/// Export format options
#[derive(Debug, Clone)]
pub enum ExportFormat {
    Json,
    Prometheus,
    Csv,
}

/// Cache statistics sources for collection
pub struct CacheStatsSources {
    /// Function to get template cache stats
    pub template_cache: Option<Box<dyn Fn() -> CacheStats>>,
    /// Function to get response cache stats
    pub response_cache: Option<Box<dyn Fn() -> CacheStats>>,
    /// Function to get query cache stats
    pub query_cache: Option<Box<dyn Fn() -> CacheStats>>,
    /// Named memory caches with their stats functions
    pub memory_caches: Vec<(String, Box<dyn Fn() -> CacheStats>)>,
}

/// Get current timestamp
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

impl Default for CacheStatsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_trend_calculation() {
        let collector = CacheStatsCollector::new();

        // Test increasing trend
        let increasing = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let trend = collector.calculate_linear_trend(&increasing);
        assert!(trend > 0.0);

        // Test decreasing trend
        let decreasing = vec![5.0, 4.0, 3.0, 2.0, 1.0];
        let trend = collector.calculate_linear_trend(&decreasing);
        assert!(trend < 0.0);

        // Test flat trend
        let flat = vec![3.0, 3.0, 3.0, 3.0, 3.0];
        let trend = collector.calculate_linear_trend(&flat);
        assert_eq!(trend, 0.0);
    }

    #[test]
    fn test_overall_stats_calculation() {
        let collector = CacheStatsCollector::new();

        let template_stats = Some(CacheTypeStats {
            base: CacheStats {
                hits: 100,
                misses: 20,
                entries: 50,
                max_entries: 100,
                evictions: 0,
                expired_cleanups: 0,
                total_access_count: 120,
                average_entry_age: 60.0,
            },
            specific_metrics: HashMap::new(),
            trends: CacheTrends {
                hit_rate_trend: 0.0,
                entry_count_trend: 0.0,
                response_time_trend_ms: None,
                data_points: 0,
            },
        });

        let overall = collector.calculate_overall_stats(&template_stats, &None, &None, &[]);

        assert_eq!(overall.total_hits, 100);
        assert_eq!(overall.total_misses, 20);
        assert_eq!(overall.total_entries, 50);
        assert!((overall.overall_hit_rate - 100.0 / 120.0).abs() < f64::EPSILON);
    }
}
