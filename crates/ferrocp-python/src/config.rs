//! Configuration for Python bindings

use ferrocp_types::{CopyMode, NetworkProtocol, OverwriteDecision, OverwritePolicy, SymlinkMode};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::IntoPyObjectExt;
use std::collections::HashMap;

/// Python wrapper for copy options
///
/// The `mode`, `overwrite`, `follow_symlinks` and `preserve_*` fields are part
/// of FerroCP's copy-semantics contract and are honoured by every copy path.
/// Unknown values raise `ValueError` instead of falling back to a default, so
/// the Python API can never promise a behaviour it does not implement.
///
/// See `docs/COPY_SEMANTICS.md` for the full contract and the platform matrix.
#[pyclass(name = "CopyOptions", from_py_object)]
#[derive(Debug)]
pub struct PyCopyOptions {
    /// Copy mode: "all", "newer" or "different"
    ///
    /// "mirror" is declared but not implemented and raises `ValueError`.
    #[pyo3(get, set)]
    pub mode: String,
    /// Overwrite policy: "always", "never", "if_newer", "if_different",
    /// "fail" or "prompt"
    ///
    /// "prompt" requires `overwrite_callback` to be set.
    #[pyo3(get, set)]
    pub overwrite: String,
    /// Optional callable used when `overwrite` is "prompt"
    ///
    /// It is called as `callback(source, destination) -> bool`: return `True`
    /// to overwrite, `False` to skip.
    #[pyo3(get, set)]
    pub overwrite_callback: Option<Py<PyAny>>,
    /// Whether to preserve timestamps
    #[pyo3(get, set)]
    pub preserve_timestamps: bool,
    /// Whether to preserve permissions
    ///
    /// On Unix the full mode is preserved; on Windows only the read-only
    /// attribute is.
    #[pyo3(get, set)]
    pub preserve_permissions: bool,
    /// Whether to follow symbolic links
    ///
    /// `True` copies the content a link points at; `False` (the default)
    /// recreates the link itself.
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

impl Clone for PyCopyOptions {
    /// Clone the options
    ///
    /// `Py<PyAny>` is `Clone` only for pyclasses, so the callback reference is
    /// cloned explicitly: cloning a Python reference needs the GIL.
    fn clone(&self) -> Self {
        Self {
            mode: self.mode.clone(),
            overwrite: self.overwrite.clone(),
            overwrite_callback: self
                .overwrite_callback
                .as_ref()
                .map(|callback| Python::attach(|py| callback.clone_ref(py))),
            preserve_timestamps: self.preserve_timestamps,
            preserve_permissions: self.preserve_permissions,
            follow_symlinks: self.follow_symlinks,
            enable_compression: self.enable_compression,
            compression_level: self.compression_level,
            buffer_size: self.buffer_size,
            num_threads: self.num_threads,
            verify: self.verify,
        }
    }
}

#[pymethods]
impl PyCopyOptions {
    /// Create new copy options with defaults
    ///
    /// The default `overwrite` is `"always"`, which is the behaviour FerroCP has
    /// always had. It used to be `"prompt"`, but nothing ever prompted, so the
    /// old default advertised protection that did not exist.
    #[new]
    #[pyo3(signature = (
        mode = "auto".to_string(),
        overwrite = "always".to_string(),
        overwrite_callback = None,
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
        overwrite_callback: Option<Py<PyAny>>,
        preserve_timestamps: bool,
        preserve_permissions: bool,
        follow_symlinks: bool,
        enable_compression: bool,
        compression_level: u8,
        buffer_size: usize,
        num_threads: usize,
        verify: bool,
    ) -> PyResult<Self> {
        let options = Self {
            mode,
            overwrite,
            overwrite_callback,
            preserve_timestamps,
            preserve_permissions,
            follow_symlinks,
            enable_compression,
            compression_level,
            buffer_size,
            num_threads,
            verify,
        };
        // Validate eagerly: an invalid value must surface when the options are
        // built, not half-way through a copy.
        options.copy_mode()?;
        options.overwrite_policy()?;
        options.overwrite_prompt()?;
        Ok(options)
    }

