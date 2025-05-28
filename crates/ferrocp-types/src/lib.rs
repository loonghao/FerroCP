//! Core type system and error handling for FerroCP
//!
//! This crate provides the foundational types, error handling, and shared data structures
//! used throughout the FerroCP ecosystem. It includes:
//!
//! - **Error handling**: Comprehensive error types with severity levels and context
//! - **Core types**: File operations, progress tracking, and device information
//! - **Traits**: Async-ready traits for polymorphic behavior
//! - **Configuration**: Type-safe configuration with validation
//!
//! # Features
//!
//! - `std` (default): Enable standard library features
//! - `async`: Enable async trait definitions
//! - `serde`: Enable serialization support
//!
//! # Examples
//!
//! ```rust
//! use ferrocp_types::{Error, Result, CopyStats, DeviceType};
//!
//! fn example_operation() -> Result<CopyStats> {
//!     let mut stats = CopyStats::new();
//!     stats.files_copied = 10;
//!     stats.bytes_copied = 1024 * 1024; // 1MB
//!     Ok(stats)
//! }
//! ```

#![deny(missing_docs)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::module_name_repetitions)]

pub mod config;
pub mod error;
pub mod result;
pub mod traits;
pub mod types;

// Re-export commonly used types
pub use config::{BufferSize, CompressionLevel, RetryConfig, ThreadCount, TimeoutConfig};
pub use error::{Error, ErrorContext, ErrorKind, ErrorSeverity};
pub use result::Result;
pub use traits::*;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copy_stats_creation() {
        let stats = CopyStats::new();
        assert_eq!(stats.files_copied, 0);
        assert_eq!(stats.bytes_copied, 0);
        assert_eq!(stats.transfer_rate(), 0.0);
    }

    #[test]
    fn test_copy_stats_merge() {
        let mut stats1 = CopyStats::new();
        stats1.files_copied = 5;
        stats1.bytes_copied = 1000;

        let mut stats2 = CopyStats::new();
        stats2.files_copied = 3;
        stats2.bytes_copied = 500;

        stats1.merge(&stats2);
        assert_eq!(stats1.files_copied, 8);
        assert_eq!(stats1.bytes_copied, 1500);
    }

    #[test]
    fn test_error_severity() {
        let io_error = Error::from(std::io::Error::new(std::io::ErrorKind::NotFound, "test"));
        assert_eq!(io_error.severity(), ErrorSeverity::Medium);

        let config_error = Error::config("invalid config");
        assert_eq!(config_error.severity(), ErrorSeverity::High);
        assert!(!config_error.is_recoverable());
    }

    #[test]
    fn test_buffer_size_validation() {
        assert!(BufferSize::new(4096).is_ok());
        assert!(BufferSize::new(8192).is_ok());
        assert!(BufferSize::new(1024).is_err()); // Too small
        assert!(BufferSize::new(5000).is_err()); // Not power of two
    }
}
