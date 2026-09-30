//! Main copy engine implementation

use crate::{
    executor::{ExecutorConfig, TaskExecutor},
    monitor::{ProgressMonitor, StatisticsCollector},
    scheduler::{SchedulerConfig, TaskScheduler},
    task::{CopyRequest, CopyResult, Task, TaskId},
};
use ferrocp_config::{Config, ConfigLoader};
use ferrocp_types::Result;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

/// How often the dispatch loop looks for queued tasks
const DISPATCH_POLL_INTERVAL: Duration = Duration::from_millis(10);

/// Shared state of the scheduler to executor dispatch loop
///
/// `CopyEngine::execute` submits a task to the scheduler and then waits for the
/// executor to finish it. The loop that moves tasks from one to the other used
/// to be spawned only by `CopyEngine::start`, which neither the Python bindings
/// nor the FFI ever call, so a copy driven through them stayed `Pending` in the
/// scheduler until the executor's one hour timeout fired.
///
/// The loop is now started on demand by `execute` and shut down when the engine
/// is explicitly stopped or dropped, so every caller gets its task dispatched
/// whether or not it called `start` first. The state lives behind an `Arc`
/// because `CopyEngine` is `Clone`: clones share one loop, and the loop stops
/// only once the last clone is gone.
#[derive(Debug)]
struct DispatchLoop {
    /// Shutdown channel of the running loop, `None` while no loop is running.
    shutdown_tx: Mutex<Option<mpsc::Sender<()>>>,
}

impl DispatchLoop {
    /// Create dispatch state with no loop running
    fn new() -> Self {
        Self {
            shutdown_tx: Mutex::new(None),
        }
    }

    /// Start the dispatch loop unless one is already running
    ///
    /// Callers must be inside a Tokio runtime; `execute` and `start` both are.
    /// The lock is held across `tokio::spawn` on purpose so two concurrent
    /// callers cannot each decide that they are the one that has to spawn.
    fn ensure_started(&self, scheduler: &Arc<TaskScheduler>, executor: &Arc<TaskExecutor>) {
        let mut slot = self.shutdown_tx.lock().unwrap_or_else(|e| e.into_inner());
        if slot.is_some() {
            return;
        }

        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        *slot = Some(shutdown_tx);
        spawn_dispatch_loop(Arc::clone(scheduler), Arc::clone(executor), shutdown_rx);
    }

    /// Stop the dispatch loop, if one is running
    fn stop(&self) {
        let mut slot = self.shutdown_tx.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(shutdown_tx) = slot.take() {
            // `try_send` never blocks and the channel is never full: this is the
            // only sender and it sends exactly once.
            let _ = shutdown_tx.try_send(());
        }
    }
}

impl Drop for DispatchLoop {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Move queued tasks from the scheduler to the executor until shut down
///
/// Runs as a background task. Shutting it down is best effort: a task already
/// handed to the executor keeps running to completion.
fn spawn_dispatch_loop(
    scheduler: Arc<TaskScheduler>,
    executor: Arc<TaskExecutor>,
    mut shutdown_rx: mpsc::Receiver<()>,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(DISPATCH_POLL_INTERVAL);

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    debug!("Copy engine dispatch loop stopped");
                    return;
                }
                _ = interval.tick() => {
                    // Drain the queue: concurrent callers can leave several
                    // tasks waiting behind a single tick.
                    while let Some(task) = scheduler.get_next_task().await {
                        debug!("Processing task {} from scheduler", task.id);

                        // Mark task as started in scheduler
                        if let Err(e) = scheduler.mark_task_started(task.clone()).await {
                            warn!("Failed to mark task as started: {}", e);
                            continue;
                        }

                        // Execute task
                        if let Err(e) = executor.execute_task(task.clone()).await {
                            warn!("Failed to execute task {}: {}", task.id, e);
                            // Mark task as failed
                            let _ = scheduler
                                .mark_task_failed(task.id, e.to_string())
                                .await;
                        }
                    }
                }
            }
        }
    });
}

