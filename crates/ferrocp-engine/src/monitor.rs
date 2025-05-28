//! Progress monitoring and statistics collection

use crate::task::TaskId;
use ferrocp_types::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info};

/// Progress information for a specific task
#[derive(Debug, Clone)]
pub struct ProgressInfo {
    /// Task ID
    pub task_id: TaskId,
    /// Current file being processed
    pub current_file: String,
    /// Bytes processed for current file
    pub current_file_bytes: u64,
    /// Total size of current file
    pub current_file_size: u64,
    /// Total files processed
    pub files_processed: u64,
    /// Total number of files
    pub total_files: u64,
    /// Total bytes processed
    pub bytes_processed: u64,
    /// Total bytes to process
    pub total_bytes: u64,
    /// Current transfer rate (bytes/sec)
    pub transfer_rate: f64,
    /// Estimated time remaining
    pub eta: Option<Duration>,
    /// Last update time
    pub last_update: Instant,
}

impl ProgressInfo {
    /// Calculate progress percentage for current file
    pub fn current_file_progress(&self) -> f64 {
        if self.current_file_size > 0 {
            (self.current_file_bytes as f64 / self.current_file_size as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Calculate overall progress percentage
    pub fn overall_progress(&self) -> f64 {
        if self.total_bytes > 0 {
            (self.bytes_processed as f64 / self.total_bytes as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Calculate files progress percentage
    pub fn files_progress(&self) -> f64 {
        if self.total_files > 0 {
            (self.files_processed as f64 / self.total_files as f64) * 100.0
        } else {
            0.0
        }
    }
}

/// Overall statistics for the copy engine
#[derive(Debug, Clone)]
pub struct Statistics {
    /// Total tasks submitted
    pub total_tasks: u64,
    /// Tasks currently running
    pub running_tasks: u64,
    /// Tasks completed successfully
    pub completed_tasks: u64,
    /// Tasks that failed
    pub failed_tasks: u64,
    /// Tasks that were cancelled
    pub cancelled_tasks: u64,
    /// Total bytes processed
    pub total_bytes_processed: u64,
    /// Total files processed
    pub total_files_processed: u64,
    /// Average transfer rate (bytes/sec)
    pub average_transfer_rate: f64,
    /// Peak transfer rate (bytes/sec)
    pub peak_transfer_rate: f64,
    /// Total execution time
    pub total_execution_time: Duration,
    /// Engine uptime
    pub uptime: Duration,
    /// Last update time
    pub last_update: Instant,
}

impl Default for Statistics {
    fn default() -> Self {
        Self {
            total_tasks: 0,
            running_tasks: 0,
            completed_tasks: 0,
            failed_tasks: 0,
            cancelled_tasks: 0,
            total_bytes_processed: 0,
            total_files_processed: 0,
            average_transfer_rate: 0.0,
            peak_transfer_rate: 0.0,
            total_execution_time: Duration::from_secs(0),
            uptime: Duration::from_secs(0),
            last_update: Instant::now(),
        }
    }
}

impl Statistics {
    /// Calculate overall success rate
    pub fn success_rate(&self) -> f64 {
        let total_finished = self.completed_tasks + self.failed_tasks + self.cancelled_tasks;
        if total_finished > 0 {
            (self.completed_tasks as f64 / total_finished as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Calculate throughput (files/sec)
    pub fn files_per_second(&self) -> f64 {
        if self.total_execution_time.as_secs_f64() > 0.0 {
            self.total_files_processed as f64 / self.total_execution_time.as_secs_f64()
        } else {
            0.0
        }
    }
}

/// Progress monitor for tracking task progress
#[derive(Debug)]
pub struct ProgressMonitor {
    progress_info: Arc<RwLock<HashMap<TaskId, ProgressInfo>>>,
    update_tx: mpsc::UnboundedSender<ProgressUpdate>,
    update_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<ProgressUpdate>>>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

#[derive(Debug)]
enum ProgressUpdate {
    Update(ProgressInfo),
    Remove(TaskId),
}

impl ProgressMonitor {
    /// Create a new progress monitor
    pub fn new() -> Self {
        let (update_tx, update_rx) = mpsc::unbounded_channel();

        Self {
            progress_info: Arc::new(RwLock::new(HashMap::new())),
            update_tx,
            update_rx: Arc::new(RwLock::new(Some(update_rx))),
            shutdown_tx: None,
        }
    }

    /// Update progress for a task
    pub async fn update_progress(&self, progress: ProgressInfo) -> Result<()> {
        self.update_tx.send(ProgressUpdate::Update(progress))
            .map_err(|e| ferrocp_types::Error::other(format!("Failed to send progress update: {}", e)))?;
        Ok(())
    }

    /// Remove progress tracking for a task
    pub async fn remove_task(&self, task_id: TaskId) -> Result<()> {
        self.update_tx.send(ProgressUpdate::Remove(task_id))
            .map_err(|e| ferrocp_types::Error::other(format!("Failed to send remove update: {}", e)))?;
        Ok(())
    }

    /// Get progress information for a task
    pub async fn get_progress(&self, task_id: TaskId) -> Result<Option<ProgressInfo>> {
        let progress_info = self.progress_info.read().await;
        Ok(progress_info.get(&task_id).cloned())
    }

    /// Get progress for all tasks
    pub async fn get_all_progress(&self) -> HashMap<TaskId, ProgressInfo> {
        self.progress_info.read().await.clone()
    }

    /// Run the progress monitor
    pub async fn run(&self) -> Result<()> {
        let mut update_rx = self.update_rx.write().await.take()
            .ok_or_else(|| ferrocp_types::Error::other("Progress monitor already running"))?;

        let progress_info = Arc::clone(&self.progress_info);
        let (_shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    update = update_rx.recv() => {
                        match update {
                            Some(ProgressUpdate::Update(progress)) => {
                                let task_id = progress.task_id;
                                progress_info.write().await.insert(task_id, progress);
                                debug!("Updated progress for task {}", task_id);
                            }
                            Some(ProgressUpdate::Remove(task_id)) => {
                                progress_info.write().await.remove(&task_id);
                                debug!("Removed progress tracking for task {}", task_id);
                            }
                            None => {
                                debug!("Progress monitor update channel closed");
                                break;
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Progress monitor shutdown requested");
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop the progress monitor
    pub async fn stop(&self) -> Result<()> {
        if let Some(shutdown_tx) = &self.shutdown_tx {
            let _ = shutdown_tx.send(()).await;
        }
        info!("Progress monitor stopped");
        Ok(())
    }
}

impl Default for ProgressMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics collector for gathering engine metrics
#[derive(Debug)]
pub struct StatisticsCollector {
    statistics: Arc<RwLock<Statistics>>,
    start_time: Instant,
    update_tx: mpsc::UnboundedSender<StatisticsUpdate>,
    update_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<StatisticsUpdate>>>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

#[derive(Debug)]
enum StatisticsUpdate {
    TaskSubmitted,
    TaskStarted,
    TaskCompleted { bytes_processed: u64, files_processed: u64, execution_time: Duration },
    TaskFailed,
    TaskCancelled,
    TransferRate(f64),
}

impl StatisticsCollector {
    /// Create a new statistics collector
    pub fn new() -> Self {
        let (update_tx, update_rx) = mpsc::unbounded_channel();

        Self {
            statistics: Arc::new(RwLock::new(Statistics::default())),
            start_time: Instant::now(),
            update_tx,
            update_rx: Arc::new(RwLock::new(Some(update_rx))),
            shutdown_tx: None,
        }
    }

    /// Record a task submission
    pub async fn record_task_submitted(&self) -> Result<()> {
        self.update_tx.send(StatisticsUpdate::TaskSubmitted)
            .map_err(|e| ferrocp_types::Error::other(format!("Failed to send statistics update: {}", e)))?;
        Ok(())
    }

    /// Record a task start
    pub async fn record_task_started(&self) -> Result<()> {
        self.update_tx.send(StatisticsUpdate::TaskStarted)
            .map_err(|e| ferrocp_types::Error::other(format!("Failed to send statistics update: {}", e)))?;
        Ok(())
    }

    /// Record a task completion
    pub async fn record_task_completed(&self, bytes_processed: u64, files_processed: u64, execution_time: Duration) -> Result<()> {
        self.update_tx.send(StatisticsUpdate::TaskCompleted {
            bytes_processed,
            files_processed,
            execution_time,
        }).map_err(|e| ferrocp_types::Error::other(format!("Failed to send statistics update: {}", e)))?;
        Ok(())
    }

    /// Record a task failure
    pub async fn record_task_failed(&self) -> Result<()> {
        self.update_tx.send(StatisticsUpdate::TaskFailed)
            .map_err(|e| ferrocp_types::Error::other(format!("Failed to send statistics update: {}", e)))?;
        Ok(())
    }

    /// Record a task cancellation
    pub async fn record_task_cancelled(&self) -> Result<()> {
        self.update_tx.send(StatisticsUpdate::TaskCancelled)
            .map_err(|e| ferrocp_types::Error::other(format!("Failed to send statistics update: {}", e)))?;
        Ok(())
    }

    /// Record transfer rate
    pub async fn record_transfer_rate(&self, rate: f64) -> Result<()> {
        self.update_tx.send(StatisticsUpdate::TransferRate(rate))
            .map_err(|e| ferrocp_types::Error::other(format!("Failed to send statistics update: {}", e)))?;
        Ok(())
    }

    /// Get current statistics
    pub async fn get_current_stats(&self) -> Statistics {
        let mut stats = self.statistics.read().await.clone();
        stats.uptime = self.start_time.elapsed();
        stats.last_update = Instant::now();
        stats
    }

    /// Run the statistics collector
    pub async fn run(&self) -> Result<()> {
        let mut update_rx = self.update_rx.write().await.take()
            .ok_or_else(|| ferrocp_types::Error::other("Statistics collector already running"))?;

        let statistics = Arc::clone(&self.statistics);
        let (_shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    update = update_rx.recv() => {
                        match update {
                            Some(update) => {
                                let mut stats = statistics.write().await;
                                match update {
                                    StatisticsUpdate::TaskSubmitted => {
                                        stats.total_tasks += 1;
                                    }
                                    StatisticsUpdate::TaskStarted => {
                                        stats.running_tasks += 1;
                                    }
                                    StatisticsUpdate::TaskCompleted { bytes_processed, files_processed, execution_time } => {
                                        stats.running_tasks = stats.running_tasks.saturating_sub(1);
                                        stats.completed_tasks += 1;
                                        stats.total_bytes_processed += bytes_processed;
                                        stats.total_files_processed += files_processed;
                                        stats.total_execution_time += execution_time;
                                        
                                        // Update average transfer rate
                                        if stats.total_execution_time.as_secs_f64() > 0.0 {
                                            stats.average_transfer_rate = stats.total_bytes_processed as f64 / stats.total_execution_time.as_secs_f64();
                                        }
                                    }
                                    StatisticsUpdate::TaskFailed => {
                                        stats.running_tasks = stats.running_tasks.saturating_sub(1);
                                        stats.failed_tasks += 1;
                                    }
                                    StatisticsUpdate::TaskCancelled => {
                                        stats.running_tasks = stats.running_tasks.saturating_sub(1);
                                        stats.cancelled_tasks += 1;
                                    }
                                    StatisticsUpdate::TransferRate(rate) => {
                                        if rate > stats.peak_transfer_rate {
                                            stats.peak_transfer_rate = rate;
                                        }
                                    }
                                }
                            }
                            None => {
                                debug!("Statistics collector update channel closed");
                                break;
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Statistics collector shutdown requested");
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop the statistics collector
    pub async fn stop(&self) -> Result<()> {
        if let Some(shutdown_tx) = &self.shutdown_tx {
            let _ = shutdown_tx.send(()).await;
        }
        info!("Statistics collector stopped");
        Ok(())
    }
}

impl Default for StatisticsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_info_calculations() {
        let progress = ProgressInfo {
            task_id: crate::task::TaskId::new(),
            current_file: "test.txt".to_string(),
            current_file_bytes: 50,
            current_file_size: 100,
            files_processed: 2,
            total_files: 10,
            bytes_processed: 500,
            total_bytes: 1000,
            transfer_rate: 1024.0,
            eta: None,
            last_update: Instant::now(),
        };

        assert_eq!(progress.current_file_progress(), 50.0);
        assert_eq!(progress.overall_progress(), 50.0);
        assert_eq!(progress.files_progress(), 20.0);
    }

    #[test]
    fn test_statistics_calculations() {
        let stats = Statistics {
            total_tasks: 10,
            running_tasks: 2,
            completed_tasks: 6,
            failed_tasks: 1,
            cancelled_tasks: 1,
            total_bytes_processed: 1024,
            total_files_processed: 10,
            average_transfer_rate: 1024.0,
            peak_transfer_rate: 2048.0,
            total_execution_time: Duration::from_secs(10),
            uptime: Duration::from_secs(60),
            last_update: Instant::now(),
        };

        assert_eq!(stats.success_rate(), 75.0); // 6 out of 8 finished tasks
        assert_eq!(stats.files_per_second(), 1.0); // 10 files in 10 seconds
    }

    #[tokio::test]
    async fn test_progress_monitor() {
        let monitor = ProgressMonitor::new();
        let task_id = crate::task::TaskId::new();
        
        let progress = ProgressInfo {
            task_id,
            current_file: "test.txt".to_string(),
            current_file_bytes: 50,
            current_file_size: 100,
            files_processed: 1,
            total_files: 5,
            bytes_processed: 50,
            total_bytes: 500,
            transfer_rate: 1024.0,
            eta: None,
            last_update: Instant::now(),
        };

        monitor.update_progress(progress.clone()).await.unwrap();
        
        // Note: In a real test, we'd need to start the monitor's run loop
        // For now, we just test the API
    }
}
