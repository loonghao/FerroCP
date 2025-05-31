//! Hash caching for efficient file synchronization

use ferrocp_types::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tokio::fs;
use tracing::{debug, info};

/// Cache entry for a file hash
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// File path
    pub path: PathBuf,
    /// File size
    pub size: u64,
    /// Last modified time
    pub modified: SystemTime,
    /// File hash
    pub hash: String,
    /// When this entry was cached
    pub cached_at: SystemTime,
    /// Number of times this entry has been accessed
    pub access_count: u64,
}

impl CacheEntry {
    /// Create a new cache entry
    pub fn new(path: PathBuf, size: u64, modified: SystemTime, hash: String) -> Self {
        Self {
            path,
            size,
            modified,
            hash,
            cached_at: SystemTime::now(),
            access_count: 0,
        }
    }

    /// Check if this cache entry is still valid for the given file metadata
    pub fn is_valid(&self, size: u64, modified: SystemTime) -> bool {
        self.size == size && self.modified == modified
    }

    /// Update access count
    pub fn access(&mut self) {
        self.access_count += 1;
    }

    /// Get the age of this cache entry
    pub fn age(&self) -> Duration {
        SystemTime::now()
            .duration_since(self.cached_at)
            .unwrap_or_default()
    }
}

/// Configuration for hash cache
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Cache file path
    pub cache_file: PathBuf,
    /// Maximum number of entries to keep in cache
    pub max_entries: usize,
    /// Maximum age for cache entries
    pub max_age: Duration,
    /// Whether to enable cache compression
    pub enable_compression: bool,
    /// Cache cleanup interval
    pub cleanup_interval: Duration,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            cache_file: PathBuf::from(".ferrocp_hash_cache"),
            max_entries: 10000,
            max_age: Duration::from_secs(30 * 24 * 60 * 60), // 30 days
            enable_compression: true,
            cleanup_interval: Duration::from_secs(60 * 60), // 1 hour
        }
    }
}

/// Hash cache for storing file hashes
#[derive(Debug)]
pub struct HashCache {
    config: CacheConfig,
    entries: HashMap<PathBuf, CacheEntry>,
    dirty: bool,
}

impl HashCache {
    /// Create a new hash cache
    pub async fn new(config: CacheConfig) -> Result<Self> {
        let mut cache = Self {
            config,
            entries: HashMap::new(),
            dirty: false,
        };

        // Load existing cache if it exists
        cache.load().await?;

        Ok(cache)
    }

    /// Create a hash cache with default configuration
    pub async fn default() -> Result<Self> {
        Self::new(CacheConfig::default()).await
    }

    /// Get a hash from cache if available and valid
    pub async fn get_hash<P: AsRef<Path>>(
        &mut self,
        path: P,
        size: u64,
        modified: SystemTime,
    ) -> Option<String> {
        let path = path.as_ref();

        if let Some(entry) = self.entries.get_mut(path) {
            if entry.is_valid(size, modified) {
                entry.access();
                debug!("Cache hit for: {}", path.display());
                return Some(entry.hash.clone());
            } else {
                // Entry is outdated, remove it
                self.entries.remove(path);
                self.dirty = true;
                debug!("Cache entry outdated for: {}", path.display());
            }
        }

        debug!("Cache miss for: {}", path.display());
        None
    }

    /// Store a hash in the cache
    pub fn store_hash<P: AsRef<Path>>(
        &mut self,
        path: P,
        size: u64,
        modified: SystemTime,
        hash: String,
    ) {
        let path = path.as_ref().to_path_buf();
        let entry = CacheEntry::new(path.clone(), size, modified, hash);

        self.entries.insert(path.clone(), entry);
        self.dirty = true;

        debug!("Stored hash in cache for: {}", path.display());

        // Check if we need to cleanup
        if self.entries.len() > self.config.max_entries {
            self.cleanup_old_entries();
        }
    }

    /// Remove a hash from the cache
    pub fn remove_hash<P: AsRef<Path>>(&mut self, path: P) {
        let path = path.as_ref();
        if self.entries.remove(path).is_some() {
            self.dirty = true;
            debug!("Removed hash from cache for: {}", path.display());
        }
    }

    /// Clear all cache entries
    pub fn clear(&mut self) {
        self.entries.clear();
        self.dirty = true;
        info!("Cleared hash cache");
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let total_entries = self.entries.len();
        let total_size = self.entries.values().map(|entry| entry.size).sum();

        let avg_access_count = if total_entries > 0 {
            self.entries
                .values()
                .map(|entry| entry.access_count)
                .sum::<u64>() as f64
                / total_entries as f64
        } else {
            0.0
        };

        CacheStats {
            total_entries,
            total_size,
            avg_access_count,
            cache_file_size: 0, // TODO: Get actual file size
        }
    }

