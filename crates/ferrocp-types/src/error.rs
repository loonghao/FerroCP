//! Error types and handling for FerroCP
//!
//! This module provides a comprehensive error handling system for FerroCP operations.
//! It includes structured error types, error context, and recovery mechanisms.

use std::path::PathBuf;

// Serde is imported conditionally through cfg_attr

/// Error severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ErrorSeverity {
    /// Low severity - operation can continue
    Low,
    /// Medium severity - operation should be retried
    Medium,
    /// High severity - operation should be aborted
    High,
    /// Critical severity - entire process should be terminated
    Critical,
}

/// Error context providing additional information
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ErrorContext {
    /// Operation that was being performed
    pub operation: String,
    /// Additional context information
    pub details: std::collections::HashMap<String, String>,
    /// Timestamp when the error occurred
    #[cfg(feature = "std")]
    pub timestamp: std::time::SystemTime,
}

impl ErrorContext {
    /// Create a new error context
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            details: std::collections::HashMap::new(),
            #[cfg(feature = "std")]
            timestamp: std::time::SystemTime::now(),
        }
    }

    /// Add a detail to the context
    pub fn with_detail(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.details.insert(key.into(), value.into());
        self
    }
}

/// Main error type for FerroCP operations
#[derive(thiserror::Error, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Error {
    /// I/O operation failed
    #[error("I/O error: {message}")]
    Io {
        /// Error message from the I/O operation
        message: String
    },

    /// File not found
    #[error("File not found: {path}")]
    FileNotFound {
        /// Path to the file that was not found
        path: PathBuf
    },

    /// Permission denied
    #[error("Permission denied: {path}")]
    PermissionDenied {
        /// Path to the file with permission issues
        path: PathBuf
    },

    /// Configuration error
    #[error("Configuration error: {message}")]
    Config {
        /// Error message describing the configuration issue
        message: String
    },

    /// Network error
    #[error("Network error: {message}")]
    Network {
        /// Error message describing the network issue
        message: String
    },

    /// Compression error
    #[error("Compression error: {message}")]
    Compression {
        /// Error message describing the compression issue
        message: String
    },

    /// Device detection error
    #[error("Device detection error: {message}")]
    DeviceDetection {
        /// Error message describing the device detection issue
        message: String
    },

    /// Zero-copy operation failed
    #[error("Zero-copy operation failed: {message}")]
    ZeroCopy {
        /// Error message describing the zero-copy failure
        message: String
    },

    /// Synchronization error
    #[error("Synchronization error: {message}")]
    Sync {
        /// Error message describing the synchronization issue
        message: String
    },

    /// Operation cancelled
    #[error("Operation cancelled")]
    Cancelled,

    /// Operation timed out
    #[error("Operation timed out after {seconds} seconds")]
    Timeout {
        /// Number of seconds after which the operation timed out
        seconds: u64
    },

    /// Generic error with custom message
    #[error("{message}")]
    Other {
        /// Custom error message
        message: String
    },
}

/// Error kind for categorizing errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// I/O related errors
    Io,
    /// Configuration errors
    Config,
    /// Network errors
    Network,
    /// Compression errors
    Compression,
    /// Device detection errors
    DeviceDetection,
    /// Zero-copy errors
    ZeroCopy,
    /// Synchronization errors
    Sync,
    /// Cancellation
    Cancelled,
    /// Timeout
    Timeout,
    /// Other errors
    Other,
}

impl Error {
    /// Get the error kind
    pub fn kind(&self) -> ErrorKind {
        match self {
            Self::Io { .. } => ErrorKind::Io,
            Self::FileNotFound { .. } | Self::PermissionDenied { .. } => ErrorKind::Io,
            Self::Config { .. } => ErrorKind::Config,
            Self::Network { .. } => ErrorKind::Network,
            Self::Compression { .. } => ErrorKind::Compression,
            Self::DeviceDetection { .. } => ErrorKind::DeviceDetection,
            Self::ZeroCopy { .. } => ErrorKind::ZeroCopy,
            Self::Sync { .. } => ErrorKind::Sync,
            Self::Cancelled => ErrorKind::Cancelled,
            Self::Timeout { .. } => ErrorKind::Timeout,
            Self::Other { .. } => ErrorKind::Other,
        }
    }

    /// Get the error severity level
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Self::Io { .. } => ErrorSeverity::Medium,
            Self::FileNotFound { .. } | Self::PermissionDenied { .. } => ErrorSeverity::High,
            Self::Config { .. } => ErrorSeverity::High,
            Self::Network { .. } => ErrorSeverity::Medium,
            Self::Compression { .. } => ErrorSeverity::Low,
            Self::DeviceDetection { .. } => ErrorSeverity::Low,
            Self::ZeroCopy { .. } => ErrorSeverity::Low,
            Self::Sync { .. } => ErrorSeverity::Medium,
            Self::Cancelled => ErrorSeverity::Low,
            Self::Timeout { .. } => ErrorSeverity::Medium,
            Self::Other { .. } => ErrorSeverity::Medium,
        }
    }

    /// Check if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::Io { message } => {
                // Check if the error message indicates a recoverable condition
                message.contains("Interrupted")
                || message.contains("WouldBlock")
                || message.contains("TimedOut")
            },
            Self::Network { .. }
            | Self::Timeout { .. }
            | Self::Compression { .. }
            | Self::ZeroCopy { .. } => true,
            Self::Cancelled => false,
            Self::FileNotFound { .. }
            | Self::PermissionDenied { .. }
            | Self::Config { .. } => false,
            Self::DeviceDetection { .. }
            | Self::Sync { .. }
            | Self::Other { .. } => true,
        }
    }

    /// Check if this error should trigger a retry
    pub fn should_retry(&self) -> bool {
        self.is_recoverable() && self.severity() <= ErrorSeverity::Medium
    }

    /// Create a new configuration error
    pub fn config<S: Into<String>>(message: S) -> Self {
        Self::Config {
            message: message.into(),
        }
    }

    /// Create a new network error
    pub fn network<S: Into<String>>(message: S) -> Self {
        Self::Network {
            message: message.into(),
        }
    }

    /// Create a new compression error
    pub fn compression<S: Into<String>>(message: S) -> Self {
        Self::Compression {
            message: message.into(),
        }
    }

    /// Create a new device detection error
    pub fn device_detection<S: Into<String>>(message: S) -> Self {
        Self::DeviceDetection {
            message: message.into(),
        }
    }

    /// Create a new zero-copy error
    pub fn zero_copy<S: Into<String>>(message: S) -> Self {
        Self::ZeroCopy {
            message: message.into(),
        }
    }

    /// Create a new sync error
    pub fn sync<S: Into<String>>(message: S) -> Self {
        Self::Sync {
            message: message.into(),
        }
    }

    /// Create a new generic error
    pub fn other<S: Into<String>>(message: S) -> Self {
        Self::Other {
            message: message.into(),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io {
            message: error.to_string(),
        }
    }
}
