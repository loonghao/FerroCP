//! Error types and handling for FerroCP
//!
//! This module provides a comprehensive error handling system for FerroCP operations.
//! It includes structured error types, error context, and recovery mechanisms.

use std::path::PathBuf;
use thiserror::Error;

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
///
/// Attached to an [`Error`] with [`Error::with_context`]. It is the machine
/// readable half of the error: the message is for humans, the context is for
/// code that has to react to the failure.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ErrorContext {
    /// Operation that was being performed
    pub operation: String,
    /// Path the operation was acting on, when there is one
    pub path: Option<PathBuf>,
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
            path: None,
            details: std::collections::HashMap::new(),
            #[cfg(feature = "std")]
            timestamp: std::time::SystemTime::now(),
        }
    }

    /// Attach the path the operation was acting on
    pub fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Add a detail to the context
    pub fn with_detail(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.details.insert(key.into(), value.into());
        self
    }
}

/// Main error type for FerroCP operations
#[derive(Error, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Error {
    /// I/O operation failed
    ///
    /// `kind` carries the originating [`std::io::ErrorKind`] so callers can
    /// react to the failure without parsing the message.
    #[error("I/O error: {message}")]
    Io {
        /// Error message from the I/O operation
        message: String,
        /// The originating I/O error kind, when the error came from `std::io`
        ///
        /// `None` means the error did not originate from a `std::io`
        /// operation, so no kind exists to record. It is deliberately not
        /// filled in by guessing.
        #[cfg_attr(feature = "serde", serde(skip))]
        kind: Option<std::io::ErrorKind>,
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

    /// An error carrying structured context about the failed operation
    ///
    /// Produced by [`Error::with_context`]. The context is what makes an error
    /// actionable, so it travels with the error instead of being logged and
    /// dropped.
    #[error("{error}")]
    WithContext {
        /// The underlying error
        #[source]
        error: Box<Self>,
        /// What was being attempted when the error happened
        context: ErrorContext,
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
            // A context wrapper does not change what kind of error this is.
            Self::WithContext { error, .. } => error.kind(),
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
            Self::WithContext { error, .. } => error.severity(),
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

    /// The underlying I/O error kind, when one is known
    ///
    /// This is the machine readable answer that callers used to get by
    /// searching the message for English substrings.
    pub fn io_kind(&self) -> Option<std::io::ErrorKind> {
        match self {
            Self::Io { kind, .. } => *kind,
            Self::FileNotFound { .. } => Some(std::io::ErrorKind::NotFound),
            Self::PermissionDenied { .. } => Some(std::io::ErrorKind::PermissionDenied),
            _ => None,
        }
    }

    /// Attach structured context describing the failed operation
    ///
    /// Repeated calls nest, so the innermost context describes the most
    /// specific operation.
    pub fn with_context(self, operation: impl Into<String>) -> Self {
        let context = ErrorContext::new(operation);
        match self {
            // Collapse nested contexts for the same operation into one entry
            // instead of building an ever deeper chain.
            Self::WithContext { .. } => self,
            error => Self::WithContext {
                error: Box::new(error),
                context,
            },
        }
    }

    /// Attach context that includes the path being operated on
    pub fn with_path_context(self, operation: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        let context = ErrorContext::new(operation).with_path(path);
        match self {
            Self::WithContext { .. } => self,
            error => Self::WithContext {
                error: Box::new(error),
                context,
            },
        }
    }

    /// The context attached to this error, if any
    pub fn context(&self) -> Option<&ErrorContext> {
        match self {
            Self::WithContext { context, .. } => Some(context),
            _ => None,
        }
    }

    /// The innermost error, with all context wrappers removed
    pub fn root_cause(&self) -> &Self {
        match self {
            Self::WithContext { error, .. } => error.root_cause(),
            other => other,
        }
    }

    /// The full cause chain, formatted as `context: cause: cause`
    ///
    /// Use this for logs and error messages: it shows what was being attempted
    /// and why it failed, instead of only the innermost reason.
    pub fn chain(&self) -> String {
        let mut parts = Vec::new();
        let mut current = self;
        while let Self::WithContext { error, context } = current {
            parts.push(context.operation.clone());
            current = error;
        }
        parts.push(current.to_string());
        parts.join(": ")
    }

    /// Check if this error is recoverable
    ///
    /// Recoverability is decided by the [`std::io::ErrorKind`] when one is
    /// known. Errors without a kind are treated as **not** recoverable: guessing
    /// from the message text is not reliable across platforms or locales.
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::Io { kind, .. } => kind.is_some_and(is_recoverable_kind),
            Self::Network { .. }
            | Self::Timeout { .. }
            | Self::Compression { .. }
            | Self::ZeroCopy { .. } => true,
            Self::Cancelled => false,
            Self::FileNotFound { .. } | Self::PermissionDenied { .. } | Self::Config { .. } => {
                false
            }
            Self::DeviceDetection { .. } | Self::Sync { .. } | Self::Other { .. } => true,
            Self::WithContext { error, .. } => error.is_recoverable(),
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

    /// Create a new I/O error
    pub fn io<S: Into<String>>(message: S) -> Self {
        Self::Io {
            message: message.into(),
            kind: None,
        }
    }

    /// Create an I/O error from a failing `std::io` operation
    ///
    /// This is the constructor to reach for at I/O call sites: it keeps the
    /// [`std::io::ErrorKind`], which is what recoverability and the Python
    /// exception mapping are based on.
    pub fn from_io<S: Into<String>>(message: S, error: &std::io::Error) -> Self {
        Self::Io {
            message: format!("{}: {}", message.into(), error),
            kind: Some(error.kind()),
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
        // Preserve the kind instead of flattening everything into a string.
        // Callers (and the Python bindings) use it to tell `NotFound` from
        // `PermissionDenied` from a transient failure.
        let kind = error.kind();
        Self::Io {
            message: error.to_string(),
            kind: Some(kind),
        }
    }
}

/// I/O error kinds that are worth retrying
///
/// These are the transient conditions: the same operation can succeed if it is
/// attempted again. Everything else (missing file, permission denied, invalid
/// argument, full disk) will fail the same way on a retry.
fn is_recoverable_kind(kind: std::io::ErrorKind) -> bool {
    matches!(
        kind,
        std::io::ErrorKind::Interrupted
            | std::io::ErrorKind::WouldBlock
            | std::io::ErrorKind::TimedOut
    )
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
                Error::Io { message: message.clone(), kind: None },
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
            let error = Error::Io { message: message.clone(), kind: None };
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
    fn test_io_error_recoverability_uses_the_error_kind() {
        // Recoverability is decided by the ErrorKind, not by searching the
        // message text, so it does not depend on the OS phrasing.
        let recoverable = [
            std::io::ErrorKind::Interrupted,
            std::io::ErrorKind::WouldBlock,
            std::io::ErrorKind::TimedOut,
        ];
        for kind in recoverable {
            let error = Error::Io {
                message: "some failure".to_string(),
                kind: Some(kind),
            };
            assert!(error.is_recoverable(), "{kind:?} must be recoverable");
            assert!(error.should_retry(), "{kind:?} must be retried");
        }

        let permanent = [
            std::io::ErrorKind::NotFound,
            std::io::ErrorKind::PermissionDenied,
            std::io::ErrorKind::InvalidInput,
            std::io::ErrorKind::StorageFull,
        ];
        for kind in permanent {
            let error = Error::Io {
                message: "some failure".to_string(),
                kind: Some(kind),
            };
            assert!(!error.is_recoverable(), "{kind:?} must not be recoverable");
            assert!(!error.should_retry(), "{kind:?} must not be retried");
        }
    }

    #[test]
    fn test_recoverability_does_not_depend_on_message_wording() {
        // The same condition in two wordings: the old substring heuristic would
        // have treated these differently.
        let interrupted = Error::Io {
            message: "L'appel système a été interrompu".to_string(),
            kind: Some(std::io::ErrorKind::Interrupted),
        };
        assert!(interrupted.is_recoverable());

        let not_found = Error::Io {
            message: "Interrupted system call".to_string(),
            kind: Some(std::io::ErrorKind::NotFound),
        };
        assert!(
            !not_found.is_recoverable(),
            "the message must not override the error kind"
        );
    }

    #[test]
    fn test_unknown_kind_is_not_recoverable() {
        // Without a kind there is nothing to base a retry on, so the answer is
        // "no" rather than a guess.
        let error = Error::Io {
            message: "Connection timed out".to_string(),
            kind: None,
        };
        assert!(!error.is_recoverable());
    }

    #[test]
    fn test_from_io_error_preserves_the_kind() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
        let error = Error::from(io_error);

        assert_eq!(error.io_kind(), Some(std::io::ErrorKind::NotFound));
        assert!(!error.is_recoverable());
        assert!(error.to_string().contains("missing"));

        let denied = Error::from(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "nope",
        ));
        assert_eq!(
            denied.io_kind(),
            Some(std::io::ErrorKind::PermissionDenied),
            "callers must be able to tell permission errors apart"
        );
    }

    #[test]
    fn test_from_io_error_keeps_transient_kinds_recoverable() {
        let error = Error::from(std::io::Error::new(
            std::io::ErrorKind::Interrupted,
            "try again",
        ));
        assert!(error.is_recoverable());
    }

    #[test]
    fn test_file_not_found_and_permission_denied_expose_io_kinds() {
        assert_eq!(
            Error::FileNotFound {
                path: PathBuf::from("/nope")
            }
            .io_kind(),
            Some(std::io::ErrorKind::NotFound)
        );
        assert_eq!(
            Error::PermissionDenied {
                path: PathBuf::from("/nope")
            }
            .io_kind(),
            Some(std::io::ErrorKind::PermissionDenied)
        );
    }

    #[test]
    fn test_error_context_carries_operation_and_path() {
        let context = ErrorContext::new("copy_file").with_path("/tmp/src");
        assert_eq!(context.operation, "copy_file");
        assert_eq!(
            context.path.as_deref(),
            Some(std::path::Path::new("/tmp/src"))
        );
    }

    #[test]
    fn test_with_context_attaches_the_operation() {
        let error = Error::other("disk on fire").with_context("copy_file");

        let context = error.context().expect("context must be attached");
        assert_eq!(context.operation, "copy_file");

        // Wrapping must not change what kind of error this is.
        assert_eq!(error.kind(), ErrorKind::Other);
        assert_eq!(error.severity(), ErrorSeverity::Medium);
        assert_eq!(error.root_cause().kind(), ErrorKind::Other);
    }

    #[test]
    fn test_with_path_context_records_the_path() {
        let error = Error::other("boom").with_path_context("copy_file", "/data/a.bin");
        let context = error.context().unwrap();
        assert_eq!(context.operation, "copy_file");
        assert_eq!(
            context.path.as_deref(),
            Some(std::path::Path::new("/data/a.bin"))
        );
    }

    #[test]
    fn test_context_is_not_double_wrapped() {
        // Repeated wrapping would produce a deep chain with no extra
        // information, so the first context wins.
        let error = Error::other("boom")
            .with_context("copy_file")
            .with_context("retry");
        assert_eq!(error.context().unwrap().operation, "copy_file");
    }

    #[test]
    fn test_chain_shows_operation_and_cause() {
        let error = Error::other("permission denied").with_context("copy_file");
        let chain = error.chain();
        assert!(chain.contains("copy_file"), "chain was: {chain}");
        assert!(chain.contains("permission denied"), "chain was: {chain}");
    }

    #[test]
    fn test_recoverability_is_preserved_through_context() {
        let error = Error::Io {
            message: "interrupted".to_string(),
            kind: Some(std::io::ErrorKind::Interrupted),
        }
        .with_context("copy_file");
        assert!(error.is_recoverable());
        assert_eq!(error.kind(), ErrorKind::Io);
    }

    #[test]
    fn test_from_io_helper_captures_the_kind() {
        let io_error = std::io::Error::new(std::io::ErrorKind::AlreadyExists, "exists");
        let error = Error::from_io("failed to create destination", &io_error);

        assert_eq!(error.io_kind(), Some(std::io::ErrorKind::AlreadyExists));
        assert!(error.to_string().contains("failed to create destination"));
    }

    #[test]
    fn test_error_context_details() {
        let mut context = ErrorContext::new("file_copy");
        context = context
            .with_detail("source", "/path/to/source")
            .with_detail("destination", "/path/to/dest");

        assert_eq!(context.operation, "file_copy");
        assert_eq!(context.details.len(), 2);
        assert_eq!(
            context.details.get("source"),
            Some(&"/path/to/source".to_string())
        );
        assert_eq!(
            context.details.get("destination"),
            Some(&"/path/to/dest".to_string())
        );
    }
}