    /// Create options optimized for speed
    #[staticmethod]
    pub fn for_speed() -> Self {
        Self {
            mode: "all".to_string(),
            overwrite: "always".to_string(),
            overwrite_callback: None,
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
    ///
    /// "safe" means never replacing an existing destination: `overwrite` is
    /// `"never"`, so existing files are reported as skipped instead of being
    /// truncated.
    #[staticmethod]
    pub fn for_safety() -> Self {
        Self {
            mode: "all".to_string(),
            overwrite: "never".to_string(),
            overwrite_callback: None,
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
            mode: "all".to_string(),
            overwrite: "always".to_string(),
            overwrite_callback: None,
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
    pub fn to_dict(&self) -> PyResult<HashMap<String, Py<PyAny>>> {
        Python::attach(|py| {
            let mut dict = HashMap::new();
            dict.insert("mode".to_string(), self.mode.as_str().into_py_any(py)?);
            dict.insert(
                "overwrite".to_string(),
                self.overwrite.as_str().into_py_any(py)?,
            );
            dict.insert(
                "preserve_timestamps".to_string(),
                self.preserve_timestamps.into_py_any(py)?,
            );
            dict.insert(
                "preserve_permissions".to_string(),
                self.preserve_permissions.into_py_any(py)?,
            );
            dict.insert(
                "follow_symlinks".to_string(),
                self.follow_symlinks.into_py_any(py)?,
            );
            dict.insert(
                "enable_compression".to_string(),
                self.enable_compression.into_py_any(py)?,
            );
            dict.insert(
                "compression_level".to_string(),
                self.compression_level.into_py_any(py)?,
            );
            dict.insert("buffer_size".to_string(), self.buffer_size.into_py_any(py)?);
            dict.insert("num_threads".to_string(), self.num_threads.into_py_any(py)?);
            dict.insert("verify".to_string(), self.verify.into_py_any(py)?);
            Ok(dict)
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
            "CopyOptions(mode='{}', overwrite='{}', overwrite_callback={}, preserve_timestamps={}, \
             preserve_permissions={}, follow_symlinks={}, enable_compression={}, \
             compression_level={}, buffer_size={}, num_threads={}, verify={})",
            self.mode,
            self.overwrite,
            if self.overwrite_callback.is_some() {
                "<callable>"
            } else {
                "None"
            },
            self.preserve_timestamps,
            self.preserve_permissions,
            self.follow_symlinks,
            self.enable_compression,
            self.compression_level,
            self.buffer_size,
            self.num_threads,
            self.verify
        )
    }
}

impl Default for PyCopyOptions {
    fn default() -> Self {
        // The defaults are literals that are known to be valid, so build the
        // struct directly instead of going through the validating constructor
        // and unwrapping the result.
        Self {
            mode: "auto".to_string(),
            overwrite: "always".to_string(),
            overwrite_callback: None,
            preserve_timestamps: true,
            preserve_permissions: true,
            follow_symlinks: false,
            enable_compression: false,
            compression_level: 6,
            buffer_size: 64 * 1024,
            num_threads: 0,
            verify: false,
        }
    }
}

/// Build the error raised for an unrecognised option value
fn invalid_value(option: &str, value: &str, accepted: &[&str]) -> PyErr {
    PyValueError::new_err(format!(
        "invalid {option} value '{value}'; accepted values: {}",
        accepted.join(", ")
    ))
}

impl PyCopyOptions {
    /// Convert to Rust `CopyMode`
    ///
    /// Raises `ValueError` for unknown values. `mirror` is rejected explicitly
    /// because it is not implemented, rather than silently behaving like `all`.
    pub fn copy_mode(&self) -> PyResult<CopyMode> {
        match self.mode.trim().to_ascii_lowercase().as_str() {
            "all" | "auto" => Ok(CopyMode::All),
            "newer" => Ok(CopyMode::Newer),
            "different" => Ok(CopyMode::Different),
            "mirror" => Err(PyValueError::new_err(
                "copy mode 'mirror' is declared but not implemented; refusing to run instead of \
                 silently degrading to 'all'",
            )),
            other => Err(invalid_value("mode", other, &["all", "newer", "different"])),
        }
    }

    /// Convert to Rust `OverwritePolicy`
    ///
    /// Raises `ValueError` for unknown values instead of defaulting, because a
    /// defaulted policy silently changes which files get replaced.
    pub fn overwrite_policy(&self) -> PyResult<OverwritePolicy> {
        OverwritePolicy::parse(&self.overwrite).ok_or_else(|| {
            invalid_value(
                "overwrite",
                &self.overwrite,
                OverwritePolicy::accepted_values(),
            )
        })
    }

    /// Convert `follow_symlinks` to Rust `SymlinkMode`
    pub fn symlink_mode(&self) -> PyResult<SymlinkMode> {
        Ok(if self.follow_symlinks {
            SymlinkMode::Follow
        } else {
            SymlinkMode::Preserve
        })
    }

    /// Build the prompt handler for `overwrite="prompt"`
    ///
    /// Returns an error when the policy is `prompt` but no callback was
    /// supplied: an unanswered prompt must never turn into "overwrite".
    pub fn overwrite_prompt(&self) -> PyResult<Option<ferrocp_io::OverwritePrompt>> {
        if self.overwrite_policy()? != OverwritePolicy::Prompt {
            return Ok(None);
        }

        let callback = self
            .overwrite_callback
            .as_ref()
            .map(|callback| Python::attach(|py| callback.clone_ref(py)))
            .ok_or_else(|| {
                PyValueError::new_err(
                    "overwrite='prompt' requires overwrite_callback to be set (it is called as \
                 callback(source, destination) -> bool)",
                )
            })?;

        Ok(Some(ferrocp_io::OverwritePrompt::new(
            move |source: &std::path::Path, destination: &std::path::Path| {
                let source = source.to_path_buf();
                let destination = destination.to_path_buf();
                // Cloning a Python reference needs the GIL.
                let callback = Python::attach(|py| callback.clone_ref(py));

                // The callback may be a Python callable, so the GIL must be
                // held while it runs.
                Python::attach(|py| {
                    let args = (
                        source.display().to_string(),
                        destination.display().to_string(),
                    );
                    match callback.call1(py, args) {
                        Ok(value) => match value.extract::<bool>(py) {
                            // An exception is not an answer: keep the existing file.
                            Ok(true) => OverwriteDecision::Proceed,
                            Ok(false) | Err(_) => OverwriteDecision::Skip,
                        },
                        Err(error) => {
                            tracing::warn!(
                                "overwrite_callback raised for '{}', skipping: {}",
                                destination.display(),
                                error
                            );
                            OverwriteDecision::Skip
                        }
                    }
                })
            },
        )))
    }
}

/// Python wrapper for network configuration
#[pyclass(name = "NetworkConfig", from_py_object)]
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

        /// Start the interpreter before a test that inspects a `PyErr`.
        ///
        /// `PyErr` renders through Python's C API, so an interpreter has to be
        /// running. pyo3 no longer initialises one implicitly (the
        /// `auto-initialize` feature was removed in 0.26). `initialize()` is
        /// idempotent and race-safe, so calling it per test also removes the
        /// dependency on which test the harness happens to schedule first.
        fn with_interpreter() {
            Python::initialize();
        }

        #[test]
        fn test_copy_options_creation() {
            let options = PyCopyOptions::new(
                "auto".to_string(),
                "always".to_string(),
                None,
                true,
                true,
                false,
                false,
                6,
                64 * 1024,
                0,
                false,
            )
            .unwrap();

            assert_eq!(options.mode, "auto");
            assert_eq!(options.overwrite, "always");
            assert!(options.preserve_timestamps);
        }

        #[test]
        fn test_default_overwrite_is_always_not_prompt() {
            // The historical default was "prompt", which implied protection
            // that never existed. The default must be the behaviour that
            // actually happens.
            with_interpreter();
            let options = PyCopyOptions::default();
            assert_eq!(options.overwrite, "always");
            assert_eq!(options.overwrite_policy().unwrap(), OverwritePolicy::Always);
        }

        #[test]
        fn test_copy_options_presets() {
            with_interpreter();
            let speed_options = PyCopyOptions::for_speed();
            assert_eq!(speed_options.mode, "all");
            assert_eq!(speed_options.overwrite, "always");
            assert!(!speed_options.verify);

            let safety_options = PyCopyOptions::for_safety();
            // "safe" must mean it never replaces an existing destination.
            assert_eq!(safety_options.overwrite, "never");
            assert!(safety_options.verify);

            let compression_options = PyCopyOptions::for_compression();
            assert!(compression_options.enable_compression);
        }

        #[test]
        fn test_presets_are_valid() {
            with_interpreter();
            for options in [
                PyCopyOptions::for_speed(),
                PyCopyOptions::for_safety(),
                PyCopyOptions::for_compression(),
            ] {
                options
                    .copy_mode()
                    .expect("preset mode must be a valid copy mode");
                options
                    .overwrite_policy()
                    .expect("preset overwrite must be a valid policy");
            }
        }

        #[test]
        fn test_unknown_overwrite_value_is_rejected() {
            with_interpreter();
            let options = PyCopyOptions {
                overwrite: "maybe".to_string(),
                ..PyCopyOptions::default()
            };
            let error = options.overwrite_policy().unwrap_err();
            assert!(error
                .to_string()
                .contains("invalid overwrite value 'maybe'"));
        }

        #[test]
        fn test_prompt_without_callback_is_rejected() {
            with_interpreter();
            let options = PyCopyOptions {
                overwrite: "prompt".to_string(),
                overwrite_callback: None,
                ..PyCopyOptions::default()
            };
            assert_eq!(options.overwrite_policy().unwrap(), OverwritePolicy::Prompt);
            let error = options.overwrite_prompt().unwrap_err();
            assert!(error.to_string().contains("requires overwrite_callback"));
        }

        #[test]
        fn test_mirror_mode_is_rejected() {
            with_interpreter();
            let options = PyCopyOptions {
                mode: "mirror".to_string(),
                ..PyCopyOptions::default()
            };
            let error = options.copy_mode().unwrap_err();
            assert!(error.to_string().contains("not implemented"));
        }

        #[test]
        fn test_follow_symlinks_maps_to_symlink_mode() {
            with_interpreter();
            let options = PyCopyOptions {
                follow_symlinks: true,
                ..PyCopyOptions::default()
            };
            assert_eq!(options.symlink_mode().unwrap(), SymlinkMode::Follow);

            let options = PyCopyOptions {
                follow_symlinks: false,
                ..PyCopyOptions::default()
            };
            assert_eq!(options.symlink_mode().unwrap(), SymlinkMode::Preserve);
        }

        #[test]
        fn test_network_config() {
            with_interpreter();
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
                "always".to_string(),
                None,
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
