//! Copy functionality for Python bindings

use crate::async_support::{create_cancellable_task, report_progress, PyAsyncManager};
use crate::config::PyCopyOptions;
use crate::gil_optimization::{GilFreeProgressReporter, GilOptimizationManager};
use crate::progress::{call_progress_callback, ProgressCallback, PyProgress};
use ferrocp_engine::{task::CopyRequest, CopyEngine};
use ferrocp_types::CopyStats;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_asyncio::tokio::future_into_py;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

/// Python wrapper for copy results
#[pyclass(name = "CopyResult")]
#[derive(Debug, Clone)]
pub struct PyCopyResult {
    /// Total bytes copied
    #[pyo3(get)]
    pub bytes_copied: u64,
    /// Total files copied
    #[pyo3(get)]
    pub files_copied: u64,
    /// Duration of the copy operation
    #[pyo3(get)]
    pub duration_seconds: f64,
    /// Average transfer rate (bytes per second)
    #[pyo3(get)]
    pub transfer_rate: f64,
    /// Whether the operation was successful
    #[pyo3(get)]
    pub success: bool,
    /// Error message if operation failed
    #[pyo3(get)]
    pub error_message: Option<String>,
}

#[pymethods]
impl PyCopyResult {
    /// Create a new copy result
    #[new]
    pub fn new() -> Self {
        Self {
            bytes_copied: 0,
            files_copied: 0,
            duration_seconds: 0.0,
            transfer_rate: 0.0,
            success: false,
            error_message: None,
        }
    }

    /// Get formatted transfer rate
    pub fn format_transfer_rate(&self) -> String {
        format_bytes_per_second(self.transfer_rate)
    }

    /// Get formatted duration
    pub fn format_duration(&self) -> String {
        format_duration(Duration::from_secs_f64(self.duration_seconds))
    }

    /// String representation with caching
    fn __str__(&self) -> String {
        // Create a cache key based on the result's content
        let cache_key = (
            self.success,
            self.files_copied,
            self.bytes_copied,
            (self.duration_seconds * 1000.0) as u64, // Convert to milliseconds for integer key
            self.error_message.as_deref().unwrap_or("").to_string(),
        );

        crate::object_cache::get_or_insert_string(cache_key, || {
            if self.success {
                format!(
                    "CopyResult(success=True, files={}, bytes={}, duration={})",
                    self.files_copied,
                    format_bytes(self.bytes_copied),
                    self.format_duration()
                )
            } else {
                format!(
                    "CopyResult(success=False, error={})",
                    self.error_message.as_deref().unwrap_or("Unknown error")
                )
            }
        })
    }
}

impl Default for PyCopyResult {
    fn default() -> Self {
        Self::new()
    }
}

impl From<CopyStats> for PyCopyResult {
    fn from(stats: CopyStats) -> Self {
        let duration_seconds = stats.duration.as_secs_f64();
        let transfer_rate = if duration_seconds > 0.0 {
            stats.bytes_copied as f64 / duration_seconds
        } else {
            0.0
        };

        Self {
            bytes_copied: stats.bytes_copied,
            files_copied: stats.files_copied,
            duration_seconds,
            transfer_rate,
            success: true,
            error_message: None,
        }
    }
}

/// Python wrapper for copy engine
#[pyclass(name = "CopyEngine")]
pub struct PyCopyEngine {
    engine: CopyEngine,
    async_manager: Arc<PyAsyncManager>,
    gil_manager: Arc<GilOptimizationManager>,
}

#[pymethods]
impl PyCopyEngine {
    /// Create a new copy engine
    #[new]
    pub fn new() -> PyResult<Self> {
        let engine = pyo3_asyncio::tokio::get_runtime()
            .block_on(async { CopyEngine::new().await })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        let async_manager = Arc::new(PyAsyncManager::new());
        let gil_manager = Arc::new(GilOptimizationManager::new());
        Ok(Self {
            engine,
            async_manager,
            gil_manager,
        })
    }

