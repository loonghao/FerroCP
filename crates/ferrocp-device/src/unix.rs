//! Unix-specific device detection and optimization
//!
//! This module provides Unix-specific implementations for device detection,
//! including filesystem type detection and mount point analysis.

#[cfg(unix)]
use crate::analyzer::{DeviceAnalyzer, FileSystemInfo};
#[cfg(unix)]
use crate::detector::DeviceDetector;
#[cfg(unix)]
use ferrocp_types::{DeviceType, Error, Result};
#[cfg(unix)]
use std::collections::HashMap;
#[cfg(unix)]
use std::path::Path;
#[cfg(unix)]
use tracing::{debug, warn};

#[cfg(unix)]
/// Mount information for Unix systems
#[derive(Debug)]
pub struct MountInfo {
    /// Device name
    pub device: String,
    /// Mount point
    pub mount_point: String,
    /// Filesystem type
    pub fs_type: String,
    /// Mount options
    pub options: Vec<String>,
    /// Whether this is a network filesystem
    pub is_network: bool,
    /// Whether this is a RAM-based filesystem
    pub is_ram_disk: bool,
}

#[cfg(unix)]
impl DeviceDetector {
    /// Get mount information for a Unix path
    pub async fn get_mount_info_unix<P: AsRef<Path>>(&self, path: P) -> Result<MountInfo> {
        let path = path.as_ref();
        debug!("Getting mount info for Unix path: {}", path.display());

        // Read /proc/mounts to get mount information
        let mounts_content = tokio::fs::read_to_string("/proc/mounts")
            .await
            .map_err(|e| Error::device_detection(format!("Failed to read /proc/mounts: {}", e)))?;

        let mut best_match: Option<MountInfo> = None;
        let mut best_match_len = 0;

        for line in mounts_content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 4 {
                continue;
            }

            let device = parts[0].to_string();
            let mount_point = parts[1].to_string();
            let fs_type = parts[2].to_string();
            let options: Vec<String> = parts[3].split(',').map(|s| s.to_string()).collect();

            // Check if this mount point is a parent of our path
            if path.starts_with(&mount_point) && mount_point.len() > best_match_len {
                let is_network = self.is_network_filesystem(&fs_type);
                let is_ram_disk = self.is_ram_filesystem(&fs_type);

                best_match = Some(MountInfo {
                    device,
                    mount_point,
                    fs_type,
                    options,
                    is_network,
                    is_ram_disk,
                });
                best_match_len = mount_point.len();
            }
        }

        best_match.ok_or_else(|| Error::device_detection("No mount point found for path"))
    }

    /// Check if a filesystem type is network-based
    fn is_network_filesystem(&self, fs_type: &str) -> bool {
        matches!(
            fs_type,
            "nfs" | "nfs4" | "cifs" | "smb" | "smbfs" | "ftp" | "sftp" | "sshfs" | "davfs" | "ceph"
        )
    }

    /// Check if a filesystem type is RAM-based
    fn is_ram_filesystem(&self, fs_type: &str) -> bool {
        matches!(fs_type, "tmpfs" | "ramfs" | "devtmpfs")
    }

    /// Detect storage type on Unix systems
    pub async fn detect_storage_type_unix<P: AsRef<Path>>(&self, path: P) -> Result<DeviceType> {
        let path = path.as_ref();
        debug!("Detecting Unix storage type for path: {}", path.display());

        let mount_info = self.get_mount_info_unix(path).await?;

        // Check for special filesystem types first
        if mount_info.is_network {
            return Ok(DeviceType::Network);
        }

        if mount_info.is_ram_disk {
            return Ok(DeviceType::RamDisk);
        }

        // For block devices, try to determine if it's SSD or HDD
        if mount_info.device.starts_with("/dev/") {
            return self.detect_block_device_type(&mount_info.device).await;
        }

        // Default to unknown for other cases
        Ok(DeviceType::Unknown)
    }

    /// Detect if a block device is SSD or HDD
    async fn detect_block_device_type(&self, device: &str) -> Result<DeviceType> {
        // Extract the base device name (e.g., sda from sda1)
        let base_device = if let Some(pos) = device.rfind(|c: char| c.is_ascii_digit()) {
            &device[..pos + 1]
        } else {
            device
        };

        let device_name = base_device.trim_start_matches("/dev/");

        // Check rotational status in /sys/block
        let rotational_path = format!("/sys/block/{}/queue/rotational", device_name);

        match tokio::fs::read_to_string(&rotational_path).await {
            Ok(content) => {
                let is_rotational = content.trim() == "1";
                if is_rotational {
                    debug!("Device {} is rotational (HDD)", device_name);
                    Ok(DeviceType::HDD)
                } else {
                    debug!("Device {} is non-rotational (SSD)", device_name);
                    Ok(DeviceType::SSD)
                }
            }
            Err(e) => {
                warn!(
                    "Failed to read rotational status for {}: {}",
                    device_name, e
                );

                // Fallback: try to guess based on device name patterns
                if device_name.starts_with("nvme") {
                    Ok(DeviceType::SSD)
                } else if device_name.starts_with("sd") {
                    // Could be either, default to SSD for modern systems
                    Ok(DeviceType::SSD)
                } else {
                    Ok(DeviceType::Unknown)
                }
            }
        }
    }
}

