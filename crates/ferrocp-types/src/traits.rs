//! Core traits for FerroCP operations
//!
//! This module defines the fundamental traits that enable polymorphic behavior
//! across different implementations of file operations, progress reporting, and device detection.

use crate::{CopyStats, DeviceType, Error, OperationId, ProgressInfo, Result};
use std::path::Path;

#[cfg(feature = "async")]
use async_trait::async_trait;

/// Trait for reporting progress during operations
pub trait ProgressReporter {
    /// Report progress information
    fn report_progress(&self, info: &ProgressInfo);

    /// Report an error that occurred during the operation
    fn report_error(&self, error: &Error);

    /// Report completion of the operation
    fn report_completion(&self, stats: &CopyStats);
}

/// Trait for file operations
#[cfg_attr(feature = "async", async_trait)]
pub trait FileOperations {
    /// Copy a single file
    #[cfg(feature = "async")]
    async fn copy_file<P: AsRef<Path> + Send>(
        &self,
        source: P,
        destination: P,
    ) -> Result<CopyStats>;

    /// Copy a single file (sync version)
    #[cfg(not(feature = "async"))]
    fn copy_file<P: AsRef<Path>>(&self, source: P, destination: P) -> Result<CopyStats>;

    /// Copy a directory tree
    #[cfg(feature = "async")]
    async fn copy_tree<P: AsRef<Path> + Send>(
        &self,
        source: P,
        destination: P,
    ) -> Result<CopyStats>;

    /// Copy a directory tree (sync version)
    #[cfg(not(feature = "async"))]
    fn copy_tree<P: AsRef<Path>>(&self, source: P, destination: P) -> Result<CopyStats>;

    /// Verify file integrity
    #[cfg(feature = "async")]
    async fn verify_file<P: AsRef<Path> + Send>(&self, path: P) -> Result<bool>;

    /// Verify file integrity (sync version)
    #[cfg(not(feature = "async"))]
    fn verify_file<P: AsRef<Path>>(&self, path: P) -> Result<bool>;
}

/// Trait for device detection and optimization
#[cfg_attr(feature = "async", async_trait)]
pub trait DeviceDetector {
    /// Detect the device type for a given path
    #[cfg(feature = "async")]
    async fn detect_device_type<P: AsRef<Path> + Send>(&self, path: P) -> Result<DeviceType>;

    /// Detect the device type for a given path (sync version)
    #[cfg(not(feature = "async"))]
    fn detect_device_type<P: AsRef<Path>>(&self, path: P) -> Result<DeviceType>;

    /// Get optimal buffer size for the device
    fn get_optimal_buffer_size(&self, device_type: DeviceType) -> usize;

    /// Check if zero-copy is supported for the device
    fn supports_zero_copy(&self, device_type: DeviceType) -> bool;
}

/// Trait for compression operations
#[cfg_attr(feature = "async", async_trait)]
pub trait CompressionEngine {
    /// Compress data
    #[cfg(feature = "async")]
    async fn compress(&self, data: &[u8]) -> Result<Vec<u8>>;

    /// Compress data (sync version)
    #[cfg(not(feature = "async"))]
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>>;

    /// Decompress data
    #[cfg(feature = "async")]
    async fn decompress(&self, data: &[u8]) -> Result<Vec<u8>>;

    /// Decompress data (sync version)
    #[cfg(not(feature = "async"))]
    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>>;

    /// Get compression ratio estimate
    fn estimate_compression_ratio(&self, data: &[u8]) -> f64;
}

/// Trait for zero-copy operations
#[cfg_attr(feature = "async", async_trait)]
pub trait ZeroCopyEngine {
    /// Attempt zero-copy operation
    #[cfg(feature = "async")]
    async fn zero_copy<P: AsRef<Path> + Send>(
        &self,
        source: P,
        destination: P,
        size: u64,
    ) -> Result<bool>;

    /// Attempt zero-copy operation (sync version)
    #[cfg(not(feature = "async"))]
    fn zero_copy<P: AsRef<Path>>(&self, source: P, destination: P, size: u64) -> Result<bool>;

    /// Check if zero-copy is available for the given paths
    fn is_zero_copy_available<P: AsRef<Path>>(&self, source: P, destination: P) -> bool;
}

/// Trait for operation cancellation
pub trait Cancellable {
    /// Cancel the operation
    fn cancel(&self);

    /// Check if the operation is cancelled
    fn is_cancelled(&self) -> bool;
}

/// Trait for operation identification
pub trait Identifiable {
    /// Get the operation ID
    fn operation_id(&self) -> OperationId;
}

/// Combined trait for all copy engine capabilities
pub trait CopyEngine:
    FileOperations + DeviceDetector + CompressionEngine + ZeroCopyEngine + Cancellable + Identifiable
{
}

// Blanket implementation for any type that implements all required traits
impl<T> CopyEngine for T where
    T: FileOperations
        + DeviceDetector
        + CompressionEngine
        + ZeroCopyEngine
        + Cancellable
        + Identifiable
{
}