    /// Copy a single file with GIL optimization
    #[pyo3(signature = (source, destination, options = None, progress_callback = None))]
    pub fn copy_file<'py>(
        &self,
        py: Python<'py>,
        source: String,
        destination: String,
        options: Option<PyCopyOptions>,
        progress_callback: Option<ProgressCallback>,
    ) -> PyResult<&'py PyAny> {
        let source_path = PathBuf::from(source);
        let dest_path = PathBuf::from(destination);
        let copy_options = options;
        let engine = self.engine.clone();
        let gil_manager = Arc::clone(&self.gil_manager);

        future_into_py(py, async move {
            // Execute the copy operation with GIL released during computation
            let gil_result = gil_manager
                .execute_with_gil_released(|progress_tx| async move {
                    let reporter = GilFreeProgressReporter::new(progress_tx);

                    // Report start
                    let _ = reporter.report_progress(0.0, "Starting file copy".to_string());

                    // Create copy request with options
                    let mut request = CopyRequest::new(source_path, dest_path);

                    // Apply copy options if provided
                    if let Some(opts) = copy_options {
                        if opts.verify {
                            request.verify_copy = true;
                        }
                        if opts.preserve_timestamps || opts.preserve_permissions {
                            request.preserve_metadata = true;
                        }
                        if opts.enable_compression {
                            request.enable_compression = true;
                        }
                        // TODO: Add exclude/include patterns to PyCopyOptions
                        // For now, we'll skip these fields
                    }

                    // Report progress
                    let _ = reporter.report_progress(10.0, "Copy request prepared".to_string());

                    // Execute the copy operation (GIL is released during this)
                    let result = engine.execute(request).await;

                    // Report completion
                    let _ = reporter.report_progress(100.0, "Copy operation completed".to_string());

                    match result {
                        Ok(copy_result) => {
                            let stats = copy_result.stats;
                            Ok(PyCopyResult::from(stats))
                        }
                        Err(e) => {
                            let mut result = PyCopyResult::new();
                            result.success = false;
                            result.error_message = Some(e.to_string());
                            Ok(result)
                        }
                    }
                })
                .await?;

            // Handle progress callback with GIL (only if needed)
            if progress_callback.is_some() {
                let progress = PyProgress::from(ferrocp_types::CopyStats {
                    bytes_copied: gil_result.result.bytes_copied,
                    files_copied: gil_result.result.files_copied,
                    duration: gil_result.duration,
                    ..Default::default()
                });
                Python::with_gil(|py| call_progress_callback(py, &progress_callback, &progress))?;
            }

            Ok(gil_result.result)
        })
    }

    /// Copy a directory recursively with GIL optimization
    #[pyo3(signature = (source, destination, options = None, progress_callback = None))]
    pub fn copy_directory<'py>(
        &self,
        py: Python<'py>,
        source: String,
        destination: String,
        options: Option<PyCopyOptions>,
        progress_callback: Option<ProgressCallback>,
    ) -> PyResult<&'py PyAny> {
        let source_path = PathBuf::from(source);
        let dest_path = PathBuf::from(destination);
        let copy_options = options;
        let engine = self.engine.clone();
        let gil_manager = Arc::clone(&self.gil_manager);

        future_into_py(py, async move {
            // Execute the directory copy operation with GIL released during computation
            let gil_result = gil_manager
                .execute_with_gil_released(|progress_tx| async move {
                    let reporter = GilFreeProgressReporter::new(progress_tx);

                    // Report start
                    let _ = reporter.report_progress(0.0, "Starting directory copy".to_string());

                    // Create copy request with options
                    let mut request = CopyRequest::new(source_path, dest_path);

                    // Apply copy options if provided
                    if let Some(opts) = copy_options {
                        if opts.verify {
                            request.verify_copy = true;
                        }
                        if opts.preserve_timestamps || opts.preserve_permissions {
                            request.preserve_metadata = true;
                        }
                        if opts.enable_compression {
                            request.enable_compression = true;
                        }
                        // TODO: Add exclude/include patterns to PyCopyOptions
                        // For now, we'll skip these fields
                    }

                    // Report progress
                    let _ = reporter.report_progress(10.0, "Directory scan started".to_string());

                    // Execute the copy operation (GIL is released during this)
                    let result = engine.execute(request).await;

                    // Report completion
                    let _ = reporter.report_progress(100.0, "Directory copy completed".to_string());

                    match result {
                        Ok(copy_result) => {
                            let stats = copy_result.stats;
                            Ok(PyCopyResult::from(stats))
                        }
                        Err(e) => {
                            let mut result = PyCopyResult::new();
                            result.success = false;
                            result.error_message = Some(e.to_string());
                            Ok(result)
                        }
                    }
                })
                .await?;

            // Handle progress callback with GIL (only if needed)
            if progress_callback.is_some() {
                let progress = PyProgress::from(ferrocp_types::CopyStats {
                    bytes_copied: gil_result.result.bytes_copied,
                    files_copied: gil_result.result.files_copied,
                    duration: gil_result.duration,
                    ..Default::default()
                });
                Python::with_gil(|py| call_progress_callback(py, &progress_callback, &progress))?;
            }

            Ok(gil_result.result)
        })
    }

    /// Get engine statistics
    pub fn get_statistics<'py>(&self, py: Python<'py>) -> PyResult<&'py PyDict> {
        let dict = PyDict::new(py);
        // TODO: Implement actual statistics collection from engine
        dict.set_item("total_operations", 0)?;
        dict.set_item("total_bytes_copied", 0)?;
        dict.set_item("average_speed", 0.0)?;
        Ok(dict)
    }

    /// Check if the engine is busy
    pub fn is_busy(&self) -> bool {
        // TODO: Implement actual busy check
        false
    }

    /// Get supported features
    pub fn get_features<'py>(&self, py: Python<'py>) -> PyResult<&'py PyDict> {
        let dict = PyDict::new(py);
        dict.set_item("zero_copy", true)?;
        dict.set_item("compression", true)?;
        dict.set_item("network_transfer", true)?;
        dict.set_item("incremental_sync", true)?;
        dict.set_item("progress_reporting", true)?;
        dict.set_item("async_operations", true)?;
        Ok(dict)
    }

    /// Copy a file asynchronously with cancellation support
    #[pyo3(signature = (source, destination, options = None))]
    pub fn copy_file_async<'py>(
        &self,
        py: Python<'py>,
        source: String,
        destination: String,
        options: Option<PyCopyOptions>,
    ) -> PyResult<&'py PyAny> {
        let source_path = PathBuf::from(source);
        let dest_path = PathBuf::from(destination);
        let engine = self.engine.clone();
        let manager = self.async_manager.clone();

        future_into_py(py, async move {
            let operation = create_cancellable_task(&*manager, move |_cancel_rx, progress_tx| {
                async move {
                    // Create copy request with options
                    let mut request = CopyRequest::new(source_path, dest_path);

                    // Apply copy options if provided
                    if let Some(opts) = options {
                        if opts.verify {
                            request.verify_copy = true;
                        }
                        if opts.preserve_timestamps || opts.preserve_permissions {
                            request.preserve_metadata = true;
                        }
                        if opts.enable_compression {
                            request.enable_compression = true;
                        }
                    }

                    // Start the copy operation
                    let result = engine.execute(request).await;

                    // Report completion
                    let _ = report_progress(&progress_tx, 1.0).await;

                    match result {
                        Ok(copy_result) => {
                            let stats = copy_result.stats;
                            Ok(PyCopyResult::from(stats))
                        }
                        Err(e) => {
                            let mut result = PyCopyResult::new();
                            result.success = false;
                            result.error_message = Some(e.to_string());
                            Ok(result)
                        }
                    }
                }
            })
            .await?;

            Ok(operation)
        })
    }

    /// Get the async manager for this engine
    pub fn get_async_manager(&self) -> PyAsyncManager {
        self.async_manager.as_ref().clone()
    }
}