#[cfg(unix)]
impl DeviceAnalyzer {
    /// Get filesystem information on Unix systems
    pub async fn get_filesystem_info_unix<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<FileSystemInfo> {
        let path = path.as_ref();
        debug!("Getting Unix filesystem info for path: {}", path.display());

        let detector = DeviceDetector::new();
        let mount_info = detector.get_mount_info_unix(path).await?;

        // Get filesystem statistics using statvfs
        let stats = self.get_statvfs_info(path).await?;

        // Determine filesystem capabilities based on type
        let supports_cow = matches!(mount_info.fs_type.as_str(), "btrfs" | "zfs");
        let supports_sparse = matches!(
            mount_info.fs_type.as_str(),
            "ext2" | "ext3" | "ext4" | "xfs" | "btrfs" | "zfs" | "ntfs"
        );
        let supports_compression = matches!(mount_info.fs_type.as_str(), "btrfs" | "zfs");
        let supports_dedup = matches!(mount_info.fs_type.as_str(), "btrfs" | "zfs");

        Ok(FileSystemInfo {
            fs_type: mount_info.fs_type,
            total_space: stats.total_space,
            available_space: stats.available_space,
            block_size: stats.block_size,
            supports_cow,
            supports_sparse,
            supports_compression,
            supports_dedup,
        })
    }

    /// Get filesystem statistics using statvfs
    async fn get_statvfs_info<P: AsRef<Path>>(&self, path: P) -> Result<FileSystemStats> {
        let path = path.as_ref();

        // Use libc statvfs to get filesystem statistics
        let path_cstring = std::ffi::CString::new(path.to_string_lossy().as_bytes())
            .map_err(|e| Error::device_detection(format!("Invalid path: {}", e)))?;

        let mut statvfs_buf: libc::statvfs = unsafe { std::mem::zeroed() };

        let result = unsafe { libc::statvfs(path_cstring.as_ptr(), &mut statvfs_buf) };

        if result != 0 {
            return Err(Error::device_detection(
                "Failed to get filesystem statistics",
            ));
        }

        let block_size = statvfs_buf.f_frsize as u64;
        let total_blocks = statvfs_buf.f_blocks as u64;
        let available_blocks = statvfs_buf.f_bavail as u64;

        Ok(FileSystemStats {
            total_space: total_blocks * block_size,
            available_space: available_blocks * block_size,
            block_size,
        })
    }
}

#[cfg(unix)]
/// Filesystem statistics from statvfs
#[derive(Debug)]
struct FileSystemStats {
    /// Total space in bytes
    total_space: u64,
    /// Available space in bytes
    available_space: u64,
    /// Block size in bytes
    block_size: u64,
}

#[cfg(test)]
#[cfg(unix)]
mod tests {
    use super::*;
    use tokio_test;

    #[tokio::test]
    async fn test_mount_info() {
        let detector = DeviceDetector::new();
        let current_dir = std::env::current_dir().unwrap();

        let result = detector.get_mount_info_unix(&current_dir).await;
        assert!(result.is_ok());

        if let Ok(info) = result {
            assert!(!info.fs_type.is_empty());
            assert!(!info.mount_point.is_empty());
            debug!("Mount info: {:?}", info);
        }
    }

    #[tokio::test]
    async fn test_filesystem_detection() {
        let detector = DeviceDetector::new();

        // Test network filesystem detection
        assert!(detector.is_network_filesystem("nfs"));
        assert!(detector.is_network_filesystem("cifs"));
        assert!(!detector.is_network_filesystem("ext4"));

        // Test RAM filesystem detection
        assert!(detector.is_ram_filesystem("tmpfs"));
        assert!(detector.is_ram_filesystem("ramfs"));
        assert!(!detector.is_ram_filesystem("ext4"));
    }

    #[tokio::test]
    async fn test_filesystem_info() {
        let analyzer = DeviceAnalyzer::new();
        let current_dir = std::env::current_dir().unwrap();

        let result = analyzer.get_filesystem_info_unix(&current_dir).await;
        assert!(result.is_ok());

        if let Ok(info) = result {
            assert!(!info.fs_type.is_empty());
            assert!(info.total_space > 0);
            assert!(info.block_size > 0);
            debug!("Filesystem info: {:?}", info);
        }
    }

    #[tokio::test]
    async fn test_storage_type_detection() {
        let detector = DeviceDetector::new();
        let current_dir = std::env::current_dir().unwrap();

        let result = detector.detect_storage_type_unix(&current_dir).await;
        assert!(result.is_ok());

        if let Ok(device_type) = result {
            debug!("Detected storage type: {:?}", device_type);
            // The result should be one of the valid device types
            assert!(matches!(
                device_type,
                DeviceType::SSD
                    | DeviceType::HDD
                    | DeviceType::Network
                    | DeviceType::RamDisk
                    | DeviceType::Unknown
            ));
        }
    }
}
