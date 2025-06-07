//! Windows-specific device detection and optimization
//!
//! This module provides Windows-specific implementations for device detection,
//! including ReFS CoW support and NTFS features.

#[cfg(windows)]
use crate::analyzer::FileSystemInfo;
#[cfg(windows)]
use crate::detector::DeviceDetector;
#[cfg(windows)]
use ferrocp_types::{DeviceType, Error, Result};
#[cfg(windows)]
use std::ffi::OsStr;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
#[cfg(windows)]
use std::path::Path;
#[cfg(windows)]
use std::ptr;
#[cfg(windows)]
use tracing::{debug, warn};
#[cfg(windows)]
use winapi::shared::minwindef::{DWORD, FALSE};
#[cfg(windows)]
use winapi::shared::ntdef::ULARGE_INTEGER;
#[cfg(windows)]
use winapi::um::fileapi::{GetDiskFreeSpaceExW, GetDriveTypeW, GetVolumeInformationW};
#[cfg(windows)]
use winapi::um::winbase::{DRIVE_CDROM, DRIVE_FIXED, DRIVE_RAMDISK, DRIVE_REMOTE, DRIVE_REMOVABLE};

#[cfg(windows)]
/// Volume information for Windows
#[derive(Debug)]
pub struct VolumeInfo {
    /// Whether this is a RAM disk
    pub is_ram_disk: bool,
    /// Whether this is a network drive
    pub is_network: bool,
    /// Drive type
    pub drive_type: u32,
    /// File system name
    pub filesystem: String,
    /// Volume serial number
    pub serial_number: u32,
    /// Maximum component length
    pub max_component_length: u32,
    /// File system flags
    pub filesystem_flags: u32,
}

