//! Configuration management system for FerroCP
//!
//! This crate provides a comprehensive configuration management system for FerroCP,
//! supporting multiple configuration formats (YAML, TOML), validation, hot-reload,
//! and environment variable overrides.
//!
//! # Features
//!
//! - **Multiple formats**: Support for YAML and TOML configuration files
//! - **Validation**: Type-safe configuration with comprehensive validation
//! - **Hot-reload**: Watch configuration files for changes and reload automatically
//! - **Environment overrides**: Override configuration values with environment variables
//! - **Defaults**: Sensible default values for all configuration options
//!
//! # Examples
//!
//! ```rust
//! use ferrocp_config::{Config, ConfigBuilder};
//!
//! // Load configuration from file
//! let config = ConfigBuilder::new()
//!     .add_source_file("ferrocp.yaml")
//!     .add_env_prefix("FERROCP")
//!     .build()
//!     .expect("Failed to load configuration");
//!
//! println!("Buffer size: {}", config.performance.buffer_size.get());
//! ```

#![deny(missing_docs)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::module_name_repetitions)]

use ferrocp_types::{
    BufferSize, CompressionAlgorithm, CompressionLevel, NetworkProtocol, RetryConfig, ThreadCount,
    TimeoutConfig,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

pub mod builder;
pub mod error;
pub mod loader;
pub mod watcher;

pub use builder::ConfigBuilder;
pub use error::{ConfigError, ConfigResult};
pub use loader::ConfigLoader;
pub use watcher::ConfigWatcher;

/// Main configuration structure for FerroCP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Performance-related configuration
    pub performance: PerformanceConfig,
    /// Compression configuration
    pub compression: CompressionConfig,
    /// Network configuration
    pub network: NetworkConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Security configuration
    pub security: SecurityConfig,
    /// Advanced features configuration
    pub features: FeaturesConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            performance: PerformanceConfig::default(),
            compression: CompressionConfig::default(),
            network: NetworkConfig::default(),
            logging: LoggingConfig::default(),
            security: SecurityConfig::default(),
            features: FeaturesConfig::default(),
        }
    }
}

/// Performance-related configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Buffer size for I/O operations
    pub buffer_size: BufferSize,
    /// Number of worker threads
    pub thread_count: ThreadCount,
    /// Enable zero-copy operations
    pub enable_zero_copy: bool,
    /// Enable memory mapping for large files
    pub enable_memory_mapping: bool,
    /// Threshold for using memory mapping (in bytes)
    pub memory_mapping_threshold: u64,
    /// Enable direct I/O
    pub enable_direct_io: bool,
    /// I/O queue depth for async operations
    pub io_queue_depth: u32,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            buffer_size: BufferSize::default(),
            thread_count: ThreadCount::default(),
            enable_zero_copy: true,
            enable_memory_mapping: true,
            memory_mapping_threshold: 64 * 1024 * 1024, // 64MB
            enable_direct_io: false,
            io_queue_depth: 32,
        }
    }
}

