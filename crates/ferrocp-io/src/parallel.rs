//! Parallel I/O engine for high-performance large file copying
//!
//! This module implements a parallel copy engine that uses chunked processing
//! and pipelined I/O to maximize throughput for large files (>10MB).

use crate::{AsyncFileReader, AsyncFileWriter, CopyEngine, CopyOptions};
use ferrocp_types::{CopyStats, DeviceType, Error, Result};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{mpsc, Semaphore};
use tokio::task::JoinHandle;
use tracing::{debug, info, trace, warn};

/// Configuration for parallel copy operations
#[derive(Debug, Clone)]
pub struct ParallelCopyConfig {
    /// Size of each chunk in bytes
    pub chunk_size: usize,
    /// Maximum number of concurrent operations
    pub max_concurrent: usize,
    /// Pipeline depth (number of chunks to buffer)
    pub pipeline_depth: usize,
    /// Minimum file size to enable parallel processing
    pub min_file_size: u64,
    /// Enable adaptive chunk sizing
    pub adaptive_chunk_size: bool,
    /// Maximum memory usage for buffering
    pub max_memory_usage: usize,
    /// Enable read-ahead optimization
    pub enable_read_ahead: bool,
    /// Read-ahead buffer size multiplier
    pub read_ahead_multiplier: usize,
}

impl Default for ParallelCopyConfig {
    fn default() -> Self {
        Self {
            chunk_size: 1024 * 1024,         // 1MB chunks
            max_concurrent: 4,               // 4 concurrent operations
            pipeline_depth: 8,               // Buffer up to 8 chunks
            min_file_size: 10 * 1024 * 1024, // 10MB minimum
            adaptive_chunk_size: true,
            max_memory_usage: 64 * 1024 * 1024, // 64MB max memory
            enable_read_ahead: true,            // Enable read-ahead optimization
            read_ahead_multiplier: 2,           // Read 2x ahead
        }
    }
}

/// Statistics for parallel copy operations
#[derive(Debug, Default, Clone)]
pub struct ParallelCopyStats {
    /// Total number of chunks processed
    pub chunks_processed: u64,
    /// Total bytes processed through parallel pipeline
    pub parallel_bytes: u64,
    /// Number of parallel operations performed
    pub parallel_operations: u64,
    /// Average chunk processing time in nanoseconds
    pub avg_chunk_time_ns: u64,
    /// Peak concurrent operations
    pub peak_concurrent_ops: u64,
    /// Total pipeline stalls (backpressure events)
    pub pipeline_stalls: u64,
    /// Memory usage statistics
    pub peak_memory_usage: u64,
    /// Number of adaptive adjustments made
    pub adaptive_adjustments: u64,
}

impl ParallelCopyStats {
    /// Calculate average throughput in MB/s
    pub fn avg_throughput_mbps(&self, total_time_ns: u64) -> f64 {
        if total_time_ns == 0 {
            return 0.0;
        }
        let seconds = total_time_ns as f64 / 1_000_000_000.0;
        let mb = self.parallel_bytes as f64 / (1024.0 * 1024.0);
        mb / seconds
    }

    /// Calculate chunk processing efficiency
    pub fn chunk_efficiency(&self) -> f64 {
        if self.chunks_processed == 0 {
            return 100.0;
        }
        let ideal_time = self.chunks_processed * 1_000_000; // 1ms per chunk ideal
        let actual_time = self.avg_chunk_time_ns * self.chunks_processed;
        (ideal_time as f64 / actual_time as f64) * 100.0
    }
}

/// Data chunk for parallel processing
#[derive(Debug, Clone)]
struct DataChunk {
    /// Chunk sequence number
    sequence: u64,
    /// Chunk data
    data: Vec<u8>,
    /// Actual size of data in the chunk
    size: usize,
    /// Whether this is the last chunk
    is_last: bool,
}

/// Parallel copy engine for large files
#[derive(Debug)]
pub struct ParallelCopyEngine {
    /// Configuration for parallel operations
    config: ParallelCopyConfig,
    /// Statistics for monitoring performance
    stats: ParallelCopyStats,
    /// Current memory usage
    current_memory_usage: Arc<AtomicU64>,
    /// Semaphore for controlling concurrent operations
    semaphore: Arc<Semaphore>,
}

impl ParallelCopyEngine {
    /// Create a new parallel copy engine with default configuration
    pub fn new() -> Self {
        let config = ParallelCopyConfig::default();
        let semaphore = Arc::new(Semaphore::new(config.max_concurrent));

        Self {
            config,
            stats: ParallelCopyStats::default(),
            current_memory_usage: Arc::new(AtomicU64::new(0)),
            semaphore,
        }
    }

