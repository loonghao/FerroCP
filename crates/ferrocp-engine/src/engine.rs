//! Main copy engine implementation

use crate::{
    executor::{ExecutorConfig, TaskExecutor},
    monitor::{ProgressMonitor, StatisticsCollector},
    scheduler::{SchedulerConfig, TaskScheduler},
    task::{CopyRequest, CopyResult, Task, TaskId},
};
use ferrocp_config::{Config, ConfigLoader};
use ferrocp_types::Result;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

/// Main copy engine that orchestrates all operations
#[derive(Debug, Clone)]
pub struct CopyEngine {
    config: Arc<Config>,
    scheduler: Arc<TaskScheduler>,
    executor: Arc<TaskExecutor>,
    progress_monitor: Arc<ProgressMonitor>,
    statistics: Arc<StatisticsCollector>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl CopyEngine {
    /// Create a new copy engine with default configuration
    pub async fn new() -> Result<Self> {
        let config = ConfigLoader::load_default()?;
        Self::with_config(config).await
    }

    /// Create a new copy engine with custom configuration
    pub async fn with_config(config: Config) -> Result<Self> {
        let config = Arc::new(config);

        // Create components
        let scheduler_config = SchedulerConfig::from_config(&config);
        let scheduler = Arc::new(TaskScheduler::new(scheduler_config));

        let executor_config = ExecutorConfig::from_config(&config);
        let executor = Arc::new(TaskExecutor::new(executor_config).await?);

        let progress_monitor = Arc::new(ProgressMonitor::new());
        let statistics = Arc::new(StatisticsCollector::new());

        info!("Copy engine initialized successfully");

        Ok(Self {
            config,
            scheduler,
            executor,
            progress_monitor,
            statistics,
            shutdown_tx: None,
        })
    }

    /// Start the copy engine
    pub async fn start(&mut self) -> Result<()> {
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);
        self.shutdown_tx = Some(shutdown_tx);

        // Start background tasks
        let _scheduler = Arc::clone(&self.scheduler);
        let _executor = Arc::clone(&self.executor);
        let _progress_monitor = Arc::clone(&self.progress_monitor);
        let _statistics = Arc::clone(&self.statistics);

        // Start the main task processing loop
        let scheduler_for_loop = Arc::clone(&self.scheduler);
        let executor_for_loop = Arc::clone(&self.executor);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));

            loop {
                interval.tick().await;

                // Get next task from scheduler
                if let Some(task) = scheduler_for_loop.get_next_task().await {
                    debug!("Processing task {} from scheduler", task.id);

                    // Mark task as started in scheduler
                    if let Err(e) = scheduler_for_loop.mark_task_started(task.clone()).await {
                        warn!("Failed to mark task as started: {}", e);
                        continue;
                    }

                    // Execute task
                    if let Err(e) = executor_for_loop.execute_task(task.clone()).await {
                        warn!("Failed to execute task {}: {}", task.id, e);
                        // Mark task as failed
                        let _ = scheduler_for_loop
                            .mark_task_failed(task.id, e.to_string())
                            .await;
                    }
                }
            }
        });

        // For now, just start a simple background task to keep the engine alive
        tokio::spawn(async move {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    info!("Copy engine shutdown requested");
                }
            }
        });

        info!("Copy engine started");
        Ok(())
    }

    /// Stop the copy engine
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(()).await;
        }

        // Stop components
        self.scheduler.stop().await?;
        self.executor.stop().await?;
        self.progress_monitor.stop().await?;
        self.statistics.stop().await?;

        info!("Copy engine stopped");
        Ok(())
    }

    /// Execute a copy request
    pub async fn execute(&self, request: CopyRequest) -> Result<CopyResult> {
        debug!("Executing copy request: {:?}", request);

        let task = Task::new(request);
        let task_id = task.id;

        // Submit task to scheduler
        self.scheduler.submit(task).await?;

        // Wait for completion
        self.wait_for_completion(task_id).await
    }

    /// Submit a copy request for asynchronous execution
    pub async fn submit(&self, request: CopyRequest) -> Result<TaskId> {
        debug!("Submitting copy request: {:?}", request);

        let task = Task::new(request);
        let task_id = task.id;

        // Submit task to scheduler
        self.scheduler.submit(task).await?;

        Ok(task_id)
    }

    /// Wait for a task to complete
    pub async fn wait_for_completion(&self, task_id: TaskId) -> Result<CopyResult> {
        self.executor.wait_for_completion(task_id).await
    }

    /// Get the status of a task
    pub async fn get_task_status(
        &self,
        task_id: TaskId,
    ) -> Result<Option<crate::task::TaskStatus>> {
        self.scheduler.get_task_status(task_id).await
    }

    /// Cancel a task
    pub async fn cancel_task(&self, task_id: TaskId) -> Result<()> {
        self.scheduler.cancel_task(task_id).await
    }

    /// Pause a task
    pub async fn pause_task(&self, task_id: TaskId) -> Result<()> {
        self.scheduler.pause_task(task_id).await
    }

    /// Resume a task
    pub async fn resume_task(&self, task_id: TaskId) -> Result<()> {
        self.scheduler.resume_task(task_id).await
    }

    /// Get current statistics
    pub async fn get_statistics(&self) -> crate::monitor::Statistics {
        self.statistics.get_current_stats().await
    }

    /// Get progress information for a task
    pub async fn get_progress(
        &self,
        task_id: TaskId,
    ) -> Result<Option<crate::monitor::ProgressInfo>> {
        self.progress_monitor.get_progress(task_id).await
    }

    /// List all active tasks
    pub async fn list_active_tasks(&self) -> Result<Vec<TaskId>> {
        self.scheduler.list_active_tasks().await
    }

    /// Get the current configuration
    pub fn get_config(&self) -> &Config {
        &self.config
    }

    /// Update the configuration
    pub async fn update_config(&mut self, config: Config) -> Result<()> {
        self.config = Arc::new(config);

        // Update component configurations
        let scheduler_config = SchedulerConfig::from_config(&self.config);
        self.scheduler.update_config(scheduler_config).await?;

        let executor_config = ExecutorConfig::from_config(&self.config);
        self.executor.update_config(executor_config).await?;

        info!("Configuration updated");
        Ok(())
    }
}