/// Compression configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    /// Default compression algorithm
    pub algorithm: CompressionAlgorithm,
    /// Default compression level
    pub level: CompressionLevel,
    /// Enable adaptive compression based on file type
    pub adaptive: bool,
    /// Minimum file size to consider for compression
    pub min_file_size: u64,
    /// File extensions to always compress
    pub always_compress: Vec<String>,
    /// File extensions to never compress
    pub never_compress: Vec<String>,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            algorithm: CompressionAlgorithm::Zstd,
            level: CompressionLevel::default(),
            adaptive: true,
            min_file_size: 1024, // 1KB
            always_compress: vec![
                "txt".to_string(),
                "log".to_string(),
                "csv".to_string(),
                "json".to_string(),
                "xml".to_string(),
            ],
            never_compress: vec![
                "zip".to_string(),
                "gz".to_string(),
                "7z".to_string(),
                "rar".to_string(),
                "jpg".to_string(),
                "png".to_string(),
                "mp4".to_string(),
                "mp3".to_string(),
            ],
        }
    }
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Default network protocol
    pub protocol: NetworkProtocol,
    /// Connection timeout configuration
    pub timeouts: TimeoutConfig,
    /// Retry configuration for network operations
    pub retry: RetryConfig,
    /// Maximum concurrent connections
    pub max_connections: u32,
    /// Enable connection pooling
    pub enable_connection_pooling: bool,
    /// Connection pool size
    pub connection_pool_size: u32,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            protocol: NetworkProtocol::Quic,
            timeouts: TimeoutConfig::default(),
            retry: RetryConfig::default(),
            max_connections: 10,
            enable_connection_pooling: true,
            connection_pool_size: 5,
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    pub level: String,
    /// Enable file logging
    pub enable_file_logging: bool,
    /// Log file path
    pub log_file: Option<PathBuf>,
    /// Maximum log file size before rotation
    pub max_file_size: u64,
    /// Number of log files to keep
    pub max_files: u32,
    /// Enable JSON formatting
    pub json_format: bool,
    /// Enable colored output
    pub colored_output: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            enable_file_logging: false,
            log_file: None,
            max_file_size: 10 * 1024 * 1024, // 10MB
            max_files: 5,
            json_format: false,
            colored_output: true,
        }
    }
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable permission preservation
    #[serde(default = "default_preserve_permissions")]
    pub preserve_permissions: bool,
    /// Enable ownership preservation
    #[serde(default = "default_preserve_ownership")]
    pub preserve_ownership: bool,
    /// Enable extended attributes preservation
    #[serde(default = "default_preserve_extended_attributes")]
    pub preserve_extended_attributes: bool,
    /// Enable ACL preservation
    #[serde(default = "default_preserve_acl")]
    pub preserve_acl: bool,
    /// Maximum path length allowed
    #[serde(default = "default_max_path_length")]
    pub max_path_length: usize,
    /// Allowed file extensions (empty = all allowed)
    #[serde(default)]
    pub allowed_extensions: Vec<String>,
    /// Blocked file extensions
    #[serde(default = "default_blocked_extensions")]
    pub blocked_extensions: Vec<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            preserve_permissions: true,
            preserve_ownership: false, // Requires elevated privileges
            preserve_extended_attributes: true,
            preserve_acl: false, // Requires elevated privileges
            max_path_length: 4096,
            allowed_extensions: Vec::new(),
            blocked_extensions: vec![
                "exe".to_string(),
                "bat".to_string(),
                "cmd".to_string(),
                "scr".to_string(),
                "com".to_string(),
                "pif".to_string(),
            ],
        }
    }
}

// Default value functions for SecurityConfig serde defaults
fn default_preserve_permissions() -> bool {
    true
}

fn default_preserve_ownership() -> bool {
    false
}

fn default_preserve_extended_attributes() -> bool {
    true
}

fn default_preserve_acl() -> bool {
    false
}

fn default_max_path_length() -> usize {
    4096
}

fn default_blocked_extensions() -> Vec<String> {
    vec![
        "exe".to_string(),
        "bat".to_string(),
        "cmd".to_string(),
        "scr".to_string(),
        "com".to_string(),
        "pif".to_string(),
    ]
}

/// Advanced features configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeaturesConfig {
    /// Enable file deduplication
    pub enable_deduplication: bool,
    /// Enable delta sync
    pub enable_delta_sync: bool,
    /// Enable file verification
    pub enable_verification: bool,
    /// Verification algorithm
    pub verification_algorithm: String,
    /// Enable progress reporting
    pub enable_progress_reporting: bool,
    /// Progress reporting interval
    pub progress_interval: Duration,
    /// Enable statistics collection
    pub enable_statistics: bool,
    /// Enable profiling
    pub enable_profiling: bool,
}

impl Default for FeaturesConfig {
    fn default() -> Self {
        Self {
            enable_deduplication: false,
            enable_delta_sync: true,
            enable_verification: true,
            verification_algorithm: "blake3".to_string(),
            enable_progress_reporting: true,
            progress_interval: Duration::from_millis(100),
            enable_statistics: true,
            enable_profiling: false,
        }
    }
}
