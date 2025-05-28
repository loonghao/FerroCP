//! Device detection functionality for FerroCP
//!
//! This module provides cross-platform device detection capabilities,
//! identifying storage device types and their characteristics.

use ferrocp_types::{DeviceDetector as DeviceDetectorTrait, DeviceType, Result};
use std::path::Path;
use tracing::debug;
#[cfg(not(any(windows, unix)))]
use tracing::warn;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Device detection implementation
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DeviceDetector {
    /// Cache for device type detection results
    cache: std::collections::HashMap<String, DeviceType>,
}

impl DeviceDetector {
    /// Create a new device detector
    pub fn new() -> Self {
        Self {
            cache: std::collections::HashMap::new(),
        }
    }

    /// Clear the detection cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Get the number of cached entries
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }

    /// Detect device type for a given path with caching
    pub async fn detect_device_type_cached<P: AsRef<Path> + Send + Sync>(
        &mut self,
        path: P,
    ) -> Result<DeviceType> {
        let path_str = path.as_ref().to_string_lossy().to_string();

        // Check cache first
        if let Some(&device_type) = self.cache.get(&path_str) {
            debug!("Device type cache hit for path: {}", path_str);
            return Ok(device_type);
        }

        // Perform detection using internal method
        let device_type = self.detect_device_type_internal(&path).await?;

        // Cache the result
        self.cache.insert(path_str, device_type);

        Ok(device_type)
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
        assert_eq!(detector.cache_size(), 0);
    }

    #[tokio::test]
    async fn test_cache_functionality() {
        let mut detector = DeviceDetector::new();

        // Cache should be empty initially
        assert_eq!(detector.cache_size(), 0);

        // Clear cache should work even when empty
        detector.clear_cache();
        assert_eq!(detector.cache_size(), 0);
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
}
