//! Python Object Cache Module
//!
//! This module provides intelligent caching for Python objects and string representations,
//! reducing the overhead of repeated object creation and GIL acquisition.

use once_cell::sync::Lazy;
use pyo3::prelude::*;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Configuration for Python object cache
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PythonCacheConfig {
    /// Maximum number of string entries in cache
    pub max_string_entries: usize,
    /// Maximum number of object entries in cache
    pub max_object_entries: usize,
    /// Time-to-live for cache entries
    pub ttl: Duration,
    /// Enable automatic cleanup of expired entries
    pub auto_cleanup: bool,
    /// Cleanup interval for expired entries
    pub cleanup_interval: Duration,
    /// Enable cache statistics collection
    pub enable_stats: bool,
    /// Memory limit for cached objects (in bytes)
    pub memory_limit: usize,
}

impl Default for PythonCacheConfig {
    fn default() -> Self {
        Self {
            max_string_entries: 5000,
            max_object_entries: 1000,
            ttl: Duration::from_secs(600), // 10 minutes
            auto_cleanup: true,
            cleanup_interval: Duration::from_secs(120), // 2 minutes
            enable_stats: true,
            memory_limit: 50 * 1024 * 1024, // 50MB
        }
    }
}

impl PythonCacheConfig {
    /// Create a high-performance cache configuration
    pub fn high_performance() -> Self {
        Self {
            max_string_entries: 10000,
            max_object_entries: 2000,
            ttl: Duration::from_secs(1800),  // 30 minutes
            memory_limit: 100 * 1024 * 1024, // 100MB
            ..Default::default()
        }
    }

    /// Create a memory-optimized cache configuration
    pub fn memory_optimized() -> Self {
        Self {
            max_string_entries: 2000,
            max_object_entries: 500,
            ttl: Duration::from_secs(300),  // 5 minutes
            memory_limit: 20 * 1024 * 1024, // 20MB
            ..Default::default()
        }
    }
}

/// Cache entry for string representations
#[derive(Debug, Clone)]
struct StringCacheEntry {
    value: String,
    created_at: Instant,
    access_count: u64,
    last_accessed: Instant,
}

impl StringCacheEntry {
    fn new(value: String) -> Self {
        let now = Instant::now();
        Self {
            value,
            created_at: now,
            access_count: 1,
            last_accessed: now,
        }
    }

    fn access(&mut self) -> &str {
        self.access_count += 1;
        self.last_accessed = Instant::now();
        &self.value
    }

    fn is_expired(&self, ttl: Duration) -> bool {
        self.created_at.elapsed() > ttl
    }

    fn estimated_size(&self) -> usize {
        self.value.len() + std::mem::size_of::<Self>()
    }
}

/// Cache entry for Python objects
#[derive(Debug)]
struct ObjectCacheEntry {
    object: PyObject,
    created_at: Instant,
    access_count: u64,
    last_accessed: Instant,
    estimated_size: usize,
}

impl ObjectCacheEntry {
    fn new(object: PyObject, estimated_size: usize) -> Self {
        let now = Instant::now();
        Self {
            object,
            created_at: now,
            access_count: 1,
            last_accessed: now,
            estimated_size,
        }
    }

    fn access(&mut self) -> &PyObject {
        self.access_count += 1;
        self.last_accessed = Instant::now();
        &self.object
    }

    fn is_expired(&self, ttl: Duration) -> bool {
        self.created_at.elapsed() > ttl
    }
}

/// Cache statistics
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PythonCacheStats {
    /// Total number of string cache hits
    pub string_hits: u64,
    /// Total number of string cache misses
    pub string_misses: u64,
    /// Total number of object cache hits
    pub object_hits: u64,
    /// Total number of object cache misses
    pub object_misses: u64,
    /// Current number of string entries
    pub string_entries: usize,
    /// Current number of object entries
    pub object_entries: usize,
    /// Estimated memory usage in bytes
    pub memory_usage: usize,
    /// Number of expired entries removed
    pub expired_removals: u64,
    /// Number of LRU evictions
    pub lru_evictions: u64,
}

impl PythonCacheStats {
    /// Calculate string cache hit rate
    pub fn string_hit_rate(&self) -> f64 {
        let total = self.string_hits + self.string_misses;
        if total == 0 {
            0.0
        } else {
            (self.string_hits as f64 / total as f64) * 100.0
        }
    }

