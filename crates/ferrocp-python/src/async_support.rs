//! Async support utilities for Python bindings

use pyo3::prelude::*;
use pyo3_asyncio::tokio::future_into_py;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::task::JoinHandle;
use uuid::Uuid;

/// Async operation handle for cancellation and progress tracking
#[pyclass(name = "AsyncOperation")]
#[derive(Debug, Clone)]
pub struct PyAsyncOperation {
    id: Uuid,
    handle: Arc<RwLock<Option<JoinHandle<()>>>>,
    cancel_tx: Arc<RwLock<Option<mpsc::Sender<()>>>>,
    progress_rx: Arc<RwLock<Option<mpsc::Receiver<f64>>>>,
}

#[pymethods]
impl PyAsyncOperation {
    /// Get operation ID
    #[getter]
    pub fn id(&self) -> String {
        self.id.to_string()
    }

    /// Check if operation is running
    pub fn is_running<'py>(&self, py: Python<'py>) -> PyResult<&'py PyAny> {
        let handle = self.handle.clone();
        future_into_py(py, async move {
            let handle_guard = handle.read().await;
            Ok(handle_guard.as_ref().map_or(false, |h| !h.is_finished()))
        })
    }

    /// Cancel the operation
    pub fn cancel<'py>(&self, py: Python<'py>) -> PyResult<&'py PyAny> {
        let cancel_tx = self.cancel_tx.clone();
        let handle = self.handle.clone();

        future_into_py(py, async move {
            // Send cancel signal
            if let Some(tx) = cancel_tx.read().await.as_ref() {
                let _ = tx.send(()).await;
            }

            // Abort the task
            if let Some(handle) = handle.write().await.take() {
                handle.abort();
            }

            Ok(true)
        })
    }

    /// Get current progress (0.0 to 1.0)
    pub fn get_progress<'py>(&self, py: Python<'py>) -> PyResult<&'py PyAny> {
        let progress_rx = self.progress_rx.clone();
        future_into_py(py, async move {
            let mut rx_guard = progress_rx.write().await;
            if let Some(rx) = rx_guard.as_mut() {
                match rx.try_recv() {
                    Ok(progress) => Ok(Some(progress)),
                    Err(_) => Ok(None), // No new progress available
                }
            } else {
                Ok(None)
            }
        })
    }

    /// Wait for operation to complete
    pub fn wait<'py>(&self, py: Python<'py>) -> PyResult<&'py PyAny> {
        let handle = self.handle.clone();
        future_into_py(py, async move {
            if let Some(handle) = handle.write().await.take() {
                match handle.await {
                    Ok(_) => Ok(true),
                    Err(_) => Ok(false), // Task was cancelled or panicked
                }
            } else {
                Ok(true) // Already completed
            }
        })
    }
}

impl PyAsyncOperation {
    /// Create a new async operation
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            handle: Arc::new(RwLock::new(None)),
            cancel_tx: Arc::new(RwLock::new(None)),
            progress_rx: Arc::new(RwLock::new(None)),
        }
    }

    /// Set the task handle
    pub async fn set_handle(&self, handle: JoinHandle<()>) {
        *self.handle.write().await = Some(handle);
    }

    /// Set the cancel sender
    pub async fn set_cancel_sender(&self, tx: mpsc::Sender<()>) {
        *self.cancel_tx.write().await = Some(tx);
    }

    /// Set the progress receiver
    pub async fn set_progress_receiver(&self, rx: mpsc::Receiver<f64>) {
        *self.progress_rx.write().await = Some(rx);
    }
}

/// Async operation manager
#[pyclass(name = "AsyncManager")]
#[derive(Debug, Clone)]
pub struct PyAsyncManager {
    operations: Arc<RwLock<std::collections::HashMap<Uuid, PyAsyncOperation>>>,
}

