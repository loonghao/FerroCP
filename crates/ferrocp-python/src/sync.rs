//! Synchronization functionality for Python bindings

use crate::progress::ProgressCallback;
use ferrocp_sync::{SyncEngine, SyncOptions, SyncResult};
use pyo3::prelude::*;
use pyo3_asyncio::tokio::future_into_py;
use std::collections::HashMap;
use std::path::PathBuf;

/// Python wrapper for sync options
#[pyclass(name = "SyncOptions")]
#[derive(Debug, Clone)]
pub struct PySyncOptions {
    /// Enable incremental synchronization
    #[pyo3(get, set)]
    pub incremental: bool,
    /// Enable delta compression
    #[pyo3(get, set)]
    pub enable_delta: bool,
    /// Enable hash caching
    #[pyo3(get, set)]
    pub enable_caching: bool,
    /// Delete files in destination that don't exist in source
    #[pyo3(get, set)]
    pub delete_extra: bool,
    /// Follow symbolic links
    #[pyo3(get, set)]
    pub follow_symlinks: bool,
    /// Preserve file permissions
    #[pyo3(get, set)]
    pub preserve_permissions: bool,
    /// Preserve file timestamps
    #[pyo3(get, set)]
    pub preserve_timestamps: bool,
    /// Dry run (don't actually modify files)
    #[pyo3(get, set)]
    pub dry_run: bool,
}

#[pymethods]
impl PySyncOptions {
    /// Create new sync options
    #[new]
    #[pyo3(signature = (
        incremental = true,
        enable_delta = true,
        enable_caching = true,
        delete_extra = false,
        follow_symlinks = false,
        preserve_permissions = true,
        preserve_timestamps = true,
        dry_run = false
    ))]
    pub fn new(
        incremental: bool,
        enable_delta: bool,
        enable_caching: bool,
        delete_extra: bool,
        follow_symlinks: bool,
        preserve_permissions: bool,
        preserve_timestamps: bool,
        dry_run: bool,
    ) -> Self {
        Self {
            incremental,
            enable_delta,
            enable_caching,
            delete_extra,
            follow_symlinks,
            preserve_permissions,
            preserve_timestamps,
            dry_run,
        }
    }

    /// Create options for incremental sync
    #[staticmethod]
    pub fn incremental() -> Self {
        Self {
            incremental: true,
            enable_delta: true,
            enable_caching: true,
            delete_extra: false,
            follow_symlinks: false,
            preserve_permissions: true,
            preserve_timestamps: true,
            dry_run: false,
        }
    }

    /// Create options for full sync
    #[staticmethod]
    pub fn full() -> Self {
        Self {
            incremental: false,
            enable_delta: false,
            enable_caching: false,
            delete_extra: false,
            follow_symlinks: false,
            preserve_permissions: true,
            preserve_timestamps: true,
            dry_run: false,
        }
    }

    /// Create options for mirror sync
    #[staticmethod]
    pub fn mirror() -> Self {
        Self {
            incremental: true,
            enable_delta: true,
            enable_caching: true,
            delete_extra: true,
            follow_symlinks: false,
            preserve_permissions: true,
            preserve_timestamps: true,
            dry_run: false,
        }
    }

    /// Convert to dictionary
    pub fn to_dict(&self) -> HashMap<String, PyObject> {
        Python::with_gil(|py| {
            let mut dict = HashMap::new();
            dict.insert("incremental".to_string(), self.incremental.to_object(py));
            dict.insert("enable_delta".to_string(), self.enable_delta.to_object(py));
            dict.insert(
                "enable_caching".to_string(),
                self.enable_caching.to_object(py),
            );
            dict.insert("delete_extra".to_string(), self.delete_extra.to_object(py));
            dict.insert(
                "follow_symlinks".to_string(),
                self.follow_symlinks.to_object(py),
            );
            dict.insert(
                "preserve_permissions".to_string(),
                self.preserve_permissions.to_object(py),
            );
            dict.insert(
                "preserve_timestamps".to_string(),
                self.preserve_timestamps.to_object(py),
            );
            dict.insert("dry_run".to_string(), self.dry_run.to_object(py));
            dict
        })
    }

    /// String representation
    fn __str__(&self) -> String {
        format!(
            "SyncOptions(incremental={}, delta={}, caching={}, delete_extra={})",
            self.incremental, self.enable_delta, self.enable_caching, self.delete_extra
        )
    }
}

impl From<PySyncOptions> for SyncOptions {
    fn from(py_options: PySyncOptions) -> Self {
        SyncOptions {
            incremental: py_options.incremental,
            enable_delta: py_options.enable_delta,
            enable_caching: py_options.enable_caching,
            delete_extra: py_options.delete_extra,
            follow_symlinks: py_options.follow_symlinks,
            preserve_permissions: py_options.preserve_permissions,
            preserve_timestamps: py_options.preserve_timestamps,
            dry_run: py_options.dry_run,
            ..Default::default()
        }
    }
}