    /// Calculate object cache hit rate
    pub fn object_hit_rate(&self) -> f64 {
        let total = self.object_hits + self.object_misses;
        if total == 0 {
            0.0
        } else {
            (self.object_hits as f64 / total as f64) * 100.0
        }
    }

    /// Calculate overall hit rate
    pub fn overall_hit_rate(&self) -> f64 {
        let total_hits = self.string_hits + self.object_hits;
        let total_misses = self.string_misses + self.object_misses;
        let total = total_hits + total_misses;
        if total == 0 {
            0.0
        } else {
            (total_hits as f64 / total as f64) * 100.0
        }
    }
}

/// Intelligent cache manager for Python objects and strings
#[derive(Debug)]
pub struct PythonCacheManager {
    config: PythonCacheConfig,
    string_cache: HashMap<u64, StringCacheEntry>,
    object_cache: HashMap<u64, ObjectCacheEntry>,
    stats: PythonCacheStats,
    last_cleanup: Instant,
}

impl PythonCacheManager {
    /// Create a new cache manager with default configuration
    pub fn new() -> Self {
        Self::with_config(PythonCacheConfig::default())
    }

    /// Create a new cache manager with custom configuration
    pub fn with_config(config: PythonCacheConfig) -> Self {
        Self {
            config,
            string_cache: HashMap::new(),
            object_cache: HashMap::new(),
            stats: PythonCacheStats::default(),
            last_cleanup: Instant::now(),
        }
    }

    /// Get or insert a string representation in cache
    pub fn get_or_insert_string<K, F>(&mut self, key: K, factory: F) -> String
    where
        K: Hash,
        F: FnOnce() -> String,
    {
        let hash_key = self.hash_key(&key);

        // Check if entry exists and is not expired
        if let Some(entry) = self.string_cache.get_mut(&hash_key) {
            if !entry.is_expired(self.config.ttl) {
                if self.config.enable_stats {
                    self.stats.string_hits += 1;
                }
                return entry.access().to_string();
            } else {
                // Entry is expired, remove it
                self.string_cache.remove(&hash_key);
                if self.config.enable_stats {
                    self.stats.expired_removals += 1;
                }
            }
        }

        // Cache miss - create new entry
        let value = factory();
        let entry = StringCacheEntry::new(value.clone());

        // Check memory usage before insertion
        let entry_size = entry.estimated_size();
        if self.stats.memory_usage + entry_size > self.config.memory_limit {
            self.evict_lru_strings();
        }

        self.string_cache.insert(hash_key, entry);

        if self.config.enable_stats {
            self.stats.string_misses += 1;
            self.stats.string_entries = self.string_cache.len();
            self.stats.memory_usage += entry_size;
        }

        // Check if we need to evict entries
        if self.string_cache.len() > self.config.max_string_entries {
            self.evict_lru_strings();
        }

        // Periodic cleanup
        if self.config.auto_cleanup && self.last_cleanup.elapsed() > self.config.cleanup_interval {
            self.cleanup_expired();
        }

        value
    }

