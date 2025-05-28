//! Task management and execution for the copy engine

use ferrocp_types::{CopyMode, CopyStats, Error, Priority, Result};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Unique identifier for a task
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(Uuid);

impl TaskId {
    /// Create a new task ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Get the inner UUID
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Status of a task
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskStatus {
    /// Task is pending execution
    Pending,
    /// Task is currently running
    Running,
    /// Task completed successfully
    Completed,
    /// Task failed with an error
    Failed(String),
    /// Task was cancelled
    Cancelled,
    /// Task is paused
    Paused,
}

impl TaskStatus {
    /// Check if the task is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed(_) | Self::Cancelled)
    }

    /// Check if the task is active (running or paused)
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Running | Self::Paused)
    }
}

/// Copy request containing all parameters for a copy operation
#[derive(Debug, Clone)]
pub struct CopyRequest {
    /// Source path
    pub source: PathBuf,
    /// Destination path
    pub destination: PathBuf,
    /// Copy mode
    pub mode: CopyMode,
    /// Task priority
    pub priority: Priority,
    /// Whether to preserve metadata
    pub preserve_metadata: bool,
    /// Whether to verify the copy
    pub verify_copy: bool,
    /// Whether to enable compression
    pub enable_compression: bool,
    /// Exclude patterns
    pub exclude_patterns: Vec<String>,
    /// Include patterns
    pub include_patterns: Vec<String>,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Retry delay
    pub retry_delay: Duration,
}

impl CopyRequest {
    /// Create a new copy request with default settings
    pub fn new<P1: Into<PathBuf>, P2: Into<PathBuf>>(source: P1, destination: P2) -> Self {
        Self {
            source: source.into(),
            destination: destination.into(),
            mode: CopyMode::All,
            priority: Priority::Normal,
            preserve_metadata: true,
            verify_copy: false,
            enable_compression: false,
            exclude_patterns: Vec::new(),
            include_patterns: Vec::new(),
            max_retries: 3,
            retry_delay: Duration::from_millis(1000),
        }
    }

    /// Set the copy mode
    pub fn with_mode(mut self, mode: CopyMode) -> Self {
        self.mode = mode;
        self
    }

    /// Set the priority
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    /// Enable metadata preservation
    pub fn preserve_metadata(mut self, preserve: bool) -> Self {
        self.preserve_metadata = preserve;
        self
    }

    /// Enable copy verification
    pub fn verify_copy(mut self, verify: bool) -> Self {
        self.verify_copy = verify;
        self
    }

    /// Enable compression
    pub fn enable_compression(mut self, enable: bool) -> Self {
        self.enable_compression = enable;
        self
    }

    /// Add exclude patterns
    pub fn exclude_patterns<I, S>(mut self, patterns: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.exclude_patterns.extend(patterns.into_iter().map(Into::into));
        self
    }

    /// Add include patterns
    pub fn include_patterns<I, S>(mut self, patterns: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.include_patterns.extend(patterns.into_iter().map(Into::into));
        self
    }

    /// Set retry configuration
    pub fn with_retry(mut self, max_retries: u32, delay: Duration) -> Self {
        self.max_retries = max_retries;
        self.retry_delay = delay;
        self
    }
}

/// Result of a copy operation
#[derive(Debug, Clone)]
pub struct CopyResult {
    /// Task ID
    pub task_id: TaskId,
    /// Copy statistics
    pub stats: CopyStats,
    /// Final status
    pub status: TaskStatus,
    /// Error message if failed
    pub error: Option<String>,
    /// Total execution time
    pub execution_time: Duration,
}

impl CopyResult {
    /// Create a successful result
    pub fn success(task_id: TaskId, stats: CopyStats, execution_time: Duration) -> Self {
        Self {
            task_id,
            stats,
            status: TaskStatus::Completed,
            error: None,
            execution_time,
        }
    }

    /// Create a failed result
    pub fn failure(task_id: TaskId, error: String, execution_time: Duration) -> Self {
        Self {
            task_id,
            stats: CopyStats::default(),
            status: TaskStatus::Failed(error.clone()),
            error: Some(error),
            execution_time,
        }
    }

    /// Check if the operation was successful
    pub fn is_success(&self) -> bool {
        matches!(self.status, TaskStatus::Completed)
    }

    /// Check if the operation failed
    pub fn is_failure(&self) -> bool {
        matches!(self.status, TaskStatus::Failed(_))
    }
}

/// A task represents a unit of work in the copy engine
#[derive(Debug, Clone)]
pub struct Task {
    /// Unique task identifier
    pub id: TaskId,
    /// Copy request
    pub request: CopyRequest,
    /// Current status
    pub status: TaskStatus,
    /// Creation time
    pub created_at: Instant,
    /// Start time (when execution began)
    pub started_at: Option<Instant>,
    /// Completion time
    pub completed_at: Option<Instant>,
    /// Number of retry attempts made
    pub retry_count: u32,
    /// Last error encountered
    pub last_error: Option<String>,
}

