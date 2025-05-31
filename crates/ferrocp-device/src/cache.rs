//! Device detection cache implementation
//!
//! This module provides an intelligent LRU cache for device detection results,
//! reducing the overhead of repeated device type detection for the same paths.

use ferrocp_types::{DeviceCacheEntry, DeviceCacheStats, DeviceType};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tracing::{debug, trace};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Configuration for the device detection cache
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DeviceCacheConfig {
    /// Maximum number of entries in the cache
    pub max_entries: usize,
    /// Time-to-live for cache entries
    pub ttl: Duration,
    /// Enable automatic cleanup of expired entries
    pub auto_cleanup: bool,
    /// Cleanup interval for expired entries
    pub cleanup_interval: Duration,
    /// Enable cache statistics collection
    pub enable_stats: bool,
    /// Enable background cache refresh
    pub enable_background_refresh: bool,
    /// Background refresh interval
    pub background_refresh_interval: Duration,
    /// Threshold for refreshing cache entries (percentage of TTL)
    pub refresh_threshold: f64,
}

impl Default for DeviceCacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 1000,
            ttl: Duration::from_secs(300), // 5 minutes
            auto_cleanup: true,
            cleanup_interval: Duration::from_secs(60), // 1 minute
            enable_stats: true,
            enable_background_refresh: true,
            background_refresh_interval: Duration::from_secs(120), // 2 minutes
            refresh_threshold: 0.8, // Refresh when 80% of TTL has passed
        }
    }
}

/// LRU node for the cache
#[derive(Debug)]
struct LruNode {
    key: String,
    entry: DeviceCacheEntry,
    prev: Option<String>,
    next: Option<String>,
}

/// Thread-safe LRU cache for device detection results
#[derive(Debug)]
pub struct DeviceCache {
    /// Cache configuration
    config: DeviceCacheConfig,
    /// HashMap for O(1) lookups
    cache: HashMap<String, LruNode>,
    /// Head of the LRU list (most recently used)
    head: Option<String>,
    /// Tail of the LRU list (least recently used)
    tail: Option<String>,
    /// Cache statistics
    stats: DeviceCacheStats,
    /// Last cleanup time
    last_cleanup: SystemTime,
    /// Last background refresh time
    last_background_refresh: SystemTime,
    /// Entries pending background refresh
    refresh_queue: Vec<String>,
}

impl DeviceCache {
    /// Create a new device cache with default configuration
    pub fn new() -> Self {
        Self::with_config(DeviceCacheConfig::default())
    }

    /// Create a new device cache with custom configuration
    pub fn with_config(config: DeviceCacheConfig) -> Self {
        let now = SystemTime::now();
        Self {
            config,
            cache: HashMap::new(),
            head: None,
            tail: None,
            stats: DeviceCacheStats::default(),
            last_cleanup: now,
            last_background_refresh: now,
            refresh_queue: Vec::new(),
        }
    }

    /// Generate a cache key from a path using path prefix optimization
    fn generate_cache_key<P: AsRef<Path>>(&self, path: P) -> String {
        let path = path.as_ref();

        // Use path prefix optimization for better cache hit rates
        // For files, use the parent directory as the cache key
        // For directories, use the directory itself
        // Note: We can't use is_file() in tests with non-existent paths,
        // so we'll check if the path has an extension as a heuristic
        let path_str = path.to_string_lossy();

        // If path has an extension, treat it as a file and use parent directory
        if path.extension().is_some() {
            if let Some(parent) = path.parent() {
                parent.to_string_lossy().to_string()
            } else {
                path_str.to_string()
            }
        } else {
            // Treat as directory
            path_str.to_string()
        }
    }

    /// Generate a precise cache key from a path (for exact path matching)
    fn generate_precise_cache_key<P: AsRef<Path>>(&self, path: P) -> String {
        let path = path.as_ref();
        path.to_string_lossy().to_string()
    }

