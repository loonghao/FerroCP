//! Task executor for running copy operations

use crate::task::{CopyResult, Task, TaskId};
use ferrocp_config::Config;
use ferrocp_io::{BufferedCopyEngine, CopyEngine as IoCopyEngine, CopyOptions};
use ferrocp_types::{Error, Result};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock, Semaphore};
use tracing::{debug, error, info, warn};

/// Configuration for the task executor
#[derive(Debug, Clone)]
pub struct ExecutorConfig {
    /// Maximum number of concurrent executions
    pub max_concurrent_executions: usize,
    /// Default buffer size for copy operations
    pub default_buffer_size: usize,
    /// Enable progress reporting
    pub enable_progress_reporting: bool,
    /// Progress reporting interval
    pub progress_interval: Duration,
    /// Enable copy verification
    pub enable_verification: bool,
    /// Enable retry on failure
    pub enable_retry: bool,
    /// Maximum retry attempts
    pub max_retry_attempts: u32,
    /// Retry delay
    pub retry_delay: Duration,
}

impl ExecutorConfig {
    /// Create executor config from main config
    pub fn from_config(config: &Config) -> Self {
        Self {
            max_concurrent_executions: config.performance.thread_count.get(),
            default_buffer_size: config.performance.buffer_size.get(),
            enable_progress_reporting: config.features.enable_progress_reporting,
            progress_interval: config.features.progress_interval,
            enable_verification: config.features.enable_verification,
            enable_retry: true,
            max_retry_attempts: 3,
            retry_delay: Duration::from_millis(1000),
        }
    }
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            max_concurrent_executions: num_cpus::get(),
            default_buffer_size: 1024 * 1024, // 1MB
            enable_progress_reporting: true,
            progress_interval: Duration::from_millis(100),
            enable_verification: false,
            enable_retry: true,
            max_retry_attempts: 3,
            retry_delay: Duration::from_millis(1000),
        }
    }
}