    /// Get or insert a Python object in cache
    pub fn get_or_insert_object<K, F>(
        &mut self,
        py: Python<'_>,
        key: K,
        factory: F,
    ) -> PyResult<PyObject>
    where
        K: Hash,
        F: FnOnce(Python<'_>) -> PyResult<PyObject>,
    {
        let hash_key = self.hash_key(&key);

        // Check if entry exists and is not expired
        if let Some(entry) = self.object_cache.get_mut(&hash_key) {
            if !entry.is_expired(self.config.ttl) {
                if self.config.enable_stats {
                    self.stats.object_hits += 1;
                }
                return Ok(entry.access().clone_ref(py));
            } else {
                // Entry is expired, remove it
                let removed_entry = self.object_cache.remove(&hash_key).unwrap();
                if self.config.enable_stats {
                    self.stats.expired_removals += 1;
                    self.stats.memory_usage = self
                        .stats
                        .memory_usage
                        .saturating_sub(removed_entry.estimated_size);
                }
            }
        }

        // Cache miss - create new entry
        let object = factory(py)?;
        let estimated_size = self.estimate_object_size(&object);

        // Check memory usage before insertion
        if self.stats.memory_usage + estimated_size > self.config.memory_limit {
            self.evict_lru_objects();
        }

        let entry = ObjectCacheEntry::new(object.clone_ref(py), estimated_size);
        self.object_cache.insert(hash_key, entry);

        if self.config.enable_stats {
            self.stats.object_misses += 1;
            self.stats.object_entries = self.object_cache.len();
            self.stats.memory_usage += estimated_size;
        }

        // Check if we need to evict entries
        if self.object_cache.len() > self.config.max_object_entries {
            self.evict_lru_objects();
        }

        // Periodic cleanup
        if self.config.auto_cleanup && self.last_cleanup.elapsed() > self.config.cleanup_interval {
            self.cleanup_expired();
        }

        Ok(object)
    }

    /// Get cache statistics
    pub fn stats(&self) -> &PythonCacheStats {
        &self.stats
    }

    /// Clear all cache entries
    pub fn clear(&mut self) {
        self.string_cache.clear();
        self.object_cache.clear();
        self.stats = PythonCacheStats::default();
    }

    // Private helper methods
    fn hash_key<K: Hash>(&self, key: &K) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }

    fn estimate_object_size(&self, _object: &PyObject) -> usize {
        // Conservative estimate - in practice, this could be more sophisticated
        std::mem::size_of::<PyObject>() + 256
    }

    fn evict_lru_strings(&mut self) {
        if self.string_cache.is_empty() {
            return;
        }

        // Find the least recently used entry
        let lru_key = self
            .string_cache
            .iter()
            .min_by_key(|(_, entry)| entry.last_accessed)
            .map(|(key, _)| *key);

        if let Some(key) = lru_key {
            if let Some(entry) = self.string_cache.remove(&key) {
                if self.config.enable_stats {
                    self.stats.lru_evictions += 1;
                    self.stats.string_entries = self.string_cache.len();
                    self.stats.memory_usage = self
                        .stats
                        .memory_usage
                        .saturating_sub(entry.estimated_size());
                }
            }
        }
    }

    fn evict_lru_objects(&mut self) {
        if self.object_cache.is_empty() {
            return;
        }

        // Find the least recently used entry
        let lru_key = self
            .object_cache
            .iter()
            .min_by_key(|(_, entry)| entry.last_accessed)
            .map(|(key, _)| *key);

        if let Some(key) = lru_key {
            if let Some(entry) = self.object_cache.remove(&key) {
                if self.config.enable_stats {
                    self.stats.lru_evictions += 1;
                    self.stats.object_entries = self.object_cache.len();
                    self.stats.memory_usage =
                        self.stats.memory_usage.saturating_sub(entry.estimated_size);
                }
            }
        }
    }

    fn cleanup_expired(&mut self) {
        let now = Instant::now();

        // Cleanup expired string entries
        let expired_string_keys: Vec<_> = self
            .string_cache
            .iter()
            .filter(|(_, entry)| entry.is_expired(self.config.ttl))
            .map(|(key, _)| *key)
            .collect();

        for key in expired_string_keys {
            if let Some(entry) = self.string_cache.remove(&key) {
                if self.config.enable_stats {
                    self.stats.expired_removals += 1;
                    self.stats.memory_usage = self
                        .stats
                        .memory_usage
                        .saturating_sub(entry.estimated_size());
                }
            }
        }

        // Cleanup expired object entries
        let expired_object_keys: Vec<_> = self
            .object_cache
            .iter()
            .filter(|(_, entry)| entry.is_expired(self.config.ttl))
            .map(|(key, _)| *key)
            .collect();

        for key in expired_object_keys {
            if let Some(entry) = self.object_cache.remove(&key) {
                if self.config.enable_stats {
                    self.stats.expired_removals += 1;
                    self.stats.memory_usage =
                        self.stats.memory_usage.saturating_sub(entry.estimated_size);
                }
            }
        }

        if self.config.enable_stats {
            self.stats.string_entries = self.string_cache.len();
            self.stats.object_entries = self.object_cache.len();
        }

        self.last_cleanup = now;
    }
}

