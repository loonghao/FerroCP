//! Progress tracking for synchronization operations

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info};

/// Serde module for Instant serialization
mod instant_serde {
    use super::*;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(instant: &Instant, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Convert to duration since a fixed point (we'll use UNIX_EPOCH)
        let duration = instant.elapsed();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            - duration.as_secs();
        serializer.serialize_u64(timestamp)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> std::result::Result<Instant, D::Error>
    where
        D: Deserializer<'de>,
    {
        let timestamp = u64::deserialize(deserializer)?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let elapsed = now.saturating_sub(timestamp);
        Ok(Instant::now() - Duration::from_secs(elapsed))
    }
}

/// Progress information for sync operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncProgress {
    /// Current operation ID
    pub operation_id: uuid::Uuid,
    /// Current phase of synchronization
    pub phase: SyncPhase,
    /// Current file being processed
    pub current_file: Option<PathBuf>,
    /// Files processed so far
    pub files_processed: u64,
    /// Total files to process
    pub total_files: u64,
    /// Bytes processed so far
    pub bytes_processed: u64,
    /// Total bytes to process
    pub total_bytes: u64,
    /// Current transfer rate (bytes per second)
    pub transfer_rate: f64,
    /// Estimated time remaining
    pub eta: Option<Duration>,
    /// Start time of the operation (as timestamp)
    #[serde(with = "instant_serde")]
    pub start_time: Instant,
    /// Number of conflicts encountered
    pub conflicts_count: u64,
    /// Number of errors encountered
    pub errors_count: u64,
}

/// Synchronization phases
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncPhase {
    /// Initializing synchronization
    Initializing,
    /// Scanning source directory
    ScanningSource,
    /// Scanning destination directory
    ScanningDestination,
    /// Detecting changes
    DetectingChanges,
    /// Resolving conflicts
    ResolvingConflicts,
    /// Transferring files
    Transferring,
    /// Applying deltas
    ApplyingDeltas,
    /// Finalizing
    Finalizing,
    /// Completed
    Completed,
    /// Failed
    Failed,
}

impl SyncProgress {
    /// Create a new sync progress
    pub fn new(operation_id: uuid::Uuid) -> Self {
        Self {
            operation_id,
            phase: SyncPhase::Initializing,
            current_file: None,
            files_processed: 0,
            total_files: 0,
            bytes_processed: 0,
            total_bytes: 0,
            transfer_rate: 0.0,
            eta: None,
            start_time: Instant::now(),
            conflicts_count: 0,
            errors_count: 0,
        }
    }

    /// Update the current phase
    pub fn set_phase(&mut self, phase: SyncPhase) {
        self.phase = phase;
        debug!("Sync phase changed to: {:?}", phase);
    }

    /// Set total counts
    pub fn set_totals(&mut self, total_files: u64, total_bytes: u64) {
        self.total_files = total_files;
        self.total_bytes = total_bytes;
    }

    /// Update current file being processed
    pub fn set_current_file(&mut self, file: Option<PathBuf>) {
        self.current_file = file;
    }

    /// Update progress counters
    pub fn update_progress(&mut self, files_processed: u64, bytes_processed: u64) {
        self.files_processed = files_processed;
        self.bytes_processed = bytes_processed;

        // Calculate transfer rate
        let elapsed = self.start_time.elapsed();
        if elapsed.as_secs_f64() > 0.0 {
            self.transfer_rate = bytes_processed as f64 / elapsed.as_secs_f64();
        }

        // Calculate ETA
        if self.transfer_rate > 0.0 && self.total_bytes > bytes_processed {
            let remaining_bytes = self.total_bytes - bytes_processed;
            let eta_seconds = remaining_bytes as f64 / self.transfer_rate;
            self.eta = Some(Duration::from_secs_f64(eta_seconds));
        }
    }

    /// Increment file counter
    pub fn increment_files(&mut self, count: u64) {
        self.files_processed += count;
    }

    /// Increment byte counter
    pub fn increment_bytes(&mut self, count: u64) {
        self.bytes_processed += count;
        self.update_progress(self.files_processed, self.bytes_processed);
    }

    /// Increment conflict counter
    pub fn increment_conflicts(&mut self, count: u64) {
        self.conflicts_count += count;
    }