impl Task {
    /// Create a new task
    pub fn new(request: CopyRequest) -> Self {
        Self {
            id: TaskId::new(),
            request,
            status: TaskStatus::Pending,
            created_at: Instant::now(),
            started_at: None,
            completed_at: None,
            retry_count: 0,
            last_error: None,
        }
    }

    /// Mark the task as started
    pub fn start(&mut self) {
        self.status = TaskStatus::Running;
        self.started_at = Some(Instant::now());
    }

    /// Mark the task as completed
    pub fn complete(&mut self) {
        self.status = TaskStatus::Completed;
        self.completed_at = Some(Instant::now());
    }

    /// Mark the task as failed
    pub fn fail(&mut self, error: String) {
        self.status = TaskStatus::Failed(error.clone());
        self.last_error = Some(error);
        self.completed_at = Some(Instant::now());
    }

    /// Mark the task as cancelled
    pub fn cancel(&mut self) {
        self.status = TaskStatus::Cancelled;
        self.completed_at = Some(Instant::now());
    }

    /// Pause the task
    pub fn pause(&mut self) {
        if matches!(self.status, TaskStatus::Running) {
            self.status = TaskStatus::Paused;
        }
    }

    /// Resume the task
    pub fn resume(&mut self) {
        if matches!(self.status, TaskStatus::Paused) {
            self.status = TaskStatus::Running;
        }
    }

    /// Increment retry count
    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }

    /// Check if the task can be retried
    pub fn can_retry(&self) -> bool {
        self.retry_count < self.request.max_retries
    }

    /// Get the execution duration
    pub fn execution_duration(&self) -> Option<Duration> {
        match (self.started_at, self.completed_at) {
            (Some(start), Some(end)) => Some(end.duration_since(start)),
            (Some(start), None) => Some(Instant::now().duration_since(start)),
            _ => None,
        }
    }

    /// Get the total duration since creation
    pub fn total_duration(&self) -> Duration {
        Instant::now().duration_since(self.created_at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id_creation() {
        let id1 = TaskId::new();
        let id2 = TaskId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_task_status() {
        assert!(TaskStatus::Completed.is_terminal());
        assert!(TaskStatus::Failed("error".to_string()).is_terminal());
        assert!(TaskStatus::Cancelled.is_terminal());
        assert!(!TaskStatus::Pending.is_terminal());
        assert!(!TaskStatus::Running.is_terminal());

        assert!(TaskStatus::Running.is_active());
        assert!(TaskStatus::Paused.is_active());
        assert!(!TaskStatus::Pending.is_active());
    }

    #[test]
    fn test_copy_request_builder() {
        let request = CopyRequest::new("src", "dst")
            .with_mode(CopyMode::Newer)
            .with_priority(Priority::High)
            .preserve_metadata(false)
            .verify_copy(true)
            .enable_compression(true)
            .exclude_patterns(vec!["*.tmp", "*.log"])
            .with_retry(5, Duration::from_millis(500));

        assert_eq!(request.mode, CopyMode::Newer);
        assert_eq!(request.priority, Priority::High);
        assert!(!request.preserve_metadata);
        assert!(request.verify_copy);
        assert!(request.enable_compression);
        assert_eq!(request.exclude_patterns, vec!["*.tmp", "*.log"]);
        assert_eq!(request.max_retries, 5);
        assert_eq!(request.retry_delay, Duration::from_millis(500));
    }

    #[test]
    fn test_task_lifecycle() {
        let request = CopyRequest::new("src", "dst");
        let mut task = Task::new(request);

        assert_eq!(task.status, TaskStatus::Pending);
        assert!(task.started_at.is_none());
        assert!(task.completed_at.is_none());

        task.start();
        assert_eq!(task.status, TaskStatus::Running);
        assert!(task.started_at.is_some());

        task.complete();
        assert_eq!(task.status, TaskStatus::Completed);
        assert!(task.completed_at.is_some());
        assert!(task.execution_duration().is_some());
    }

    #[test]
    fn test_copy_result() {
        let task_id = TaskId::new();
        let stats = CopyStats::default();
        let duration = Duration::from_secs(1);

        let success = CopyResult::success(task_id, stats.clone(), duration);
        assert!(success.is_success());
        assert!(!success.is_failure());

        let failure = CopyResult::failure(task_id, "test error".to_string(), duration);
        assert!(!failure.is_success());
        assert!(failure.is_failure());
        assert_eq!(failure.error, Some("test error".to_string()));
    }
}
