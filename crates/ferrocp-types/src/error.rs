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
        message: String,
    },

    /// File not found
    #[error("File not found: {path}")]
    FileNotFound {
        /// Path to the file that was not found
        path: PathBuf,
    },

    /// Permission denied
    #[error("Permission denied: {path}")]
    PermissionDenied {
        /// Path to the file with permission issues
        path: PathBuf,
    },

    /// Configuration error
    #[error("Configuration error: {message}")]
    Config {
        /// Error message describing the configuration issue
        message: String,
    },

    /// Network error
    #[error("Network error: {message}")]
    Network {
        /// Error message describing the network issue
        message: String,
    },

    /// Compression error
    #[error("Compression error: {message}")]
    Compression {
        /// Error message describing the compression issue
        message: String,
    },

    /// Device detection error
    #[error("Device detection error: {message}")]
    DeviceDetection {
        /// Error message describing the device detection issue
        message: String,
    },

    /// Zero-copy operation failed
    #[error("Zero-copy operation failed: {message}")]
    ZeroCopy {
        /// Error message describing the zero-copy failure
        message: String,
    },

    /// Synchronization error
    #[error("Synchronization error: {message}")]
    Sync {
        /// Error message describing the synchronization issue
        message: String,
    },

    /// Operation cancelled
    #[error("Operation cancelled")]
    Cancelled,

    /// Operation timed out
    #[error("Operation timed out after {seconds} seconds")]
    Timeout {
        /// Number of seconds after which the operation timed out
        seconds: u64,
    },

    /// Generic error with custom message
    #[error("{message}")]
    Other {
        /// Custom error message
        message: String,
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
            }
            Self::Network { .. }
            | Self::Timeout { .. }
            | Self::Compression { .. }
            | Self::ZeroCopy { .. } => true,
            Self::Cancelled => false,
            Self::FileNotFound { .. } | Self::PermissionDenied { .. } | Self::Config { .. } => {
                false
            }
            Self::DeviceDetection { .. } | Self::Sync { .. } | Self::Other { .. } => true,
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

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::path::PathBuf;

    // Property tests for Error severity mapping
    proptest! {
        #[test]
        fn test_error_severity_consistency(
            message in ".*"
        ) {
            // Test that all error variants have consistent severity mapping
            let errors = vec![
                Error::Io { message: message.clone() },
                Error::Config { message: message.clone() },
                Error::Network { message: message.clone() },
                Error::Compression { message: message.clone() },
                Error::DeviceDetection { message: message.clone() },
                Error::ZeroCopy { message: message.clone() },
                Error::Sync { message: message.clone() },
                Error::Other { message: message.clone() },
            ];

            for error in errors {
                let severity = error.severity();
                let kind = error.kind();

                // Verify severity is within valid range
                prop_assert!(matches!(severity,
                    ErrorSeverity::Low | ErrorSeverity::Medium |
                    ErrorSeverity::High | ErrorSeverity::Critical));

                // Verify kind matches error type
                match error {
                    Error::Io { .. } => prop_assert_eq!(kind, ErrorKind::Io),
                    Error::Config { .. } => prop_assert_eq!(kind, ErrorKind::Config),
                    Error::Network { .. } => prop_assert_eq!(kind, ErrorKind::Network),
                    Error::Compression { .. } => prop_assert_eq!(kind, ErrorKind::Compression),
                    Error::DeviceDetection { .. } => prop_assert_eq!(kind, ErrorKind::DeviceDetection),
                    Error::ZeroCopy { .. } => prop_assert_eq!(kind, ErrorKind::ZeroCopy),
                    Error::Sync { .. } => prop_assert_eq!(kind, ErrorKind::Sync),
                    Error::Other { .. } => prop_assert_eq!(kind, ErrorKind::Other),
                    _ => {}
                }
            }
        }

        #[test]
        fn test_error_recoverability_logic(
            message in ".*"
        ) {
            let error = Error::Io { message: message.clone() };
            let is_recoverable = error.is_recoverable();
            let should_retry = error.should_retry();

            // If an error should retry, it must be recoverable
            if should_retry {
                prop_assert!(is_recoverable);
            }

            // If an error should retry, its severity must be Medium or lower
            if should_retry {
                prop_assert!(error.severity() <= ErrorSeverity::Medium);
            }
        }

        #[test]
        fn test_timeout_error_properties(
            seconds in 1u64..3600u64
        ) {
            let error = Error::Timeout { seconds };

            prop_assert_eq!(error.kind(), ErrorKind::Timeout);
            prop_assert_eq!(error.severity(), ErrorSeverity::Medium);
            prop_assert!(error.is_recoverable());
            prop_assert!(error.should_retry());
        }
    }

    // Property tests for ErrorContext
    proptest! {
        #[test]
        fn test_error_context_creation(
            operation in ".*",
            key in ".*",
            value in ".*"
        ) {
            let context = ErrorContext::new(operation.clone())
                .with_detail(key.clone(), value.clone());

            prop_assert_eq!(context.operation, operation);
            prop_assert_eq!(context.details.get(&key), Some(&value));
        }
    }

    // Unit tests for specific error behaviors
    #[test]
    fn test_error_severity_ordering() {
        assert!(ErrorSeverity::Low < ErrorSeverity::Medium);
        assert!(ErrorSeverity::Medium < ErrorSeverity::High);
        assert!(ErrorSeverity::High < ErrorSeverity::Critical);
    }

    #[test]
    fn test_io_error_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "test file");
        let ferrocp_error = Error::from(io_error);

        assert_eq!(ferrocp_error.kind(), ErrorKind::Io);
        assert_eq!(ferrocp_error.severity(), ErrorSeverity::Medium);
        assert!(ferrocp_error.to_string().contains("test file"));
    }

    #[test]
    fn test_file_not_found_error() {
        let path = PathBuf::from("/nonexistent/file.txt");
        let error = Error::FileNotFound { path: path.clone() };

        assert_eq!(error.kind(), ErrorKind::Io);
        assert_eq!(error.severity(), ErrorSeverity::High);
        assert!(!error.is_recoverable());
        assert!(!error.should_retry());
        assert!(error.to_string().contains("/nonexistent/file.txt"));
    }

    #[test]
    fn test_permission_denied_error() {
        let path = PathBuf::from("/protected/file.txt");
        let error = Error::PermissionDenied { path: path.clone() };

        assert_eq!(error.kind(), ErrorKind::Io);
        assert_eq!(error.severity(), ErrorSeverity::High);
        assert!(!error.is_recoverable());
        assert!(!error.should_retry());
    }

    #[test]
    fn test_config_error() {
        let error = Error::config("invalid buffer size");

        assert_eq!(error.kind(), ErrorKind::Config);
        assert_eq!(error.severity(), ErrorSeverity::High);
        assert!(!error.is_recoverable());
        assert!(!error.should_retry());
    }

    #[test]
    fn test_network_error() {
        let error = Error::network("connection refused");

        assert_eq!(error.kind(), ErrorKind::Network);
        assert_eq!(error.severity(), ErrorSeverity::Medium);
        assert!(error.is_recoverable());
        assert!(error.should_retry());
    }

    #[test]
    fn test_compression_error() {
        let error = Error::compression("invalid compression level");

        assert_eq!(error.kind(), ErrorKind::Compression);
        assert_eq!(error.severity(), ErrorSeverity::Low);
        assert!(error.is_recoverable());
        assert!(error.should_retry());
    }

    #[test]
    fn test_cancelled_error() {
        let error = Error::Cancelled;

        assert_eq!(error.kind(), ErrorKind::Cancelled);
        assert_eq!(error.severity(), ErrorSeverity::Low);
        assert!(!error.is_recoverable());
        assert!(!error.should_retry());
    }

    #[test]
    fn test_io_error_recoverability() {
        // Test recoverable I/O errors
        let recoverable_errors = vec![
            Error::Io { message: "Interrupted system call".to_string() },
            Error::Io { message: "Operation would block".to_string() },
            Error::Io { message: "Connection timed out".to_string() },
        ];

        for error in recoverable_errors {
            assert!(error.is_recoverable());
            assert!(error.should_retry());
        }

        // Test non-recoverable I/O errors
        let non_recoverable_errors = vec![
            Error::Io { message: "No space left on device".to_string() },
            Error::Io { message: "Invalid argument".to_string() },
        ];

        for error in non_recoverable_errors {
            assert!(!error.is_recoverable());
            assert!(!error.should_retry());
        }
    }

    #[test]
    fn test_error_context_details() {
        let mut context = ErrorContext::new("file_copy");
        context = context
            .with_detail("source", "/path/to/source")
            .with_detail("destination", "/path/to/dest");

        assert_eq!(context.operation, "file_copy");
        assert_eq!(context.details.len(), 2);
        assert_eq!(context.details.get("source"), Some(&"/path/to/source".to_string()));
        assert_eq!(context.details.get("destination"), Some(&"/path/to/dest".to_string()));
    }
}