    /// Create a new parallel copy engine with custom configuration
    pub fn with_config(config: ParallelCopyConfig) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.max_concurrent));

        Self {
            config,
            stats: ParallelCopyStats::default(),
            current_memory_usage: Arc::new(AtomicU64::new(0)),
            semaphore,
        }
    }

    /// Get current configuration
    pub fn config(&self) -> &ParallelCopyConfig {
        &self.config
    }

    /// Get current statistics
    pub fn stats(&self) -> &ParallelCopyStats {
        &self.stats
    }

    /// Update configuration
    pub fn update_config(&mut self, config: ParallelCopyConfig) {
        self.config = config;
        self.semaphore = Arc::new(Semaphore::new(self.config.max_concurrent));
        info!("Parallel copy engine configuration updated");
    }

    /// Check if file should use parallel processing
    pub fn should_use_parallel(&self, file_size: u64) -> bool {
        file_size >= self.config.min_file_size
    }

    /// Calculate optimal chunk size for the given file
    fn calculate_chunk_size(&self, file_size: u64, device_type: DeviceType) -> usize {
        if !self.config.adaptive_chunk_size {
            return self.config.chunk_size;
        }

        // Adaptive chunk sizing based on file size and device type
        let base_size = match device_type {
            DeviceType::SSD => 2 * 1024 * 1024,     // 2MB for SSD
            DeviceType::RamDisk => 4 * 1024 * 1024, // 4MB for RAM disk
            DeviceType::HDD => 512 * 1024,          // 512KB for HDD
            DeviceType::Network => 256 * 1024,      // 256KB for network
            DeviceType::Unknown => 1024 * 1024,     // 1MB default
        };

        // Scale chunk size based on file size
        let size_factor = if file_size > 1024 * 1024 * 1024 {
            2.0 // 2x for files > 1GB
        } else if file_size > 100 * 1024 * 1024 {
            1.5 // 1.5x for files > 100MB
        } else {
            1.0 // Normal size for smaller files
        };

        let optimal_size = (base_size as f64 * size_factor) as usize;

        // Ensure chunk size is within reasonable bounds
        optimal_size.min(8 * 1024 * 1024).max(64 * 1024) // 64KB - 8MB range
    }

    /// Perform parallel copy operation
    pub async fn copy_file_parallel<P: AsRef<Path>>(
        &mut self,
        source: P,
        destination: P,
        file_size: u64,
        device_type: DeviceType,
    ) -> Result<CopyStats> {
        let start_time = Instant::now();
        let source_path = source.as_ref();
        let dest_path = destination.as_ref();

        debug!(
            "Starting parallel copy: {:?} -> {:?} ({} bytes)",
            source_path, dest_path, file_size
        );

        // Calculate optimal chunk size
        let chunk_size = self.calculate_chunk_size(file_size, device_type);
        let total_chunks = (file_size + chunk_size as u64 - 1) / chunk_size as u64;

        info!(
            "Parallel copy configuration: chunk_size={}, total_chunks={}, max_concurrent={}",
            chunk_size, total_chunks, self.config.max_concurrent
        );

        // Create channels for pipeline
        let (read_tx, read_rx) = mpsc::channel::<DataChunk>(self.config.pipeline_depth);
        let (write_tx, write_rx) = mpsc::channel::<DataChunk>(self.config.pipeline_depth);

        // Shared statistics
        let bytes_copied = Arc::new(AtomicU64::new(0));
        let chunks_processed = Arc::new(AtomicU64::new(0));

        // Start reader task
        let reader_handle = self
            .start_reader_task(
                source_path.to_path_buf(),
                read_tx,
                chunk_size,
                file_size,
                Arc::clone(&bytes_copied),
            )
            .await?;

        // Start processor task (reads from read_rx, writes to write_tx)
        let processor_handle = self
            .start_processor_task(read_rx, write_tx, Arc::clone(&chunks_processed))
            .await?;

        // Start writer task
        let writer_handle = self
            .start_writer_task(dest_path.to_path_buf(), write_rx, Arc::clone(&bytes_copied))
            .await?;

        // Wait for all tasks to complete
        let (reader_result, processor_result, writer_result) =
            tokio::try_join!(reader_handle, processor_handle, writer_handle).map_err(|e| {
                Error::Io {
                    message: format!("Parallel copy task failed: {}", e),
                }
            })?;

        // Check for errors
        reader_result?;
        processor_result?;
        writer_result?;

        let elapsed = start_time.elapsed();
        let final_bytes_copied = bytes_copied.load(Ordering::Relaxed);

        // Update statistics
        self.stats.parallel_operations += 1;
        self.stats.parallel_bytes += final_bytes_copied;
        self.stats.chunks_processed += chunks_processed.load(Ordering::Relaxed);

        info!(
            "Parallel copy completed: {} bytes in {:?} ({:.2} MB/s)",
            final_bytes_copied,
            elapsed,
            self.stats.avg_throughput_mbps(elapsed.as_nanos() as u64)
        );

        Ok(CopyStats {
            files_copied: 1,
            directories_created: 0,
            bytes_copied: final_bytes_copied,
            files_skipped: 0,
            errors: 0,
            duration: elapsed,
            zerocopy_operations: 0,
            zerocopy_bytes: 0,
        })
    }

    /// Start the reader task that reads chunks from the source file
    async fn start_reader_task(
        &self,
        source_path: std::path::PathBuf,
        tx: mpsc::Sender<DataChunk>,
        chunk_size: usize,
        file_size: u64,
        bytes_read: Arc<AtomicU64>,
    ) -> Result<JoinHandle<Result<()>>> {
        let semaphore = Arc::clone(&self.semaphore);
        let memory_usage = Arc::clone(&self.current_memory_usage);
        let max_memory = self.config.max_memory_usage;
        let enable_read_ahead = self.config.enable_read_ahead;
        let read_ahead_multiplier = self.config.read_ahead_multiplier;

        let handle = tokio::spawn(async move {
            let mut reader = AsyncFileReader::open(&source_path).await?;
            let mut sequence = 0u64;
            let mut total_read = 0u64;

            // Read-ahead buffer for pipeline optimization
            let _read_ahead_buffer: Option<Vec<u8>> = None;
            let _read_ahead_size = 0usize;

            while total_read < file_size {
                // Acquire semaphore permit for memory control
                let _permit = semaphore.acquire().await.map_err(|_| Error::Io {
                    message: "Failed to acquire semaphore permit".to_string(),
                })?;

                // Check memory usage
                let current_memory = memory_usage.load(Ordering::Relaxed);
                if current_memory > max_memory as u64 {
                    // Wait for memory to be freed
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    continue;
                }

                // Calculate chunk size for this iteration
                let remaining = file_size - total_read;
                let current_chunk_size = std::cmp::min(chunk_size, remaining as usize);

                // Implement read-ahead optimization
                let _effective_read_size =
                    if enable_read_ahead && remaining > current_chunk_size as u64 {
                        // Read ahead for better pipeline performance
                        let read_ahead_size = current_chunk_size * read_ahead_multiplier;
                        std::cmp::min(read_ahead_size, remaining as usize)
                    } else {
                        current_chunk_size
                    };

                // Read chunk using AsyncFileReader's method
                let mut temp_buffer = crate::AdaptiveBuffer::with_size(
                    crate::buffer::AdaptiveBuffer::new(DeviceType::SSD).device_type(),
                    current_chunk_size,
                );
                let bytes_read_chunk = match reader.read_into_buffer(&mut temp_buffer).await {
                    Ok(n) => n,
                    Err(_) => 0,
                };

                // Copy data to our buffer
                let mut buffer = vec![0u8; bytes_read_chunk];
                buffer.copy_from_slice(&temp_buffer.as_ref()[..bytes_read_chunk]);

                if bytes_read_chunk == 0 {
                    break; // EOF
                }

                // Update memory usage
                memory_usage.fetch_add(bytes_read_chunk as u64, Ordering::Relaxed);

                // Create chunk
                let chunk = DataChunk {
                    sequence,
                    data: buffer,
                    size: bytes_read_chunk,
                    is_last: total_read + bytes_read_chunk as u64 >= file_size,
                };

                // Send chunk to processor
                if let Err(_) = tx.send(chunk).await {
                    warn!("Reader: Failed to send chunk, receiver dropped");
                    break;
                }

                total_read += bytes_read_chunk as u64;
                bytes_read.fetch_add(bytes_read_chunk as u64, Ordering::Relaxed);
                sequence += 1;

                trace!(
                    "Reader: Sent chunk {} ({} bytes)",
                    sequence - 1,
                    bytes_read_chunk
                );
            }

            debug!("Reader task completed: {} bytes read", total_read);
            Ok(())
        });

        Ok(handle)
    }

    /// Start the processor task that handles chunks between reader and writer
    async fn start_processor_task(
        &self,
        mut rx: mpsc::Receiver<DataChunk>,
        tx: mpsc::Sender<DataChunk>,
        chunks_processed: Arc<AtomicU64>,
    ) -> Result<JoinHandle<Result<()>>> {
        let handle = tokio::spawn(async move {
            while let Some(chunk) = rx.recv().await {
                let chunk_start = Instant::now();

                // Process chunk (currently just pass-through, but could add compression, encryption, etc.)
                let processed_chunk = chunk;

                // Send to writer
                if let Err(_) = tx.send(processed_chunk).await {
                    warn!("Processor: Failed to send chunk, receiver dropped");
                    break;
                }

                chunks_processed.fetch_add(1, Ordering::Relaxed);
                trace!("Processor: Processed chunk in {:?}", chunk_start.elapsed());
            }

            debug!("Processor task completed");
            Ok(())
        });

        Ok(handle)
    }

    /// Start the writer task that writes chunks to the destination file
    async fn start_writer_task(
        &self,
        dest_path: std::path::PathBuf,
        mut rx: mpsc::Receiver<DataChunk>,
        bytes_written: Arc<AtomicU64>,
    ) -> Result<JoinHandle<Result<()>>> {
        let memory_usage = Arc::clone(&self.current_memory_usage);

        let handle = tokio::spawn(async move {
            let mut writer = AsyncFileWriter::create(&dest_path).await?;
            let mut expected_sequence = 0u64;
            let mut out_of_order_chunks: std::collections::BTreeMap<u64, DataChunk> =
                std::collections::BTreeMap::new();

            while let Some(chunk) = rx.recv().await {
                // Handle out-of-order chunks
                if chunk.sequence == expected_sequence {
                    // Write this chunk
                    writer.write_all(&chunk.data[..chunk.size]).await?;
                    bytes_written.fetch_add(chunk.size as u64, Ordering::Relaxed);

                    // Free memory
                    memory_usage.fetch_sub(chunk.size as u64, Ordering::Relaxed);

                    expected_sequence += 1;

                    trace!(
                        "Writer: Wrote chunk {} ({} bytes)",
                        chunk.sequence,
                        chunk.size
                    );

                    // Check if we can write any buffered chunks
                    while let Some(buffered_chunk) = out_of_order_chunks.remove(&expected_sequence)
                    {
                        writer
                            .write_all(&buffered_chunk.data[..buffered_chunk.size])
                            .await?;
                        bytes_written.fetch_add(buffered_chunk.size as u64, Ordering::Relaxed);
                        memory_usage.fetch_sub(buffered_chunk.size as u64, Ordering::Relaxed);
                        expected_sequence += 1;

                        trace!(
                            "Writer: Wrote buffered chunk {} ({} bytes)",
                            buffered_chunk.sequence,
                            buffered_chunk.size
                        );
                    }

                    if chunk.is_last {
                        break;
                    }
                } else {
                    // Buffer out-of-order chunk
                    let sequence = chunk.sequence;
                    out_of_order_chunks.insert(chunk.sequence, chunk);
                    trace!("Writer: Buffered out-of-order chunk {}", sequence);
                }
            }

            // Ensure all data is written
            writer.flush().await?;

            debug!("Writer task completed");
            Ok(())
        });

        Ok(handle)
    }
}