impl Default for PythonCacheManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Global cache manager instance
static GLOBAL_CACHE: Lazy<Arc<RwLock<PythonCacheManager>>> = Lazy::new(|| {
    Arc::new(RwLock::new(PythonCacheManager::with_config(
        PythonCacheConfig::high_performance(),
    )))
});

/// Get a reference to the global cache manager
pub fn global_cache() -> Arc<RwLock<PythonCacheManager>> {
    GLOBAL_CACHE.clone()
}

/// Convenience function to get or insert a string in the global cache
pub fn get_or_insert_string<K, F>(key: K, factory: F) -> String
where
    K: Hash,
    F: FnOnce() -> String,
{
    let binding = global_cache();
    let mut cache = binding.write().unwrap();
    cache.get_or_insert_string(key, factory)
}

/// Convenience function to get or insert an object in the global cache
pub fn get_or_insert_object<K, F>(py: Python<'_>, key: K, factory: F) -> PyResult<PyObject>
where
    K: Hash,
    F: FnOnce(Python<'_>) -> PyResult<PyObject>,
{
    let binding = global_cache();
    let mut cache = binding.write().unwrap();
    cache.get_or_insert_object(py, key, factory)
}

/// Get global cache statistics
pub fn global_cache_stats() -> PythonCacheStats {
    let binding = global_cache();
    let cache = binding.read().unwrap();
    cache.stats().clone()
}

/// Python binding for cache statistics
#[pyclass(name = "CacheStats")]
#[derive(Debug, Clone)]
pub struct PyCacheStats {
    /// Total number of string cache hits
    #[pyo3(get)]
    pub string_hits: u64,
    /// Total number of string cache misses
    #[pyo3(get)]
    pub string_misses: u64,
    /// Total number of object cache hits
    #[pyo3(get)]
    pub object_hits: u64,
    /// Total number of object cache misses
    #[pyo3(get)]
    pub object_misses: u64,
    /// Current number of string entries
    #[pyo3(get)]
    pub string_entries: usize,
    /// Current number of object entries
    #[pyo3(get)]
    pub object_entries: usize,
    /// Estimated memory usage in bytes
    #[pyo3(get)]
    pub memory_usage: usize,
    /// Number of expired entries removed
    #[pyo3(get)]
    pub expired_removals: u64,
    /// Number of LRU evictions
    #[pyo3(get)]
    pub lru_evictions: u64,
}

#[pymethods]
impl PyCacheStats {
    /// Calculate string cache hit rate
    #[getter]
    pub fn string_hit_rate(&self) -> f64 {
        let total = self.string_hits + self.string_misses;
        if total == 0 {
            0.0
        } else {
            (self.string_hits as f64 / total as f64) * 100.0
        }
    }

    /// Calculate object cache hit rate
    #[getter]
    pub fn object_hit_rate(&self) -> f64 {
        let total = self.object_hits + self.object_misses;
        if total == 0 {
            0.0
        } else {
            (self.object_hits as f64 / total as f64) * 100.0
        }
    }

    /// Calculate overall hit rate
    #[getter]
    pub fn overall_hit_rate(&self) -> f64 {
        let total_hits = self.string_hits + self.object_hits;
        let total_misses = self.string_misses + self.object_misses;
        let total = total_hits + total_misses;
        if total == 0 {
            0.0
        } else {
            (total_hits as f64 / total as f64) * 100.0
        }
    }

    /// String representation
    fn __repr__(&self) -> String {
        format!(
            "CacheStats(string_hit_rate={:.1}%, object_hit_rate={:.1}%, memory_usage={}MB)",
            self.string_hit_rate(),
            self.object_hit_rate(),
            self.memory_usage / (1024 * 1024)
        )
    }
}

impl From<PythonCacheStats> for PyCacheStats {
    fn from(stats: PythonCacheStats) -> Self {
        Self {
            string_hits: stats.string_hits,
            string_misses: stats.string_misses,
            object_hits: stats.object_hits,
            object_misses: stats.object_misses,
            string_entries: stats.string_entries,
            object_entries: stats.object_entries,
            memory_usage: stats.memory_usage,
            expired_removals: stats.expired_removals,
            lru_evictions: stats.lru_evictions,
        }
    }
}

/// Python binding for cache configuration
#[pyclass(name = "CacheConfig")]
#[derive(Debug, Clone)]
pub struct PyCacheConfig {
    /// Maximum number of string entries in cache
    #[pyo3(get, set)]
    pub max_string_entries: usize,
    /// Maximum number of object entries in cache
    #[pyo3(get, set)]
    pub max_object_entries: usize,
    /// Time-to-live for cache entries in seconds
    #[pyo3(get, set)]
    pub ttl_seconds: u64,
    /// Enable automatic cleanup of expired entries
    #[pyo3(get, set)]
    pub auto_cleanup: bool,
    /// Cleanup interval for expired entries in seconds
    #[pyo3(get, set)]
    pub cleanup_interval_seconds: u64,
    /// Enable cache statistics collection
    #[pyo3(get, set)]
    pub enable_stats: bool,
    /// Memory limit for cached objects in megabytes
    #[pyo3(get, set)]
    pub memory_limit_mb: usize,
}

#[pymethods]
impl PyCacheConfig {
    /// Create a new cache configuration
    #[new]
    #[pyo3(signature = (
        max_string_entries = 5000,
        max_object_entries = 1000,
        ttl_seconds = 600,
        auto_cleanup = true,
        cleanup_interval_seconds = 120,
        enable_stats = true,
        memory_limit_mb = 50
    ))]
    pub fn new(
        max_string_entries: usize,
        max_object_entries: usize,
        ttl_seconds: u64,
        auto_cleanup: bool,
        cleanup_interval_seconds: u64,
        enable_stats: bool,
        memory_limit_mb: usize,
    ) -> Self {
        Self {
            max_string_entries,
            max_object_entries,
            ttl_seconds,
            auto_cleanup,
            cleanup_interval_seconds,
            enable_stats,
            memory_limit_mb,
        }
    }

    /// Create a high-performance cache configuration
    #[staticmethod]
    pub fn high_performance() -> Self {
        Self {
            max_string_entries: 10000,
            max_object_entries: 2000,
            ttl_seconds: 1800, // 30 minutes
            auto_cleanup: true,
            cleanup_interval_seconds: 120,
            enable_stats: true,
            memory_limit_mb: 100,
        }
    }

    /// Create a memory-optimized cache configuration
    #[staticmethod]
    pub fn memory_optimized() -> Self {
        Self {
            max_string_entries: 2000,
            max_object_entries: 500,
            ttl_seconds: 300, // 5 minutes
            auto_cleanup: true,
            cleanup_interval_seconds: 120,
            enable_stats: true,
            memory_limit_mb: 20,
        }
    }

    /// String representation
    fn __repr__(&self) -> String {
        format!(
            "CacheConfig(string_entries={}, object_entries={}, ttl={}s, memory_limit={}MB)",
            self.max_string_entries,
            self.max_object_entries,
            self.ttl_seconds,
            self.memory_limit_mb
        )
    }
}

