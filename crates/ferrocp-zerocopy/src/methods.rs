//! Zero-copy method implementations and capabilities detection
//!
//! This module provides abstractions for different zero-copy methods
//! and platform-specific capability detection.

use ferrocp_types::{Error, Result, ZeroCopyMethod as ZeroCopyMethodType};
use std::path::Path;
use std::sync::Arc;
use tracing::debug;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Zero-copy capabilities for the current platform
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ZeroCopyCapabilities {
    /// Available zero-copy methods
    pub available_methods: Vec<ZeroCopyMethodType>,
    /// Whether the platform supports hardware acceleration
    pub hardware_acceleration: bool,
    /// Maximum file size supported by zero-copy
    pub max_file_size: u64,
    /// Whether cross-filesystem zero-copy is supported
    pub cross_filesystem: bool,
    /// Platform-specific features
    pub platform_features: PlatformFeatures,
}

/// Platform-specific zero-copy features
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PlatformFeatures {
    /// Linux-specific features
    #[cfg(target_os = "linux")]
    pub linux: LinuxFeatures,
    /// Windows-specific features
    #[cfg(target_os = "windows")]
    pub windows: WindowsFeatures,
    /// macOS-specific features
    #[cfg(target_os = "macos")]
    pub macos: MacOSFeatures,
}

#[cfg(target_os = "linux")]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct LinuxFeatures {
    /// Whether copy_file_range is available
    pub copy_file_range: bool,
    /// Whether sendfile is available
    pub sendfile: bool,
    /// Whether io_uring is available
    pub io_uring: bool,
    /// Whether BTRFS reflink is available
    pub btrfs_reflink: bool,
    /// Whether XFS reflink is available
    pub xfs_reflink: bool,
}

#[cfg(target_os = "windows")]
/// Windows-specific zero-copy features
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct WindowsFeatures {
    /// Whether ReFS Copy-on-Write is available
    pub refs_cow: bool,
    /// Whether NTFS hardlinks are available
    pub ntfs_hardlinks: bool,
    /// Whether Windows Storage Spaces is available
    pub storage_spaces: bool,
}

#[cfg(target_os = "macos")]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MacOSFeatures {
    /// Whether copyfile is available
    pub copyfile: bool,
    /// Whether APFS cloning is available
    pub apfs_cloning: bool,
    /// Whether HFS+ is supported
    pub hfs_plus: bool,
}

impl ZeroCopyCapabilities {
    /// Detect zero-copy capabilities for the current platform
    pub fn detect() -> Self {
        debug!("Detecting zero-copy capabilities for current platform");

        let available_methods = Self::detect_available_methods();
        let hardware_acceleration = Self::detect_hardware_acceleration();
        let max_file_size = Self::detect_max_file_size();
        let cross_filesystem = Self::detect_cross_filesystem_support();
        let platform_features = Self::detect_platform_features();

        Self {
            available_methods,
            hardware_acceleration,
            max_file_size,
            cross_filesystem,
            platform_features,
        }
    }

    /// Detect available zero-copy methods
    fn detect_available_methods() -> Vec<ZeroCopyMethodType> {
        let mut methods = Vec::new();

        #[cfg(target_os = "linux")]
        {
            // Check for copy_file_range availability
            if Self::check_copy_file_range() {
                methods.push(ZeroCopyMethodType::CopyFileRange);
            }

            // Check for sendfile availability
            if Self::check_sendfile() {
                methods.push(ZeroCopyMethodType::SendFile);
            }

            // Check for io_uring availability
            if Self::check_io_uring() {
                methods.push(ZeroCopyMethodType::IoUring);
            }
        }

        #[cfg(target_os = "windows")]
        {
            // Check for ReFS CoW availability
            if Self::check_refs_cow() {
                methods.push(ZeroCopyMethodType::RefsCoW);
            }

            // Check for NTFS hardlinks
            if Self::check_ntfs_hardlinks() {
                methods.push(ZeroCopyMethodType::NtfsHardlink);
            }
        }

        #[cfg(target_os = "macos")]
        {
            // Check for copyfile availability
            if Self::check_copyfile() {
                methods.push(ZeroCopyMethodType::CopyFile);
            }
        }

        // Always add fallback as the last option
        methods.push(ZeroCopyMethodType::Fallback);

        debug!("Detected zero-copy methods: {:?}", methods);
        methods
    }

