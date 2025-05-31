//! Device detection functionality for FerroCP
//!
//! This module provides cross-platform device detection capabilities,
//! identifying storage device types and their characteristics.

use crate::cache::{create_shared_cache, SharedDeviceCache};
use ferrocp_types::{DeviceDetector as DeviceDetectorTrait, DeviceType, Result};
use std::path::Path;
use tracing::debug;
#[cfg(not(any(windows, unix)))]
use tracing::warn;

/// Device detection implementation
#[derive(Debug)]
pub struct DeviceDetector {
    /// Intelligent cache for device type detection results
    cache: SharedDeviceCache,
}

impl DeviceDetector {
    /// Create a new device detector
    pub fn new() -> Self {
        Self {
            cache: create_shared_cache(),
        }
    }

    /// Create a new device detector with custom cache configuration
    pub fn with_cache_config(config: crate::cache::DeviceCacheConfig) -> Self {
        Self {
            cache: crate::cache::create_shared_cache_with_config(config),
        }
    }

    /// Clear the detection cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Get the number of cached entries
    pub async fn cache_size(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> ferrocp_types::DeviceCacheStats {
        let cache = self.cache.read().await;
        cache.stats().clone()
    }

    /// Detect device type for a given path with caching
    pub async fn detect_device_type_cached<P: AsRef<Path> + Send + Sync>(
        &self,
        path: P,
    ) -> Result<DeviceType> {
        // Check cache first
        {
            let mut cache = self.cache.write().await;
            if let Some(device_type) = cache.get(&path) {
                debug!(
                    "Device type cache hit for path: {}",
                    path.as_ref().display()
                );
                return Ok(device_type);
            }
        }

        // Perform detection using internal method
        let device_type = self.detect_device_type_internal(&path).await?;

        // Cache the result
        {
            let mut cache = self.cache.write().await;
            cache.insert(&path, device_type);
        }

        Ok(device_type)
    }

    /// Process background refresh queue
    pub async fn process_background_refresh(&self) -> Result<usize> {
        let refresh_queue = {
            let mut cache = self.cache.write().await;
            cache.get_refresh_queue()
        };

        if refresh_queue.is_empty() {
            return Ok(0);
        }

        let mut refreshed_count = 0;
        for path_key in refresh_queue {
            // Convert cache key back to path for detection
            let path = std::path::Path::new(&path_key);

            // Perform fresh detection
            if let Ok(device_type) = self.detect_device_type_internal(path).await {
                // Update cache with fresh result
                let mut cache = self.cache.write().await;
                cache.update_refreshed_entry(path, device_type);
                refreshed_count += 1;

                debug!(
                    "Background refreshed device type for path: {}",
                    path.display()
                );
            }
        }

        debug!(
            "Background refresh completed: {} entries refreshed",
            refreshed_count
        );
        Ok(refreshed_count)
    }

    /// Check if background refresh is needed
    pub async fn needs_background_refresh(&self) -> bool {
        let cache = self.cache.read().await;
        cache.needs_background_refresh()
    }

    /// Internal device type detection logic
    async fn detect_device_type_internal<P: AsRef<Path> + Send + Sync>(
        &self,
        path: P,
    ) -> Result<DeviceType> {
        debug!(
            "Detecting device type for path: {}",
            path.as_ref().display()
        );

        // Check for network paths first
        if self.is_network_path(&path).await? {
            debug!(
                "Detected network device for path: {}",
                path.as_ref().display()
            );
            return Ok(DeviceType::Network);
        }

        // Check for RAM disk
        if self.is_ram_disk(&path).await? {
            debug!("Detected RAM disk for path: {}", path.as_ref().display());
            return Ok(DeviceType::RamDisk);
        }

        // Detect underlying storage type
        let device_type = self.detect_storage_type(path).await?;
        debug!("Detected storage type: {:?}", device_type);

        Ok(device_type)
    }

    /// Detect if the path is on a network drive
    async fn is_network_path<P: AsRef<Path> + Send + Sync>(&self, path: P) -> Result<bool> {
        let path = path.as_ref();

        #[cfg(windows)]
        {
            // Check for UNC paths (\\server\share) or mapped network drives
            if let Some(path_str) = path.to_str() {
                if path_str.starts_with("\\\\") {
                    return Ok(true);
                }

                // Check if it's a mapped network drive
                if let Some(drive_letter) = path_str.chars().next() {
                    if drive_letter.is_ascii_alphabetic()
                        && path_str.len() > 2
                        && &path_str[1..3] == ":\\"
                    {
                        return self.is_network_drive_windows_impl(drive_letter).await;
                    }
                }
            }
            // If not a network path, continue to other checks
            return Ok(false);
        }

        #[cfg(unix)]
        {
            // Check for common network filesystem mount points
            return self.get_mount_info_unix_impl(path).await;
        }

        #[cfg(not(any(windows, unix)))]
        {
            Ok(false)
        }
    }

    /// Detect if the path is on a RAM disk
    async fn is_ram_disk<P: AsRef<Path> + Send + Sync>(&self, path: P) -> Result<bool> {
        let path = path.as_ref();

        #[cfg(windows)]
        {
            // Check for ImDisk or other RAM disk solutions
            return self.get_volume_info_windows_impl(path).await;
        }

        #[cfg(unix)]
        {
            // Check for tmpfs, ramfs, or other memory-based filesystems
            return self.get_mount_info_unix_impl(path).await;
        }

        #[cfg(not(any(windows, unix)))]
        {
            Ok(false)
        }
    }

    /// Detect the underlying storage device type (SSD vs HDD)
    async fn detect_storage_type<P: AsRef<Path> + Send + Sync>(
        &self,
        path: P,
    ) -> Result<DeviceType> {
        #[cfg(windows)]
        {
            return self.detect_storage_type_windows_impl(path).await;
        }

        #[cfg(unix)]
        {
            return self.detect_storage_type_unix_impl(path).await;
        }

        #[cfg(not(any(windows, unix)))]
        {
            warn!("Storage type detection not implemented for this platform");
            Ok(DeviceType::Unknown)
        }
    }

    /// Windows-specific storage type detection
    #[cfg(windows)]
    async fn detect_storage_type_windows_impl<P: AsRef<Path> + Send + Sync>(
        &self,
        path: P,
    ) -> Result<DeviceType> {
        // Simplified implementation - in a real scenario, this would use WMI or registry
        // to determine if the storage device is SSD or HDD
        let path = path.as_ref();
        debug!(
            "Detecting Windows storage type for path: {}",
            path.display()
        );

        // For now, default to SSD for modern systems
        // This could be enhanced with actual Windows API calls
        Ok(DeviceType::SSD)
    }

    /// Unix-specific storage type detection
    #[cfg(unix)]
    async fn detect_storage_type_unix_impl<P: AsRef<Path> + Send + Sync>(
        &self,
        path: P,
    ) -> Result<DeviceType> {
        // Simplified implementation - in a real scenario, this would check /sys/block
        // to determine rotational status
        let path = path.as_ref();
        debug!("Detecting Unix storage type for path: {}", path.display());

        // For now, default to SSD for modern systems
        // This could be enhanced with actual /sys/block checks
        Ok(DeviceType::SSD)
    }

    /// Get volume information on Windows
    #[cfg(windows)]
    async fn get_volume_info_windows_impl<P: AsRef<Path> + Send + Sync>(
        &self,
        _path: P,
    ) -> Result<bool> {
        // Simplified implementation
        Ok(false)
    }

    /// Check if a drive letter is a network drive on Windows
    #[cfg(windows)]
    async fn is_network_drive_windows_impl(&self, _drive_letter: char) -> Result<bool> {
        // Simplified implementation
        Ok(false)
    }

    /// Get mount information on Unix
    #[cfg(unix)]
    async fn get_mount_info_unix_impl<P: AsRef<Path> + Send + Sync>(
        &self,
        _path: P,
    ) -> Result<bool> {
        // Simplified implementation
        Ok(false)
    }
}

impl Default for DeviceDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl DeviceDetectorTrait for DeviceDetector {
    async fn detect_device_type<P: AsRef<Path> + Send + Sync>(
        &self,
        path: P,
    ) -> Result<DeviceType> {
        self.detect_device_type_internal(path).await
    }