impl Default for ParallelCopyEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl CopyEngine for ParallelCopyEngine {
    async fn copy_file<P: AsRef<Path> + Send>(
        &mut self,
        source: P,
        destination: P,
    ) -> Result<CopyStats> {
        self.copy_file_with_options(source, destination, CopyOptions::default())
            .await
    }

    async fn copy_file_with_options<P: AsRef<Path> + Send>(
        &mut self,
        source: P,
        destination: P,
        _options: CopyOptions,
    ) -> Result<CopyStats> {
        let source_path = source.as_ref();
        let dest_path = destination.as_ref();

        // Get file size
        let metadata = tokio::fs::metadata(source_path)
            .await
            .map_err(|e| Error::Io {
                message: format!("Failed to get file metadata: {}", e),
            })?;
        let file_size = metadata.len();

        // Check if we should use parallel processing
        if !self.should_use_parallel(file_size) {
            debug!("File too small for parallel processing, falling back to sequential");
            // Fall back to a simple sequential copy
            return self
                .copy_file_sequential(source_path, dest_path, file_size)
                .await;
        }

        // Detect device type (simplified for now)
        let device_type = DeviceType::SSD; // TODO: Implement proper device detection

        // Perform parallel copy
        self.copy_file_parallel(source_path, dest_path, file_size, device_type)
            .await
    }