    /// Load cache from disk
    async fn load(&mut self) -> Result<()> {
        if !self.config.cache_file.exists() {
            debug!("Cache file does not exist, starting with empty cache");
            return Ok(());
        }

        let data = fs::read(&self.config.cache_file)
            .await
            .map_err(|e| Error::Io {
                message: format!(
                    "Failed to read cache file '{}': {}",
                    self.config.cache_file.display(),
                    e
                ),
            })?;

        let entries: HashMap<PathBuf, CacheEntry> =
            bincode::deserialize(&data).map_err(|e| Error::Network {
                message: format!("Failed to deserialize cache: {}", e),
            })?;

        // Filter out expired entries
        let now = SystemTime::now();
        let valid_entries: HashMap<PathBuf, CacheEntry> = entries
            .into_iter()
            .filter(|(_, entry)| {
                now.duration_since(entry.cached_at)
                    .map(|age| age <= self.config.max_age)
                    .unwrap_or(false)
            })
            .collect();

        let loaded_count = valid_entries.len();
        self.entries = valid_entries;

        info!("Loaded {} cache entries from disk", loaded_count);
        Ok(())
    }

    /// Save cache to disk
    pub async fn save(&mut self) -> Result<()> {
        if !self.dirty {
            return Ok(());
        }

        let data = bincode::serialize(&self.entries).map_err(|e| Error::Network {
            message: format!("Failed to serialize cache: {}", e),
        })?;

        fs::write(&self.config.cache_file, data)
            .await
            .map_err(|e| Error::Io {
                message: format!(
                    "Failed to write cache file '{}': {}",
                    self.config.cache_file.display(),
                    e
                ),
            })?;

        self.dirty = false;
        debug!("Saved cache to disk with {} entries", self.entries.len());
        Ok(())
    }

    /// Cleanup old entries based on age and access count
    fn cleanup_old_entries(&mut self) {
        let now = SystemTime::now();
        let initial_count = self.entries.len();

        // Remove entries that are too old
        self.entries.retain(|_, entry| {
            now.duration_since(entry.cached_at)
                .map(|age| age <= self.config.max_age)
                .unwrap_or(false)
        });

        // If still too many entries, remove least recently used
        if self.entries.len() > self.config.max_entries {
            let mut entries: Vec<_> = self.entries.drain().collect();

            // Sort by access count (ascending) and age (descending)
            entries.sort_by(|a, b| {
                a.1.access_count
                    .cmp(&b.1.access_count)
                    .then_with(|| b.1.cached_at.cmp(&a.1.cached_at))
            });

            // Keep only the most accessed entries
            entries.truncate(self.config.max_entries);
            self.entries = entries.into_iter().collect();
        }

        let removed_count = initial_count - self.entries.len();
        if removed_count > 0 {
            info!("Cleaned up {} old cache entries", removed_count);
            self.dirty = true;
        }
    }

    /// Perform periodic cleanup
    pub async fn periodic_cleanup(&mut self) -> Result<()> {
        self.cleanup_old_entries();
        self.save().await?;
        Ok(())
    }
}

impl Drop for HashCache {
    fn drop(&mut self) {
        if self.dirty {
            // Try to save cache on drop, but ignore errors since we're in a destructor
            let cache_file = self.config.cache_file.clone();
            let entries = self.entries.clone();

            tokio::spawn(async move {
                if let Ok(data) = bincode::serialize(&entries) {
                    let _ = fs::write(&cache_file, data).await;
                }
            });
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Total number of cache entries
    pub total_entries: usize,
    /// Total size of cached files
    pub total_size: u64,
    /// Average access count per entry
    pub avg_access_count: f64,
    /// Size of cache file on disk
    pub cache_file_size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_cache_entry_validity() {
        let entry = CacheEntry::new(
            PathBuf::from("test.txt"),
            100,
            SystemTime::now(),
            "hash123".to_string(),
        );

        assert!(entry.is_valid(100, entry.modified));
        assert!(!entry.is_valid(200, entry.modified));
    }

    #[tokio::test]
    async fn test_hash_cache_operations() {
        let temp_dir = TempDir::new().unwrap();
        let cache_file = temp_dir.path().join("cache");

        let config = CacheConfig {
            cache_file,
            ..Default::default()
        };

        let mut cache = HashCache::new(config).await.unwrap();

        let path = PathBuf::from("test.txt");
        let size = 100;
        let modified = SystemTime::now();
        let hash = "hash123".to_string();

        // Store hash
        cache.store_hash(&path, size, modified, hash.clone());

        // Retrieve hash
        let retrieved = cache.get_hash(&path, size, modified).await;
        assert_eq!(retrieved, Some(hash));

        // Test cache miss with different size
        let miss = cache.get_hash(&path, 200, modified).await;
        assert_eq!(miss, None);
    }

    #[tokio::test]
    async fn test_cache_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let cache_file = temp_dir.path().join("cache");

        let config = CacheConfig {
            cache_file: cache_file.clone(),
            ..Default::default()
        };

        // Create cache and store data
        {
            let mut cache = HashCache::new(config.clone()).await.unwrap();
            cache.store_hash("test.txt", 100, SystemTime::now(), "hash123".to_string());
            cache.save().await.unwrap();
        }

        // Load cache and verify data
        {
            let cache = HashCache::new(config).await.unwrap();
            assert_eq!(cache.entries.len(), 1);
            assert!(cache.entries.contains_key(Path::new("test.txt")));
        }
    }
}