    fn get_optimal_buffer_size(&self, device_type: DeviceType) -> usize {
        match device_type {
            DeviceType::SSD => 1024 * 1024,         // 1MB for SSD
            DeviceType::HDD => 64 * 1024,           // 64KB for HDD
            DeviceType::Network => 8 * 1024,        // 8KB for network
            DeviceType::RamDisk => 4 * 1024 * 1024, // 4MB for RAM disk
            DeviceType::Unknown => 256 * 1024,      // 256KB default
        }
    }

    fn supports_zero_copy(&self, device_type: DeviceType) -> bool {
        match device_type {
            DeviceType::SSD | DeviceType::HDD => true,
            DeviceType::RamDisk => true,
            DeviceType::Network => false,
            DeviceType::Unknown => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_device_detector_creation() {
        let detector = DeviceDetector::new();
        assert_eq!(detector.cache_size().await, 0);
    }

    #[tokio::test]
    async fn test_cache_functionality() {
        let detector = DeviceDetector::new();

        // Cache should be empty initially
        assert_eq!(detector.cache_size().await, 0);

        // Clear cache should work even when empty
        detector.clear_cache().await;
        assert_eq!(detector.cache_size().await, 0);
    }

    #[tokio::test]
    async fn test_optimal_buffer_sizes() {
        let detector = DeviceDetector::new();

        assert_eq!(
            detector.get_optimal_buffer_size(DeviceType::SSD),
            1024 * 1024
        );
        assert_eq!(detector.get_optimal_buffer_size(DeviceType::HDD), 64 * 1024);
        assert_eq!(
            detector.get_optimal_buffer_size(DeviceType::Network),
            8 * 1024
        );
        assert_eq!(
            detector.get_optimal_buffer_size(DeviceType::RamDisk),
            4 * 1024 * 1024
        );
        assert_eq!(
            detector.get_optimal_buffer_size(DeviceType::Unknown),
            256 * 1024
        );
    }

    #[tokio::test]
    async fn test_zero_copy_support() {
        let detector = DeviceDetector::new();

        assert!(detector.supports_zero_copy(DeviceType::SSD));
        assert!(detector.supports_zero_copy(DeviceType::HDD));
        assert!(detector.supports_zero_copy(DeviceType::RamDisk));
        assert!(!detector.supports_zero_copy(DeviceType::Network));
        assert!(!detector.supports_zero_copy(DeviceType::Unknown));
    }

    #[tokio::test]
    async fn test_cache_hit_and_miss() {
        use tempfile::TempDir;

        let detector = DeviceDetector::new();
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("test_file.txt");
        std::fs::write(&test_path, "test content").unwrap();

        // First call should be a cache miss
        let stats_before = detector.cache_stats().await;
        let device_type1 = detector
            .detect_device_type_cached(&test_path)
            .await
            .unwrap();
        let stats_after_first = detector.cache_stats().await;

        assert_eq!(
            stats_after_first.total_lookups,
            stats_before.total_lookups + 1
        );
        assert_eq!(
            stats_after_first.cache_misses,
            stats_before.cache_misses + 1
        );

        // Second call should be a cache hit
        let device_type2 = detector
            .detect_device_type_cached(&test_path)
            .await
            .unwrap();
        let stats_after_second = detector.cache_stats().await;

        assert_eq!(device_type1, device_type2);
        assert_eq!(
            stats_after_second.total_lookups,
            stats_after_first.total_lookups + 1
        );
        assert_eq!(
            stats_after_second.cache_hits,
            stats_after_first.cache_hits + 1
        );

        // Cache hit rate should be 50% (1 hit out of 2 lookups)
        assert_eq!(stats_after_second.hit_rate(), 50.0);
    }

    #[tokio::test]
    async fn test_cache_statistics() {
        let detector = DeviceDetector::new();

        // Initial stats should be empty
        let stats = detector.cache_stats().await;
        assert_eq!(stats.total_lookups, 0);
        assert_eq!(stats.cache_hits, 0);
        assert_eq!(stats.cache_misses, 0);
        assert_eq!(stats.hit_rate(), 0.0);
    }

    #[tokio::test]
    async fn test_path_prefix_caching() {
        use tempfile::TempDir;

        let detector = DeviceDetector::new();
        let temp_dir = TempDir::new().unwrap();

        // Create two files in the same directory
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");
        std::fs::write(&file1, "content1").unwrap();
        std::fs::write(&file2, "content2").unwrap();

        // First file detection should be a cache miss
        let device_type1 = detector.detect_device_type_cached(&file1).await.unwrap();
        let stats_after_first = detector.cache_stats().await;
        assert_eq!(stats_after_first.cache_misses, 1);

        // Second file in same directory should be a cache hit due to path prefix optimization
        let device_type2 = detector.detect_device_type_cached(&file2).await.unwrap();
        let stats_after_second = detector.cache_stats().await;

        // Both files should have the same device type (same directory)
        assert_eq!(device_type1, device_type2);
        // Should have one cache hit due to path prefix optimization
        assert_eq!(stats_after_second.cache_hits, 1);
    }

    #[tokio::test]
    async fn test_background_refresh() {
        use crate::cache::DeviceCacheConfig;
        use std::time::Duration;

        let config = DeviceCacheConfig {
            enable_background_refresh: true,
            background_refresh_interval: Duration::from_millis(100),
            refresh_threshold: 0.1, // Refresh after 10% of TTL
            ttl: Duration::from_millis(1000),
            ..DeviceCacheConfig::default()
        };

        let detector = DeviceDetector::with_cache_config(config);
        let temp_dir = tempfile::TempDir::new().unwrap();
        let test_path = temp_dir.path().join("test_file.txt");
        std::fs::write(&test_path, "test content").unwrap();

        // Initial detection
        detector
            .detect_device_type_cached(&test_path)
            .await
            .unwrap();

        // Wait for refresh threshold
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Access the cache to trigger refresh queue population
        detector
            .detect_device_type_cached(&test_path)
            .await
            .unwrap();

        // Check if background refresh is needed
        assert!(detector.needs_background_refresh().await);

        // Process background refresh
        let refreshed_count = detector.process_background_refresh().await.unwrap();
        assert!(refreshed_count > 0);
    }
}