/// Main copy engine that orchestrates all operations
#[derive(Debug, Clone)]
pub struct CopyEngine {
    config: Arc<Config>,
    scheduler: Arc<TaskScheduler>,
    executor: Arc<TaskExecutor>,
    progress_monitor: Arc<ProgressMonitor>,
    statistics: Arc<StatisticsCollector>,
    /// Whether `start` was called and `stop` has not been called since
    started: bool,
    dispatch: Arc<DispatchLoop>,
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
            started: false,
            dispatch: Arc::new(DispatchLoop::new()),
        })
    }

    /// Start the copy engine
    ///
    /// Optional: `execute` starts the dispatch loop on demand, so calling this
    /// only matters for callers that want the loop running before the first
    /// copy is submitted. An engine that was started must be stopped.
    pub async fn start(&mut self) -> Result<()> {
        self.dispatch
            .ensure_started(&self.scheduler, &self.executor);
        self.started = true;

        info!("Copy engine started");
        Ok(())
    }

    /// Stop the copy engine
    pub async fn stop(&mut self) -> Result<()> {
        self.dispatch.stop();
        self.started = false;

        // Stop components
        self.scheduler.stop().await?;
        self.executor.stop().await?;
        self.progress_monitor.stop().await?;
        self.statistics.stop().await?;

        info!("Copy engine stopped");
        Ok(())
    }

    /// Execute a copy request
    ///
    /// Runs the copy to completion without any other setup: the dispatch loop
    /// that feeds the scheduler's queue to the executor is started here when
    /// nothing else started it, so callers that never call `start` still get
    /// their task executed instead of waiting for the executor's timeout.
    pub async fn execute(&self, request: CopyRequest) -> Result<CopyResult> {
        debug!("Executing copy request: {:?}", request);

        let task = Task::new(request);
        let task_id = task.id;

        // Submit task to scheduler
        self.scheduler.submit(task).await?;

        self.dispatch
            .ensure_started(&self.scheduler, &self.executor);

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
        if self.started {
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

    /// `execute` submits to the scheduler and then waits on the executor, so it
    /// only terminates if the dispatch loop between the two is running. That
    /// loop used to be started by `start()` alone, which the Python bindings
    /// and the FFI never call, so their copies hung until the executor's one
    /// hour timeout fired.
    ///
    /// The 15 second envelope is the assertion: a dispatched copy finishes in
    /// milliseconds, a copy that never gets dispatched does not finish at all.
    async fn execute_within_timeout(engine: &CopyEngine, request: CopyRequest) -> CopyResult {
        let outcome =
            tokio::time::timeout(std::time::Duration::from_secs(15), engine.execute(request)).await;
        assert!(outcome.is_ok(), "copy was never dispatched by the engine");
        outcome.unwrap().expect("copy failed")
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_execute_dispatches_without_start() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.txt");
        let destination = temp_dir.path().join("destination.txt");
        let payload = b"payload".repeat(10_000);
        tokio::fs::write(&source, &payload).await.unwrap();

        let engine = CopyEngine::with_config(Config::default()).await.unwrap();
        let result = execute_within_timeout(&engine, CopyRequest::new(&source, &destination)).await;

        assert!(
            result.is_success(),
            "unexpected failure: {:?}",
            result.error
        );
        assert!(destination.exists(), "destination was not created");
        assert_eq!(std::fs::read(&destination).unwrap(), payload);
        assert_eq!(result.stats.bytes_copied, payload.len() as u64);
    }

    /// An engine that *is* started must still work, and must not run two
    /// dispatch loops: a task handed to the executor twice would be copied
    /// twice.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_execute_dispatches_after_start() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.txt");
        let destination = temp_dir.path().join("destination.txt");
        tokio::fs::write(&source, b"started engine payload")
            .await
            .unwrap();

        let mut engine = CopyEngine::with_config(Config::default()).await.unwrap();
        engine.start().await.unwrap();

        let result = execute_within_timeout(&engine, CopyRequest::new(&source, &destination)).await;

        assert!(
            result.is_success(),
            "unexpected failure: {:?}",
            result.error
        );
        assert_eq!(
            std::fs::read(&destination).unwrap(),
            b"started engine payload"
        );

        engine.stop().await.unwrap();
    }

    /// Several concurrent `execute` calls share one dispatch loop. Every one of
    /// them has to be dispatched, not just the first.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_concurrent_executes_are_all_dispatched() {
        let temp_dir = TempDir::new().unwrap();
        let engine = CopyEngine::with_config(Config::default()).await.unwrap();

        let mut handles = Vec::new();
        for index in 0..8u32 {
            let source = temp_dir.path().join(format!("source-{index}.txt"));
            let destination = temp_dir.path().join(format!("destination-{index}.txt"));
            let payload = format!("payload {index}").into_bytes();
            tokio::fs::write(&source, &payload).await.unwrap();

            let engine = engine.clone();
            handles.push(tokio::spawn(async move {
                let result = engine
                    .execute(CopyRequest::new(&source, &destination))
                    .await;
                (destination, payload, result)
            }));
        }

        let outcome = tokio::time::timeout(std::time::Duration::from_secs(30), async {
            for handle in handles {
                let (destination, payload, result) = handle.await.unwrap();
                let result = result.expect("copy failed");
                assert!(
                    result.is_success(),
                    "unexpected failure: {:?}",
                    result.error
                );
                assert_eq!(std::fs::read(&destination).unwrap(), payload);
            }
        })
        .await;

        assert!(outcome.is_ok(), "a concurrent copy was never dispatched");
    }

    /// A copy that fails is reported as a failed result rather than hanging:
    /// the dispatch loop has to hand the task over even when it cannot succeed.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_execute_reports_failure_without_hanging() {
        let temp_dir = TempDir::new().unwrap();
        let missing = temp_dir.path().join("does-not-exist.txt");
        let destination = temp_dir.path().join("destination.txt");

        let engine = CopyEngine::with_config(Config::default()).await.unwrap();

        let outcome = tokio::time::timeout(
            std::time::Duration::from_secs(15),
            engine.execute(CopyRequest::new(&missing, &destination)),
        )
        .await;

        assert!(outcome.is_ok(), "a failing copy was never dispatched");
        let result = outcome.unwrap().expect("copy should not return Err");
        assert!(!result.is_success());
        assert!(result.error.is_some(), "a failed copy must carry a message");
        assert!(!destination.exists());
    }
}
