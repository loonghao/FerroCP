//! Resume functionality for interrupted network transfers

use crate::transfer::{TransferProgress, TransferRequest};
use ferrocp_types::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::fs;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Resume information for a transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeInfo {
    /// Transfer request ID
    pub request_id: uuid::Uuid,
    /// Source file path
    pub source: PathBuf,
    /// Destination file path
    pub destination: PathBuf,
    /// Bytes already transferred
    pub bytes_transferred: u64,
    /// Total file size
    pub total_size: u64,
    /// Last chunk sequence number
    pub last_chunk_sequence: u64,
    /// File checksum for integrity verification
    pub file_checksum: Option<String>,
    /// Timestamp when transfer was interrupted
    pub interrupted_at: u64,
    /// Number of retry attempts
    pub retry_count: u32,
    /// Transfer options
    pub transfer_options: crate::transfer::TransferOptions,
}

impl ResumeInfo {
    /// Create new resume info
    pub fn new(
        request: &TransferRequest,
        progress: &TransferProgress,
        file_checksum: Option<String>,
    ) -> Self {
        let interrupted_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            request_id: request.request_id,
            source: request.source.clone(),
            destination: request.destination.clone(),
            bytes_transferred: progress.bytes_transferred,
            total_size: progress.total_bytes,
            last_chunk_sequence: progress.current_chunk,
            file_checksum,
            interrupted_at,
            retry_count: 0,
            transfer_options: request.options.clone(),
        }
    }

    /// Check if resume info is still valid
    pub fn is_valid(&self, max_age: Duration) -> bool {
        let age = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .saturating_sub(self.interrupted_at);

        age <= max_age.as_secs()
    }

    /// Get completion percentage
    pub fn completion_percentage(&self) -> f64 {
        if self.total_size > 0 {
            (self.bytes_transferred as f64 / self.total_size as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Increment retry count
    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }
}

/// Resume manager configuration
#[derive(Debug, Clone)]
pub struct ResumeConfig {
    /// Directory to store resume files
    pub resume_dir: PathBuf,
    /// Maximum age for resume files
    pub max_resume_age: Duration,
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Enable automatic cleanup of old resume files
    pub auto_cleanup: bool,
    /// Cleanup interval
    pub cleanup_interval: Duration,
}

impl Default for ResumeConfig {
    fn default() -> Self {
        Self {
            resume_dir: PathBuf::from(".ferrocp_resume"),
            max_resume_age: Duration::from_secs(24 * 60 * 60), // 24 hours
            max_retries: 3,
            auto_cleanup: true,
            cleanup_interval: Duration::from_secs(60 * 60), // 1 hour
        }
    }
}

/// Resume manager for handling transfer resumption
#[derive(Debug)]
pub struct ResumeManager {
    config: ResumeConfig,
    resume_info: Arc<RwLock<HashMap<uuid::Uuid, ResumeInfo>>>,
}

impl ResumeManager {
    /// Create a new resume manager
    pub async fn new(config: ResumeConfig) -> Result<Self> {
        // Create resume directory if it doesn't exist
        if !config.resume_dir.exists() {
            fs::create_dir_all(&config.resume_dir)
                .await
                .map_err(|e| Error::Io {
                    message: format!(
                        "Failed to create resume directory '{}': {}",
                        config.resume_dir.display(),
                        e
                    ),
                })?;
        }

        let manager = Self {
            config,
            resume_info: Arc::new(RwLock::new(HashMap::new())),
        };

        // Load existing resume files
        manager.load_resume_files().await?;

        // Start cleanup task if enabled
        if manager.config.auto_cleanup {
            manager.start_cleanup_task().await?;
        }

        Ok(manager)
    }

    /// Save resume information for a transfer
    pub async fn save_resume_info(&self, resume_info: ResumeInfo) -> Result<()> {
        let resume_file = self.get_resume_file_path(&resume_info.request_id);

        // Serialize resume info
        let data = bincode::serialize(&resume_info).map_err(|e| Error::Network {
            message: format!("Failed to serialize resume info: {}", e),
        })?;

        // Write to file
        fs::write(&resume_file, data).await.map_err(|e| Error::Io {
            message: format!(
                "Failed to write resume file '{}': {}",
                resume_file.display(),
                e
            ),
        })?;

        // Store in memory
        let request_id = resume_info.request_id;
        self.resume_info
            .write()
            .await
            .insert(request_id, resume_info);

        debug!("Saved resume info for request {}", request_id);
        Ok(())
    }

    /// Load resume information for a transfer
    pub async fn load_resume_info(&self, request_id: &uuid::Uuid) -> Result<Option<ResumeInfo>> {
        // Check memory first
        if let Some(info) = self.resume_info.read().await.get(request_id) {
            if info.is_valid(self.config.max_resume_age) {
                return Ok(Some(info.clone()));
            }
        }

        // Try to load from file
        let resume_file = self.get_resume_file_path(request_id);
        if !resume_file.exists() {
            return Ok(None);
        }

        let data = fs::read(&resume_file).await.map_err(|e| Error::Io {
            message: format!(
                "Failed to read resume file '{}': {}",
                resume_file.display(),
                e
            ),
        })?;

        let resume_info: ResumeInfo = bincode::deserialize(&data).map_err(|e| Error::Network {
            message: format!("Failed to deserialize resume info: {}", e),
        })?;

        // Check if still valid
        if !resume_info.is_valid(self.config.max_resume_age) {
            self.remove_resume_info(request_id).await?;
            return Ok(None);
        }

        // Store in memory
        self.resume_info
            .write()
            .await
            .insert(*request_id, resume_info.clone());

        debug!("Loaded resume info for request {}", request_id);
        Ok(Some(resume_info))
    }

    /// Remove resume information
    pub async fn remove_resume_info(&self, request_id: &uuid::Uuid) -> Result<()> {
        // Remove from memory
        self.resume_info.write().await.remove(request_id);

        // Remove file
        let resume_file = self.get_resume_file_path(request_id);
        if resume_file.exists() {
            fs::remove_file(&resume_file).await.map_err(|e| Error::Io {
                message: format!(
                    "Failed to remove resume file '{}': {}",
                    resume_file.display(),
                    e
                ),
            })?;
        }

        debug!("Removed resume info for request {}", request_id);
        Ok(())
    }

    /// Check if a transfer can be resumed
    pub async fn can_resume(&self, request_id: &uuid::Uuid) -> bool {
        if let Ok(Some(info)) = self.load_resume_info(request_id).await {
            info.retry_count < self.config.max_retries
        } else {
            false
        }
    }

    /// Get all resumable transfers
    pub async fn get_resumable_transfers(&self) -> Result<Vec<ResumeInfo>> {
        let mut resumable = Vec::new();

        for info in self.resume_info.read().await.values() {
            if info.is_valid(self.config.max_resume_age)
                && info.retry_count < self.config.max_retries
            {
                resumable.push(info.clone());
            }
        }

        Ok(resumable)
    }

    /// Load resume files from disk
    async fn load_resume_files(&self) -> Result<()> {
        let mut entries = fs::read_dir(&self.config.resume_dir)
            .await
            .map_err(|e| Error::Io {
                message: format!(
                    "Failed to read resume directory '{}': {}",
                    self.config.resume_dir.display(),
                    e
                ),
            })?;

        let mut loaded_count = 0;
        while let Some(entry) = entries.next_entry().await.map_err(|e| Error::Io {
            message: format!("Failed to read directory entry: {}", e),
        })? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("resume") {
                if let Ok(data) = fs::read(&path).await {
                    if let Ok(resume_info) = bincode::deserialize::<ResumeInfo>(&data) {
                        if resume_info.is_valid(self.config.max_resume_age) {
                            self.resume_info
                                .write()
                                .await
                                .insert(resume_info.request_id, resume_info);
                            loaded_count += 1;
                        } else {
                            // Remove expired resume file
                            let _ = fs::remove_file(&path).await;
                        }
                    }
                }
            }
        }

        if loaded_count > 0 {
            info!("Loaded {} resume files", loaded_count);
        }

        Ok(())
    }

    /// Start cleanup task for old resume files
    async fn start_cleanup_task(&self) -> Result<()> {
        let resume_info = Arc::clone(&self.resume_info);
        let resume_dir = self.config.resume_dir.clone();
        let max_age = self.config.max_resume_age;
        let cleanup_interval = self.config.cleanup_interval;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(cleanup_interval);
            loop {
                interval.tick().await;

                // Clean up expired resume info from memory
                let mut expired_ids = Vec::new();
                {
                    let info_map = resume_info.read().await;
                    for (id, info) in info_map.iter() {
                        if !info.is_valid(max_age) {
                            expired_ids.push(*id);
                        }
                    }
                }

                if !expired_ids.is_empty() {
                    let mut info_map = resume_info.write().await;
                    for id in expired_ids {
                        info_map.remove(&id);
                    }
                }

                // Clean up old resume files
                if let Ok(mut entries) = fs::read_dir(&resume_dir).await {
                    while let Ok(Some(entry)) = entries.next_entry().await {
                        let path = entry.path();
                        if path.extension().and_then(|s| s.to_str()) == Some("resume") {
                            if let Ok(metadata) = entry.metadata().await {
                                if let Ok(modified) = metadata.modified() {
                                    if let Ok(age) = modified.elapsed() {
                                        if age > max_age {
                                            let _ = fs::remove_file(&path).await;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Get resume file path for a request ID
    fn get_resume_file_path(&self, request_id: &uuid::Uuid) -> PathBuf {
        self.config
            .resume_dir
            .join(format!("{}.resume", request_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transfer::TransferProgress;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_resume_info_creation() {
        let request = TransferRequest::new("source.txt", "dest.txt");
        let progress = TransferProgress::new(request.request_id, 1000, 100);
        let resume_info = ResumeInfo::new(&request, &progress, None);

        assert_eq!(resume_info.request_id, request.request_id);
        assert_eq!(resume_info.bytes_transferred, 0);
        assert_eq!(resume_info.total_size, 1000);
    }

    #[tokio::test]
    async fn test_resume_manager() {
        let temp_dir = TempDir::new().unwrap();
        let config = ResumeConfig {
            resume_dir: temp_dir.path().to_path_buf(),
            auto_cleanup: false,
            ..Default::default()
        };

        let manager = ResumeManager::new(config).await.unwrap();
        let request = TransferRequest::new("source.txt", "dest.txt");
        let progress = TransferProgress::new(request.request_id, 1000, 100);
        let resume_info = ResumeInfo::new(&request, &progress, None);

        // Save resume info
        manager.save_resume_info(resume_info.clone()).await.unwrap();

        // Load resume info
        let loaded = manager.load_resume_info(&request.request_id).await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().request_id, request.request_id);

        // Remove resume info
        manager
            .remove_resume_info(&request.request_id)
            .await
            .unwrap();
        let removed = manager.load_resume_info(&request.request_id).await.unwrap();
        assert!(removed.is_none());
    }
}
