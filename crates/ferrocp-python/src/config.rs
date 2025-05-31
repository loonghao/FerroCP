//! Configuration for Python bindings

use ferrocp_types::{CopyMode, NetworkProtocol};
use pyo3::prelude::*;
use std::collections::HashMap;

/// Python wrapper for copy options
#[pyclass(name = "CopyOptions")]
#[derive(Debug, Clone)]
pub struct PyCopyOptions {
    /// Copy mode
    #[pyo3(get, set)]
    pub mode: String,
    /// Overwrite mode
    #[pyo3(get, set)]
    pub overwrite: String,
    /// Whether to preserve timestamps
    #[pyo3(get, set)]
    pub preserve_timestamps: bool,
    /// Whether to preserve permissions
    #[pyo3(get, set)]
    pub preserve_permissions: bool,
    /// Whether to follow symbolic links
    #[pyo3(get, set)]
    pub follow_symlinks: bool,
    /// Whether to enable compression
    #[pyo3(get, set)]
    pub enable_compression: bool,
    /// Compression level (0-9)
    #[pyo3(get, set)]
    pub compression_level: u8,
    /// Buffer size in bytes
    #[pyo3(get, set)]
    pub buffer_size: usize,
    /// Number of worker threads
    #[pyo3(get, set)]
    pub num_threads: usize,
    /// Whether to verify copied files
    #[pyo3(get, set)]
    pub verify: bool,
}

#[pymethods]
impl PyCopyOptions {
    /// Create new copy options with defaults
    #[new]
    #[pyo3(signature = (
        mode = "auto".to_string(),
        overwrite = "prompt".to_string(),
        preserve_timestamps = true,
        preserve_permissions = true,
        follow_symlinks = false,
        enable_compression = false,
        compression_level = 6,
        buffer_size = 64 * 1024,
        num_threads = 0,
        verify = false
    ))]
    pub fn new(
        mode: String,
        overwrite: String,
        preserve_timestamps: bool,
        preserve_permissions: bool,
        follow_symlinks: bool,
        enable_compression: bool,
        compression_level: u8,
        buffer_size: usize,
        num_threads: usize,
        verify: bool,
    ) -> Self {
        Self {
            mode,
            overwrite,
            preserve_timestamps,
            preserve_permissions,
            follow_symlinks,
            enable_compression,
            compression_level,
            buffer_size,
            num_threads,
            verify,
        }
    }

    /// Create options optimized for speed
    #[staticmethod]
    pub fn for_speed() -> Self {
        Self {
            mode: "fast".to_string(),
            overwrite: "overwrite".to_string(),
            preserve_timestamps: false,
            preserve_permissions: false,
            follow_symlinks: false,
            enable_compression: false,
            compression_level: 1,
            buffer_size: 1024 * 1024, // 1MB
            num_threads: 0,           // Auto-detect
            verify: false,
        }
    }

    /// Create options optimized for safety
    #[staticmethod]
    pub fn for_safety() -> Self {
        Self {
            mode: "safe".to_string(),
            overwrite: "prompt".to_string(),
            preserve_timestamps: true,
            preserve_permissions: true,
            follow_symlinks: false,
            enable_compression: false,
            compression_level: 6,
            buffer_size: 64 * 1024, // 64KB
            num_threads: 1,
            verify: true,
        }
    }

    /// Create options optimized for compression
    #[staticmethod]
    pub fn for_compression() -> Self {
        Self {
            mode: "auto".to_string(),
            overwrite: "overwrite".to_string(),
            preserve_timestamps: true,
            preserve_permissions: true,
            follow_symlinks: false,
            enable_compression: true,
            compression_level: 6,
            buffer_size: 256 * 1024, // 256KB
            num_threads: 0,          // Auto-detect
            verify: false,
        }
    }

    /// Convert to dictionary
    pub fn to_dict(&self) -> HashMap<String, PyObject> {
        Python::with_gil(|py| {
            let mut dict = HashMap::new();
            dict.insert("mode".to_string(), self.mode.to_object(py));
            dict.insert("overwrite".to_string(), self.overwrite.to_object(py));
            dict.insert(
                "preserve_timestamps".to_string(),
                self.preserve_timestamps.to_object(py),
            );
            dict.insert(
                "preserve_permissions".to_string(),
                self.preserve_permissions.to_object(py),
            );
            dict.insert(
                "follow_symlinks".to_string(),
                self.follow_symlinks.to_object(py),
            );
            dict.insert(
                "enable_compression".to_string(),
                self.enable_compression.to_object(py),
            );
            dict.insert(
                "compression_level".to_string(),
                self.compression_level.to_object(py),
            );
            dict.insert("buffer_size".to_string(), self.buffer_size.to_object(py));
            dict.insert("num_threads".to_string(), self.num_threads.to_object(py));
            dict.insert("verify".to_string(), self.verify.to_object(py));
            dict
        })
    }

    /// String representation
    fn __str__(&self) -> String {
        format!(
            "CopyOptions(mode={}, overwrite={}, compression={}, threads={})",
            self.mode, self.overwrite, self.enable_compression, self.num_threads
        )
    }

    /// Representation
    fn __repr__(&self) -> String {
        format!(
            "CopyOptions(mode='{}', overwrite='{}', preserve_timestamps={}, preserve_permissions={}, \
             follow_symlinks={}, enable_compression={}, compression_level={}, buffer_size={}, \
             num_threads={}, verify={})",
            self.mode, self.overwrite, self.preserve_timestamps, self.preserve_permissions,
            self.follow_symlinks, self.enable_compression, self.compression_level,
            self.buffer_size, self.num_threads, self.verify
        )
    }
}