#[pymethods]
impl PyAsyncManager {
    /// Create new async manager
    #[new]
    pub fn new() -> Self {
        Self {
            operations: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Get all active operations
    pub fn get_active_operations<'py>(&self, py: Python<'py>) -> PyResult<&'py PyAny> {
        let operations = self.operations.clone();
        future_into_py(py, async move {
            let ops_guard = operations.read().await;
            let active_ids: Vec<String> = ops_guard.keys().map(|id| id.to_string()).collect();
            Ok(active_ids)
        })
    }

    /// Get operation by ID
    pub fn get_operation<'py>(
        &self,
        py: Python<'py>,
        operation_id: String,
    ) -> PyResult<&'py PyAny> {
        let operations = self.operations.clone();
        future_into_py(py, async move {
            if let Ok(id) = Uuid::parse_str(&operation_id) {
                let ops_guard = operations.read().await;
                if ops_guard.contains_key(&id) {
                    Ok(Some(operation_id))
                } else {
                    Ok(None)
                }
            } else {
                Ok(None)
            }
        })
    }

    /// Cancel all operations
    pub fn cancel_all<'py>(&self, py: Python<'py>) -> PyResult<&'py PyAny> {
        let operations = self.operations.clone();
        future_into_py(py, async move {
            let mut ops_guard = operations.write().await;
            let mut cancelled_count = 0;

            for (_, operation) in ops_guard.iter() {
                // Send cancel signal
                if let Some(tx) = operation.cancel_tx.read().await.as_ref() {
                    if tx.send(()).await.is_ok() {
                        cancelled_count += 1;
                    }
                }

                // Abort the task
                if let Some(handle) = operation.handle.write().await.take() {
                    handle.abort();
                }
            }

            ops_guard.clear();
            Ok(cancelled_count)
        })
    }
}

impl PyAsyncManager {
    /// Register a new operation
    pub async fn register_operation(&self, operation: PyAsyncOperation) -> Uuid {
        let id = operation.id;
        self.operations.write().await.insert(id, operation.clone());
        id
    }

    /// Unregister an operation
    pub async fn unregister_operation(&self, id: Uuid) {
        self.operations.write().await.remove(&id);
    }
}

/// Create a cancellable async task
pub async fn create_cancellable_task<F, Fut, T>(
    manager: &PyAsyncManager,
    task_fn: F,
) -> PyResult<PyAsyncOperation>
where
    F: FnOnce(mpsc::Receiver<()>, mpsc::Sender<f64>) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = PyResult<T>> + Send,
    T: Send + 'static,
{
    let operation = PyAsyncOperation::new();
    let (cancel_tx, cancel_rx) = mpsc::channel::<()>(1);
    let (progress_tx, progress_rx) = mpsc::channel::<f64>(100);

    // Set up the operation
    operation.set_cancel_sender(cancel_tx).await;
    operation.set_progress_receiver(progress_rx).await;

    // Spawn the task
    let handle = tokio::spawn(async move {
        match task_fn(cancel_rx, progress_tx).await {
            Ok(_) => {
                // Task completed successfully
            }
            Err(_) => {
                // Task failed
            }
        }
    });

    operation.set_handle(handle).await;
    let operation_clone = operation.clone();
    manager.register_operation(operation).await;

    Ok(operation_clone)
}

/// Utility function to check for cancellation
pub async fn check_cancellation(cancel_rx: &mut mpsc::Receiver<()>) -> bool {
    match cancel_rx.try_recv() {
        Ok(_) => true,                                        // Cancellation requested
        Err(mpsc::error::TryRecvError::Empty) => false,       // No cancellation
        Err(mpsc::error::TryRecvError::Disconnected) => true, // Channel closed, treat as cancelled
    }
}

/// Utility function to report progress
pub async fn report_progress(progress_tx: &mpsc::Sender<f64>, progress: f64) -> bool {
    progress_tx.send(progress.clamp(0.0, 1.0)).await.is_ok()
}