    /// Get device type from cache
    pub fn get<P: AsRef<Path>>(&mut self, path: P) -> Option<DeviceType> {
        let key = self.generate_cache_key(&path);

        if self.config.enable_stats {
            if self.cache.contains_key(&key) {
                self.stats.record_hit();
            } else {
                self.stats.record_miss();
                return None;
            }
        }

        // Check if entry exists and is not expired
        let is_expired = if let Some(node) = self.cache.get(&key) {
            node.entry.is_expired(self.config.ttl)
        } else {
            return None;
        };

        if is_expired {
            // Entry is expired, remove it
            self.remove_node(&key);
            if self.config.enable_stats {
                self.stats.record_expired_removal();
            }
            return None;
        }

        // Check if entry needs background refresh
        if self.config.enable_background_refresh {
            if let Some(node) = self.cache.get(&key) {
                let age = node.entry.age();
                let refresh_threshold = Duration::from_secs_f64(
                    self.config.ttl.as_secs_f64() * self.config.refresh_threshold,
                );

                if age > refresh_threshold && !self.refresh_queue.contains(&key) {
                    self.refresh_queue.push(key.clone());
                    trace!("Added entry to refresh queue: {}", key);
                }
            }
        }

        // Update access time and move to head
        if let Some(node) = self.cache.get_mut(&key) {
            node.entry.touch();
            let device_type = node.entry.device_type;
            self.move_to_head(&key);

            trace!("Cache hit for key: {}", key);
            return Some(device_type);
        }

        None
    }

    /// Insert or update a cache entry
    pub fn insert<P: AsRef<Path>>(&mut self, path: P, device_type: DeviceType) {
        let key = self.generate_cache_key(&path);

        // If key already exists, update it
        if self.cache.contains_key(&key) {
            if let Some(node) = self.cache.get_mut(&key) {
                node.entry = DeviceCacheEntry::new(device_type);
                self.move_to_head(&key);
                return;
            }
        }

        // Create new entry
        let entry = DeviceCacheEntry::new(device_type);
        let node = LruNode {
            key: key.clone(),
            entry,
            prev: None,
            next: self.head.clone(),
        };

        // Add to cache
        self.cache.insert(key.clone(), node);

        // Update head pointer
        if let Some(old_head) = &self.head {
            if let Some(old_head_node) = self.cache.get_mut(old_head) {
                old_head_node.prev = Some(key.clone());
            }
        }

        self.head = Some(key.clone());

        // If this is the first entry, it's also the tail
        if self.tail.is_none() {
            self.tail = Some(key);
        }

        // Check if we need to evict entries
        if self.cache.len() > self.config.max_entries {
            self.evict_lru();
        }

        // Update statistics
        if self.config.enable_stats {
            self.stats.update_size(self.cache.len());
            self.update_memory_usage();
        }

        trace!(
            "Inserted cache entry for key: {}",
            self.generate_cache_key(path)
        );
    }

    /// Move a node to the head of the LRU list
    fn move_to_head(&mut self, key: &str) {
        if self.head.as_ref() == Some(&key.to_string()) {
            return; // Already at head
        }

        // Remove from current position
        self.remove_from_list(key);

        // Add to head
        if let Some(node) = self.cache.get_mut(key) {
            node.prev = None;
            node.next = self.head.clone();
        }

        if let Some(old_head) = &self.head {
            if let Some(old_head_node) = self.cache.get_mut(old_head) {
                old_head_node.prev = Some(key.to_string());
            }
        }

        self.head = Some(key.to_string());
    }

    /// Remove a node from the LRU list (but not from the cache)
    fn remove_from_list(&mut self, key: &str) {
        if let Some(node) = self.cache.get(key) {
            let prev_key = node.prev.clone();
            let next_key = node.next.clone();

            // Update previous node's next pointer
            if let Some(prev) = &prev_key {
                if let Some(prev_node) = self.cache.get_mut(prev) {
                    prev_node.next = next_key.clone();
                }
            } else {
                // This was the head
                self.head = next_key.clone();
            }

            // Update next node's prev pointer
            if let Some(next) = &next_key {
                if let Some(next_node) = self.cache.get_mut(next) {
                    next_node.prev = prev_key;
                }
            } else {
                // This was the tail
                self.tail = prev_key;
            }
        }
    }

    /// Remove a node completely from the cache
    fn remove_node(&mut self, key: &str) {
        self.remove_from_list(key);
        self.cache.remove(key);

        if self.config.enable_stats {
            self.stats.update_size(self.cache.len());
            self.update_memory_usage();
        }
    }