impl Default for PyCopyOptions {
    fn default() -> Self {
        Self::new(
            "auto".to_string(),
            "prompt".to_string(),
            true,
            true,
            false,
            false,
            6,
            64 * 1024,
            0,
            false,
        )
    }
}

impl PyCopyOptions {
    /// Convert to Rust CopyMode
    pub fn to_copy_mode(&self) -> CopyMode {
        match self.mode.as_str() {
            "all" => CopyMode::All,
            "newer" => CopyMode::Newer,
            "different" => CopyMode::Different,
            "mirror" => CopyMode::Mirror,
            _ => CopyMode::All,
        }
    }
}

/// Python wrapper for network configuration
#[pyclass(name = "NetworkConfig")]
#[derive(Debug, Clone)]
pub struct PyNetworkConfig {
    /// Network protocol
    #[pyo3(get, set)]
    pub protocol: String,
    /// Maximum connections
    #[pyo3(get, set)]
    pub max_connections: u32,
    /// Connection timeout in seconds
    #[pyo3(get, set)]
    pub connect_timeout: f64,
    /// Operation timeout in seconds
    #[pyo3(get, set)]
    pub operation_timeout: f64,
    /// Enable connection pooling
    #[pyo3(get, set)]
    pub enable_pooling: bool,
    /// Maximum retry attempts
    #[pyo3(get, set)]
    pub max_retries: u32,
}

#[pymethods]
impl PyNetworkConfig {
    /// Create new network configuration
    #[new]
    #[pyo3(signature = (
        protocol = "quic".to_string(),
        max_connections = 10,
        connect_timeout = 10.0,
        operation_timeout = 300.0,
        enable_pooling = true,
        max_retries = 3
    ))]
    pub fn new(
        protocol: String,
        max_connections: u32,
        connect_timeout: f64,
        operation_timeout: f64,
        enable_pooling: bool,
        max_retries: u32,
    ) -> Self {
        Self {
            protocol,
            max_connections,
            connect_timeout,
            operation_timeout,
            enable_pooling,
            max_retries,
        }
    }

    /// String representation
    fn __str__(&self) -> String {
        format!(
            "NetworkConfig(protocol={}, max_connections={}, pooling={})",
            self.protocol, self.max_connections, self.enable_pooling
        )
    }
}

impl PyNetworkConfig {
    /// Convert to Rust NetworkProtocol
    pub fn to_network_protocol(&self) -> NetworkProtocol {
        match self.protocol.as_str() {
            "quic" => NetworkProtocol::Quic,
            "http3" => NetworkProtocol::Http3,
            "http2" => NetworkProtocol::Http2,
            "tcp" => NetworkProtocol::Tcp,
            _ => NetworkProtocol::Quic,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Full tests on non-Windows platforms
    #[cfg(not(target_os = "windows"))]
    mod full_tests {
        use super::*;

        #[test]
        fn test_copy_options_creation() {
            let options = PyCopyOptions::new(
                "auto".to_string(),
                "prompt".to_string(),
                true,
                true,
                false,
                false,
                6,
                64 * 1024,
                0,
                false,
            );

            assert_eq!(options.mode, "auto");
            assert_eq!(options.overwrite, "prompt");
            assert!(options.preserve_timestamps);
        }

        #[test]
        fn test_copy_options_presets() {
            let speed_options = PyCopyOptions::for_speed();
            assert_eq!(speed_options.mode, "fast");
            assert!(!speed_options.verify);

            let safety_options = PyCopyOptions::for_safety();
            assert_eq!(safety_options.mode, "safe");
            assert!(safety_options.verify);

            let compression_options = PyCopyOptions::for_compression();
            assert!(compression_options.enable_compression);
        }

        #[test]
        fn test_network_config() {
            let config = PyNetworkConfig::new("quic".to_string(), 10, 10.0, 300.0, true, 3);

            assert_eq!(config.protocol, "quic");
            assert_eq!(config.max_connections, 10);
            assert!(config.enable_pooling);
        }
    }

    // Compilation-only tests on Windows
    #[cfg(target_os = "windows")]
    mod compilation_tests {
        use super::*;

        #[test]
        fn test_copy_options_compilation() {
            // Test compilation only, avoid Python runtime dependencies
            let _options = PyCopyOptions::new(
                "auto".to_string(),
                "prompt".to_string(),
                true,
                true,
                false,
                false,
                6,
                64 * 1024,
                0,
                false,
            );
            // Just verify compilation
        }

        #[test]
        fn test_presets_compilation() {
            // Test compilation only
            let _speed_options = PyCopyOptions::for_speed();
            let _safety_options = PyCopyOptions::for_safety();
            let _compression_options = PyCopyOptions::for_compression();
            // Just verify compilation
        }

        #[test]
        fn test_network_config_compilation() {
            // Test compilation only
            let _config = PyNetworkConfig::new("quic".to_string(), 10, 10.0, 300.0, true, 3);
            // Just verify compilation
        }
    }
}
