//! Task executor for running copy operations

use crate::selector::EngineSelector;
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
            default_buffer_size: 8 * 1024 * 1024, // 8MB - consistent with BufferSize::DEFAULT
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
pub struct TaskExecutor {
    config: Arc<RwLock<ExecutorConfig>>,
    engine_selector: Arc<EngineSelector>,
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
        let engine_selector = Arc::new(EngineSelector::new());
        let semaphore = Arc::new(Semaphore::new(max_concurrent));

        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            engine_selector,
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
        let engine_selector = Arc::clone(&self.engine_selector);
        let active_tasks = Arc::clone(&self.active_tasks);
        let completed_results = Arc::clone(&self.completed_results);

        // Spawn execution task
        tokio::spawn(async move {
            let start_time = Instant::now();
            let result = tokio::select! {
                result = Self::execute_copy_operation(copy_engine, &task, &config, engine_selector) => {
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
        copy_engine: BufferedCopyEngine,
        task: &Task,
        config: &ExecutorConfig,
        engine_selector: Arc<EngineSelector>,
    ) -> CopyResult {
        let task_id = task.id;
        let start_time = Instant::now();

        info!(
            "Starting copy: {} -> {}",
            task.request.source.display(),
            task.request.destination.display()
        );

        // Check if source is a file or directory
        let source_metadata = match tokio::fs::metadata(&task.request.source).await {
            Ok(metadata) => metadata,
            Err(error) => {
                return CopyResult::failure(
                    task_id,
                    format!(
                        "Failed to open file '{}': {}",
                        task.request.source.display(),
                        error
                    ),
                    start_time.elapsed(),
                );
            }
        };

        if source_metadata.is_file() {
            // Handle file copy
            Self::execute_file_copy(copy_engine, task, config, start_time).await
        } else if source_metadata.is_dir() {
            // Handle directory copy using high-performance engines
            Self::execute_directory_copy(task, config, start_time, engine_selector).await
        } else {
            CopyResult::failure(
                task_id,
                format!(
                    "Source '{}' is neither a file nor a directory",
                    task.request.source.display()
                ),
                start_time.elapsed(),
            )
        }
    }

    /// Execute file copy operation
    async fn execute_file_copy(
        mut copy_engine: BufferedCopyEngine,
        task: &Task,
        config: &ExecutorConfig,
        start_time: Instant,
    ) -> CopyResult {
        let task_id = task.id;

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
            enable_preread: true,   // Enable pre-read by default
            preread_strategy: None, // Auto-detect based on device
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
                    info!(
                        "Copy completed: {} bytes in {:?}",
                        stats.bytes_copied,
                        start_time.elapsed()
                    );
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

    /// Execute directory copy operation
    async fn execute_directory_copy(
        task: &Task,
        config: &ExecutorConfig,
        start_time: Instant,
        engine_selector: Arc<EngineSelector>,
    ) -> CopyResult {
        let task_id = task.id;
        let source = &task.request.source;
        let destination = &task.request.destination;

        // Use high-performance engines for directory copying
        match Self::copy_directory_recursive(source, destination, engine_selector, config).await {
            Ok(stats) => {
                info!(
                    "Directory copy completed: {} -> {}",
                    source.display(),
                    destination.display()
                );
                CopyResult::success(task_id, stats, start_time.elapsed())
            }
            Err(error) => {
                error!("Directory copy failed: {}", error);
                CopyResult::failure(task_id, error.to_string(), start_time.elapsed())
            }
        }
    }

    /// Recursively copy a directory using high-performance engines
    fn copy_directory_recursive<'a>(
        source: &'a std::path::Path,
        destination: &'a std::path::Path,
        engine_selector: Arc<EngineSelector>,
        config: &'a ExecutorConfig,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<ferrocp_types::CopyStats>> + Send + 'a>,
    > {
        Box::pin(async move {
            use ferrocp_types::CopyStats;
            use std::time::Duration;
            use tokio::fs;

            let mut files_copied = 0;
            let mut directories_created = 0;
            let mut bytes_copied = 0;
            let mut files_skipped = 0;
            let mut errors = 0;

            // Create destination directory
            if let Err(e) = fs::create_dir_all(destination).await {
                return Err(Error::other(format!(
                    "Failed to create destination directory '{}': {}",
                    destination.display(),
                    e
                )));
            }
            directories_created += 1;

            // Read source directory
            let mut entries = match fs::read_dir(source).await {
                Ok(entries) => entries,
                Err(e) => {
                    return Err(Error::other(format!(
                        "Failed to read source directory '{}': {}",
                        source.display(),
                        e
                    )));
                }
            };

            while let Some(entry) = entries
                .next_entry()
                .await
                .map_err(|e| Error::other(format!("Failed to read directory entry: {}", e)))?
            {
                let source_path = entry.path();
                let file_name = match source_path.file_name() {
                    Some(name) => name,
                    None => {
                        files_skipped += 1;
                        continue;
                    }
                };
                let dest_path = destination.join(file_name);

                let metadata = match entry.metadata().await {
                    Ok(metadata) => metadata,
                    Err(e) => {
                        warn!(
                            "Failed to get metadata for '{}': {}",
                            source_path.display(),
                            e
                        );
                        errors += 1;
                        continue;
                    }
                };

                if metadata.is_file() {
                    // Copy file using high-performance engines
                    match Self::copy_single_file_with_engine(
                        &source_path,
                        &dest_path,
                        &engine_selector,
                        config,
                    )
                    .await
                    {
                        Ok(stats) => {
                            files_copied += 1;
                            bytes_copied += stats.bytes_copied;
                            debug!(
                                "Copied file: {} -> {} ({} bytes)",
                                source_path.display(),
                                dest_path.display(),
                                stats.bytes_copied
                            );
                        }
                        Err(e) => {
                            warn!("Failed to copy file '{}': {}", source_path.display(), e);
                            errors += 1;
                        }
                    }
                } else if metadata.is_dir() {
                    // Recursively copy subdirectory
                    match Self::copy_directory_recursive(
                        &source_path,
                        &dest_path,
                        Arc::clone(&engine_selector),
                        config,
                    )
                    .await
                    {
                        Ok(sub_stats) => {
                            files_copied += sub_stats.files_copied;
                            directories_created += sub_stats.directories_created;
                            bytes_copied += sub_stats.bytes_copied;
                            files_skipped += sub_stats.files_skipped;
                            errors += sub_stats.errors;
                        }
                        Err(e) => {
                            warn!(
                                "Failed to copy directory '{}': {}",
                                source_path.display(),
                                e
                            );
                            errors += 1;
                        }
                    }
                } else {
                    // Skip special files (symlinks, etc.)
                    files_skipped += 1;
                }
            }

            Ok(CopyStats {
                files_copied,
                directories_created,
                bytes_copied,
                files_skipped,
                errors,
                duration: Duration::from_secs(0), // Will be set by caller
                zerocopy_operations: 0,
                zerocopy_bytes: 0,
            })
        })
    }

    /// Copy a single file using the optimal engine selected by EngineSelector
    async fn copy_single_file_with_engine(
        source: &std::path::Path,
        destination: &std::path::Path,
        engine_selector: &EngineSelector,
        _config: &ExecutorConfig,
    ) -> Result<ferrocp_types::CopyStats> {
        use crate::selector::EngineType;
        use ferrocp_io::CopyEngine as IoCopyEngine;

        // Select the optimal engine for this file
        let selection = engine_selector
            .select_optimal_engine(source, destination)
            .await?;

        debug!(
            "Selected engine {:?} for file {} -> {} ({})",
            selection.engine_type,
            source.display(),
            destination.display(),
            selection.reasoning
        );

        // Execute copy using the selected engine
        match selection.engine_type {
            EngineType::MicroFile => {
                let engine = engine_selector.get_micro_engine().await;
                let mut engine_guard = engine.lock().await;
                engine_guard
                    .copy_file_with_options(source, destination, selection.copy_options)
                    .await
            }
            EngineType::Buffered => {
                let engine = engine_selector.get_buffered_engine().await;
                let mut engine_guard = engine.lock().await;
                engine_guard
                    .copy_file_with_options(source, destination, selection.copy_options)
                    .await
            }
            EngineType::Parallel => {
                let engine = engine_selector.get_parallel_engine().await;
                let mut engine_guard = engine.lock().await;
                engine_guard
                    .copy_file_with_options(source, destination, selection.copy_options)
                    .await
            }
            EngineType::ZeroCopy => {
                // For zero-copy, we still use buffered engine but with zero-copy options
                let engine = engine_selector.get_buffered_engine().await;
                let mut engine_guard = engine.lock().await;
                engine_guard
                    .copy_file_with_options(source, destination, selection.copy_options)
                    .await
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
                        let _cutoff = Instant::now() - Duration::from_secs(3600); // Keep for 1 hour

                        results.retain(|_, _result| {
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

impl std::fmt::Debug for TaskExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskExecutor")
            .field("config", &self.config)
            .field("engine_selector", &"Arc<EngineSelector>")
            .field("semaphore", &self.semaphore)
            .field("active_tasks", &self.active_tasks)
            .field("completed_results", &self.completed_results)
            .field("shutdown_tx", &self.shutdown_tx)
            .finish()
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
    async fn test_buffer_size_configuration() {
        // Test that default ExecutorConfig uses 8MB buffer size
        let config = ExecutorConfig::default();
        assert_eq!(config.default_buffer_size, 8 * 1024 * 1024); // 8MB

        // Test that from_config uses the configured buffer size
        use ferrocp_config::Config;
        let main_config = Config::default();
        let executor_config = ExecutorConfig::from_config(&main_config);
        assert_eq!(
            executor_config.default_buffer_size,
            main_config.performance.buffer_size.get()
        );
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

    #[tokio::test]
    async fn test_directory_copy_with_high_performance_engines() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source_dir");
        let dest_dir = temp_dir.path().join("dest_dir");

        // Create source directory with files
        tokio::fs::create_dir_all(&source_dir).await.unwrap();
        tokio::fs::write(source_dir.join("file1.txt"), b"content1")
            .await
            .unwrap();
        tokio::fs::write(source_dir.join("file2.txt"), b"content2")
            .await
            .unwrap();

        // Create subdirectory
        let sub_dir = source_dir.join("subdir");
        tokio::fs::create_dir_all(&sub_dir).await.unwrap();
        tokio::fs::write(sub_dir.join("file3.txt"), b"content3")
            .await
            .unwrap();

        let executor = TaskExecutor::new(ExecutorConfig::default()).await.unwrap();
        let request = CopyRequest::new(source_dir, dest_dir.clone());
        let task = crate::task::Task::new(request);
        let task_id = task.id;

        executor.execute_task(task).await.unwrap();

        let result = executor.wait_for_completion(task_id).await.unwrap();
        assert!(result.is_success());

        // Verify directory structure was copied
        assert!(dest_dir.exists());
        assert!(dest_dir.join("file1.txt").exists());
        assert!(dest_dir.join("file2.txt").exists());
        assert!(dest_dir.join("subdir").exists());
        assert!(dest_dir.join("subdir").join("file3.txt").exists());

        // Verify file contents
        let content1 = tokio::fs::read_to_string(dest_dir.join("file1.txt"))
            .await
            .unwrap();
        assert_eq!(content1, "content1");
        let content3 = tokio::fs::read_to_string(dest_dir.join("subdir").join("file3.txt"))
            .await
            .unwrap();
        assert_eq!(content3, "content3");

        // Verify statistics
        let stats = &result.stats;
        assert_eq!(stats.files_copied, 3);
        assert_eq!(stats.directories_created, 2); // source_dir + subdir
        assert!(stats.bytes_copied > 0);
    }
}