    /// Increment error counter
    pub fn increment_errors(&mut self, count: u64) {
        self.errors_count += count;
    }

    /// Get overall progress percentage
    pub fn overall_progress(&self) -> f64 {
        if self.total_bytes > 0 {
            (self.bytes_processed as f64 / self.total_bytes as f64) * 100.0
        } else if self.total_files > 0 {
            (self.files_processed as f64 / self.total_files as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Get file progress percentage
    pub fn file_progress(&self) -> f64 {
        if self.total_files > 0 {
            (self.files_processed as f64 / self.total_files as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Get elapsed time
    pub fn elapsed_time(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Check if sync is complete
    pub fn is_complete(&self) -> bool {
        matches!(self.phase, SyncPhase::Completed | SyncPhase::Failed)
    }

    /// Format transfer rate as human-readable string
    pub fn format_transfer_rate(&self) -> String {
        format_bytes_per_second(self.transfer_rate)
    }

    /// Format ETA as human-readable string
    pub fn format_eta(&self) -> String {
        match self.eta {
            Some(eta) => format_duration(eta),
            None => "Unknown".to_string(),
        }
    }
}

/// Progress event types
#[derive(Debug, Clone)]
pub enum ProgressEvent {
    /// Progress update
    Update(SyncProgress),
    /// Phase changed
    PhaseChanged(SyncPhase),
    /// File started processing
    FileStarted(PathBuf),
    /// File completed processing
    FileCompleted(PathBuf, u64), // path, bytes
    /// Conflict encountered
    ConflictEncountered(PathBuf),
    /// Error encountered
    ErrorEncountered(String),
    /// Sync completed
    Completed(SyncProgress),
    /// Sync failed
    Failed(String),
}

/// Progress reporter for sync operations
#[derive(Debug)]
pub struct ProgressReporter {
    progress: Arc<RwLock<SyncProgress>>,
    event_tx: mpsc::UnboundedSender<ProgressEvent>,
    event_rx: Option<mpsc::UnboundedReceiver<ProgressEvent>>,
}

impl ProgressReporter {
    /// Create a new progress reporter
    pub fn new(operation_id: uuid::Uuid) -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let progress = Arc::new(RwLock::new(SyncProgress::new(operation_id)));

        Self {
            progress,
            event_tx,
            event_rx: Some(event_rx),
        }
    }

    /// Get the current progress
    pub async fn get_progress(&self) -> SyncProgress {
        self.progress.read().await.clone()
    }

    /// Take the event receiver (can only be called once)
    pub fn take_event_receiver(&mut self) -> Option<mpsc::UnboundedReceiver<ProgressEvent>> {
        self.event_rx.take()
    }

    /// Update the current phase
    pub async fn set_phase(&self, phase: SyncPhase) {
        {
            let mut progress = self.progress.write().await;
            progress.set_phase(phase);
        }

        let _ = self.event_tx.send(ProgressEvent::PhaseChanged(phase));
        let progress = self.get_progress().await;
        let _ = self.event_tx.send(ProgressEvent::Update(progress));
    }

    /// Set total counts
    pub async fn set_totals(&self, total_files: u64, total_bytes: u64) {
        {
            let mut progress = self.progress.write().await;
            progress.set_totals(total_files, total_bytes);
        }

        let progress = self.get_progress().await;
        let _ = self.event_tx.send(ProgressEvent::Update(progress));
    }

    /// Report file started
    pub async fn file_started(&self, file: PathBuf) {
        {
            let mut progress = self.progress.write().await;
            progress.set_current_file(Some(file.clone()));
        }

        let _ = self.event_tx.send(ProgressEvent::FileStarted(file));
        let progress = self.get_progress().await;
        let _ = self.event_tx.send(ProgressEvent::Update(progress));
    }

    /// Report file completed
    pub async fn file_completed(&self, file: PathBuf, bytes: u64) {
        {
            let mut progress = self.progress.write().await;
            progress.increment_files(1);
            progress.increment_bytes(bytes);
            progress.set_current_file(None);
        }

        let _ = self
            .event_tx
            .send(ProgressEvent::FileCompleted(file, bytes));
        let progress = self.get_progress().await;
        let _ = self.event_tx.send(ProgressEvent::Update(progress));
    }

    /// Report conflict encountered
    pub async fn conflict_encountered(&self, file: PathBuf) {
        {
            let mut progress = self.progress.write().await;
            progress.increment_conflicts(1);
        }

        let _ = self.event_tx.send(ProgressEvent::ConflictEncountered(file));
        let progress = self.get_progress().await;
        let _ = self.event_tx.send(ProgressEvent::Update(progress));
    }

    /// Report error encountered
    pub async fn error_encountered(&self, error: String) {
        {
            let mut progress = self.progress.write().await;
            progress.increment_errors(1);
        }

        let _ = self.event_tx.send(ProgressEvent::ErrorEncountered(error));
        let progress = self.get_progress().await;
        let _ = self.event_tx.send(ProgressEvent::Update(progress));
    }

    /// Report sync completed
    pub async fn completed(&self) {
        {
            let mut progress = self.progress.write().await;
            progress.set_phase(SyncPhase::Completed);
        }

        let progress = self.get_progress().await;
        let _ = self
            .event_tx
            .send(ProgressEvent::Completed(progress.clone()));
        let _ = self.event_tx.send(ProgressEvent::Update(progress));

        info!("Sync completed successfully");
    }

    /// Report sync failed
    pub async fn failed(&self, error: String) {
        {
            let mut progress = self.progress.write().await;
            progress.set_phase(SyncPhase::Failed);
        }

        let _ = self.event_tx.send(ProgressEvent::Failed(error));
        let progress = self.get_progress().await;
        let _ = self.event_tx.send(ProgressEvent::Update(progress));
    }
}

impl Clone for ProgressReporter {
    fn clone(&self) -> Self {
        Self {
            progress: Arc::clone(&self.progress),
            event_tx: self.event_tx.clone(),
            event_rx: None, // Clone doesn't get the receiver
        }
    }
}

/// Format bytes per second as human-readable string
fn format_bytes_per_second(bytes_per_sec: f64) -> String {
    const UNITS: &[&str] = &["B/s", "KB/s", "MB/s", "GB/s", "TB/s"];
    let mut size = bytes_per_sec;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.1} {}", size, UNITS[unit_index])
}

/// Format duration as human-readable string
fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.as_secs();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_progress_creation() {
        let operation_id = uuid::Uuid::new_v4();
        let progress = SyncProgress::new(operation_id);

        assert_eq!(progress.operation_id, operation_id);
        assert_eq!(progress.phase, SyncPhase::Initializing);
        assert_eq!(progress.files_processed, 0);
        assert_eq!(progress.bytes_processed, 0);
    }

    #[test]
    fn test_progress_calculations() {
        let mut progress = SyncProgress::new(uuid::Uuid::new_v4());
        progress.set_totals(100, 1000);
        progress.update_progress(50, 500);

        assert_eq!(progress.overall_progress(), 50.0);
        assert_eq!(progress.file_progress(), 50.0);
    }

    #[tokio::test]
    async fn test_progress_reporter() {
        let operation_id = uuid::Uuid::new_v4();
        let mut reporter = ProgressReporter::new(operation_id);
        let mut event_rx = reporter.take_event_receiver().unwrap();

        // Test phase change
        reporter.set_phase(SyncPhase::ScanningSource).await;

        // Check events
        let event = event_rx.recv().await.unwrap();
        assert!(matches!(
            event,
            ProgressEvent::PhaseChanged(SyncPhase::ScanningSource)
        ));

        let event = event_rx.recv().await.unwrap();
        assert!(matches!(event, ProgressEvent::Update(_)));

        // Test file operations
        let file_path = PathBuf::from("test.txt");
        reporter.file_started(file_path.clone()).await;

        let event = event_rx.recv().await.unwrap();
        assert!(matches!(event, ProgressEvent::FileStarted(_)));
    }

    #[test]
    fn test_format_functions() {
        assert_eq!(format_bytes_per_second(1024.0), "1.0 KB/s");
        assert_eq!(format_bytes_per_second(1048576.0), "1.0 MB/s");

        assert_eq!(format_duration(Duration::from_secs(65)), "1m 5s");
        assert_eq!(format_duration(Duration::from_secs(3665)), "1h 1m 5s");
    }
}