/// Convenience function to copy a file
#[pyfunction]
#[pyo3(signature = (source, destination, options = None, progress_callback = None))]
pub fn copy_file<'py>(
    py: Python<'py>,
    source: String,
    destination: String,
    options: Option<PyCopyOptions>,
    progress_callback: Option<ProgressCallback>,
) -> PyResult<&'py PyAny> {
    let engine = PyCopyEngine::new()?;
    engine.copy_file(py, source, destination, options, progress_callback)
}

/// Convenience function to copy a directory
#[pyfunction]
#[pyo3(signature = (source, destination, options = None, progress_callback = None))]
pub fn copy_directory<'py>(
    py: Python<'py>,
    source: String,
    destination: String,
    options: Option<PyCopyOptions>,
    progress_callback: Option<ProgressCallback>,
) -> PyResult<&'py PyAny> {
    let engine = PyCopyEngine::new()?;
    engine.copy_directory(py, source, destination, options, progress_callback)
}

/// Get FerroCP version
#[pyfunction]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Quick copy function with default options
#[pyfunction]
#[pyo3(signature = (source, destination))]
pub fn quick_copy<'py>(
    py: Python<'py>,
    source: String,
    destination: String,
) -> PyResult<&'py PyAny> {
    copy_file(py, source, destination, None, None)
}

