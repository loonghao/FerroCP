//! Batch Operations Module
//!
//! This module provides batch operations for FerroCP Python bindings,
//! allowing efficient processing of multiple file operations with GIL optimization
//! and parallel processing using Rayon.

use pyo3::prelude::*;
use rayon::prelude::*;
use std::path::PathBuf;
use std::sync::Arc;

use crate::config::PyCopyOptions;
use crate::copy::PyCopyResult;
use crate::gil_optimization::GilOptimizationManager;
use crate::progress::{ProgressCallback, PyProgress};

/// Batch copy request containing source and destination paths
#[pyclass(name = "BatchCopyRequest")]
#[derive(Debug, Clone)]
pub struct PyBatchCopyRequest {
    /// Source file path
    #[pyo3(get, set)]
    pub source: String,
    /// Destination file path
    #[pyo3(get, set)]
    pub destination: String,
}

#[pymethods]
impl PyBatchCopyRequest {
    /// Create a new batch copy request
    #[new]
    pub fn new(source: String, destination: String) -> Self {
        Self {
            source,
            destination,
        }
    }

    /// String representation
    fn __repr__(&self) -> String {
        format!(
            "BatchCopyRequest(source='{}', destination='{}')",
            self.source, self.destination
        )
    }
}

/// Batch copy result containing individual operation results
#[pyclass(name = "BatchCopyResult")]
#[derive(Debug, Clone)]
pub struct PyBatchCopyResult {
    /// Individual copy results
    #[pyo3(get)]
    pub results: Vec<PyCopyResult>,
    /// Total files processed
    #[pyo3(get)]
    pub total_files: usize,
    /// Number of successful operations
    #[pyo3(get)]
    pub successful_operations: usize,
    /// Number of failed operations
    #[pyo3(get)]
    pub failed_operations: usize,
    /// Total bytes copied across all operations
    #[pyo3(get)]
    pub total_bytes_copied: u64,
    /// Total duration for all operations
    #[pyo3(get)]
    pub total_duration_ms: u64,
}

#[pymethods]
impl PyBatchCopyResult {
    /// Create a new batch copy result
    #[new]
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
            total_files: 0,
            successful_operations: 0,
            failed_operations: 0,
            total_bytes_copied: 0,
            total_duration_ms: 0,
        }
    }

    /// Check if all operations were successful
    #[getter]
    pub fn all_successful(&self) -> bool {
        self.failed_operations == 0 && self.successful_operations > 0
    }

    /// Get success rate as percentage
    #[getter]
    pub fn success_rate(&self) -> f64 {
        if self.total_files == 0 {
            0.0
        } else {
            (self.successful_operations as f64 / self.total_files as f64) * 100.0
        }
    }

    /// String representation with caching
    fn __repr__(&self) -> String {
        // Create a cache key based on the result's content
        let cache_key = (
            self.total_files,
            self.successful_operations,
            self.failed_operations,
            self.total_bytes_copied,
            self.total_duration_ms,
        );

        crate::object_cache::get_or_insert_string(cache_key, || {
            format!(
                "BatchCopyResult(total={}, successful={}, failed={}, success_rate={:.1}%)",
                self.total_files,
                self.successful_operations,
                self.failed_operations,
                self.success_rate()
            )
        })
    }
}

impl Default for PyBatchCopyResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Batch operations engine for efficient bulk file operations
#[pyclass(name = "BatchCopyEngine")]
pub struct PyBatchCopyEngine {
    gil_manager: Arc<GilOptimizationManager>,
    batch_size: usize,
}

#[pymethods]
impl PyBatchCopyEngine {
    /// Create a new batch copy engine
    #[new]
    #[pyo3(signature = (batch_size = 100))]
    pub fn new(batch_size: usize) -> PyResult<Self> {
        let gil_manager = Arc::new(GilOptimizationManager::new());
        Ok(Self {
            gil_manager,
            batch_size,
        })
    }