impl Drop for CopyEngine {
    fn drop(&mut self) {
        if self.shutdown_tx.is_some() {
            warn!("Copy engine dropped without proper shutdown");
        }
    }
}

/// Builder for creating a copy engine with custom configuration
#[derive(Debug, Default)]
pub struct EngineBuilder {
    config: Option<Config>,
    scheduler_config: Option<SchedulerConfig>,
    executor_config: Option<ExecutorConfig>,
}

impl EngineBuilder {
    /// Create a new engine builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the configuration
    pub fn with_config(mut self, config: Config) -> Self {
        self.config = Some(config);
        self
    }

    /// Set the scheduler configuration
    pub fn with_scheduler_config(mut self, config: SchedulerConfig) -> Self {
        self.scheduler_config = Some(config);
        self
    }

    /// Set the executor configuration
    pub fn with_executor_config(mut self, config: ExecutorConfig) -> Self {
        self.executor_config = Some(config);
        self
    }

    /// Build the copy engine
    pub async fn build(self) -> Result<CopyEngine> {
        let config = match self.config {
            Some(config) => config,
            None => ConfigLoader::load_default()?,
        };

        CopyEngine::with_config(config).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::CopyRequest;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_engine_creation() {
        let config = Config::default();
        let engine = CopyEngine::with_config(config).await.unwrap();
        assert!(engine.config.performance.enable_zero_copy);
    }

    #[tokio::test]
    async fn test_engine_builder() {
        let config = Config::default();
        let engine = EngineBuilder::new()
            .with_config(config)
            .build()
            .await
            .unwrap();

        assert!(engine.get_config().performance.enable_zero_copy);
    }

    #[tokio::test]
    async fn test_task_submission() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.txt");
        let destination = temp_dir.path().join("destination.txt");

        // Create source file
        tokio::fs::write(&source, b"test content").await.unwrap();

        let config = Config::default();
        let engine = CopyEngine::with_config(config).await.unwrap();
        let request = CopyRequest::new(source, destination);

        let task_id = engine.submit(request).await.unwrap();
        assert!(!task_id.as_uuid().is_nil());
    }
}
