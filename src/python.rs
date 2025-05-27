//! Python bindings for ferrocp
//!
//! This module provides PyO3 bindings to expose the Rust EACopy functionality
//! to Python. It maintains compatibility with the existing Python API while
//! providing access to all the new Rust-based features.

use crate::core::{EACopy, CopyStats, ProgressInfo, FileOperations};
use crate::config::Config;
use crate::error::Error;
use pyo3::prelude::*;
use pyo3::exceptions::{PyRuntimeError, PyFileNotFoundError, PyIOError};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Convert Rust Error to Python exception
fn error_to_py_err(error: Error) -> PyErr {
    match error {
        Error::FileNotFound { path } => {
            PyFileNotFoundError::new_err(format!("File not found: {:?}", path))
        }
        Error::DirectoryNotFound { path } => {
            PyFileNotFoundError::new_err(format!("Directory not found: {:?}", path))
        }
        Error::PermissionDenied { path } => {
            PyIOError::new_err(format!("Permission denied: {:?}", path))
        }
        Error::Io(io_err) => PyIOError::new_err(io_err.to_string()),
        _ => PyRuntimeError::new_err(error.to_string()),
    }
}

/// Python wrapper for CopyStats
#[pyclass(name = "CopyStats")]
#[derive(Clone)]
pub struct PyCopyStats {
    inner: CopyStats,
}

#[pymethods]
impl PyCopyStats {
    #[getter]
    fn files_copied(&self) -> u64 {
        self.inner.files_copied
    }

    #[getter]
    fn bytes_copied(&self) -> u64 {
        self.inner.bytes_copied
    }

    #[getter]
    fn directories_created(&self) -> u64 {
        self.inner.directories_created
    }

    #[getter]
    fn files_skipped(&self) -> u64 {
        self.inner.files_skipped
    }

    #[getter]
    fn errors(&self) -> u64 {
        self.inner.errors
    }

    #[getter]
    fn duration_seconds(&self) -> f64 {
        self.inner.duration.as_secs_f64()
    }

    #[getter]
    fn avg_speed(&self) -> f64 {
        self.inner.avg_speed
    }

    #[getter]
    fn zerocopy_used(&self) -> u64 {
        self.inner.zerocopy_used
    }

    #[getter]
    fn zerocopy_bytes(&self) -> u64 {
        self.inner.zerocopy_bytes
    }

    #[getter]
    fn zerocopy_rate(&self) -> f64 {
        self.inner.zerocopy_rate()
    }

    #[getter]
    fn zerocopy_bytes_rate(&self) -> f64 {
        self.inner.zerocopy_bytes_rate()
    }

    fn __repr__(&self) -> String {
        format!(
            "CopyStats(files={}, bytes={}, dirs={}, skipped={}, errors={}, duration={:.2}s, speed={:.2} MB/s, zerocopy={})",
            self.inner.files_copied,
            self.inner.bytes_copied,
            self.inner.directories_created,
            self.inner.files_skipped,
            self.inner.errors,
            self.inner.duration.as_secs_f64(),
            self.inner.avg_speed / (1024.0 * 1024.0),
            self.inner.zerocopy_used
        )
    }
}

impl From<CopyStats> for PyCopyStats {
    fn from(stats: CopyStats) -> Self {
        Self { inner: stats }
    }
}

/// Python wrapper for EACopy
#[pyclass(name = "EACopy")]
pub struct PyEACopy {
    inner: EACopy,
    runtime: Arc<Runtime>,
}

