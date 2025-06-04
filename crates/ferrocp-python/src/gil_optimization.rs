//! GIL Release Optimization Module
//!
//! This module provides optimized mechanisms for releasing the Python Global Interpreter Lock (GIL)
//! during CPU-intensive and I/O operations to improve performance in multi-threaded environments.

use pyo3::prelude::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

/// Progress callback that can be called without holding the GIL
pub type GilFreeProgressCallback = Arc<dyn Fn(f64, String) + Send + Sync>;

/// Result of a GIL-free operation
#[derive(Debug, Clone)]
pub struct GilFreeResult<T> {
    /// The result of the operation
    pub result: T,
    /// Progress updates collected during the operation
    pub progress_updates: Vec<ProgressUpdate>,
    /// Duration of the operation
    pub duration: Duration,
}

/// Progress update information
#[derive(Debug, Clone)]
pub struct ProgressUpdate {
    /// Progress percentage (0.0 to 100.0)
    pub percentage: f64,
    /// Progress message
    pub message: String,
    /// Timestamp when the update was created
    pub timestamp: std::time::Instant,
}

/// GIL optimization manager for handling async operations without GIL contention
pub struct GilOptimizationManager {
    progress_tx: mpsc::UnboundedSender<ProgressUpdate>,
    progress_rx: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<ProgressUpdate>>>,
}

impl GilOptimizationManager {
    /// Create a new GIL optimization manager
    pub fn new() -> Self {
        let (progress_tx, progress_rx) = mpsc::unbounded_channel();
        Self {
            progress_tx,
            progress_rx: Arc::new(tokio::sync::Mutex::new(progress_rx)),
        }
    }

    /// Execute an operation with GIL released during the computation
    pub async fn execute_with_gil_released<F, T, Fut>(
        &self,
        operation: F,
    ) -> PyResult<GilFreeResult<T>>
    where
        F: FnOnce(mpsc::UnboundedSender<ProgressUpdate>) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = PyResult<T>> + Send,
        T: Send + 'static,
    {
        let start_time = std::time::Instant::now();
        let progress_tx = self.progress_tx.clone();

        // Execute the operation without holding the GIL
        let (result_tx, mut result_rx) = oneshot::channel();

        // Spawn the operation in a separate task
        tokio::spawn(async move {
            let result = operation(progress_tx).await;
            let _ = result_tx.send(result);
        });

        // Collect progress updates while waiting for completion
        let mut progress_updates = Vec::new();
        let mut progress_rx = self.progress_rx.lock().await;

        loop {
            tokio::select! {
                // Check for completion
                result = &mut result_rx => {
                    match result {
                        Ok(op_result) => {
                            // Collect any remaining progress updates
                            while let Ok(update) = progress_rx.try_recv() {
                                progress_updates.push(update);
                            }

                            return op_result.map(|r| GilFreeResult {
                                result: r,
                                progress_updates,
                                duration: start_time.elapsed(),
                            });
                        }
                        Err(_) => {
                            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                                "Operation was cancelled or failed"
                            ));
                        }
                    }
                }

                // Collect progress updates
                update = progress_rx.recv() => {
                    if let Some(update) = update {
                        progress_updates.push(update);
                    }
                }

                // Timeout to prevent infinite waiting
                _ = tokio::time::sleep(Duration::from_millis(100)) => {
                    // Continue the loop
                }
            }
        }
    }

    /// Create a progress reporter that doesn't require GIL
    pub fn create_progress_reporter(&self) -> GilFreeProgressReporter {
        GilFreeProgressReporter {
            tx: self.progress_tx.clone(),
        }
    }
}

impl Default for GilOptimizationManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Progress reporter that can be used without holding the GIL
#[derive(Clone)]
pub struct GilFreeProgressReporter {
    tx: mpsc::UnboundedSender<ProgressUpdate>,
}

impl GilFreeProgressReporter {
    /// Create a new progress reporter
    pub fn new(tx: mpsc::UnboundedSender<ProgressUpdate>) -> Self {
        Self { tx }
    }

    /// Report progress without requiring GIL
    pub fn report_progress(&self, percentage: f64, message: String) -> Result<(), String> {
        let update = ProgressUpdate {
            percentage,
            message,
            timestamp: std::time::Instant::now(),
        };

        self.tx
            .send(update)
            .map_err(|_| "Failed to send progress update".to_string())
    }

    /// Report progress with formatted message
    pub fn report_progress_formatted(
        &self,
        percentage: f64,
        format_args: std::fmt::Arguments,
    ) -> Result<(), String> {
        self.report_progress(percentage, format_args.to_string())
    }
}

/// Macro for executing operations with GIL released
#[macro_export]
macro_rules! execute_with_gil_released {
    ($manager:expr, $operation:expr) => {
        $manager
            .execute_with_gil_released(move |progress_tx| async move {
                let reporter = GilFreeProgressReporter { tx: progress_tx };
                $operation(reporter).await
            })
            .await
    };
}

/// Utility function to create a GIL-free callback wrapper
pub fn create_gil_free_callback<F>(callback: F) -> GilFreeProgressCallback
where
    F: Fn(f64, String) + Send + Sync + 'static,
{
    Arc::new(callback)
}

/// Optimized async operation wrapper that releases GIL during computation
pub async fn execute_async_with_gil_optimization<F, T>(py: Python<'_>, operation: F) -> PyResult<T>
where
    F: FnOnce() -> std::pin::Pin<Box<dyn std::future::Future<Output = PyResult<T>> + Send>>
        + Send
        + 'static,
    T: Send + 'static,
{
    // Release GIL during the async operation
    py.allow_threads(|| {
        // Create a new Tokio runtime for this operation if needed
        let rt = tokio::runtime::Handle::try_current().unwrap_or_else(|_| {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all() // Enable all features including timers
                .build()
                .expect("Failed to create Tokio runtime")
                .handle()
                .clone()
        });

        rt.block_on(operation())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_gil_optimization_manager() {
        let manager = GilOptimizationManager::new();

        let result = manager
            .execute_with_gil_released(|progress_tx| async move {
                let reporter = GilFreeProgressReporter::new(progress_tx);

                // Simulate some work with progress reporting
                reporter
                    .report_progress(0.0, "Starting operation".to_string())
                    .unwrap();
                tokio::time::sleep(Duration::from_millis(10)).await;

                reporter
                    .report_progress(50.0, "Half way done".to_string())
                    .unwrap();
                tokio::time::sleep(Duration::from_millis(10)).await;

                reporter
                    .report_progress(100.0, "Operation complete".to_string())
                    .unwrap();

                Ok::<i32, PyErr>(42)
            })
            .await;

        assert!(result.is_ok());
        let gil_free_result = result.unwrap();
        assert_eq!(gil_free_result.result, 42);
        assert!(!gil_free_result.progress_updates.is_empty());
    }

    #[test]
    fn test_progress_reporter() {
        let manager = GilOptimizationManager::new();
        let reporter = manager.create_progress_reporter();

        let result = reporter.report_progress(50.0, "Test progress".to_string());
        assert!(result.is_ok());
    }
}
