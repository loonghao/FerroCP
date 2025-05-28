//! Core data types for FerroCP
//!
//! This module provides the fundamental data types used throughout the FerroCP ecosystem.
//! It includes statistics, progress tracking, device information, and operation results.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

#[cfg(feature = "async")]
use async_trait::async_trait;

/// Unique identifier for operations
pub type OperationId = uuid::Uuid;

/// Unique identifier for files
pub type FileId = uuid::Uuid;

/// File size in bytes
pub type FileSize = u64;

/// Transfer rate in bytes per second
pub type TransferRate = f64;

/// File copy statistics
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CopyStats {
    /// Number of files copied
    pub files_copied: u64,
    /// Number of directories created
    pub directories_created: u64,
    /// Total bytes copied
    pub bytes_copied: u64,
    /// Number of files skipped
    pub files_skipped: u64,
    /// Number of errors encountered
    pub errors: u64,
    /// Total duration of the operation
    pub duration: Duration,
    /// Number of zero-copy operations used
    pub zerocopy_operations: u64,
    /// Bytes transferred using zero-copy
    pub zerocopy_bytes: u64,
}

impl CopyStats {
    /// Create a new empty statistics instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculate the overall transfer rate
    pub fn transfer_rate(&self) -> TransferRate {
        if self.duration.as_secs_f64() > 0.0 {
            self.bytes_copied as f64 / self.duration.as_secs_f64()
        } else {
            0.0
        }
    }

    /// Calculate the zero-copy efficiency ratio
    pub fn zerocopy_efficiency(&self) -> f64 {
        if self.bytes_copied > 0 {
            self.zerocopy_bytes as f64 / self.bytes_copied as f64
        } else {
            0.0
        }
    }

    /// Merge statistics from another instance
    pub fn merge(&mut self, other: &CopyStats) {
        self.files_copied += other.files_copied;
        self.directories_created += other.directories_created;
        self.bytes_copied += other.bytes_copied;
        self.files_skipped += other.files_skipped;
        self.errors += other.errors;
        self.duration += other.duration;
        self.zerocopy_operations += other.zerocopy_operations;
        self.zerocopy_bytes += other.zerocopy_bytes;
    }
}

/// Progress information for copy operations
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ProgressInfo {
    /// Current file being processed
    pub current_file: PathBuf,
    /// Bytes processed for current file
    pub current_file_bytes: u64,
    /// Total size of current file
    pub current_file_size: u64,
    /// Total files processed
    pub files_processed: u64,
    /// Total files to process
    pub total_files: u64,
    /// Total bytes processed
    pub bytes_processed: u64,
    /// Total bytes to process
    pub total_bytes: u64,
    /// Current transfer rate in bytes per second
    pub transfer_rate: f64,
    /// Estimated time remaining
    pub eta: Option<Duration>,
}

impl ProgressInfo {
    /// Calculate the progress percentage for the current file
    pub fn current_file_progress(&self) -> f64 {
        if self.current_file_size > 0 {
            (self.current_file_bytes as f64 / self.current_file_size as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Calculate the overall progress percentage
    pub fn overall_progress(&self) -> f64 {
        if self.total_bytes > 0 {
            (self.bytes_processed as f64 / self.total_bytes as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Calculate the file progress percentage
    pub fn file_progress(&self) -> f64 {
        if self.total_files > 0 {
            (self.files_processed as f64 / self.total_files as f64) * 100.0
        } else {
            0.0
        }
    }
}

/// Device type for optimization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DeviceType {
    /// Solid State Drive
    SSD,
    /// Hard Disk Drive
    HDD,
    /// Network storage
    Network,
    /// RAM disk
    RamDisk,
    /// Unknown device type
    Unknown,
}

/// Zero-copy method used
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ZeroCopyMethod {
    /// Linux sendfile
    SendFile,
    /// Linux copy_file_range
    CopyFileRange,
    /// Linux io_uring
    IoUring,
    /// macOS copyfile
    CopyFile,
    /// Windows ReFS CoW
    RefsCoW,
    /// Windows NTFS hardlink
    NtfsHardlink,
    /// Fallback to regular copy
    Fallback,
}

/// Zero-copy operation result
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ZeroCopyResult {
    /// Number of bytes copied
    pub bytes_copied: u64,
    /// Whether zero-copy was actually used
    pub zerocopy_used: bool,
    /// Method used for zero-copy
    pub method: ZeroCopyMethod,
}

/// File operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FileOperation {
    /// Copy file
    Copy,
    /// Move file
    Move,
    /// Sync file
    Sync,
    /// Verify file
    Verify,
}

/// Copy mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CopyMode {
    /// Copy all files
    All,
    /// Copy only newer files
    Newer,
    /// Copy only if different
    Different,
    /// Mirror (delete extra files in destination)
    Mirror,
}

/// Compression algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CompressionAlgorithm {
    /// No compression
    None,
    /// Zstandard
    Zstd,
    /// LZ4
    Lz4,
    /// Brotli
    Brotli,
}

/// Network protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum NetworkProtocol {
    /// QUIC protocol
    Quic,
    /// HTTP/3
    Http3,
    /// HTTP/2
    Http2,
    /// TCP
    Tcp,
}

/// Task priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Priority {
    /// Low priority
    Low = 0,
    /// Normal priority
    Normal = 1,
    /// High priority
    High = 2,
    /// Critical priority
    Critical = 3,
}

/// File metadata
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FileMetadata {
    /// File size in bytes
    pub size: u64,
    /// Last modified time
    pub modified: SystemTime,
    /// Creation time (if available)
    pub created: Option<SystemTime>,
    /// File permissions
    pub permissions: u32,
    /// Whether the file is a directory
    pub is_dir: bool,
    /// Whether the file is a symlink
    pub is_symlink: bool,
}