/// Copy with verification enabled
#[pyfunction]
#[pyo3(signature = (source, destination, progress_callback = None))]
pub fn copy_with_verification<'py>(
    py: Python<'py>,
    source: String,
    destination: String,
    progress_callback: Option<ProgressCallback>,
) -> PyResult<&'py PyAny> {
    let mut options = PyCopyOptions::default();
    options.verify = true;
    copy_file(py, source, destination, Some(options), progress_callback)
}

/// Copy with compression enabled
#[pyfunction]
#[pyo3(signature = (source, destination, progress_callback = None))]
pub fn copy_with_compression<'py>(
    py: Python<'py>,
    source: String,
    destination: String,
    progress_callback: Option<ProgressCallback>,
) -> PyResult<&'py PyAny> {
    let mut options = PyCopyOptions::default();
    options.enable_compression = true;
    copy_file(py, source, destination, Some(options), progress_callback)
}

/// Async copy function with cancellation support
#[pyfunction]
#[pyo3(signature = (source, destination, options = None))]
pub fn copy_file_async<'py>(
    py: Python<'py>,
    source: String,
    destination: String,
    options: Option<PyCopyOptions>,
) -> PyResult<&'py PyAny> {
    let engine = PyCopyEngine::new()?;
    engine.copy_file_async(py, source, destination, options)
}

/// Create a new async manager
#[pyfunction]
pub fn create_async_manager() -> PyAsyncManager {
    PyAsyncManager::new()
}

/// Format bytes as human-readable string
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.1} {}", size, UNITS[unit_index])
}

/// Format bytes per second as human-readable string
fn format_bytes_per_second(bytes_per_sec: f64) -> String {
    const UNITS: &[&str] = &["B/s", "KB/s", "MB/s", "GB/s", "TB/s"];
    let mut size = bytes_per_sec;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.1} {}", size, UNITS[unit_index])
}

/// Format duration as human-readable string
fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.as_secs();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
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
        fn test_copy_result_creation() {
            let result = PyCopyResult::new();
            assert_eq!(result.bytes_copied, 0);
            assert_eq!(result.files_copied, 0);
            assert!(!result.success);
        }

        #[test]
        fn test_copy_result_from_stats() {
            let stats = CopyStats {
                bytes_copied: 1000,
                files_copied: 5,
                duration: Duration::from_secs(1),
                ..Default::default()
            };

            let result = PyCopyResult::from(stats);
            assert_eq!(result.bytes_copied, 1000);
            assert_eq!(result.files_copied, 5);
            assert_eq!(result.transfer_rate, 1000.0);
            assert!(result.success);
        }

        #[test]
        fn test_format_functions() {
            assert_eq!(format_bytes(1024), "1.0 KB");
            assert_eq!(format_bytes_per_second(1024.0), "1.0 KB/s");
            assert_eq!(format_duration(Duration::from_secs(65)), "1m 5s");
        }
    }

    // Compilation-only tests on Windows
    #[cfg(target_os = "windows")]
    mod compilation_tests {
        use super::*;

        #[test]
        fn test_copy_result_compilation() {
            // Test compilation only, avoid Python runtime dependencies
            let _result = PyCopyResult::new();
            // Just verify compilation
        }

        #[test]
        fn test_copy_result_from_stats_compilation() {
            // Test compilation only
            let stats = CopyStats {
                bytes_copied: 1000,
                files_copied: 5,
                duration: Duration::from_secs(1),
                ..Default::default()
            };
            let _result = PyCopyResult::from(stats);
            // Just verify compilation
        }

        #[test]
        fn test_format_functions_compilation() {
            // Test compilation only
            let _bytes = format_bytes(1024);
            let _rate = format_bytes_per_second(1024.0);
            let _duration = format_duration(Duration::from_secs(65));
            // Just verify compilation
        }
    }
}