    /// Detect hardware acceleration support
    fn detect_hardware_acceleration() -> bool {
        // Platform-specific hardware acceleration detection
        #[cfg(target_os = "linux")]
        {
            // Check for DMA engines, RDMA, etc.
            return Self::check_linux_hardware_acceleration();
        }

        #[cfg(target_os = "windows")]
        {
            // Check for Windows hardware acceleration features
            return Self::check_windows_hardware_acceleration();
        }

        #[cfg(target_os = "macos")]
        {
            // Check for macOS hardware acceleration features
            return Self::check_macos_hardware_acceleration();
        }

        #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
        {
            false
        }
    }

    /// Detect maximum file size for zero-copy operations
    fn detect_max_file_size() -> u64 {
        // Platform-specific limits
        #[cfg(target_os = "linux")]
        {
            // Linux copy_file_range has a limit of 2GB per call
            return 2 * 1024 * 1024 * 1024; // 2GB
        }

        #[cfg(target_os = "windows")]
        {
            // Windows ReFS supports very large files
            return 100 * 1024 * 1024 * 1024; // 100GB
        }

        #[cfg(target_os = "macos")]
        {
            // macOS copyfile supports large files
            return 50 * 1024 * 1024 * 1024; // 50GB
        }

        #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
        {
            1024 * 1024 * 1024 // 1GB default
        }
    }

    /// Detect cross-filesystem zero-copy support
    fn detect_cross_filesystem_support() -> bool {
        // Most zero-copy methods require same filesystem
        false
    }

    /// Detect platform-specific features
    fn detect_platform_features() -> PlatformFeatures {
        PlatformFeatures {
            #[cfg(target_os = "linux")]
            linux: LinuxFeatures {
                copy_file_range: Self::check_copy_file_range(),
                sendfile: Self::check_sendfile(),
                io_uring: Self::check_io_uring(),
                btrfs_reflink: Self::check_btrfs_reflink(),
                xfs_reflink: Self::check_xfs_reflink(),
            },
            #[cfg(target_os = "windows")]
            windows: WindowsFeatures {
                refs_cow: Self::check_refs_cow(),
                ntfs_hardlinks: Self::check_ntfs_hardlinks(),
                storage_spaces: Self::check_storage_spaces(),
            },
            #[cfg(target_os = "macos")]
            macos: MacOSFeatures {
                copyfile: Self::check_copyfile(),
                apfs_cloning: Self::check_apfs_cloning(),
                hfs_plus: Self::check_hfs_plus(),
            },
        }
    }

    // Platform-specific capability checks (simplified implementations)

    #[cfg(target_os = "linux")]
    fn check_copy_file_range() -> bool {
        // Check if copy_file_range syscall is available
        // This would typically involve checking kernel version or attempting the syscall
        true // Simplified: assume available on Linux
    }

    #[cfg(target_os = "linux")]
    fn check_sendfile() -> bool {
        // Check if sendfile syscall is available
        true // Simplified: assume available on Linux
    }

    #[cfg(target_os = "linux")]
    fn check_io_uring() -> bool {
        // Check if io_uring is available (kernel 5.1+)
        false // Simplified: assume not available for compatibility
    }

    #[cfg(target_os = "linux")]
    fn check_btrfs_reflink() -> bool {
        // Check if BTRFS reflink is available
        false // Simplified: would need to check filesystem type
    }

    #[cfg(target_os = "linux")]
    fn check_xfs_reflink() -> bool {
        // Check if XFS reflink is available
        false // Simplified: would need to check filesystem type
    }

    #[cfg(target_os = "linux")]
    fn check_linux_hardware_acceleration() -> bool {
        // Check for Linux hardware acceleration features
        false // Simplified
    }

    #[cfg(target_os = "windows")]
    fn check_refs_cow() -> bool {
        // Check if ReFS Copy-on-Write is available
        false // Simplified: would need to check filesystem type
    }

    #[cfg(target_os = "windows")]
    fn check_ntfs_hardlinks() -> bool {
        // Check if NTFS hardlinks are available
        true // Simplified: assume available on Windows
    }