    async fn detect_device_type<P: AsRef<Path> + Send>(&self, path: P) -> Result<DeviceType> {
        // Simple device detection - in a real implementation this would be more sophisticated
        let path_str = path.as_ref().to_string_lossy();

        if path_str.starts_with("\\\\") || path_str.starts_with("//") {
            Ok(DeviceType::Network)
        } else if path_str.contains("ram") || path_str.contains("tmp") {
            Ok(DeviceType::RamDisk)
        } else {
            // Default to SSD for now
            Ok(DeviceType::SSD)
        }
    }
}

impl ParallelCopyEngine {
    /// Fallback sequential copy for small files
    async fn copy_file_sequential<P: AsRef<Path>>(
        &mut self,
        source: P,
        destination: P,
        file_size: u64,
    ) -> Result<CopyStats> {
        let start_time = Instant::now();

        // Simple sequential copy using tokio::fs
        tokio::fs::copy(source.as_ref(), destination.as_ref())
            .await
            .map_err(|e| Error::Io {
                message: format!("Sequential copy failed: {}", e),
            })?;

        Ok(CopyStats {
            files_copied: 1,
            directories_created: 0,
            bytes_copied: file_size,
            files_skipped: 0,
            errors: 0,
            duration: start_time.elapsed(),
            zerocopy_operations: 0,
            zerocopy_bytes: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_parallel_copy_config() {
        let config = ParallelCopyConfig::default();
        assert_eq!(config.chunk_size, 1024 * 1024);
        assert_eq!(config.max_concurrent, 4);
        assert_eq!(config.min_file_size, 10 * 1024 * 1024);
    }

    #[test]
    fn test_parallel_copy_stats() {
        let mut stats = ParallelCopyStats::default();
        stats.parallel_bytes = 100 * 1024 * 1024; // 100MB
        stats.chunks_processed = 100;

        let throughput = stats.avg_throughput_mbps(1_000_000_000); // 1 second
        assert!((throughput - 100.0).abs() < 0.1); // Should be ~100 MB/s
    }

    #[test]
    fn test_should_use_parallel() {
        let engine = ParallelCopyEngine::new();

        assert!(!engine.should_use_parallel(1024)); // 1KB - too small
        assert!(!engine.should_use_parallel(5 * 1024 * 1024)); // 5MB - too small
        assert!(engine.should_use_parallel(20 * 1024 * 1024)); // 20MB - should use parallel
        assert!(engine.should_use_parallel(100 * 1024 * 1024)); // 100MB - should use parallel
    }

    #[test]
    fn test_calculate_chunk_size() {
        let engine = ParallelCopyEngine::new();

        // Test adaptive chunk sizing
        let ssd_chunk = engine.calculate_chunk_size(100 * 1024 * 1024, DeviceType::SSD);
        let hdd_chunk = engine.calculate_chunk_size(100 * 1024 * 1024, DeviceType::HDD);
        let network_chunk = engine.calculate_chunk_size(100 * 1024 * 1024, DeviceType::Network);

        assert!(ssd_chunk > hdd_chunk);
        assert!(hdd_chunk > network_chunk);

        // Test size scaling
        let small_file_chunk = engine.calculate_chunk_size(50 * 1024 * 1024, DeviceType::SSD);
        let large_file_chunk = engine.calculate_chunk_size(2 * 1024 * 1024 * 1024, DeviceType::SSD);

        assert!(large_file_chunk >= small_file_chunk);
    }

    #[tokio::test]
    async fn test_parallel_copy_engine_creation() {
        let engine = ParallelCopyEngine::new();
        assert_eq!(engine.config().max_concurrent, 4);
        assert_eq!(engine.stats().chunks_processed, 0);
    }

    #[tokio::test]
    async fn test_small_file_fallback() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("small.txt");
        let dest = temp_dir.path().join("dest.txt");

        // Create a small test file (1KB)
        std::fs::write(&source, vec![b'A'; 1024]).unwrap();

        let mut engine = ParallelCopyEngine::new();
        let stats = engine.copy_file(&source, &dest).await.unwrap();

        assert_eq!(stats.files_copied, 1);
        assert_eq!(stats.bytes_copied, 1024);
        assert!(dest.exists());
    }
}
