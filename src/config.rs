use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::device_detector::{DeviceType, IOOptimizationConfig};

/// Error handling strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorStrategy {
    /// Raise exceptions immediately
    Raise,
    /// Retry the operation
    Retry,
    /// Ignore errors and continue
    Ignore,
}

impl Default for ErrorStrategy {
    fn default() -> Self {
        ErrorStrategy::Raise
    }
}

/// Log levels for EACopy operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    None,
    Error,
    Warning,
    Info,
    Debug,
}

impl Default for LogLevel {
    fn default() -> Self {
        LogLevel::Error
    }
}

/// Type for progress callback function
pub type ProgressCallback = Option<Arc<dyn Fn(u64, u64, &str) + Send + Sync>>;

/// Compression configuration
#[derive(Debug, Clone)]
pub struct CompressionConfig {
    /// Whether compression is enabled
    pub enabled: bool,
    /// Compression level (1-22 for zstd)
    pub level: i32,
    /// Whether to use adaptive compression based on network speed
    pub adaptive: bool,
    /// Minimum file size to compress (bytes)
    pub min_file_size: u64,
    /// Maximum compression buffer size
    pub buffer_size: usize,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            level: 3,
            adaptive: true,
            min_file_size: 1024, // 1KB minimum
            buffer_size: 64 * 1024, // 64KB buffer
        }
    }
}

/// Network configuration for EACopy service
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// Server bind address
    pub bind_address: String,
    /// Server port
    pub port: u16,
    /// Maximum number of concurrent connections
    pub max_connections: usize,
    /// Connection timeout in seconds
    pub connection_timeout: u64,
    /// Buffer size for network operations
    pub buffer_size: usize,
    /// Whether to enable compression for network transfers
    pub enable_compression: bool,
    /// Compression level (0-9)
    pub compression_level: u32,
    /// Cache directory for file deduplication
    pub cache_directory: Option<String>,
    /// Maximum cache size in bytes
    pub max_cache_size: u64,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0".to_string(),
            port: 31337,
            max_connections: 100,
            connection_timeout: 30,
            buffer_size: 64 * 1024, // 64KB
            enable_compression: true,
            compression_level: 3,
            cache_directory: None,
            max_cache_size: 10 * 1024 * 1024 * 1024, // 10GB
        }
    }
}

/// Configuration options for EACopy
#[derive(Clone)]
pub struct Config {
    /// Default number of threads to use for copy operations
    pub thread_count: usize,
    /// Default compression level (0-9) for network transfers
    pub compression_level: u32,
    /// Size of the buffer used for copy operations (in bytes)
    pub buffer_size: usize,
    /// How to handle errors during copy operations
    pub error_strategy: ErrorStrategy,
    /// Number of retries for failed operations when error_strategy is Retry
    pub retry_count: u32,
    /// Delay between retries in seconds
    pub retry_delay: f64,
    /// Verbosity level for logging
    pub log_level: LogLevel,
    /// Whether to preserve file metadata by default
    pub preserve_metadata: bool,
    /// Whether to follow symbolic links by default
    pub follow_symlinks: bool,
    /// Whether to allow existing directories by default
    pub dirs_exist_ok: bool,
    /// Function to call to report progress
    pub progress_callback: ProgressCallback,
    /// Advanced options
    pub extra_options: HashMap<String, String>,
    /// Whether to enable zero-copy operations
    pub zerocopy_enabled: bool,
    /// Minimum file size for zero-copy operations
    pub zerocopy_min_size: u64,
    /// Whether to automatically detect device types for optimization
    pub auto_detect_device: bool,
    /// I/O optimization configuration
    pub io_optimization: IOOptimizationConfig,
    /// Batch size for small file operations
    pub small_file_batch_size: usize,
    /// Network configuration
    pub network: NetworkConfig,
    /// Compression configuration
    pub compression: CompressionConfig,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            thread_count: num_cpus::get(),
            compression_level: 0,
            buffer_size: 8 * 1024 * 1024, // 8MB buffer
            error_strategy: ErrorStrategy::default(),
            retry_count: 3,
            retry_delay: 1.0,
            log_level: LogLevel::default(),
            preserve_metadata: true,
            follow_symlinks: false,
            dirs_exist_ok: false,
            progress_callback: None,
            extra_options: HashMap::new(),
            zerocopy_enabled: true,
            zerocopy_min_size: 64 * 1024, // 64KB minimum for zero-copy
            auto_detect_device: true,
            io_optimization: IOOptimizationConfig::default(),
            small_file_batch_size: 100,
            network: NetworkConfig::default(),
            compression: CompressionConfig::default(),
        }
    }
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("thread_count", &self.thread_count)
            .field("compression_level", &self.compression_level)
            .field("buffer_size", &self.buffer_size)
            .field("error_strategy", &self.error_strategy)
            .field("retry_count", &self.retry_count)
            .field("retry_delay", &self.retry_delay)
            .field("log_level", &self.log_level)
            .field("preserve_metadata", &self.preserve_metadata)
            .field("follow_symlinks", &self.follow_symlinks)
            .field("dirs_exist_ok", &self.dirs_exist_ok)
            .field("progress_callback", &self.progress_callback.is_some())
            .field("extra_options", &self.extra_options)
            .field("zerocopy_enabled", &self.zerocopy_enabled)
            .field("zerocopy_min_size", &self.zerocopy_min_size)
            .field("auto_detect_device", &self.auto_detect_device)
            .field("io_optimization", &self.io_optimization)
            .field("small_file_batch_size", &self.small_file_batch_size)
            .field("network", &self.network)
            .field("compression", &self.compression)
            .finish()
    }
}