impl From<PyCacheConfig> for PythonCacheConfig {
    fn from(config: PyCacheConfig) -> Self {
        Self {
            max_string_entries: config.max_string_entries,
            max_object_entries: config.max_object_entries,
            ttl: Duration::from_secs(config.ttl_seconds),
            auto_cleanup: config.auto_cleanup,
            cleanup_interval: Duration::from_secs(config.cleanup_interval_seconds),
            enable_stats: config.enable_stats,
            memory_limit: config.memory_limit_mb * 1024 * 1024,
        }
    }
}

/// Python functions for cache management
#[pyfunction]
pub fn get_cache_stats() -> PyCacheStats {
    global_cache_stats().into()
}

/// Clear all cache entries
#[pyfunction]
pub fn clear_cache() {
    let binding = global_cache();
    let mut cache = binding.write().unwrap();
    cache.clear();
}

/// Configure the global cache with new settings
#[pyfunction]
pub fn configure_cache(config: PyCacheConfig) {
    let rust_config = PythonCacheConfig::from(config);
    let binding = global_cache();
    let mut cache = binding.write().unwrap();
    *cache = PythonCacheManager::with_config(rust_config);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_manager_creation() {
        let manager = PythonCacheManager::new();
        let stats = manager.stats();
        assert_eq!(stats.string_hits, 0);
        assert_eq!(stats.object_hits, 0);
        assert_eq!(stats.string_entries, 0);
        assert_eq!(stats.object_entries, 0);
    }

    #[test]
    fn test_string_cache() {
        let mut manager = PythonCacheManager::new();

        // First access - cache miss
        let result1 = manager.get_or_insert_string("test_key", || "test_value".to_string());
        assert_eq!(result1, "test_value");
        assert_eq!(manager.stats().string_misses, 1);
        assert_eq!(manager.stats().string_entries, 1);

        // Second access - cache hit
        let result2 = manager.get_or_insert_string("test_key", || "different_value".to_string());
        assert_eq!(result2, "test_value"); // Should return cached value
        assert_eq!(manager.stats().string_hits, 1);
        assert_eq!(manager.stats().string_entries, 1);
    }

    #[test]
    fn test_cache_config() {
        let config = PythonCacheConfig::high_performance();
        assert_eq!(config.max_string_entries, 10000);
        assert_eq!(config.max_object_entries, 2000);

        let config = PythonCacheConfig::memory_optimized();
        assert_eq!(config.max_string_entries, 2000);
        assert_eq!(config.max_object_entries, 500);
    }

    #[test]
    fn test_cache_stats() {
        let mut manager = PythonCacheManager::new();

        // Add some cache entries
        manager.get_or_insert_string("key1", || "value1".to_string());
        manager.get_or_insert_string("key2", || "value2".to_string());
        manager.get_or_insert_string("key1", || "value1_new".to_string()); // Hit

        let stats = manager.stats();
        assert_eq!(stats.string_misses, 2);
        assert_eq!(stats.string_hits, 1);
        assert_eq!(stats.string_entries, 2);
        assert_eq!(stats.string_hit_rate(), 33.333333333333336); // 1/3 * 100
    }

    #[test]
    fn test_cache_clear() {
        let mut manager = PythonCacheManager::new();

        // Add some entries
        manager.get_or_insert_string("key1", || "value1".to_string());
        manager.get_or_insert_string("key2", || "value2".to_string());

        assert_eq!(manager.stats().string_entries, 2);

        // Clear cache
        manager.clear();

        let stats = manager.stats();
        assert_eq!(stats.string_entries, 0);
        assert_eq!(stats.object_entries, 0);
        assert_eq!(stats.string_hits, 0);
        assert_eq!(stats.string_misses, 0);
    }

    #[test]
    fn test_global_cache_functions() {
        // Clear cache first
        clear_cache();

        // Test string caching
        let result1 = get_or_insert_string("global_key", || "global_value".to_string());
        assert_eq!(result1, "global_value");

        let result2 = get_or_insert_string("global_key", || "different_value".to_string());
        assert_eq!(result2, "global_value"); // Should return cached value

        // Test stats
        let stats = global_cache_stats();
        assert_eq!(stats.string_misses, 1);
        assert_eq!(stats.string_hits, 1);
        assert!(stats.string_hit_rate() > 0.0);
    }

    #[test]
    fn test_py_cache_config_conversion() {
        let py_config = PyCacheConfig::high_performance();
        let rust_config = PythonCacheConfig::from(py_config);

        assert_eq!(rust_config.max_string_entries, 10000);
        assert_eq!(rust_config.max_object_entries, 2000);
        assert_eq!(rust_config.ttl, Duration::from_secs(1800));
        assert_eq!(rust_config.memory_limit, 100 * 1024 * 1024);
    }

    #[test]
    fn test_py_cache_stats_conversion() {
        let rust_stats = PythonCacheStats {
            string_hits: 10,
            string_misses: 5,
            object_hits: 8,
            object_misses: 2,
            string_entries: 15,
            object_entries: 10,
            memory_usage: 1024,
            expired_removals: 3,
            lru_evictions: 1,
        };

        let py_stats = PyCacheStats::from(rust_stats);
        assert_eq!(py_stats.string_hits, 10);
        assert_eq!(py_stats.string_misses, 5);
        assert_eq!(py_stats.string_hit_rate(), 66.66666666666667); // 10/15 * 100
        assert_eq!(py_stats.object_hit_rate(), 80.0); // 8/10 * 100
        assert_eq!(py_stats.overall_hit_rate(), 72.0); // 18/25 * 100
    }
}