#[pymethods]
impl PyEACopy {
    #[new]
    #[pyo3(signature = (
        thread_count = None,
        compression_level = None,
        buffer_size = None,
        preserve_metadata = None,
        follow_symlinks = None,
        dirs_exist_ok = None,
        progress_callback = None
    ))]
    fn new(
        thread_count: Option<usize>,
        compression_level: Option<i32>,
        buffer_size: Option<usize>,
        preserve_metadata: Option<bool>,
        follow_symlinks: Option<bool>,
        dirs_exist_ok: Option<bool>,
        progress_callback: Option<PyObject>,
    ) -> PyResult<Self> {
        let mut config = Config::new();

        if let Some(threads) = thread_count {
            config = config.with_thread_count(threads);
        }
        if let Some(level) = compression_level {
            config = config.with_compression_level(level as u32);
        }
        if let Some(size) = buffer_size {
            config = config.with_buffer_size(size);
        }
        if let Some(preserve) = preserve_metadata {
            config = config.with_preserve_metadata(preserve);
        }
        if let Some(follow) = follow_symlinks {
            config = config.with_follow_symlinks(follow);
        }
        if let Some(exist_ok) = dirs_exist_ok {
            config = config.with_dirs_exist_ok(exist_ok);
        }

        let mut eacopy = EACopy::with_config(config);

        // Set up progress callback if provided
        if let Some(callback) = progress_callback {
            eacopy = eacopy.with_progress_callback(move |progress: &ProgressInfo| {
                Python::with_gil(|py| {
                    let args = (
                        progress.current_bytes,
                        progress.current_total,
                        progress.current_file.to_string_lossy().to_string(),
                    );
                    if let Err(e) = callback.call1(py, args) {
                        eprintln!("Progress callback error: {}", e);
                    }
                });
            });
        }

        let runtime = Arc::new(
            Runtime::new()
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to create async runtime: {}", e)))?
        );

        Ok(Self {
            inner: eacopy,
            runtime,
        })
    }

    /// Copy a file from source to destination
    #[pyo3(signature = (source, destination, skip_existing = None))]
    fn copy_file(&self, source: &str, destination: &str, skip_existing: Option<bool>) -> PyResult<PyCopyStats> {
        let source = PathBuf::from(source);
        let destination = PathBuf::from(destination);

        // Create a new EACopy instance with skip_existing configuration if needed
        let eacopy = if let Some(skip) = skip_existing {
            let config = self.inner.get_config().clone().with_skip_existing(skip);
            EACopy::with_config(config)
        } else {
            EACopy::with_config(self.inner.get_config().clone())
        };

        let stats = self.runtime
            .block_on(eacopy.copy_file(&source, &destination))
            .map_err(error_to_py_err)?;

        Ok(stats.into())
    }

    /// Copy a directory tree from source to destination
    #[pyo3(signature = (source, destination, skip_existing = None))]
    fn copy_directory(&self, source: &str, destination: &str, skip_existing: Option<bool>) -> PyResult<PyCopyStats> {
        let source = PathBuf::from(source);
        let destination = PathBuf::from(destination);

        // Create a new EACopy instance with skip_existing configuration if needed
        let eacopy = if let Some(skip) = skip_existing {
            let config = self.inner.get_config().clone().with_skip_existing(skip);
            EACopy::with_config(config)
        } else {
            EACopy::with_config(self.inner.get_config().clone())
        };

        let stats = self.runtime
            .block_on(eacopy.copy_directory(&source, &destination))
            .map_err(error_to_py_err)?;

        Ok(stats.into())
    }

    /// Copy using network server acceleration
    fn copy_with_server(
        &self,
        source: &str,
        destination: &str,
        server_addr: &str,
        port: Option<u16>,
    ) -> PyResult<PyCopyStats> {
        let source = PathBuf::from(source);
        let destination = PathBuf::from(destination);
        let port = port.unwrap_or(31337);

        let stats = self.runtime
            .block_on(self.inner.copy_with_server(&source, &destination, server_addr, port))
            .map_err(error_to_py_err)?;

        Ok(stats.into())
    }

    /// Perform delta copy using a reference file
    fn delta_copy(
        &self,
        source: &str,
        destination: &str,
        reference: &str,
    ) -> PyResult<PyCopyStats> {
        let source = PathBuf::from(source);
        let destination = PathBuf::from(destination);
        let reference = PathBuf::from(reference);

        let stats = self.runtime
            .block_on(self.inner.delta_copy(&source, &destination, &reference))
            .map_err(error_to_py_err)?;

        Ok(stats.into())
    }

    /// Get current statistics
    fn get_stats(&self) -> PyCopyStats {
        self.inner.stats().into()
    }

    /// Reset statistics
    fn reset_stats(&mut self) {
        self.inner.reset_stats();
    }

    /// Copy a file using zero-copy methods when possible
    fn copy_file_zerocopy(&self, source: &str, destination: &str) -> PyResult<PyCopyStats> {
        let source = PathBuf::from(source);
        let destination = PathBuf::from(destination);

        let stats = self.runtime
            .block_on(self.inner.copy_file_zerocopy(&source, &destination))
            .map_err(error_to_py_err)?;

        Ok(stats.into())
    }
}

/// High-level convenience functions

/// Copy a file from source to destination
#[pyfunction]
fn copy(source: &str, destination: &str) -> PyResult<PyCopyStats> {
    let runtime = Runtime::new()
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to create async runtime: {}", e)))?;

    let eacopy = EACopy::new();
    let source = PathBuf::from(source);
    let destination = PathBuf::from(destination);

    let stats = runtime
        .block_on(eacopy.copy_file(&source, &destination))
        .map_err(error_to_py_err)?;

    Ok(stats.into())
}