/// Task executor that runs copy operations
#[derive(Debug)]
pub struct TaskExecutor {
    config: Arc<RwLock<ExecutorConfig>>,
    copy_engine: Arc<BufferedCopyEngine>,
    semaphore: Arc<Semaphore>,
    active_tasks: Arc<RwLock<HashMap<TaskId, TaskHandle>>>,
    completed_results: Arc<RwLock<HashMap<TaskId, CopyResult>>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

#[derive(Debug)]
struct TaskHandle {
    task: Task,
    cancel_tx: mpsc::Sender<()>,
}

impl TaskExecutor {
    /// Create a new task executor
    pub async fn new(config: ExecutorConfig) -> Result<Self> {
        let max_concurrent = config.max_concurrent_executions;
        let copy_engine = Arc::new(BufferedCopyEngine::new());
        let semaphore = Arc::new(Semaphore::new(max_concurrent));

        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            copy_engine,
            semaphore,
            active_tasks: Arc::new(RwLock::new(HashMap::new())),
            completed_results: Arc::new(RwLock::new(HashMap::new())),
            shutdown_tx: None,
        })
    }

    /// Execute a task
    pub async fn execute_task(&self, mut task: Task) -> Result<()> {
        let task_id = task.id;
        debug!("Starting execution of task {}", task_id);

        // Acquire semaphore permit
        let permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| Error::other(format!("Failed to acquire execution permit: {}", e)))?;

        // Create cancellation channel
        let (cancel_tx, mut cancel_rx) = mpsc::channel(1);

        // Store task handle
        let handle = TaskHandle {
            task: task.clone(),
            cancel_tx,
        };
        self.active_tasks.write().await.insert(task_id, handle);

        // Mark task as started
        task.start();

        let config = self.config.read().await.clone();
        let copy_engine = BufferedCopyEngine::new(); // Create a new instance for this task
        let active_tasks = Arc::clone(&self.active_tasks);
        let completed_results = Arc::clone(&self.completed_results);

        // Spawn execution task
        tokio::spawn(async move {
            let start_time = Instant::now();
            let result = tokio::select! {
                result = Self::execute_copy_operation(copy_engine, &task, &config) => {
                    result
                }
                _ = cancel_rx.recv() => {
                    warn!("Task {} was cancelled", task_id);
                    CopyResult::failure(
                        task_id,
                        "Task was cancelled".to_string(),
                        start_time.elapsed(),
                    )
                }
            };

            // Store result
            completed_results.write().await.insert(task_id, result);

            // Remove from active tasks
            active_tasks.write().await.remove(&task_id);

            // Release permit
            drop(permit);

            info!("Task {} execution completed", task_id);
        });

        Ok(())
    }

    /// Execute the actual copy operation
    async fn execute_copy_operation(
        mut copy_engine: BufferedCopyEngine,
        task: &Task,
        config: &ExecutorConfig,
    ) -> CopyResult {
        let task_id = task.id;
        let start_time = Instant::now();

        // Create copy options from task request
        let copy_options = CopyOptions {
            buffer_size: Some(config.default_buffer_size),
            enable_progress: config.enable_progress_reporting,
            progress_interval: config.progress_interval,
            verify_copy: config.enable_verification || task.request.verify_copy,
            preserve_metadata: task.request.preserve_metadata,
            enable_zero_copy: true,
            max_retries: if config.enable_retry {
                config.max_retry_attempts.max(task.request.max_retries)
            } else {
                0
            },
        };

        // Execute copy with retry logic
        let mut retry_count = 0;
        let max_retries = copy_options.max_retries;

        loop {
            match copy_engine
                .copy_file_with_options(
                    &task.request.source,
                    &task.request.destination,
                    copy_options.clone(),
                )
                .await
            {
                Ok(stats) => {
                    debug!("Copy operation successful for task {}", task_id);
                    return CopyResult::success(task_id, stats, start_time.elapsed());
                }
                Err(error) => {
                    error!("Copy operation failed for task {}: {}", task_id, error);

                    if retry_count < max_retries {
                        retry_count += 1;
                        warn!(
                            "Retrying task {} (attempt {}/{})",
                            task_id, retry_count, max_retries
                        );

                        // Wait before retry
                        tokio::time::sleep(config.retry_delay).await;
                        continue;
                    } else {
                        return CopyResult::failure(
                            task_id,
                            error.to_string(),
                            start_time.elapsed(),
                        );
                    }
                }
            }
        }
    }

    /// Cancel a task
    pub async fn cancel_task(&self, task_id: TaskId) -> Result<()> {
        let active_tasks = self.active_tasks.read().await;

        if let Some(handle) = active_tasks.get(&task_id) {
            let _ = handle.cancel_tx.send(()).await;
            debug!("Cancellation signal sent for task {}", task_id);
            Ok(())
        } else {
            Err(Error::other(format!("Active task {} not found", task_id)))
        }
    }

    /// Wait for a task to complete
    pub async fn wait_for_completion(&self, task_id: TaskId) -> Result<CopyResult> {
        // Check if already completed
        if let Some(result) = self.completed_results.read().await.get(&task_id) {
            return Ok(result.clone());
        }

        // Poll for completion
        let mut interval = tokio::time::interval(Duration::from_millis(100));
        let timeout = Duration::from_secs(3600); // 1 hour timeout
        let start = Instant::now();

        loop {
            interval.tick().await;

            if start.elapsed() > timeout {
                return Err(Error::other(format!(
                    "Timeout waiting for task {}",
                    task_id
                )));
            }

            if let Some(result) = self.completed_results.read().await.get(&task_id) {
                return Ok(result.clone());
            }
        }
    }

    /// Get execution statistics
    pub async fn get_execution_stats(&self) -> (usize, usize) {
        let active = self.active_tasks.read().await.len();
        let completed = self.completed_results.read().await.len();
        (active, completed)
    }

    /// Update executor configuration
    pub async fn update_config(&self, config: ExecutorConfig) -> Result<()> {
        *self.config.write().await = config;
        info!("Executor configuration updated");
        Ok(())
    }

    /// Run the executor main loop
    pub async fn run(&self) -> Result<()> {
        let (_shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        // Note: In a real implementation, we'd store shutdown_tx somewhere accessible

        let completed_results = Arc::clone(&self.completed_results);

        // Cleanup task for removing old results
        let cleanup_task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 minutes

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let mut results = completed_results.write().await;
                        let cutoff = Instant::now() - Duration::from_secs(3600); // Keep for 1 hour

                        results.retain(|_, result| {
                            // Keep recent results based on when they were created
                            // For now, just keep all results (proper cleanup would need timestamps)
                            true
                        });
                    }
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                }
            }
        });

        cleanup_task
            .await
            .map_err(|e| Error::other(format!("Executor error: {}", e)))?;
        Ok(())
    }

    /// Stop the executor
    pub async fn stop(&self) -> Result<()> {
        // Cancel all active tasks
        let active_tasks = self.active_tasks.read().await;
        for (task_id, handle) in active_tasks.iter() {
            let _ = handle.cancel_tx.send(()).await;
            debug!("Cancelled task {} during shutdown", task_id);
        }
        drop(active_tasks);

        if let Some(shutdown_tx) = &self.shutdown_tx {
            let _ = shutdown_tx.send(()).await;
        }

        info!("Executor stopped");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::CopyRequest;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_executor_creation() {
        let config = ExecutorConfig::default();
        let executor = TaskExecutor::new(config).await.unwrap();

        let (active, completed) = executor.get_execution_stats().await;
        assert_eq!(active, 0);
        assert_eq!(completed, 0);
    }

    #[tokio::test]
    async fn test_task_execution() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.txt");
        let destination = temp_dir.path().join("destination.txt");

        // Create source file
        tokio::fs::write(&source, b"test content").await.unwrap();

        let executor = TaskExecutor::new(ExecutorConfig::default()).await.unwrap();
        let request = CopyRequest::new(source, destination.clone());
        let task = crate::task::Task::new(request);
        let task_id = task.id;

        executor.execute_task(task).await.unwrap();

        let result = executor.wait_for_completion(task_id).await.unwrap();
        assert!(result.is_success());

        // Verify file was copied
        assert!(destination.exists());
        let content = tokio::fs::read_to_string(&destination).await.unwrap();
        assert_eq!(content, "test content");
    }
}
