//! FFI-safe type definitions
//!
//! This module contains C-compatible type definitions that can be safely
//! passed across FFI boundaries to Python, C++, and other languages.

use std::os::raw::{c_char, c_int, c_uint, c_ulonglong};

/// Copy operation modes
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FerrocpCopyMode {
    /// Copy files (default)
    Copy = 0,
    /// Move files (copy then delete source)
    Move = 1,
    /// Synchronize directories
    Sync = 2,
}

impl From<c_int> for FerrocpCopyMode {
    fn from(value: c_int) -> Self {
        match value {
            1 => FerrocpCopyMode::Move,
            2 => FerrocpCopyMode::Sync,
            _ => FerrocpCopyMode::Copy,
        }
    }
}

impl From<FerrocpCopyMode> for ferrocp_types::CopyMode {
    fn from(mode: FerrocpCopyMode) -> Self {
        match mode {
            FerrocpCopyMode::Copy => ferrocp_types::CopyMode::All,
            FerrocpCopyMode::Move => ferrocp_types::CopyMode::Newer,
            FerrocpCopyMode::Sync => ferrocp_types::CopyMode::Mirror,
        }
    }
}

/// Device types
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FerrocpDeviceType {
    /// Unknown device type
    Unknown = 0,
    /// Hard Disk Drive
    Hdd = 1,
    /// Solid State Drive
    Ssd = 2,
    /// Network storage
    Network = 3,
    /// RAM disk
    RamDisk = 4,
}

impl From<ferrocp_types::DeviceType> for FerrocpDeviceType {
    fn from(device_type: ferrocp_types::DeviceType) -> Self {
        match device_type {
            ferrocp_types::DeviceType::HDD => FerrocpDeviceType::Hdd,
            ferrocp_types::DeviceType::SSD => FerrocpDeviceType::Ssd,
            ferrocp_types::DeviceType::Network => FerrocpDeviceType::Network,
            ferrocp_types::DeviceType::RamDisk => FerrocpDeviceType::RamDisk,
            ferrocp_types::DeviceType::Unknown => FerrocpDeviceType::Unknown,
        }
    }
}

/// Error codes
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FerrocpErrorCode {
    /// Success
    Success = 0,
    /// Generic error
    GenericError = 1,
    /// File not found
    FileNotFound = 2,
    /// Permission denied
    PermissionDenied = 3,
    /// Insufficient space
    InsufficientSpace = 4,
    /// Invalid path
    InvalidPath = 5,
    /// Network error
    NetworkError = 6,
    /// Compression error
    CompressionError = 7,
    /// Verification error
    VerificationError = 8,
    /// Cancelled by user
    Cancelled = 9,
    /// Invalid argument
    InvalidArgument = 10,
    /// Out of memory
    OutOfMemory = 11,
    /// Timeout
    Timeout = 12,
}

/// Performance rating
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FerrocpPerformanceRating {
    /// Poor performance (< 25% efficiency)
    Poor = 0,
    /// Fair performance (25-50% efficiency)
    Fair = 1,
    /// Good performance (50-75% efficiency)
    Good = 2,
    /// Excellent performance (> 75% efficiency)
    Excellent = 3,
}

/// FFI-safe copy options
#[repr(C)]
#[derive(Debug, Clone)]
pub struct FerrocpCopyOptions {
    /// Copy mode
    pub mode: FerrocpCopyMode,
    /// Enable compression
    pub compress: c_int,
    /// Preserve metadata (timestamps, permissions)
    pub preserve_metadata: c_int,
    /// Verify copy integrity
    pub verify_copy: c_int,
    /// Number of worker threads (0 = auto)
    pub threads: c_uint,
    /// Buffer size in bytes (0 = auto)
    pub buffer_size: c_ulonglong,
    /// Overwrite existing files
    pub overwrite: c_int,
    /// Create destination directories
    pub create_dirs: c_int,
    /// Follow symbolic links
    pub follow_symlinks: c_int,
    /// Exclude hidden files
    pub exclude_hidden: c_int,
}