impl Config {
    /// Create a new configuration with default values
    pub fn new() -> Self {
        Config::default()
    }

    /// Set the number of threads to use for copy operations
    pub fn with_thread_count(mut self, thread_count: usize) -> Self {
        self.thread_count = thread_count;
        self
    }

    /// Set the compression level (0-9) for network transfers
    pub fn with_compression_level(mut self, compression_level: u32) -> Self {
        self.compression_level = compression_level;
        self
    }

    /// Set the buffer size for copy operations
    pub fn with_buffer_size(mut self, buffer_size: usize) -> Self {
        self.buffer_size = buffer_size;
        self
    }

    /// Set the error handling strategy
    pub fn with_error_strategy(mut self, error_strategy: ErrorStrategy) -> Self {
        self.error_strategy = error_strategy;
        self
    }

    /// Set the number of retries for failed operations
    pub fn with_retry_count(mut self, retry_count: u32) -> Self {
        self.retry_count = retry_count;
        self
    }

    /// Set the delay between retries in seconds
    pub fn with_retry_delay(mut self, retry_delay: f64) -> Self {
        self.retry_delay = retry_delay;
        self
    }

    /// Set the verbosity level for logging
    pub fn with_log_level(mut self, log_level: LogLevel) -> Self {
        self.log_level = log_level;
        self
    }

    /// Set whether to preserve file metadata
    pub fn with_preserve_metadata(mut self, preserve_metadata: bool) -> Self {
        self.preserve_metadata = preserve_metadata;
        self
    }

    /// Set whether to follow symbolic links
    pub fn with_follow_symlinks(mut self, follow_symlinks: bool) -> Self {
        self.follow_symlinks = follow_symlinks;
        self
    }

    /// Set whether to allow existing directories
    pub fn with_dirs_exist_ok(mut self, dirs_exist_ok: bool) -> Self {
        self.dirs_exist_ok = dirs_exist_ok;
        self
    }

    /// Set the progress callback function
    pub fn with_progress_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(u64, u64, &str) + Send + Sync + 'static,
    {
        self.progress_callback = Some(Arc::new(callback));
        self
    }

    /// Set whether to enable zero-copy operations
    pub fn with_zerocopy_enabled(mut self, enabled: bool) -> Self {
        self.zerocopy_enabled = enabled;
        self
    }

    /// Set minimum file size for zero-copy operations
    pub fn with_zerocopy_min_size(mut self, min_size: u64) -> Self {
        self.zerocopy_min_size = min_size;
        self
    }

    /// Set whether to auto-detect device types
    pub fn with_auto_detect_device(mut self, enabled: bool) -> Self {
        self.auto_detect_device = enabled;
        self
    }

    /// Set small file batch size
    pub fn with_small_file_batch_size(mut self, batch_size: usize) -> Self {
        self.small_file_batch_size = batch_size;
        self
    }

    /// Set network configuration
    pub fn with_network_config(mut self, network: NetworkConfig) -> Self {
        self.network = network;
        self
    }

    /// Set compression configuration
    pub fn with_compression_config(mut self, compression: CompressionConfig) -> Self {
        self.compression = compression;
        self
    }
}

/// Global configuration instance
lazy_static::lazy_static! {
    pub static ref GLOBAL_CONFIG: Arc<Mutex<Config>> = Arc::new(Mutex::new(Config::default()));
}

/// Get a reference to the global configuration
pub fn global_config() -> Arc<Mutex<Config>> {
    GLOBAL_CONFIG.clone()
}

/// Set the global configuration
pub fn set_global_config(config: Config) {
    let mut global = GLOBAL_CONFIG.lock().unwrap();
    *global = config;
}

/// Reset the global configuration to defaults
pub fn reset_global_config() {
    let mut global = GLOBAL_CONFIG.lock().unwrap();
    *global = Config::default();
}