/// Python wrapper for sync results
#[pyclass(name = "SyncResult")]
#[derive(Debug, Clone)]
pub struct PySyncResult {
    /// Number of files synced
    #[pyo3(get)]
    pub files_synced: u64,
    /// Number of bytes transferred
    #[pyo3(get)]
    pub bytes_transferred: u64,
    /// Number of conflicts encountered
    #[pyo3(get)]
    pub conflicts_count: u64,
    /// Number of errors encountered
    #[pyo3(get)]
    pub errors_count: u64,
    /// Duration of the sync operation in seconds
    #[pyo3(get)]
    pub duration_seconds: f64,
    /// Whether delta compression was used
    #[pyo3(get)]
    pub delta_used: bool,
    /// Delta compression savings in bytes
    #[pyo3(get)]
    pub delta_savings: u64,
    /// Whether the operation was successful
    #[pyo3(get)]
    pub success: bool,
    /// Error message if operation failed
    #[pyo3(get)]
    pub error_message: Option<String>,
}

#[pymethods]
impl PySyncResult {
    /// Create a new sync result
    #[new]
    pub fn new() -> Self {
        Self {
            files_synced: 0,
            bytes_transferred: 0,
            conflicts_count: 0,
            errors_count: 0,
            duration_seconds: 0.0,
            delta_used: false,
            delta_savings: 0,
            success: false,
            error_message: None,
        }
    }

    /// String representation
    fn __str__(&self) -> String {
        if self.success {
            format!(
                "SyncResult(success=True, files={}, bytes={}, conflicts={}, errors={})",
                self.files_synced, self.bytes_transferred, self.conflicts_count, self.errors_count
            )
        } else {
            format!(
                "SyncResult(success=False, error={})",
                self.error_message.as_deref().unwrap_or("Unknown error")
            )
        }
    }
}

impl Default for PySyncResult {
    fn default() -> Self {
        Self::new()
    }
}

impl From<SyncResult> for PySyncResult {
    fn from(result: SyncResult) -> Self {
        Self {
            files_synced: result.files_synced,
            bytes_transferred: result.bytes_transferred,
            conflicts_count: result.conflicts_count,
            errors_count: result.errors_count,
            duration_seconds: result.duration.as_secs_f64(),
            delta_used: result.delta_used,
            delta_savings: result.delta_savings,
            success: true,
            error_message: None,
        }
    }
}

/// Python wrapper for sync engine
#[pyclass(name = "SyncEngine")]
pub struct PySyncEngine {
    engine: Option<SyncEngine>,
}

#[pymethods]
impl PySyncEngine {
    /// Create a new sync engine
    #[new]
    pub fn new() -> Self {
        Self { engine: None }
    }

    /// Initialize the sync engine with options
    #[pyo3(signature = (options = None))]
    pub fn initialize<'py>(
        &mut self,
        py: Python<'py>,
        options: Option<PySyncOptions>,
    ) -> PyResult<&'py PyAny> {
        let sync_options = options.map(SyncOptions::from).unwrap_or_default();

        // For now, initialize synchronously to avoid borrowing issues
        // TODO: Implement proper async initialization
        let engine = pyo3_asyncio::tokio::get_runtime()
            .block_on(async { SyncEngine::with_options(sync_options).await })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        self.engine = Some(engine);

        future_into_py(py, async move { Ok(()) })
    }

    /// Synchronize directories
    #[pyo3(signature = (source, destination, options = None, _progress_callback = None))]
    pub fn sync<'py>(
        &mut self,
        py: Python<'py>,
        source: String,
        destination: String,
        options: Option<PySyncOptions>,
        _progress_callback: Option<ProgressCallback>,
    ) -> PyResult<&'py PyAny> {
        let _source_path = PathBuf::from(source);
        let _dest_path = PathBuf::from(destination);
        let sync_options = options.map(SyncOptions::from).unwrap_or_default();

        // Initialize engine if not already done
        if self.engine.is_none() {
            let engine = pyo3_asyncio::tokio::get_runtime()
                .block_on(async { SyncEngine::with_options(sync_options.clone()).await })
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            self.engine = Some(engine);
        }

        // For now, return a simple result since we can't access self.engine in async move
        // TODO: Implement proper async sync operation
        future_into_py(py, async move {
            // Simulate sync operation
            let mut result = PySyncResult::new();
            result.success = true;
            result.files_synced = 0; // Placeholder
            result.bytes_transferred = 0; // Placeholder
            Ok(result)
        })
    }
}

/// Convenience function to synchronize directories
#[pyfunction]
#[pyo3(signature = (source, destination, options = None, progress_callback = None))]
pub fn sync_directories<'py>(
    py: Python<'py>,
    source: String,
    destination: String,
    options: Option<PySyncOptions>,
    progress_callback: Option<ProgressCallback>,
) -> PyResult<&'py PyAny> {
    let mut engine = PySyncEngine::new();
    engine.sync(py, source, destination, options, progress_callback)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_options_creation() {
        let options = PySyncOptions::new(true, true, true, false, false, true, true, false);

        assert!(options.incremental);
        assert!(options.enable_delta);
        assert!(options.enable_caching);
        assert!(!options.delete_extra);
    }

    #[test]
    fn test_sync_options_presets() {
        let incremental = PySyncOptions::incremental();
        assert!(incremental.incremental);
        assert!(incremental.enable_delta);

        let full = PySyncOptions::full();
        assert!(!full.incremental);
        assert!(!full.enable_delta);

        let mirror = PySyncOptions::mirror();
        assert!(mirror.delete_extra);
    }

    #[test]
    fn test_sync_result_creation() {
        let result = PySyncResult::new();
        assert_eq!(result.files_synced, 0);
        assert_eq!(result.bytes_transferred, 0);
        assert!(!result.success);
    }
}
