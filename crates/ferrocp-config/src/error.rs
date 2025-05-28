//! Error types for configuration management

use ferrocp_types::Error as FerrocpError;
use std::path::PathBuf;
use thiserror::Error;

/// Configuration error type
#[derive(Error, Debug)]
pub enum ConfigError {
    /// I/O error when reading configuration file
    #[error("I/O error reading config file '{path}': {source}")]
    Io {
        /// Path to the configuration file
        path: PathBuf,
        /// Underlying I/O error
        source: std::io::Error,
    },

    /// Configuration file parsing error
    #[error("Failed to parse config file '{path}': {message}")]
    Parse {
        /// Path to the configuration file
        path: PathBuf,
        /// Error message
        message: String,
    },

    /// Configuration validation error
    #[error("Configuration validation failed: {message}")]
    Validation {
        /// Validation error message
        message: String,
    },

    /// Missing required configuration
    #[error("Missing required configuration: {key}")]
    MissingRequired {
        /// Configuration key that is missing
        key: String,
    },

    /// Invalid configuration value
    #[error("Invalid value for '{key}': {message}")]
    InvalidValue {
        /// Configuration key
        key: String,
        /// Error message
        message: String,
    },

    /// Environment variable error
    #[error("Environment variable error: {message}")]
    Environment {
        /// Error message
        message: String,
    },

    /// File watcher error
    #[error("File watcher error: {message}")]
    Watcher {
        /// Error message
        message: String,
    },

    /// Serialization error
    #[error("Serialization error: {message}")]
    Serialization {
        /// Error message
        message: String,
    },

    /// Generic configuration error
    #[error("Configuration error: {message}")]
    Other {
        /// Error message
        message: String,
    },
}

impl From<std::io::Error> for ConfigError {
    fn from(error: std::io::Error) -> Self {
        Self::Other {
            message: error.to_string(),
        }
    }
}

impl From<serde_yaml::Error> for ConfigError {
    fn from(error: serde_yaml::Error) -> Self {
        Self::Serialization {
            message: error.to_string(),
        }
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(error: toml::de::Error) -> Self {
        Self::Serialization {
            message: error.to_string(),
        }
    }
}

impl From<config::ConfigError> for ConfigError {
    fn from(error: config::ConfigError) -> Self {
        Self::Other {
            message: error.to_string(),
        }
    }
}

impl From<ConfigError> for FerrocpError {
    fn from(error: ConfigError) -> Self {
        FerrocpError::config(error.to_string())
    }
}

/// Result type for configuration operations
pub type ConfigResult<T> = Result<T, ConfigError>;

impl ConfigError {
    /// Create a new validation error
    pub fn validation<S: Into<String>>(message: S) -> Self {
        Self::Validation {
            message: message.into(),
        }
    }

    /// Create a new missing required error
    pub fn missing_required<S: Into<String>>(key: S) -> Self {
        Self::MissingRequired { key: key.into() }
    }

    /// Create a new invalid value error
    pub fn invalid_value<S: Into<String>>(key: S, message: S) -> Self {
        Self::InvalidValue {
            key: key.into(),
            message: message.into(),
        }
    }

    /// Create a new environment error
    pub fn environment<S: Into<String>>(message: S) -> Self {
        Self::Environment {
            message: message.into(),
        }
    }

    /// Create a new watcher error
    pub fn watcher<S: Into<String>>(message: S) -> Self {
        Self::Watcher {
            message: message.into(),
        }
    }

    /// Create a new other error
    pub fn other<S: Into<String>>(message: S) -> Self {
        Self::Other {
            message: message.into(),
        }
    }
}
