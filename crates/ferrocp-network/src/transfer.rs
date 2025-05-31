//! Network file transfer functionality

use ferrocp_types::CopyStats;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// File transfer request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferRequest {
    /// Source file path
    pub source: PathBuf,
    /// Destination file path
    pub destination: PathBuf,
    /// Transfer options
    pub options: TransferOptions,
    /// Request ID for tracking
    pub request_id: uuid::Uuid,
}

impl TransferRequest {
    /// Create a new transfer request
    pub fn new<P: AsRef<Path>>(source: P, destination: P) -> Self {
        Self {
            source: source.as_ref().to_path_buf(),
            destination: destination.as_ref().to_path_buf(),
            options: TransferOptions::default(),
            request_id: uuid::Uuid::new_v4(),
        }
    }

    /// Create a transfer request with custom options
    pub fn with_options<P: AsRef<Path>>(
        source: P,
        destination: P,
        options: TransferOptions,
    ) -> Self {
        Self {
            source: source.as_ref().to_path_buf(),
            destination: destination.as_ref().to_path_buf(),
            options,
            request_id: uuid::Uuid::new_v4(),
        }
    }
}

/// Transfer options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferOptions {
    /// Enable compression during transfer
    pub enable_compression: bool,
    /// Compression level (0-9)
    pub compression_level: u8,
    /// Chunk size for transfer
    pub chunk_size: usize,
    /// Enable resume support
    pub enable_resume: bool,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Timeout for each chunk transfer
    pub chunk_timeout: Duration,
    /// Verify file integrity after transfer
    pub verify_integrity: bool,
}

impl Default for TransferOptions {
    fn default() -> Self {
        Self {
            enable_compression: true,
            compression_level: 6,
            chunk_size: 64 * 1024, // 64KB
            enable_resume: true,
            max_retries: 3,
            chunk_timeout: Duration::from_secs(30),
            verify_integrity: true,
        }
    }
}

/// Transfer result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferResult {
    /// Request ID
    pub request_id: uuid::Uuid,
    /// Transfer statistics
    pub stats: CopyStats,
    /// Total bytes transferred
    pub bytes_transferred: u64,
    /// Transfer duration
    pub duration: Duration,
    /// Average transfer speed (bytes per second)
    pub average_speed: f64,
    /// Whether the transfer was resumed
    pub was_resumed: bool,
    /// Number of retry attempts
    pub retry_count: u32,
}

impl TransferResult {
    /// Create a new transfer result
    pub fn new(request_id: uuid::Uuid, stats: CopyStats) -> Self {
        let duration = stats.duration;
        let bytes_transferred = stats.bytes_copied;
        let average_speed = if duration.as_secs_f64() > 0.0 {
            bytes_transferred as f64 / duration.as_secs_f64()
        } else {
            0.0
        };

        Self {
            request_id,
            stats,
            bytes_transferred,
            duration,
            average_speed,
            was_resumed: false,
            retry_count: 0,
        }
    }

    /// Update transfer result with resume information
    pub fn with_resume_info(mut self, was_resumed: bool, retry_count: u32) -> Self {
        self.was_resumed = was_resumed;
        self.retry_count = retry_count;
        self
    }
}

/// Transfer progress information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferProgress {
    /// Request ID
    pub request_id: uuid::Uuid,
    /// Total bytes to transfer
    pub total_bytes: u64,
    /// Bytes transferred so far
    pub bytes_transferred: u64,
    /// Current transfer speed (bytes per second)
    pub current_speed: f64,
    /// Estimated time remaining
    pub eta: Option<Duration>,
    /// Progress percentage (0.0 - 100.0)
    pub percentage: f64,
    /// Current chunk being transferred
    pub current_chunk: u64,
    /// Total number of chunks
    pub total_chunks: u64,
}

impl TransferProgress {
    /// Create a new transfer progress
    pub fn new(request_id: uuid::Uuid, total_bytes: u64, chunk_size: usize) -> Self {
        let total_chunks = (total_bytes + chunk_size as u64 - 1) / chunk_size as u64;

        Self {
            request_id,
            total_bytes,
            bytes_transferred: 0,
            current_speed: 0.0,
            eta: None,
            percentage: 0.0,
            current_chunk: 0,
            total_chunks,
        }
    }

    /// Update progress with new transferred bytes
    pub fn update(&mut self, bytes_transferred: u64, current_speed: f64) {
        self.bytes_transferred = bytes_transferred;
        self.current_speed = current_speed;

        if self.total_bytes > 0 {
            self.percentage = (bytes_transferred as f64 / self.total_bytes as f64) * 100.0;
        }

        if current_speed > 0.0 {
            let remaining_bytes = self.total_bytes.saturating_sub(bytes_transferred);
            let eta_seconds = remaining_bytes as f64 / current_speed;
            self.eta = Some(Duration::from_secs_f64(eta_seconds));
        }
    }

    /// Update current chunk
    pub fn update_chunk(&mut self, current_chunk: u64) {
        self.current_chunk = current_chunk;
    }

    /// Check if transfer is complete
    pub fn is_complete(&self) -> bool {
        self.bytes_transferred >= self.total_bytes
    }
}

/// Transfer chunk information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferChunk {
    /// Request ID
    pub request_id: uuid::Uuid,
    /// Chunk sequence number
    pub sequence: u64,
    /// Chunk offset in the file
    pub offset: u64,
    /// Chunk data
    pub data: Vec<u8>,
    /// Chunk checksum for integrity verification
    pub checksum: u32,
    /// Whether this is the last chunk
    pub is_last: bool,
}

impl TransferChunk {
    /// Create a new transfer chunk
    pub fn new(
        request_id: uuid::Uuid,
        sequence: u64,
        offset: u64,
        data: Vec<u8>,
        is_last: bool,
    ) -> Self {
        let checksum = crc32fast::hash(&data);

        Self {
            request_id,
            sequence,
            offset,
            data,
            checksum,
            is_last,
        }
    }

    /// Verify chunk integrity
    pub fn verify_integrity(&self) -> bool {
        crc32fast::hash(&self.data) == self.checksum
    }

    /// Get chunk size
    pub fn size(&self) -> usize {
        self.data.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transfer_request_creation() {
        let request = TransferRequest::new("source.txt", "dest.txt");

        assert_eq!(request.source, PathBuf::from("source.txt"));
        assert_eq!(request.destination, PathBuf::from("dest.txt"));
        assert!(request.options.enable_compression);
    }

    #[test]
    fn test_transfer_progress_update() {
        let request_id = uuid::Uuid::new_v4();
        let mut progress = TransferProgress::new(request_id, 1000, 100);

        progress.update(500, 100.0);

        assert_eq!(progress.bytes_transferred, 500);
        assert_eq!(progress.percentage, 50.0);
        assert!(progress.eta.is_some());
    }

    #[test]
    fn test_transfer_chunk_integrity() {
        let request_id = uuid::Uuid::new_v4();
        let data = b"test data".to_vec();
        let chunk = TransferChunk::new(request_id, 0, 0, data, false);

        assert!(chunk.verify_integrity());
        assert_eq!(chunk.size(), 9);
    }
}