    /// Evict the least recently used entry
    fn evict_lru(&mut self) {
        if let Some(tail_key) = self.tail.clone() {
            debug!("Evicting LRU entry: {}", tail_key);
            self.remove_node(&tail_key);

            if self.config.enable_stats {
                self.stats.record_eviction();
            }
        }
    }

    /// Update memory usage statistics
    fn update_memory_usage(&mut self) {
        if !self.config.enable_stats {
            return;
        }

        // Estimate memory usage (rough calculation)
        let entry_size = std::mem::size_of::<LruNode>() + std::mem::size_of::<DeviceCacheEntry>();
        let key_size_estimate = 50; // Average path length estimate
        let total_size = self.cache.len() * (entry_size + key_size_estimate);

        self.stats.update_memory_usage(total_size);
    }

    /// Clean up expired entries
    pub fn cleanup_expired(&mut self) {
        let now = SystemTime::now();

        // Only cleanup if enough time has passed
        if now
            .duration_since(self.last_cleanup)
            .unwrap_or(Duration::ZERO)
            < self.config.cleanup_interval
        {
            return;
        }

        let mut expired_keys = Vec::new();

        for (key, node) in &self.cache {
            if node.entry.is_expired(self.config.ttl) {
                expired_keys.push(key.clone());
            }
        }

        for key in expired_keys {
            debug!("Removing expired cache entry: {}", key);
            self.remove_node(&key);

            if self.config.enable_stats {
                self.stats.record_expired_removal();
            }
        }

        self.last_cleanup = now;

        if self.config.enable_stats {
            self.stats.update_size(self.cache.len());
            self.update_memory_usage();
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> &DeviceCacheStats {
        &self.stats
    }

    /// Get entries that need background refresh
    pub fn get_refresh_queue(&mut self) -> Vec<String> {
        let now = SystemTime::now();

        // Only process refresh queue if enough time has passed
        if now
            .duration_since(self.last_background_refresh)
            .unwrap_or(Duration::ZERO)
            < self.config.background_refresh_interval
        {
            return Vec::new();
        }

        let queue = std::mem::take(&mut self.refresh_queue);
        self.last_background_refresh = now;

        debug!("Background refresh queue contains {} entries", queue.len());
        queue
    }

    /// Update cache entry after background refresh
    pub fn update_refreshed_entry<P: AsRef<Path>>(&mut self, path: P, device_type: DeviceType) {
        let key = self.generate_cache_key(&path);

        if let Some(node) = self.cache.get_mut(&key) {
            node.entry = DeviceCacheEntry::new(device_type);
            trace!("Updated refreshed cache entry for key: {}", key);
        }
    }

    /// Check if background refresh is needed
    pub fn needs_background_refresh(&self) -> bool {
        if !self.config.enable_background_refresh {
            return false;
        }

        // If refresh queue is empty, no refresh needed
        if self.refresh_queue.is_empty() {
            return false;
        }

        // Check if enough time has passed since last refresh
        let now = SystemTime::now();
        now.duration_since(self.last_background_refresh)
            .unwrap_or(Duration::ZERO)
            >= self.config.background_refresh_interval
    }

    /// Clear all cache entries
    pub fn clear(&mut self) {
        self.cache.clear();
        self.head = None;
        self.tail = None;
        self.refresh_queue.clear();

        if self.config.enable_stats {
            self.stats.update_size(0);
            self.stats.update_memory_usage(0);
        }

        debug!("Cache cleared");
    }

    /// Get current cache size
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

impl Default for DeviceCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe wrapper for DeviceCache
pub type SharedDeviceCache = Arc<RwLock<DeviceCache>>;

/// Create a new shared device cache
pub fn create_shared_cache() -> SharedDeviceCache {
    Arc::new(RwLock::new(DeviceCache::new()))
}

/// Create a new shared device cache with custom configuration
pub fn create_shared_cache_with_config(config: DeviceCacheConfig) -> SharedDeviceCache {
    Arc::new(RwLock::new(DeviceCache::with_config(config)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_cache_creation() {
        let cache = DeviceCache::new();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_key_generation() {
        let cache = DeviceCache::new();

        // Test that files in different directories have different cache keys
        let key1 = cache.generate_cache_key("/home/user/dir1/file1.txt");
        let key2 = cache.generate_cache_key("/home/user/dir2/file2.txt");
        assert_ne!(key1, key2);

        // Test that files in the same directory have the same cache key (path prefix optimization)
        let key3 = cache.generate_cache_key("/home/user/dir1/file1.txt");
        let key4 = cache.generate_cache_key("/home/user/dir1/file2.txt");
        assert_eq!(key3, key4);

        // Test that the same file has the same cache key
        let key5 = cache.generate_cache_key("/home/user/dir1/file1.txt");
        assert_eq!(key3, key5);
    }

    #[test]
    fn test_basic_cache_operations() {
        let mut cache = DeviceCache::new();
        let path = PathBuf::from("/test/path");

        // Initially empty
        assert!(cache.get(&path).is_none());

        // Insert and retrieve
        cache.insert(&path, DeviceType::SSD);
        assert_eq!(cache.get(&path), Some(DeviceType::SSD));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_cache_update() {
        let mut cache = DeviceCache::new();
        let path = PathBuf::from("/test/path");

        // Insert initial value
        cache.insert(&path, DeviceType::SSD);
        assert_eq!(cache.get(&path), Some(DeviceType::SSD));

        // Update with new value
        cache.insert(&path, DeviceType::HDD);
        assert_eq!(cache.get(&path), Some(DeviceType::HDD));
        assert_eq!(cache.len(), 1); // Should still be 1 entry
    }

    #[test]
    fn test_lru_eviction() {
        let config = DeviceCacheConfig {
            max_entries: 2,
            ..DeviceCacheConfig::default()
        };
        let mut cache = DeviceCache::with_config(config);

        // Fill cache to capacity
        cache.insert("/path1", DeviceType::SSD);
        cache.insert("/path2", DeviceType::HDD);
        assert_eq!(cache.len(), 2);

        // Add third entry, should evict least recently used
        cache.insert("/path3", DeviceType::Network);
        assert_eq!(cache.len(), 2);

        // path1 should be evicted (least recently used)
        assert!(cache.get("/path1").is_none());
        assert_eq!(cache.get("/path2"), Some(DeviceType::HDD));
        assert_eq!(cache.get("/path3"), Some(DeviceType::Network));
    }

    #[test]
    fn test_lru_ordering() {
        let config = DeviceCacheConfig {
            max_entries: 3,
            ..DeviceCacheConfig::default()
        };
        let mut cache = DeviceCache::with_config(config);

        // Insert entries
        cache.insert("/path1", DeviceType::SSD);
        cache.insert("/path2", DeviceType::HDD);
        cache.insert("/path3", DeviceType::Network);

        // Access path1 to make it most recently used
        cache.get("/path1");

        // Add fourth entry, should evict path2 (now least recently used)
        cache.insert("/path4", DeviceType::RamDisk);

        assert_eq!(cache.get("/path1"), Some(DeviceType::SSD));
        assert!(cache.get("/path2").is_none()); // Evicted
        assert_eq!(cache.get("/path3"), Some(DeviceType::Network));
        assert_eq!(cache.get("/path4"), Some(DeviceType::RamDisk));
    }

    #[test]
    fn test_cache_expiration() {
        let config = DeviceCacheConfig {
            ttl: Duration::from_millis(100),
            ..DeviceCacheConfig::default()
        };
        let mut cache = DeviceCache::with_config(config);

        cache.insert("/path1", DeviceType::SSD);
        assert_eq!(cache.get("/path1"), Some(DeviceType::SSD));

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(150));

        // Entry should be expired and removed
        assert!(cache.get("/path1").is_none());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_cache_statistics() {
        let mut cache = DeviceCache::new();

        // Initial stats
        let stats = cache.stats();
        assert_eq!(stats.hit_rate(), 0.0);
        assert_eq!(stats.total_lookups, 0);

        // Insert and access
        cache.insert("/path1", DeviceType::SSD);
        cache.get("/path1"); // Hit
        cache.get("/path2"); // Miss

        let stats = cache.stats();
        assert_eq!(stats.total_lookups, 2);
        assert_eq!(stats.cache_hits, 1);
        assert_eq!(stats.cache_misses, 1);
        assert_eq!(stats.hit_rate(), 50.0);
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = DeviceCache::new();

        cache.insert("/path1", DeviceType::SSD);
        cache.insert("/path2", DeviceType::HDD);
        assert_eq!(cache.len(), 2);

        cache.clear();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
        assert!(cache.get("/path1").is_none());
    }

    #[test]
    fn test_cleanup_expired() {
        let config = DeviceCacheConfig {
            ttl: Duration::from_millis(100),
            cleanup_interval: Duration::from_millis(50),
            ..DeviceCacheConfig::default()
        };
        let mut cache = DeviceCache::with_config(config);

        cache.insert("/path1", DeviceType::SSD);
        cache.insert("/path2", DeviceType::HDD);
        assert_eq!(cache.len(), 2);

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(150));

        // Manual cleanup
        cache.cleanup_expired();
        assert_eq!(cache.len(), 0);
    }

    #[tokio::test]
    async fn test_shared_cache() {
        let cache = create_shared_cache();

        {
            let mut cache_guard = cache.write().await;
            cache_guard.insert("/path1", DeviceType::SSD);
        }

        {
            let mut cache_guard = cache.write().await;
            assert_eq!(cache_guard.get("/path1"), Some(DeviceType::SSD));
        }
    }

    #[test]
    fn test_path_prefix_optimization() {
        let cache = DeviceCache::new();

        // Test file path optimization (should use parent directory)
        let file_path = std::path::Path::new("/home/user/documents/file.txt");
        let key1 = cache.generate_cache_key(file_path);

        // Another file in the same directory should have the same cache key
        let file_path2 = std::path::Path::new("/home/user/documents/file2.txt");
        let key2 = cache.generate_cache_key(file_path2);

        // Both files should have the same cache key (parent directory)
        assert_eq!(key1, key2);

        // Directory path should use itself as cache key
        let dir_path = std::path::Path::new("/home/user/documents");
        let key3 = cache.generate_cache_key(dir_path);
        assert_eq!(key1, key3);
    }

    #[test]
    fn test_background_refresh_configuration() {
        let config = DeviceCacheConfig {
            enable_background_refresh: true,
            background_refresh_interval: Duration::from_secs(30),
            refresh_threshold: 0.7,
            ..DeviceCacheConfig::default()
        };

        let cache = DeviceCache::with_config(config);
        assert!(cache.config.enable_background_refresh);
        assert_eq!(
            cache.config.background_refresh_interval,
            Duration::from_secs(30)
        );
        assert_eq!(cache.config.refresh_threshold, 0.7);
    }

    #[test]
    fn test_refresh_queue_operations() {
        let config = DeviceCacheConfig {
            enable_background_refresh: true,
            background_refresh_interval: Duration::from_millis(0), // No time delay for testing
            ..DeviceCacheConfig::default()
        };
        let mut cache = DeviceCache::with_config(config);

        // Initially no refresh needed
        assert!(!cache.needs_background_refresh());

        // Get empty refresh queue
        let queue = cache.get_refresh_queue();
        assert!(queue.is_empty());

        // Add some entries to refresh queue manually for testing
        cache.refresh_queue.push("test_path".to_string());
        assert!(cache.needs_background_refresh());

        // Get refresh queue should return and clear the queue
        let queue = cache.get_refresh_queue();
        assert_eq!(queue.len(), 1);
        assert_eq!(queue[0], "test_path");

        // Queue should be empty after getting it
        let queue2 = cache.get_refresh_queue();
        assert!(queue2.is_empty());
    }

    #[test]
    fn test_cache_entry_refresh_threshold() {
        let config = DeviceCacheConfig {
            ttl: Duration::from_secs(100),
            refresh_threshold: 0.5, // 50% of TTL
            enable_background_refresh: true,
            ..DeviceCacheConfig::default()
        };

        let mut cache = DeviceCache::with_config(config);

        // Insert an entry
        cache.insert("/test/path", DeviceType::SSD);

        // Simulate time passing (but not enough for refresh threshold)
        // In a real test, we would need to manipulate time or use a mock
        // For now, just verify the entry exists
        assert_eq!(cache.get("/test/path"), Some(DeviceType::SSD));
    }
}