impl Default for FerrocpCopyOptions {
    fn default() -> Self {
        Self {
            mode: FerrocpCopyMode::Copy,
            compress: 0,
            preserve_metadata: 1,
            verify_copy: 0,
            threads: 0,
            buffer_size: 0,
            overwrite: 0,
            create_dirs: 1,
            follow_symlinks: 0,
            exclude_hidden: 0,
        }
    }
}

/// FFI-safe progress information
#[repr(C)]
#[derive(Debug, Clone)]
pub struct FerrocpProgress {
    /// Progress percentage (0.0 - 100.0)
    pub percent: f64,
    /// Bytes copied so far
    pub bytes_copied: c_ulonglong,
    /// Total bytes to copy
    pub total_bytes: c_ulonglong,
    /// Files copied so far
    pub files_copied: c_ulonglong,
    /// Total files to copy
    pub total_files: c_ulonglong,
    /// Current transfer rate in MB/s
    pub transfer_rate_mbps: f64,
    /// Estimated time remaining in seconds
    pub eta_seconds: c_ulonglong,
    /// Current file being processed
    pub current_file: *const c_char,
}

/// FFI-safe performance analysis
#[repr(C)]
#[derive(Debug, Clone)]
pub struct FerrocpPerformanceAnalysis {
    /// Expected transfer speed in MB/s
    pub expected_speed_mbps: f64,
    /// Actual transfer speed in MB/s
    pub actual_speed_mbps: f64,
    /// Performance efficiency percentage
    pub efficiency_percent: f64,
    /// Performance rating
    pub rating: FerrocpPerformanceRating,
    /// Bottleneck device type
    pub bottleneck_device: FerrocpDeviceType,
    /// Bottleneck description
    pub bottleneck_description: *const c_char,
    /// Number of recommendations
    pub recommendation_count: c_uint,
    /// Array of recommendation strings
    pub recommendations: *const *const c_char,
}

/// FFI-safe operation result
#[repr(C)]
#[derive(Debug, Clone)]
pub struct FerrocpOperationResult {
    /// Whether the operation was successful
    pub success: c_int,
    /// Error code (if not successful)
    pub error_code: FerrocpErrorCode,
    /// Error message
    pub error_message: *const c_char,
    /// Copy statistics
    pub stats: super::FerrocpStats,
    /// Performance analysis
    pub performance: FerrocpPerformanceAnalysis,
    /// Source device information
    pub source_device: super::FerrocpDeviceInfo,
    /// Destination device information
    pub destination_device: super::FerrocpDeviceInfo,
}

/// Convert boolean to C int
pub(crate) fn bool_to_c_int(value: bool) -> c_int {
    if value {
        1
    } else {
        0
    }
}

/// Convert C int to boolean
pub(crate) fn c_int_to_bool(value: c_int) -> bool {
    value != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copy_mode_conversion() {
        assert_eq!(FerrocpCopyMode::from(0), FerrocpCopyMode::Copy);
        assert_eq!(FerrocpCopyMode::from(1), FerrocpCopyMode::Move);
        assert_eq!(FerrocpCopyMode::from(2), FerrocpCopyMode::Sync);
        assert_eq!(FerrocpCopyMode::from(999), FerrocpCopyMode::Copy); // Default
    }

    #[test]
    fn test_bool_conversion() {
        assert_eq!(bool_to_c_int(true), 1);
        assert_eq!(bool_to_c_int(false), 0);
        assert_eq!(c_int_to_bool(1), true);
        assert_eq!(c_int_to_bool(0), false);
        assert_eq!(c_int_to_bool(-1), true); // Non-zero is true
    }

    #[test]
    fn test_default_options() {
        let options = FerrocpCopyOptions::default();
        assert_eq!(options.mode, FerrocpCopyMode::Copy);
        assert_eq!(options.preserve_metadata, 1);
        assert_eq!(options.create_dirs, 1);
    }
}