    /// Copy multiple files in batch with parallel processing and GIL optimization
    #[pyo3(signature = (requests, _options = None, progress_callback = None))]
    pub fn copy_files_batch<'py>(
        &self,
        py: Python<'py>,
        requests: Vec<PyBatchCopyRequest>,
        _options: Option<PyCopyOptions>,
        progress_callback: Option<ProgressCallback>,
    ) -> PyResult<&'py PyAny> {
        let gil_manager = Arc::clone(&self.gil_manager);
        let batch_size = self.batch_size;

        pyo3_asyncio::tokio::future_into_py(py, async move {
            // Execute batch operations with GIL released during computation
            let gil_result = gil_manager
                .execute_with_gil_released(move |progress_tx| async move {
                    let reporter =
                        crate::gil_optimization::GilFreeProgressReporter::new(progress_tx);

                    // Report start
                    let _ = reporter.report_progress(
                        0.0,
                        format!("Starting batch copy of {} files", requests.len()),
                    );

                    let total_requests = requests.len();
                    let mut batch_result = PyBatchCopyResult::new();
                    batch_result.total_files = total_requests;

                    // Process requests in parallel batches
                    let chunks: Vec<_> = requests.chunks(batch_size).collect();
                    let total_chunks = chunks.len();

                    for (chunk_idx, chunk) in chunks.into_iter().enumerate() {
                        // Process chunk in parallel using Rayon
                        let chunk_results: Vec<_> = chunk
                            .par_iter()
                            .map(|request| {
                                // Create individual copy request
                                let source_path = PathBuf::from(&request.source);
                                let dest_path = PathBuf::from(&request.destination);

                                // Perform actual file copy operation
                                let mut result = PyCopyResult::new();

                                // Check if source exists
                                if !source_path.exists() {
                                    result.success = false;
                                    result.error_message =
                                        Some(format!("Source file not found: {}", request.source));
                                    return result;
                                }

                                // Attempt to copy the file
                                match std::fs::copy(&source_path, &dest_path) {
                                    Ok(bytes_copied) => {
                                        result.success = true;
                                        result.bytes_copied = bytes_copied;
                                        result.files_copied = 1;
                                    }
                                    Err(e) => {
                                        result.success = false;
                                        result.error_message = Some(format!("Copy failed: {}", e));
                                    }
                                }

                                result
                            })
                            .collect();

                        // Aggregate results
                        for result in chunk_results {
                            if result.success {
                                batch_result.successful_operations += 1;
                                batch_result.total_bytes_copied += result.bytes_copied;
                            } else {
                                batch_result.failed_operations += 1;
                            }
                            batch_result.results.push(result);
                        }

                        // Report progress
                        let progress = ((chunk_idx + 1) as f64 / total_chunks as f64) * 100.0;
                        let _ = reporter.report_progress(
                            progress,
                            format!(
                                "Processed chunk {} of {} ({} files)",
                                chunk_idx + 1,
                                total_chunks,
                                batch_result.results.len()
                            ),
                        );
                    }

                    // Report completion
                    let _ = reporter.report_progress(
                        100.0,
                        format!(
                            "Batch copy completed: {}/{} successful",
                            batch_result.successful_operations, batch_result.total_files
                        ),
                    );

                    Ok::<PyBatchCopyResult, PyErr>(batch_result)
                })
                .await?;

            // Handle progress callback with GIL (only if needed)
            if progress_callback.is_some() {
                let progress = PyProgress::from(ferrocp_types::CopyStats {
                    bytes_copied: gil_result.result.total_bytes_copied,
                    files_copied: gil_result.result.successful_operations as u64,
                    duration: gil_result.duration,
                    ..Default::default()
                });
                Python::with_gil(|py| {
                    crate::progress::call_progress_callback(py, &progress_callback, &progress)
                })?;
            }

            Ok(gil_result.result)
        })
    }

    /// Get current batch size
    pub fn get_batch_size(&self) -> usize {
        self.batch_size
    }

    /// Set batch size for parallel processing
    pub fn set_batch_size(&mut self, batch_size: usize) {
        self.batch_size = batch_size;
    }
}

/// Convenience function for batch file copying
#[pyfunction]
#[pyo3(signature = (requests, options = None, progress_callback = None, batch_size = 100))]
pub fn copy_files_batch<'py>(
    py: Python<'py>,
    requests: Vec<PyBatchCopyRequest>,
    options: Option<PyCopyOptions>,
    progress_callback: Option<ProgressCallback>,
    batch_size: usize,
) -> PyResult<&'py PyAny> {
    let engine = PyBatchCopyEngine::new(batch_size)?;
    engine.copy_files_batch(py, requests, options, progress_callback)
}

/// Convenience function to create batch requests from lists
#[pyfunction]
pub fn create_batch_requests(
    sources: Vec<String>,
    destinations: Vec<String>,
) -> PyResult<Vec<PyBatchCopyRequest>> {
    if sources.len() != destinations.len() {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Sources and destinations lists must have the same length",
        ));
    }

    Ok(sources
        .into_iter()
        .zip(destinations.into_iter())
        .map(|(source, destination)| PyBatchCopyRequest::new(source, destination))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_batch_copy_engine_creation() {
        let engine = PyBatchCopyEngine::new(50).unwrap();
        assert_eq!(engine.get_batch_size(), 50);
    }

    #[test]
    fn test_batch_copy_request() {
        let request = PyBatchCopyRequest::new("source.txt".to_string(), "dest.txt".to_string());
        assert_eq!(request.source, "source.txt");
        assert_eq!(request.destination, "dest.txt");
    }

    #[test]
    fn test_batch_copy_result() {
        let mut result = PyBatchCopyResult::new();
        result.total_files = 10;
        result.successful_operations = 8;
        result.failed_operations = 2;

        assert_eq!(result.success_rate(), 80.0);
        assert!(!result.all_successful());
    }

    #[test]
    fn test_create_batch_requests() {
        let sources = vec!["file1.txt".to_string(), "file2.txt".to_string()];
        let destinations = vec!["dest1.txt".to_string(), "dest2.txt".to_string()];

        let requests = create_batch_requests(sources, destinations).unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].source, "file1.txt");
        assert_eq!(requests[1].destination, "dest2.txt");
    }

    #[test]
    fn test_create_batch_requests_mismatched_lengths() {
        let sources = vec!["file1.txt".to_string()];
        let destinations = vec!["dest1.txt".to_string(), "dest2.txt".to_string()];

        let result = create_batch_requests(sources, destinations);
        assert!(result.is_err());
    }
}