    #[cfg(target_os = "windows")]
    fn check_storage_spaces() -> bool {
        // Check if Windows Storage Spaces is available
        false // Simplified
    }

    #[cfg(target_os = "windows")]
    fn check_windows_hardware_acceleration() -> bool {
        // Check for Windows hardware acceleration features
        false // Simplified
    }

    #[cfg(target_os = "macos")]
    fn check_copyfile() -> bool {
        // Check if copyfile is available
        true // Simplified: assume available on macOS
    }

    #[cfg(target_os = "macos")]
    fn check_apfs_cloning() -> bool {
        // Check if APFS cloning is available
        false // Simplified: would need to check filesystem type
    }

    #[cfg(target_os = "macos")]
    fn check_hfs_plus() -> bool {
        // Check if HFS+ is supported
        true // Simplified: assume available on macOS
    }

    #[cfg(target_os = "macos")]
    fn check_macos_hardware_acceleration() -> bool {
        // Check for macOS hardware acceleration features
        false // Simplified
    }
}

/// Trait for zero-copy method implementations
#[async_trait::async_trait]
pub trait ZeroCopyMethod {
    /// Check if zero-copy is available for the given paths
    fn is_zero_copy_available(&self, source: &Path, destination: &Path) -> bool;

    /// Attempt zero-copy operation with the specified method
    async fn try_zero_copy(
        &self,
        source: &Path,
        destination: &Path,
        size: u64,
        method: ZeroCopyMethodType,
    ) -> Result<()>;

    /// Get the preferred method for the current platform
    fn preferred_method(&self) -> ZeroCopyMethodType;

    /// Get all supported methods
    fn supported_methods(&self) -> Vec<ZeroCopyMethodType>;
}

/// Create platform-specific zero-copy method implementation
pub fn create_platform_method() -> Arc<dyn ZeroCopyMethod + Send + Sync> {
    #[cfg(target_os = "linux")]
    {
        Arc::new(crate::linux::LinuxZeroCopy::new())
    }
    #[cfg(target_os = "windows")]
    {
        Arc::new(crate::windows::WindowsZeroCopy::new())
    }
    #[cfg(target_os = "macos")]
    {
        Arc::new(crate::macos::MacOSZeroCopy::new())
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        Arc::new(FallbackZeroCopy::new())
    }
}

/// Fallback zero-copy implementation for unsupported platforms
#[derive(Debug)]
pub struct FallbackZeroCopy;

impl FallbackZeroCopy {
    /// Create a new fallback zero-copy implementation
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl ZeroCopyMethod for FallbackZeroCopy {
    fn is_zero_copy_available(&self, _source: &Path, _destination: &Path) -> bool {
        false
    }

    async fn try_zero_copy(
        &self,
        _source: &Path,
        _destination: &Path,
        _size: u64,
        _method: ZeroCopyMethodType,
    ) -> Result<()> {
        Err(Error::zero_copy("Zero-copy not supported on this platform"))
    }

    fn preferred_method(&self) -> ZeroCopyMethodType {
        ZeroCopyMethodType::Fallback
    }

    fn supported_methods(&self) -> Vec<ZeroCopyMethodType> {
        vec![ZeroCopyMethodType::Fallback]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_copy_capabilities_detection() {
        let capabilities = ZeroCopyCapabilities::detect();
        assert!(!capabilities.available_methods.is_empty());
        assert!(capabilities
            .available_methods
            .contains(&ZeroCopyMethodType::Fallback));
    }

    #[test]
    fn test_fallback_zero_copy() {
        use std::path::Path;
        let fallback = FallbackZeroCopy::new();
        assert!(!fallback.is_zero_copy_available(Path::new("source"), Path::new("dest")));
        assert_eq!(fallback.preferred_method(), ZeroCopyMethodType::Fallback);
        assert_eq!(
            fallback.supported_methods(),
            vec![ZeroCopyMethodType::Fallback]
        );
    }

    #[tokio::test]
    async fn test_fallback_zero_copy_operation() {
        use std::path::Path;
        let fallback = FallbackZeroCopy::new();
        let result = fallback
            .try_zero_copy(
                Path::new("source"),
                Path::new("dest"),
                1024,
                ZeroCopyMethodType::Fallback,
            )
            .await;
        assert!(result.is_err());
    }
}