#[cfg(windows)]
impl DeviceDetector {
    /// Check if a drive letter corresponds to a network drive on Windows
    pub async fn is_network_drive_windows(&self, drive_letter: char) -> Result<bool> {
        let drive_path = format!("{}:\\", drive_letter);
        let drive_path_wide: Vec<u16> = OsStr::new(&drive_path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let drive_type = unsafe { GetDriveTypeW(drive_path_wide.as_ptr()) };

        Ok(drive_type == DRIVE_REMOTE)
    }

    /// Get volume information for a Windows path
    pub async fn get_volume_info_windows<P: AsRef<Path>>(&self, path: P) -> Result<VolumeInfo> {
        let path = path.as_ref();

        // Get the root path (drive letter)
        let root_path = if let Some(root) = path.ancestors().last() {
            root.to_string_lossy().to_string()
        } else {
            return Err(Error::device_detection("Cannot determine root path"));
        };

        let root_path_wide: Vec<u16> = OsStr::new(&root_path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let drive_type = unsafe { GetDriveTypeW(root_path_wide.as_ptr()) };

        let mut volume_name = vec![0u16; 256];
        let mut filesystem_name = vec![0u16; 256];
        let mut serial_number: DWORD = 0;
        let mut max_component_length: DWORD = 0;
        let mut filesystem_flags: DWORD = 0;

        let result = unsafe {
            GetVolumeInformationW(
                root_path_wide.as_ptr(),
                volume_name.as_mut_ptr(),
                volume_name.len() as DWORD,
                &mut serial_number,
                &mut max_component_length,
                &mut filesystem_flags,
                filesystem_name.as_mut_ptr(),
                filesystem_name.len() as DWORD,
            )
        };

        if result == FALSE {
            return Err(Error::device_detection("Failed to get volume information"));
        }

        // Convert filesystem name from wide string
        let filesystem = String::from_utf16_lossy(&filesystem_name)
            .trim_end_matches('\0')
            .to_string();

        Ok(VolumeInfo {
            is_ram_disk: drive_type == DRIVE_RAMDISK,
            is_network: drive_type == DRIVE_REMOTE,
            drive_type,
            filesystem,
            serial_number,
            max_component_length,
            filesystem_flags,
        })
    }

    /// Detect storage type on Windows using WMI or registry
    pub async fn detect_storage_type_windows<P: AsRef<Path>>(&self, path: P) -> Result<DeviceType> {
        let path = path.as_ref();
        debug!(
            "Detecting Windows storage type for path: {}",
            path.display()
        );

        // Get volume information first
        let volume_info = self.get_volume_info_windows(path).await?;

        // Check for special drive types
        match volume_info.drive_type {
            DRIVE_REMOTE => return Ok(DeviceType::Network),
            DRIVE_RAMDISK => return Ok(DeviceType::RamDisk),
            DRIVE_CDROM | DRIVE_REMOVABLE => return Ok(DeviceType::Unknown),
            DRIVE_FIXED => {
                // Continue with SSD/HDD detection for fixed drives
                return self.detect_fixed_drive_type(path).await;
            }
            _ => return Ok(DeviceType::Unknown),
        }
    }

    /// Detect fixed drive type (SSD vs HDD) using Windows APIs
    async fn detect_fixed_drive_type<P: AsRef<Path>>(&self, path: P) -> Result<DeviceType> {
        let path = path.as_ref();

        // Get volume information for filesystem-based detection
        let volume_info = self.get_volume_info_windows(path).await?;

        // Check if it's likely an SSD based on filesystem features
        if volume_info.filesystem == "ReFS" {
            // ReFS is typically used on SSDs and supports CoW
            debug!("Detected ReFS filesystem, likely SSD");
            return Ok(DeviceType::SSD);
        }

        // Get the drive letter for performance testing
        let drive_letter = if let Some(root) = path.ancestors().last() {
            root.to_string_lossy().chars().next().unwrap_or('C')
        } else {
            'C'
        };

        // Try to detect using performance characteristics
        if let Ok(device_type) = self.detect_drive_type_by_performance(drive_letter).await {
            return Ok(device_type);
        }

        // For NTFS and other filesystems, default to SSD for modern systems
        if volume_info.filesystem == "NTFS" {
            debug!("Detected NTFS filesystem, defaulting to SSD");
            Ok(DeviceType::SSD)
        } else {
            debug!(
                "Unknown filesystem: {}, defaulting to Unknown",
                volume_info.filesystem
            );
            Ok(DeviceType::Unknown)
        }
    }

    /// Detect drive type by performance characteristics
    async fn detect_drive_type_by_performance(&self, _drive_letter: char) -> Result<DeviceType> {
        // For now, skip the performance test to avoid potential issues
        // In a production implementation, this could use Windows APIs like
        // DeviceIoControl with IOCTL_STORAGE_QUERY_PROPERTY to get device characteristics

        // Default to SSD for modern systems
        Ok(DeviceType::SSD)
    }
}

#[cfg(windows)]
use crate::analyzer::DeviceAnalyzer;

#[cfg(windows)]
impl DeviceAnalyzer {
    /// Get filesystem information on Windows
    pub async fn get_filesystem_info_windows<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<FileSystemInfo> {
        let path = path.as_ref();
        debug!(
            "Getting Windows filesystem info for path: {}",
            path.display()
        );

        // Get the root path
        let root_path = if let Some(root) = path.ancestors().last() {
            root.to_string_lossy().to_string()
        } else {
            return Err(Error::device_detection("Cannot determine root path"));
        };

        let root_path_wide: Vec<u16> = OsStr::new(&root_path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        // Get disk space information
        let mut free_bytes_available: ULARGE_INTEGER = unsafe { std::mem::zeroed() };
        let mut total_bytes: ULARGE_INTEGER = unsafe { std::mem::zeroed() };
        let mut total_free_bytes: ULARGE_INTEGER = unsafe { std::mem::zeroed() };

        let space_result = unsafe {
            GetDiskFreeSpaceExW(
                root_path_wide.as_ptr(),
                &mut free_bytes_available,
                &mut total_bytes,
                &mut total_free_bytes,
            )
        };

        if space_result == FALSE {
            warn!("Failed to get disk space information");
        }

        // Get volume information
        let mut filesystem_name = vec![0u16; 256];
        let mut serial_number: DWORD = 0;
        let mut max_component_length: DWORD = 0;
        let mut filesystem_flags: DWORD = 0;

        let volume_result = unsafe {
            GetVolumeInformationW(
                root_path_wide.as_ptr(),
                ptr::null_mut(),
                0,
                &mut serial_number,
                &mut max_component_length,
                &mut filesystem_flags,
                filesystem_name.as_mut_ptr(),
                filesystem_name.len() as DWORD,
            )
        };

        let filesystem = if volume_result != FALSE {
            String::from_utf16_lossy(&filesystem_name)
                .trim_end_matches('\0')
                .to_string()
        } else {
            "unknown".to_string()
        };

        // Determine filesystem capabilities based on type and flags
        let supports_cow = filesystem == "ReFS";
        let supports_sparse = filesystem == "NTFS" || filesystem == "ReFS";
        let supports_compression = filesystem == "NTFS";
        let supports_dedup = filesystem == "ReFS";

        // Default block size (could be queried more precisely)
        let block_size = match filesystem.as_str() {
            "NTFS" => 4096,
            "ReFS" => 65536,
            "FAT32" => 4096,
            "exFAT" => 4096,
            _ => 4096,
        };

        Ok(FileSystemInfo {
            fs_type: filesystem,
            total_space: unsafe { *total_bytes.QuadPart() as u64 },
            available_space: unsafe { *free_bytes_available.QuadPart() as u64 },
            block_size,
            supports_cow,
            supports_sparse,
            supports_compression,
            supports_dedup,
        })
    }
}

#[cfg(test)]
#[cfg(windows)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_drive_detection() {
        let detector = DeviceDetector::new();

        // Test with a typical drive letter (this won't be a network drive in most cases)
        let result = detector.is_network_drive_windows('C').await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_volume_info() {
        let detector = DeviceDetector::new();
        let current_dir = std::env::current_dir().unwrap();

        let result = detector.get_volume_info_windows(&current_dir).await;
        assert!(result.is_ok());

        if let Ok(info) = result {
            assert!(!info.filesystem.is_empty());
            debug!("Volume info: {:?}", info);
        }
    }

    #[tokio::test]
    async fn test_filesystem_info() {
        let analyzer = DeviceAnalyzer::new();
        let current_dir = std::env::current_dir().unwrap();

        let result = analyzer.get_filesystem_info_windows(&current_dir).await;
        assert!(result.is_ok());

        if let Ok(info) = result {
            assert!(!info.fs_type.is_empty());
            assert!(info.total_space > 0);
            debug!("Filesystem info: {:?}", info);
        }
    }
}
