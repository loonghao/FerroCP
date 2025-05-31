//! Task scheduler for managing and prioritizing copy operations

use crate::task::{Task, TaskId, TaskStatus};
use ferrocp_config::Config;
use ferrocp_types::{Error, Priority, Result};
use std::collections::{BinaryHeap, HashMap};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, warn};

/// Configuration for the task scheduler
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Maximum number of concurrent tasks
    pub max_concurrent_tasks: usize,
    /// Maximum queue size
    pub max_queue_size: usize,
    /// Task timeout duration
    pub task_timeout: Duration,
    /// Enable priority scheduling
    pub enable_priority_scheduling: bool,
    /// Cleanup interval for completed tasks
    pub cleanup_interval: Duration,
}

impl SchedulerConfig {
    /// Create scheduler config from main config
    pub fn from_config(config: &Config) -> Self {
        Self {
            max_concurrent_tasks: config.performance.thread_count.get(),
            max_queue_size: 1000,
            task_timeout: Duration::from_secs(3600), // 1 hour
            enable_priority_scheduling: true,
            cleanup_interval: Duration::from_secs(300), // 5 minutes
        }
    }
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: num_cpus::get(),
            max_queue_size: 1000,
            task_timeout: Duration::from_secs(3600),
            enable_priority_scheduling: true,
            cleanup_interval: Duration::from_secs(300),
        }
    }
}

/// Priority wrapper for tasks in the scheduler queue
#[derive(Debug, Clone)]
struct PriorityTask {
    task: Task,
    priority_score: i32,
    submitted_at: Instant,
}

impl PriorityTask {
    fn new(task: Task) -> Self {
        let priority_score = match task.request.priority {
            Priority::Low => 1,
            Priority::Normal => 5,
            Priority::High => 10,
            Priority::Critical => 20,
        };

        Self {
            task,
            priority_score,
            submitted_at: Instant::now(),
        }
    }
}

impl PartialEq for PriorityTask {
    fn eq(&self, other: &Self) -> bool {
        self.priority_score == other.priority_score
    }
}

impl Eq for PriorityTask {}

impl PartialOrd for PriorityTask {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PriorityTask {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Higher priority first, then older tasks first
        self.priority_score
            .cmp(&other.priority_score)
            .then_with(|| other.submitted_at.cmp(&self.submitted_at))
    }
}

