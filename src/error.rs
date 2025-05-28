//! Error handling for ferrocp
//!
//! This module defines the error types used throughout the library.
//! All errors implement the standard Error trait and can be converted
//! to/from other common error types.

use std::io;
use std::path::PathBuf;

/// Result type alias for ferrocp operations
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for ferrocp operations
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// I/O operation failed
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// File not found
    #[error("File not found: {path}")]
    FileNotFound { path: PathBuf },

    /// Directory not found
    #[error("Directory not found: {path}")]
    DirectoryNotFound { path: PathBuf },

    /// Permission denied
    #[error("Permission denied: {path}")]
    PermissionDenied { path: PathBuf },

    /// Compression error
    #[error("Compression error: {message}")]
    Compression { message: String },

    /// Network error
    #[error("Network error: {message}")]
    Network { message: String },

    /// Protocol error
    #[error("Protocol error: {message}")]
    Protocol { message: String },

    /// Configuration error
    #[error("Configuration error: {message}")]
    Config { message: String },

    /// Delta copy error
    #[error("Delta copy error: {message}")]
    DeltaCopy { message: String },

    /// Hash mismatch error
    #[error("Hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },

    /// Operation cancelled
    #[error("Operation cancelled")]
    Cancelled,

    /// Operation timed out
    #[error("Operation timed out after {seconds} seconds")]
    Timeout { seconds: u64 },

    /// Invalid path
    #[error("Invalid path: {path}")]
    InvalidPath { path: PathBuf },

    /// Insufficient disk space
    #[error("Insufficient disk space: need {needed} bytes, have {available} bytes")]
    InsufficientSpace { needed: u64, available: u64 },

    /// Device detection error
    #[error("Device detection error: {message}")]
    DeviceDetection { message: String },

    /// Generic error with custom message
    #[error("{message}")]
    Other { message: String },
}

impl Error {
    /// Create a new file not found error
    pub fn file_not_found<P: Into<PathBuf>>(path: P) -> Self {
        Self::FileNotFound { path: path.into() }
    }

    /// Create a new directory not found error
    pub fn directory_not_found<P: Into<PathBuf>>(path: P) -> Self {
        Self::DirectoryNotFound { path: path.into() }
    }

    /// Create a new permission denied error
    pub fn permission_denied<P: Into<PathBuf>>(path: P) -> Self {
        Self::PermissionDenied { path: path.into() }
    }

    /// Create a new compression error
    pub fn compression<S: Into<String>>(message: S) -> Self {
        Self::Compression {
            message: message.into(),
        }
    }

    /// Create a new network error
    pub fn network<S: Into<String>>(message: S) -> Self {
        Self::Network {
            message: message.into(),
        }
    }

    /// Create a new protocol error
    pub fn protocol<S: Into<String>>(message: S) -> Self {
        Self::Protocol {
            message: message.into(),
        }
    }

    /// Create a new configuration error
    pub fn config<S: Into<String>>(message: S) -> Self {
        Self::Config {
            message: message.into(),
        }
    }

    /// Create a new delta copy error
    pub fn delta_copy<S: Into<String>>(message: S) -> Self {
        Self::DeltaCopy {
            message: message.into(),
        }
    }

    /// Create a new hash mismatch error
    pub fn hash_mismatch<S: Into<String>>(expected: S, actual: S) -> Self {
        Self::HashMismatch {
            expected: expected.into(),
            actual: actual.into(),
        }
    }

    /// Create a new timeout error
    pub fn timeout(seconds: u64) -> Self {
        Self::Timeout { seconds }
    }

    /// Create a new invalid path error
    pub fn invalid_path<P: Into<PathBuf>>(path: P) -> Self {
        Self::InvalidPath { path: path.into() }
    }

    /// Create a new insufficient space error
    pub fn insufficient_space(needed: u64, available: u64) -> Self {
        Self::InsufficientSpace { needed, available }
    }

    /// Create a new device detection error
    pub fn device_detection<S: Into<String>>(message: S) -> Self {
        Self::DeviceDetection {
            message: message.into(),
        }
    }

    /// Create a new generic error
    pub fn other<S: Into<String>>(message: S) -> Self {
        Self::Other {
            message: message.into(),
        }
    }

    /// Check if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::Io(e) => match e.kind() {
                io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock => true,
                _ => false,
            },
            Self::Network { .. } | Self::Timeout { .. } => true,
            _ => false,
        }
    }

    /// Check if this error is a network-related error
    pub fn is_network_error(&self) -> bool {
        matches!(self, Self::Network { .. } | Self::Protocol { .. })
    }

    /// Check if this error is a file system error
    pub fn is_filesystem_error(&self) -> bool {
        matches!(
            self,
            Self::Io(_)
                | Self::FileNotFound { .. }
                | Self::DirectoryNotFound { .. }
                | Self::PermissionDenied { .. }
                | Self::InvalidPath { .. }
                | Self::InsufficientSpace { .. }
        )
    }
}

// Convert from common error types
// Note: zstd 0.13 doesn't expose these error types directly
// We'll handle zstd errors through the io::Error conversion

impl From<bincode::Error> for Error {
    fn from(err: bincode::Error) -> Self {
        Self::protocol(err.to_string())
    }
}

impl From<toml::de::Error> for Error {
    fn from(err: toml::de::Error) -> Self {
        Self::config(err.to_string())
    }
}

impl From<toml::ser::Error> for Error {
    fn from(err: toml::ser::Error) -> Self {
        Self::config(err.to_string())
    }
}
