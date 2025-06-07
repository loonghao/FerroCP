//! Core data types for FerroCP
//!
//! This module provides the fundamental data types used throughout the FerroCP ecosystem.
//! It includes statistics, progress tracking, device information, and operation results.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

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
    /// Note: Duration is taken as the maximum to avoid cumulative time issues
    pub fn merge(&mut self, other: &CopyStats) {
        self.files_copied += other.files_copied;
        self.directories_created += other.directories_created;
        self.bytes_copied += other.bytes_copied;
        self.files_skipped += other.files_skipped;
        self.errors += other.errors;
        // Take the maximum duration instead of adding them to avoid cumulative time
        self.duration = self.duration.max(other.duration);
        self.zerocopy_operations += other.zerocopy_operations;
        self.zerocopy_bytes += other.zerocopy_bytes;
    }

    /// Merge statistics with proper duration handling for parallel operations
    pub fn merge_with_duration(&mut self, other: &CopyStats, total_duration: Duration) {
        self.files_copied += other.files_copied;
        self.directories_created += other.directories_created;
        self.bytes_copied += other.bytes_copied;
        self.files_skipped += other.files_skipped;
        self.errors += other.errors;
        self.duration = total_duration; // Use the actual total duration
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

/// Device detection cache entry
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DeviceCacheEntry {
    /// Detected device type
    pub device_type: DeviceType,
    /// Timestamp when the entry was created
    pub created_at: SystemTime,
    /// Timestamp when the entry was last accessed
    pub last_accessed: SystemTime,
    /// Number of times this entry has been accessed
    pub access_count: u64,
}

impl DeviceCacheEntry {
    /// Create a new cache entry
    pub fn new(device_type: DeviceType) -> Self {
        let now = SystemTime::now();
        Self {
            device_type,
            created_at: now,
            last_accessed: now,
            access_count: 1,
        }
    }

    /// Update the last accessed time and increment access count
    pub fn touch(&mut self) {
        self.last_accessed = SystemTime::now();
        self.access_count += 1;
    }

    /// Check if the entry has expired based on the given TTL
    pub fn is_expired(&self, ttl: Duration) -> bool {
        self.created_at.elapsed().unwrap_or(Duration::MAX) > ttl
    }

    /// Get the age of this cache entry
    pub fn age(&self) -> Duration {
        self.created_at.elapsed().unwrap_or(Duration::ZERO)
    }
}

/// Device detection cache statistics
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DeviceCacheStats {
    /// Total number of cache lookups
    pub total_lookups: u64,
    /// Number of cache hits
    pub cache_hits: u64,
    /// Number of cache misses
    pub cache_misses: u64,
    /// Number of cache evictions
    pub evictions: u64,
    /// Number of expired entries removed
    pub expired_removals: u64,
    /// Current cache size
    pub current_size: usize,
    /// Maximum cache size reached
    pub max_size_reached: usize,
    /// Total memory usage in bytes (estimated)
    pub memory_usage_bytes: usize,
}

impl DeviceCacheStats {
    /// Calculate cache hit rate as a percentage
    pub fn hit_rate(&self) -> f64 {
        if self.total_lookups == 0 {
            0.0
        } else {
            (self.cache_hits as f64 / self.total_lookups as f64) * 100.0
        }
    }

    /// Calculate cache miss rate as a percentage
    pub fn miss_rate(&self) -> f64 {
        100.0 - self.hit_rate()
    }

    /// Record a cache hit
    pub fn record_hit(&mut self) {
        self.total_lookups += 1;
        self.cache_hits += 1;
    }

    /// Record a cache miss
    pub fn record_miss(&mut self) {
        self.total_lookups += 1;
        self.cache_misses += 1;
    }

    /// Record a cache eviction
    pub fn record_eviction(&mut self) {
        self.evictions += 1;
    }

    /// Record an expired entry removal
    pub fn record_expired_removal(&mut self) {
        self.expired_removals += 1;
    }

    /// Update cache size statistics
    pub fn update_size(&mut self, current_size: usize) {
        self.current_size = current_size;
        self.max_size_reached = self.max_size_reached.max(current_size);
    }

    /// Update memory usage estimate
    pub fn update_memory_usage(&mut self, memory_bytes: usize) {
        self.memory_usage_bytes = memory_bytes;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    // Property tests for CopyStats
    proptest! {
        #[test]
        fn test_copy_stats_transfer_rate(
            bytes_copied in 0u64..1_000_000_000u64,
            duration_secs in 1u64..3600u64
        ) {
            let mut stats = CopyStats::new();
            stats.bytes_copied = bytes_copied;
            stats.duration = Duration::from_secs(duration_secs);

            let rate = stats.transfer_rate();
            let expected_rate = bytes_copied as f64 / duration_secs as f64;

            prop_assert!((rate - expected_rate).abs() < 0.001);
            prop_assert!(rate >= 0.0);
        }

        #[test]
        fn test_copy_stats_zerocopy_efficiency(
            bytes_copied in 1u64..1_000_000u64,
            zerocopy_bytes in 0u64..1_000_000u64
        ) {
            let mut stats = CopyStats::new();
            stats.bytes_copied = bytes_copied;
            stats.zerocopy_bytes = zerocopy_bytes.min(bytes_copied);

            let efficiency = stats.zerocopy_efficiency();

            prop_assert!(efficiency >= 0.0);
            prop_assert!(efficiency <= 1.0);

            if stats.zerocopy_bytes == stats.bytes_copied {
                prop_assert!((efficiency - 1.0).abs() < 0.001);
            }
        }

        #[test]
        fn test_copy_stats_merge(
            files1 in 0u64..1000u64,
            files2 in 0u64..1000u64,
            bytes1 in 0u64..1_000_000u64,
            bytes2 in 0u64..1_000_000u64
        ) {
            let mut stats1 = CopyStats::new();
            stats1.files_copied = files1;
            stats1.bytes_copied = bytes1;

            let stats2 = CopyStats {
                files_copied: files2,
                bytes_copied: bytes2,
                ..CopyStats::new()
            };

            stats1.merge(&stats2);

            prop_assert_eq!(stats1.files_copied, files1 + files2);
            prop_assert_eq!(stats1.bytes_copied, bytes1 + bytes2);
        }
    }

    // Property tests for ProgressInfo
    proptest! {
        #[test]
        fn test_progress_info_current_file_progress(
            current_bytes in 0u64..1_000_000u64,
            total_bytes in 1u64..1_000_000u64
        ) {
            let current_bytes = current_bytes.min(total_bytes);
            let progress = ProgressInfo {
                current_file: PathBuf::from("test.txt"),
                current_file_bytes: current_bytes,
                current_file_size: total_bytes,
                files_processed: 0,
                total_files: 1,
                bytes_processed: 0,
                total_bytes: 0,
                transfer_rate: 0.0,
                eta: None,
            };

            let percentage = progress.current_file_progress();

            prop_assert!(percentage >= 0.0);
            prop_assert!(percentage <= 100.0);

            if current_bytes == total_bytes {
                prop_assert!((percentage - 100.0).abs() < 0.001);
            }
        }

        #[test]
        fn test_progress_info_overall_progress(
            processed_bytes in 0u64..1_000_000u64,
            total_bytes in 1u64..1_000_000u64
        ) {
            let processed_bytes = processed_bytes.min(total_bytes);
            let progress = ProgressInfo {
                current_file: PathBuf::from("test.txt"),
                current_file_bytes: 0,
                current_file_size: 0,
                files_processed: 0,
                total_files: 1,
                bytes_processed: processed_bytes,
                total_bytes,
                transfer_rate: 0.0,
                eta: None,
            };

            let percentage = progress.overall_progress();

            prop_assert!(percentage >= 0.0);
            prop_assert!(percentage <= 100.0);
        }

        #[test]
        fn test_progress_info_file_progress(
            processed_files in 0u64..1000u64,
            total_files in 1u64..1000u64
        ) {
            let processed_files = processed_files.min(total_files);
            let progress = ProgressInfo {
                current_file: PathBuf::from("test.txt"),
                current_file_bytes: 0,
                current_file_size: 0,
                files_processed: processed_files,
                total_files,
                bytes_processed: 0,
                total_bytes: 0,
                transfer_rate: 0.0,
                eta: None,
            };

            let percentage = progress.file_progress();

            prop_assert!(percentage >= 0.0);
            prop_assert!(percentage <= 100.0);
        }
    }

    // Property tests for ZeroCopyResult
    proptest! {
        #[test]
        fn test_zerocopy_result_consistency(
            bytes_copied in 0u64..1_000_000u64,
            zerocopy_used in any::<bool>(),
            method in prop_oneof![
                Just(ZeroCopyMethod::SendFile),
                Just(ZeroCopyMethod::CopyFileRange),
                Just(ZeroCopyMethod::IoUring),
                Just(ZeroCopyMethod::CopyFile),
                Just(ZeroCopyMethod::RefsCoW),
                Just(ZeroCopyMethod::NtfsHardlink),
                Just(ZeroCopyMethod::Fallback),
            ]
        ) {
            // Ensure logical consistency: if method is Fallback, zerocopy_used should be false
            let consistent_zerocopy_used = if matches!(method, ZeroCopyMethod::Fallback) {
                false
            } else {
                zerocopy_used
            };

            let result = ZeroCopyResult {
                bytes_copied,
                zerocopy_used: consistent_zerocopy_used,
                method,
            };

            // If fallback method is used, zerocopy_used should be false
            if matches!(method, ZeroCopyMethod::Fallback) {
                prop_assert!(!result.zerocopy_used);
            }

            // Bytes copied should be consistent
            prop_assert_eq!(result.bytes_copied, bytes_copied);
        }
    }

    // Unit tests for enum properties
    #[test]
    fn test_device_type_variants() {
        let devices = vec![
            DeviceType::SSD,
            DeviceType::HDD,
            DeviceType::Network,
            DeviceType::RamDisk,
            DeviceType::Unknown,
        ];

        // Test that all variants are distinct
        for (i, device1) in devices.iter().enumerate() {
            for (j, device2) in devices.iter().enumerate() {
                if i != j {
                    assert_ne!(device1, device2);
                }
            }
        }
    }

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::Low < Priority::Normal);
        assert!(Priority::Normal < Priority::High);
        assert!(Priority::High < Priority::Critical);

        // Test numeric values
        assert_eq!(Priority::Low as u8, 0);
        assert_eq!(Priority::Normal as u8, 1);
        assert_eq!(Priority::High as u8, 2);
        assert_eq!(Priority::Critical as u8, 3);
    }

    #[test]
    fn test_copy_mode_variants() {
        let modes = vec![
            CopyMode::All,
            CopyMode::Newer,
            CopyMode::Different,
            CopyMode::Mirror,
        ];

        // Test that all variants are distinct
        for (i, mode1) in modes.iter().enumerate() {
            for (j, mode2) in modes.iter().enumerate() {
                if i != j {
                    assert_ne!(mode1, mode2);
                }
            }
        }
    }

    #[test]
    fn test_compression_algorithm_variants() {
        let algorithms = vec![
            CompressionAlgorithm::None,
            CompressionAlgorithm::Zstd,
            CompressionAlgorithm::Lz4,
            CompressionAlgorithm::Brotli,
        ];

        // Test that all variants are distinct
        for (i, alg1) in algorithms.iter().enumerate() {
            for (j, alg2) in algorithms.iter().enumerate() {
                if i != j {
                    assert_ne!(alg1, alg2);
                }
            }
        }
    }

    #[test]
    fn test_network_protocol_variants() {
        let protocols = vec![
            NetworkProtocol::Quic,
            NetworkProtocol::Http3,
            NetworkProtocol::Http2,
            NetworkProtocol::Tcp,
        ];

        // Test that all variants are distinct
        for (i, proto1) in protocols.iter().enumerate() {
            for (j, proto2) in protocols.iter().enumerate() {
                if i != j {
                    assert_ne!(proto1, proto2);
                }
            }
        }
    }

    #[test]
    fn test_copy_stats_zero_duration() {
        let mut stats = CopyStats::new();
        stats.bytes_copied = 1000;
        stats.duration = Duration::from_secs(0);

        // Transfer rate should be 0 when duration is 0
        assert_eq!(stats.transfer_rate(), 0.0);
    }

    #[test]
    fn test_copy_stats_zero_bytes() {
        let stats = CopyStats::new();

        // Zero-copy efficiency should be 0 when no bytes copied
        assert_eq!(stats.zerocopy_efficiency(), 0.0);
    }

    #[test]
    fn test_progress_info_zero_totals() {
        let progress = ProgressInfo {
            current_file: PathBuf::from("test.txt"),
            current_file_bytes: 100,
            current_file_size: 0,
            files_processed: 5,
            total_files: 0,
            bytes_processed: 1000,
            total_bytes: 0,
            transfer_rate: 0.0,
            eta: None,
        };

        // Progress should be 0 when totals are 0
        assert_eq!(progress.current_file_progress(), 0.0);
        assert_eq!(progress.overall_progress(), 0.0);
        assert_eq!(progress.file_progress(), 0.0);
    }

    #[test]
    fn test_file_metadata_creation() {
        let metadata = FileMetadata {
            size: 1024,
            modified: SystemTime::now(),
            created: Some(UNIX_EPOCH),
            permissions: 0o644,
            is_dir: false,
            is_symlink: false,
        };

        assert_eq!(metadata.size, 1024);
        assert_eq!(metadata.permissions, 0o644);
        assert!(!metadata.is_dir);
        assert!(!metadata.is_symlink);
        assert!(metadata.created.is_some());
    }
}
