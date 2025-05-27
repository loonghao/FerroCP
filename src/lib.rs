//! # FerroCP: High-Performance Cross-Platform File Copying with Rust
//!
//! FerroCP is a high-performance cross-platform file copying tool written in Rust, providing:
//! - High-performance file copying with async I/O
//! - Network acceleration with file deduplication
//! - Adaptive compression based on network conditions
//! - Delta copying for incremental backups
//! - Python bindings for easy integration
//! - CLI tool for command-line usage
//!
//! ## Architecture
//!
//! The library is organized into several core modules:
//!
//! - `core`: Core file operations and async utilities
//! - `compression`: Adaptive compression engine
//! - `network`: Network service and client communication
//! - `delta`: Delta copying and incremental sync
//! - `config`: Configuration management
//! - `error`: Error handling and types
//!
//! ## Features
//!
//! - **Async I/O**: Built on tokio for high-performance async operations
//! - **Memory Safety**: Rust's ownership system prevents memory leaks
//! - **Cross-Platform**: Works on Windows, Linux, and macOS
//! - **Compression**: Adaptive zstd compression for network transfers
//! - **Deduplication**: File-level deduplication using Blake3 hashing
//! - **Progress Reporting**: Real-time progress callbacks
//! - **Network Acceleration**: Compatible with EACopy network protocol

// use std::path::Path; // Unused for now

// Core modules
pub mod core;
pub mod compression;
pub mod network;
pub mod delta;
pub mod config;
pub mod error;
pub mod zerocopy;
pub mod device_detector;
pub mod windows_optimization;

// Re-export commonly used types
pub use crate::core::{EACopy, CopyStats, FileOperations};
pub use crate::config::Config;
pub use crate::error::{Error, Result};

// Python bindings (optional)
#[cfg(feature = "python")]
pub mod python;

/// High-level convenience functions for file operations
pub mod ops {
    use super::*;
    use crate::core::EACopy;
    use std::path::Path;

    /// Copy a file from source to destination
    ///
    /// This is equivalent to `shutil.copy()` in Python.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ferrocp::ops;
    ///
    /// # tokio_test::block_on(async {
    /// ops::copy("source.txt", "destination.txt").await?;
    /// # Ok::<(), ferrocp::Error>(())
    /// # });
    /// ```
    pub async fn copy<P: AsRef<Path>, Q: AsRef<Path>>(
        source: P,
        destination: Q,
    ) -> Result<CopyStats> {
        let eacopy = EACopy::new();
        eacopy.copy_file(source, destination).await
    }

    /// Copy a file from source to destination, preserving metadata
    ///
    /// This is equivalent to `shutil.copy2()` in Python.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ferrocp::ops;
    ///
    /// # tokio_test::block_on(async {
    /// ops::copy2("source.txt", "destination.txt").await?;
    /// # Ok::<(), ferrocp::Error>(())
    /// # });
    /// ```
    pub async fn copy2<P: AsRef<Path>, Q: AsRef<Path>>(
        source: P,
        destination: Q,
    ) -> Result<CopyStats> {
        let config = Config::new().with_preserve_metadata(true);
        let eacopy = EACopy::with_config(config);
        eacopy.copy_file(source, destination).await
    }

    /// Copy a directory tree from source to destination
    ///
    /// This is equivalent to `shutil.copytree()` in Python.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ferrocp::ops;
    ///
    /// # tokio_test::block_on(async {
    /// ops::copytree("source_dir", "destination_dir").await?;
    /// # Ok::<(), ferrocp::Error>(())
    /// # });
    /// ```
    pub async fn copytree<P: AsRef<Path>, Q: AsRef<Path>>(
        source: P,
        destination: Q,
    ) -> Result<CopyStats> {
        let eacopy = EACopy::new();
        eacopy.copy_directory(source, destination).await
    }

    /// Copy a file using zero-copy methods when possible
    ///
    /// This function attempts to use platform-specific zero-copy operations
    /// like copy_file_range (Linux), reflink (BTRFS/XFS), or CoW (ReFS).
    /// Falls back to regular copy if zero-copy is not available.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ferrocp::ops;
    ///
    /// # tokio_test::block_on(async {
    /// ops::copy_zerocopy("large_file.bin", "backup.bin").await?;
    /// # Ok::<(), ferrocp::Error>(())
    /// # });
    /// ```
    pub async fn copy_zerocopy<P: AsRef<Path>, Q: AsRef<Path>>(
        source: P,
        destination: Q,
    ) -> Result<CopyStats> {
        let eacopy = EACopy::new();
        eacopy.copy_file_zerocopy(source, destination).await
    }
}