/// Copy a file from source to destination, preserving metadata
#[pyfunction]
fn copy2(source: &str, destination: &str) -> PyResult<PyCopyStats> {
    let runtime = Runtime::new()
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to create async runtime: {}", e)))?;

    let config = Config::new().with_preserve_metadata(true);
    let eacopy = EACopy::with_config(config);
    let source = PathBuf::from(source);
    let destination = PathBuf::from(destination);

    let stats = runtime
        .block_on(eacopy.copy_file(&source, &destination))
        .map_err(error_to_py_err)?;

    Ok(stats.into())
}

/// Copy file content from source to destination (alias for copy)
#[pyfunction]
fn copyfile(source: &str, destination: &str) -> PyResult<PyCopyStats> {
    copy(source, destination)
}

/// Copy a directory tree from source to destination
#[pyfunction]
#[pyo3(signature = (source, destination, symlinks = None, _ignore_dangling_symlinks = None, dirs_exist_ok = None, skip_existing = None))]
fn copytree(
    source: &str,
    destination: &str,
    symlinks: Option<bool>,
    _ignore_dangling_symlinks: Option<bool>,
    dirs_exist_ok: Option<bool>,
    skip_existing: Option<bool>,
) -> PyResult<PyCopyStats> {
    let runtime = Runtime::new()
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to create async runtime: {}", e)))?;

    let mut config = Config::new();
    if let Some(follow) = symlinks {
        config = config.with_follow_symlinks(follow);
    }
    if let Some(exist_ok) = dirs_exist_ok {
        config = config.with_dirs_exist_ok(exist_ok);
    }
    if let Some(skip) = skip_existing {
        config = config.with_skip_existing(skip);
    }

    let eacopy = EACopy::with_config(config);
    let source = PathBuf::from(source);
    let destination = PathBuf::from(destination);

    let stats = runtime
        .block_on(eacopy.copy_directory(&source, &destination))
        .map_err(error_to_py_err)?;

    Ok(stats.into())
}

/// Copy using network server acceleration
#[pyfunction]
fn copy_with_server(
    source: &str,
    destination: &str,
    server_addr: &str,
    port: Option<u16>,
    compression_level: Option<i32>,
) -> PyResult<PyCopyStats> {
    let runtime = Runtime::new()
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to create async runtime: {}", e)))?;

    let mut config = Config::new();
    if let Some(level) = compression_level {
        config = config.with_compression_level(level as u32);
    }

    let eacopy = EACopy::with_config(config);
    let source = PathBuf::from(source);
    let destination = PathBuf::from(destination);
    let port = port.unwrap_or(31337);

    let stats = runtime
        .block_on(eacopy.copy_with_server(&source, &destination, server_addr, port))
        .map_err(error_to_py_err)?;

    Ok(stats.into())
}

/// Perform delta copy using a reference file
#[pyfunction]
fn delta_copy(source: &str, destination: &str, reference: &str) -> PyResult<PyCopyStats> {
    let runtime = Runtime::new()
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to create async runtime: {}", e)))?;

    let eacopy = EACopy::new();
    let source = PathBuf::from(source);
    let destination = PathBuf::from(destination);
    let reference = PathBuf::from(reference);

    let stats = runtime
        .block_on(eacopy.delta_copy(&source, &destination, &reference))
        .map_err(error_to_py_err)?;

    Ok(stats.into())
}

/// Copy a file using zero-copy methods when possible
#[pyfunction]
fn copy_zerocopy(source: &str, destination: &str) -> PyResult<PyCopyStats> {
    let runtime = Runtime::new()
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to create async runtime: {}", e)))?;

    let eacopy = EACopy::new();
    let source = PathBuf::from(source);
    let destination = PathBuf::from(destination);

    let stats = runtime
        .block_on(eacopy.copy_file_zerocopy(&source, &destination))
        .map_err(error_to_py_err)?;

    Ok(stats.into())
}

/// Get version information
#[pyfunction]
fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Python module definition
#[pymodule]
fn _ferrocp_binding(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyEACopy>()?;
    m.add_class::<PyCopyStats>()?;
    m.add_function(wrap_pyfunction!(copy, m)?)?;
    m.add_function(wrap_pyfunction!(copy2, m)?)?;
    m.add_function(wrap_pyfunction!(copyfile, m)?)?;
    m.add_function(wrap_pyfunction!(copytree, m)?)?;
    m.add_function(wrap_pyfunction!(copy_with_server, m)?)?;
    m.add_function(wrap_pyfunction!(delta_copy, m)?)?;
    m.add_function(wrap_pyfunction!(copy_zerocopy, m)?)?;
    m.add_function(wrap_pyfunction!(version, m)?)?;

    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("__eacopy_version__", env!("CARGO_PKG_VERSION"))?;

    Ok(())
}