/// Task scheduler that manages task queuing and execution
#[derive(Debug)]
pub struct TaskScheduler {
    config: Arc<RwLock<SchedulerConfig>>,
    pending_queue: Arc<RwLock<BinaryHeap<PriorityTask>>>,
    active_tasks: Arc<RwLock<HashMap<TaskId, Task>>>,
    completed_tasks: Arc<RwLock<HashMap<TaskId, Task>>>,
    #[allow(dead_code)]
    task_tx: mpsc::UnboundedSender<Task>,
    #[allow(dead_code)]
    task_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<Task>>>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl TaskScheduler {
    /// Create a new task scheduler
    pub fn new(config: SchedulerConfig) -> Self {
        let (task_tx, task_rx) = mpsc::unbounded_channel();

        Self {
            config: Arc::new(RwLock::new(config)),
            pending_queue: Arc::new(RwLock::new(BinaryHeap::new())),
            active_tasks: Arc::new(RwLock::new(HashMap::new())),
            completed_tasks: Arc::new(RwLock::new(HashMap::new())),
            task_tx,
            task_rx: Arc::new(RwLock::new(Some(task_rx))),
            shutdown_tx: None,
        }
    }

    /// Submit a task for execution
    pub async fn submit(&self, task: Task) -> Result<()> {
        let config = self.config.read().await;
        let queue_size = self.pending_queue.read().await.len();

        if queue_size >= config.max_queue_size {
            return Err(Error::other("Task queue is full"));
        }

        let enable_priority_scheduling = config.enable_priority_scheduling;
        drop(config);

        let task_id = task.id;
        debug!("Submitting task {} to scheduler", task_id);

        if enable_priority_scheduling {
            let priority_task = PriorityTask::new(task);
            self.pending_queue.write().await.push(priority_task);
        } else {
            // Simple FIFO scheduling
            let priority_task = PriorityTask::new(task);
            self.pending_queue.write().await.push(priority_task);
        }

        info!("Task {} submitted to scheduler", task_id);
        Ok(())
    }

    /// Get the next task to execute
    pub async fn get_next_task(&self) -> Option<Task> {
        let mut queue = self.pending_queue.write().await;
        queue.pop().map(|priority_task| priority_task.task)
    }

    /// Mark a task as started
    pub async fn mark_task_started(&self, mut task: Task) -> Result<()> {
        task.start();
        let task_id = task.id;

        self.active_tasks.write().await.insert(task_id, task);
        debug!("Task {} marked as started", task_id);
        Ok(())
    }

    /// Mark a task as completed
    pub async fn mark_task_completed(&self, task_id: TaskId) -> Result<()> {
        let mut active_tasks = self.active_tasks.write().await;

        if let Some(mut task) = active_tasks.remove(&task_id) {
            task.complete();
            self.completed_tasks.write().await.insert(task_id, task);
            debug!("Task {} marked as completed", task_id);
        } else {
            warn!("Attempted to complete non-existent task {}", task_id);
        }

        Ok(())
    }

    /// Mark a task as failed
    pub async fn mark_task_failed(&self, task_id: TaskId, error: String) -> Result<()> {
        let mut active_tasks = self.active_tasks.write().await;

        if let Some(mut task) = active_tasks.remove(&task_id) {
            task.fail(error);
            self.completed_tasks.write().await.insert(task_id, task);
            debug!("Task {} marked as failed", task_id);
        } else {
            warn!("Attempted to fail non-existent task {}", task_id);
        }

        Ok(())
    }

    /// Get task status
    pub async fn get_task_status(&self, task_id: TaskId) -> Result<Option<TaskStatus>> {
        // Check active tasks first
        if let Some(task) = self.active_tasks.read().await.get(&task_id) {
            return Ok(Some(task.status.clone()));
        }

        // Check completed tasks
        if let Some(task) = self.completed_tasks.read().await.get(&task_id) {
            return Ok(Some(task.status.clone()));
        }

        // Check pending queue
        let queue = self.pending_queue.read().await;
        for priority_task in queue.iter() {
            if priority_task.task.id == task_id {
                return Ok(Some(priority_task.task.status.clone()));
            }
        }

        Ok(None)
    }

    /// Cancel a task
    pub async fn cancel_task(&self, task_id: TaskId) -> Result<()> {
        // Try to remove from pending queue first
        let mut queue = self.pending_queue.write().await;
        let original_len = queue.len();
        *queue = queue.drain().filter(|pt| pt.task.id != task_id).collect();

        if queue.len() < original_len {
            debug!("Task {} cancelled from pending queue", task_id);
            return Ok(());
        }
        drop(queue);

        // Try to cancel active task
        let mut active_tasks = self.active_tasks.write().await;
        if let Some(mut task) = active_tasks.remove(&task_id) {
            task.cancel();
            self.completed_tasks.write().await.insert(task_id, task);
            debug!("Task {} cancelled from active tasks", task_id);
            return Ok(());
        }

        Err(Error::other(format!("Task {} not found", task_id)))
    }

    /// Pause a task
    pub async fn pause_task(&self, task_id: TaskId) -> Result<()> {
        let mut active_tasks = self.active_tasks.write().await;
        if let Some(task) = active_tasks.get_mut(&task_id) {
            task.pause();
            debug!("Task {} paused", task_id);
            Ok(())
        } else {
            Err(Error::other(format!("Active task {} not found", task_id)))
        }
    }

    /// Resume a task
    pub async fn resume_task(&self, task_id: TaskId) -> Result<()> {
        let mut active_tasks = self.active_tasks.write().await;
        if let Some(task) = active_tasks.get_mut(&task_id) {
            task.resume();
            debug!("Task {} resumed", task_id);
            Ok(())
        } else {
            Err(Error::other(format!("Active task {} not found", task_id)))
        }
    }

    /// List all active tasks
    pub async fn list_active_tasks(&self) -> Result<Vec<TaskId>> {
        let active_tasks = self.active_tasks.read().await;
        Ok(active_tasks.keys().copied().collect())
    }

    /// Get queue statistics
    pub async fn get_queue_stats(&self) -> (usize, usize, usize) {
        let pending = self.pending_queue.read().await.len();
        let active = self.active_tasks.read().await.len();
        let completed = self.completed_tasks.read().await.len();
        (pending, active, completed)
    }

    /// Update scheduler configuration
    pub async fn update_config(&self, config: SchedulerConfig) -> Result<()> {
        *self.config.write().await = config;
        info!("Scheduler configuration updated");
        Ok(())
    }

    /// Run the scheduler main loop
    pub async fn run(&self) -> Result<()> {
        let (_shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        // Note: In a real implementation, we'd store shutdown_tx somewhere accessible

        let config = self.config.clone();
        let completed_tasks = self.completed_tasks.clone();

        // Cleanup task for removing old completed tasks
        let cleanup_task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.read().await.cleanup_interval);

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let mut completed = completed_tasks.write().await;
                        let cutoff = Instant::now() - Duration::from_secs(3600); // Keep for 1 hour

                        completed.retain(|_, task| {
                            task.completed_at.map_or(true, |time| time > cutoff)
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
            .map_err(|e| Error::other(format!("Scheduler error: {}", e)))?;
        Ok(())
    }

    /// Stop the scheduler
    pub async fn stop(&self) -> Result<()> {
        if let Some(shutdown_tx) = &self.shutdown_tx {
            let _ = shutdown_tx.send(()).await;
        }
        info!("Scheduler stopped");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::CopyRequest;

    #[tokio::test]
    async fn test_scheduler_creation() {
        let config = SchedulerConfig::default();
        let scheduler = TaskScheduler::new(config);

        let (pending, active, completed) = scheduler.get_queue_stats().await;
        assert_eq!(pending, 0);
        assert_eq!(active, 0);
        assert_eq!(completed, 0);
    }

    #[tokio::test]
    async fn test_task_submission() {
        let scheduler = TaskScheduler::new(SchedulerConfig::default());
        let request = CopyRequest::new("src", "dst");
        let task = Task::new(request);
        let task_id = task.id;

        scheduler.submit(task).await.unwrap();

        let status = scheduler.get_task_status(task_id).await.unwrap();
        assert!(matches!(status, Some(TaskStatus::Pending)));
    }

    #[tokio::test]
    async fn test_priority_scheduling() {
        let scheduler = TaskScheduler::new(SchedulerConfig::default());

        // Submit tasks with different priorities
        let low_task = Task::new(CopyRequest::new("src1", "dst1").with_priority(Priority::Low));
        let high_task = Task::new(CopyRequest::new("src2", "dst2").with_priority(Priority::High));

        scheduler.submit(low_task).await.unwrap();
        scheduler.submit(high_task.clone()).await.unwrap();

        // High priority task should be returned first
        let next_task = scheduler.get_next_task().await.unwrap();
        assert_eq!(next_task.id, high_task.id);
    }
}
