//! Core file operations for py-eacopy
//!
//! This module provides the main EACopy struct and file operation functionality.
//! It includes async file copying, directory traversal, progress reporting,
//! and integration with compression and network modules.

use crate::config::Config;
use crate::error::{Error, Result};
use crate::zerocopy::ZeroCopyEngine;
use crate::device_detector::{DeviceDetector, DeviceType, IOOptimizationConfig};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::sync::Semaphore;
use tracing::{debug, error, info, warn};
use walkdir::WalkDir;

/// Statistics for copy operations
#[derive(Debug, Default, Clone)]
pub struct CopyStats {
    /// Number of files copied
    pub files_copied: u64,
    /// Number of bytes copied
    pub bytes_copied: u64,
    /// Number of directories created
    pub directories_created: u64,
    /// Number of files skipped
    pub files_skipped: u64,
    /// Number of errors encountered
    pub errors: u64,
    /// Time taken for the operation
    pub duration: Duration,
    /// Average transfer speed (bytes per second)
    pub avg_speed: f64,
    /// Peak transfer speed (bytes per second)
    pub peak_speed: f64,
    /// Number of concurrent operations
    pub concurrent_operations: u64,
    /// Total number of files processed (including skipped and errors)
    pub total_files_processed: u64,
    /// Compression ratio (if compression was used)
    pub compression_ratio: Option<f64>,
    /// Number of files copied using zero-copy methods
    pub zerocopy_used: u64,
    /// Total bytes copied using zero-copy methods
    pub zerocopy_bytes: u64,
    /// Detected device type for source
    pub source_device_type: Option<DeviceType>,
    /// Detected device type for destination
    pub dest_device_type: Option<DeviceType>,
    /// Number of small files processed in batches
    pub small_files_batched: u64,
    /// Total batch operations performed
    pub batch_operations: u64,
}

impl CopyStats {
    /// Create new empty statistics
    pub fn new() -> Self {
        Self::default()
    }

    /// Add another stats instance to this one
    pub fn add(&mut self, other: &CopyStats) {
        self.files_copied += other.files_copied;
        self.bytes_copied += other.bytes_copied;
        self.directories_created += other.directories_created;
        self.files_skipped += other.files_skipped;
        self.errors += other.errors;
        self.duration += other.duration;
        self.total_files_processed += other.total_files_processed;
        self.concurrent_operations = self.concurrent_operations.max(other.concurrent_operations);
        self.peak_speed = self.peak_speed.max(other.peak_speed);
        self.zerocopy_used += other.zerocopy_used;
        self.zerocopy_bytes += other.zerocopy_bytes;
        self.small_files_batched += other.small_files_batched;
        self.batch_operations += other.batch_operations;

        // Update device types if not set
        if self.source_device_type.is_none() {
            self.source_device_type = other.source_device_type;
        }
        if self.dest_device_type.is_none() {
            self.dest_device_type = other.dest_device_type;
        }

        // Update compression ratio (weighted average)
        if let (Some(ratio1), Some(ratio2)) = (self.compression_ratio, other.compression_ratio) {
            let total_bytes = self.bytes_copied + other.bytes_copied;
            if total_bytes > 0 {
                self.compression_ratio = Some(
                    (ratio1 * self.bytes_copied as f64 + ratio2 * other.bytes_copied as f64) / total_bytes as f64
                );
            }
        } else if other.compression_ratio.is_some() {
            self.compression_ratio = other.compression_ratio;
        }
    }

    /// Calculate average speed
    pub fn calculate_speed(&mut self) {
        if self.duration.as_secs_f64() > 0.0 {
            self.avg_speed = self.bytes_copied as f64 / self.duration.as_secs_f64();
        }
    }

    /// Update peak speed if current speed is higher
    pub fn update_peak_speed(&mut self, current_speed: f64) {
        self.peak_speed = self.peak_speed.max(current_speed);
    }

    /// Get success rate as percentage
    pub fn success_rate(&self) -> f64 {
        if self.total_files_processed == 0 {
            return 100.0;
        }
        (self.files_copied as f64 / self.total_files_processed as f64) * 100.0
    }

    /// Get throughput in MB/s
    pub fn throughput_mbps(&self) -> f64 {
        self.avg_speed / (1024.0 * 1024.0)
    }

    /// Get zero-copy usage rate as percentage
    pub fn zerocopy_rate(&self) -> f64 {
        if self.files_copied == 0 {
            return 0.0;
        }
        (self.zerocopy_used as f64 / self.files_copied as f64) * 100.0
    }

    /// Get zero-copy bytes percentage
    pub fn zerocopy_bytes_rate(&self) -> f64 {
        if self.bytes_copied == 0 {
            return 0.0;
        }
        (self.zerocopy_bytes as f64 / self.bytes_copied as f64) * 100.0
    }

    /// Get batch processing efficiency (files per batch)
    pub fn batch_efficiency(&self) -> f64 {
        if self.batch_operations == 0 {
            return 0.0;
        }
        self.small_files_batched as f64 / self.batch_operations as f64
    }

    /// Get device type summary
    pub fn device_summary(&self) -> String {
        match (self.source_device_type, self.dest_device_type) {
            (Some(src), Some(dst)) => format!("{:?} -> {:?}", src, dst),
            (Some(src), None) => format!("{:?} -> Unknown", src),
            (None, Some(dst)) => format!("Unknown -> {:?}", dst),
            (None, None) => "Unknown -> Unknown".to_string(),
        }
    }
}

/// Progress callback function type
pub type ProgressCallback = Arc<dyn Fn(&ProgressInfo) + Send + Sync>;

/// Progress information passed to callbacks
#[derive(Debug, Clone)]
pub struct ProgressInfo {
    /// Current file being processed
    pub current_file: PathBuf,
    /// Bytes processed for current file
    pub current_bytes: u64,
    /// Total bytes for current file
    pub current_total: u64,
    /// Total bytes processed overall
    pub total_bytes: u64,
    /// Total bytes to process overall
    pub total_size: u64,
    /// Number of files processed
    pub files_processed: u64,
    /// Total number of files
    pub total_files: u64,
    /// Current transfer speed (bytes per second)
    pub speed: f64,
    /// Estimated time remaining
    pub eta: Option<Duration>,
}

/// File operations trait for different implementations
pub trait FileOperations {
    /// Copy a single file
    async fn copy_file<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        source: P,
        destination: Q,
    ) -> Result<CopyStats>;

    /// Copy a single file using zero-copy methods when possible
    async fn copy_file_zerocopy<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        source: P,
        destination: Q,
    ) -> Result<CopyStats>;

    /// Copy a directory tree
    async fn copy_directory<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        source: P,
        destination: Q,
    ) -> Result<CopyStats>;

    /// Check if a path exists
    async fn exists<P: AsRef<Path>>(&self, path: P) -> bool;

    /// Get file metadata
    async fn metadata<P: AsRef<Path>>(&self, path: P) -> Result<std::fs::Metadata>;
}

/// Main EACopy structure
pub struct EACopy {
    /// Configuration
    config: Config,
    /// Progress callback
    progress_callback: Option<ProgressCallback>,
    /// Statistics (thread-safe)
    stats: Arc<Mutex<CopyStats>>,
    /// Semaphore for controlling concurrent operations
    semaphore: Arc<Semaphore>,
    /// Global start time for overall operation tracking
    start_time: Option<Instant>,
    /// Zero-copy engine
    zerocopy_engine: ZeroCopyEngine,
    /// Device detector for I/O optimization
    device_detector: Arc<Mutex<DeviceDetector>>,
}

impl EACopy {
    /// Create a new EACopy instance with default configuration
    pub fn new() -> Self {
        let config = Config::new();
        let max_concurrent = config.thread_count.max(1);
        let zerocopy_engine = ZeroCopyEngine::new(config.zerocopy_enabled);

        Self {
            config,
            progress_callback: None,
            stats: Arc::new(Mutex::new(CopyStats::new())),
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            start_time: None,
            zerocopy_engine,
            device_detector: Arc::new(Mutex::new(DeviceDetector::new())),
        }
    }

    /// Create a new EACopy instance with custom configuration
    pub fn with_config(config: Config) -> Self {
        let max_concurrent = config.thread_count.max(1);
        let zerocopy_engine = ZeroCopyEngine::new(config.zerocopy_enabled);

        Self {
            config,
            progress_callback: None,
            stats: Arc::new(Mutex::new(CopyStats::new())),
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            start_time: None,
            zerocopy_engine,
            device_detector: Arc::new(Mutex::new(DeviceDetector::new())),
        }
    }

    /// Set progress callback
    pub fn with_progress_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(&ProgressInfo) + Send + Sync + 'static,
    {
        self.progress_callback = Some(Arc::new(callback));
        self
    }

    /// Get current statistics
    pub fn stats(&self) -> CopyStats {
        self.stats.lock().unwrap().clone()
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        *self.stats.lock().unwrap() = CopyStats::new();
        self.start_time = None;
    }

    /// Detect device types and optimize configuration
    pub async fn optimize_for_devices<P: AsRef<Path>, Q: AsRef<Path>>(
        &mut self,
        source: P,
        destination: Q,
    ) -> Result<()> {
        if !self.config.auto_detect_device {
            return Ok(());
        }

        let source_device = {
            let mut detector = self.device_detector.lock().unwrap();
            detector.detect_device_type(&source).await?
        };

        let dest_device = {
            let mut detector = self.device_detector.lock().unwrap();
            detector.detect_device_type(&destination).await?
        };

        debug!("Detected devices: source={:?}, dest={:?}", source_device, dest_device);

        // Update statistics with device types
        {
            let mut stats = self.stats.lock().unwrap();
            stats.source_device_type = Some(source_device);
            stats.dest_device_type = Some(dest_device);
        }

        // Optimize configuration based on detected devices
        let detector = self.device_detector.lock().unwrap();
        let source_config = detector.get_io_config(source_device);
        let dest_config = detector.get_io_config(dest_device);

        // Use the more conservative configuration
        let optimized_config = IOOptimizationConfig {
            device_type: if source_device == dest_device { source_device } else { DeviceType::Unknown },
            thread_count: source_config.thread_count.min(dest_config.thread_count),
            buffer_size: source_config.buffer_size.min(dest_config.buffer_size),
            batch_size: source_config.batch_size.min(dest_config.batch_size),
            enable_zerocopy: source_config.enable_zerocopy && dest_config.enable_zerocopy,
            read_ahead: source_config.read_ahead || dest_config.read_ahead,
        };

        // Update configuration
        self.config.io_optimization = optimized_config;
        self.config.thread_count = self.config.io_optimization.thread_count;
        self.config.buffer_size = self.config.io_optimization.buffer_size;
        self.config.small_file_batch_size = self.config.io_optimization.batch_size;
        self.config.zerocopy_enabled = self.config.io_optimization.enable_zerocopy;

        // Recreate semaphore with new thread count
        self.semaphore = Arc::new(Semaphore::new(self.config.thread_count));

        info!("Optimized configuration for devices: {:?}", self.config.io_optimization);
        Ok(())
    }



    /// Start timing for the operation
    fn start_timing(&mut self) {
        if self.start_time.is_none() {
            self.start_time = Some(Instant::now());
        }
    }

    /// Update statistics with timing information
    fn update_timing(&self) {
        if let Some(start) = self.start_time {
            let mut stats = self.stats.lock().unwrap();
            stats.duration = start.elapsed();
            stats.calculate_speed();
        }
    }

    /// Update statistics safely
    fn update_stats<F>(&self, updater: F)
    where
        F: FnOnce(&mut CopyStats),
    {
        let mut stats = self.stats.lock().unwrap();
        updater(&mut stats);
    }
}

impl Default for EACopy {
    fn default() -> Self {
        Self::new()
    }
}

impl FileOperations for EACopy {
    /// Copy a single file asynchronously
    async fn copy_file<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        source: P,
        destination: Q,
    ) -> Result<CopyStats> {
        let source = source.as_ref();
        let destination = destination.as_ref();
        let start_time = Instant::now();

        debug!("Copying file: {:?} -> {:?}", source, destination);

        // Check if source exists
        if !self.exists(source).await {
            return Err(Error::file_not_found(source));
        }

        // Get source metadata
        let metadata = self.metadata(source).await?;
        if !metadata.is_file() {
            return Err(Error::other(format!("{:?} is not a file", source)));
        }

        // Create destination directory if needed
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Check if destination exists and handle overwrite
        if self.exists(destination).await {
            return Err(Error::other(format!("Destination {:?} already exists", destination)));
        }

        // Try zero-copy first if enabled and file size meets minimum requirement
        let file_size = metadata.len();
        let (bytes_copied, zerocopy_used, zerocopy_bytes) = if self.config.zerocopy_enabled
            && file_size >= self.config.zerocopy_min_size {

            debug!("Attempting zero-copy for file size: {}", file_size);
            let zerocopy_result = self.zerocopy_engine.copy_file(source, destination, file_size).await?;

            if zerocopy_result.zerocopy_used {
                debug!("Zero-copy successful using method: {:?}", zerocopy_result.method);
                (zerocopy_result.bytes_copied, 1, zerocopy_result.bytes_copied)
            } else {
                debug!("Zero-copy failed, falling back to regular copy");
                let bytes = self.copy_file_impl(source, destination, file_size).await?;
                (bytes, 0, 0)
            }
        } else {
            debug!("Using regular copy (zero-copy disabled or file too small)");
            let bytes = self.copy_file_impl(source, destination, file_size).await?;
            (bytes, 0, 0)
        };

        // Preserve metadata if configured
        if self.config.preserve_metadata {
            self.preserve_metadata(source, destination).await?;
        }

        let duration = start_time.elapsed();
        let mut stats = CopyStats {
            files_copied: 1,
            bytes_copied,
            directories_created: 0,
            files_skipped: 0,
            errors: 0,
            duration,
            avg_speed: 0.0,
            peak_speed: 0.0,
            concurrent_operations: 1,
            total_files_processed: 1,
            compression_ratio: None,
            zerocopy_used,
            zerocopy_bytes,
            source_device_type: None,
            dest_device_type: None,
            small_files_batched: 0,
            batch_operations: 0,
        };
        stats.calculate_speed();

        info!("File copied successfully: {:?} ({} bytes)", destination, bytes_copied);
        Ok(stats)
    }

    /// Copy a single file using zero-copy methods when possible
    async fn copy_file_zerocopy<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        source: P,
        destination: Q,
    ) -> Result<CopyStats> {
        let source = source.as_ref();
        let destination = destination.as_ref();
        let start_time = Instant::now();

        debug!("Attempting zero-copy for file: {:?} -> {:?}", source, destination);

        // Check if source exists
        if !self.exists(source).await {
            return Err(Error::file_not_found(source));
        }

        // Get source metadata
        let metadata = self.metadata(source).await?;
        if !metadata.is_file() {
            return Err(Error::other(format!("{:?} is not a file", source)));
        }

        let file_size = metadata.len();

        // Check if file size meets minimum requirement for zero-copy
        if file_size < self.config.zerocopy_min_size {
            debug!("File size {} below minimum {}, falling back to regular copy",
                   file_size, self.config.zerocopy_min_size);
            return self.copy_file(source, destination).await;
        }

        // Create destination directory if needed
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Check if destination exists and handle overwrite
        if self.exists(destination).await {
            return Err(Error::other(format!("Destination {:?} already exists", destination)));
        }

        // Try zero-copy first
        let zerocopy_result = self.zerocopy_engine.copy_file(source, destination, file_size).await?;

        let (bytes_copied, zerocopy_used, zerocopy_bytes) = if zerocopy_result.zerocopy_used {
            debug!("Zero-copy successful using method: {:?}", zerocopy_result.method);
            (zerocopy_result.bytes_copied, 1, zerocopy_result.bytes_copied)
        } else {
            debug!("Zero-copy failed, falling back to regular copy");
            let bytes = self.copy_file_impl(source, destination, file_size).await?;
            (bytes, 0, 0)
        };

        // Preserve metadata if configured
        if self.config.preserve_metadata {
            self.preserve_metadata(source, destination).await?;
        }

        let duration = start_time.elapsed();
        let mut stats = CopyStats {
            files_copied: 1,
            bytes_copied,
            directories_created: 0,
            files_skipped: 0,
            errors: 0,
            duration,
            avg_speed: 0.0,
            peak_speed: 0.0,
            concurrent_operations: 1,
            total_files_processed: 1,
            compression_ratio: None,
            zerocopy_used,
            zerocopy_bytes,
            source_device_type: None,
            dest_device_type: None,
            small_files_batched: 0,
            batch_operations: 0,
        };
        stats.calculate_speed();

        info!("File copied successfully: {:?} ({} bytes, zero-copy: {})",
              destination, bytes_copied, zerocopy_used > 0);
        Ok(stats)
    }

    /// Copy a directory tree asynchronously
    async fn copy_directory<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        source: P,
        destination: Q,
    ) -> Result<CopyStats> {
        let source = source.as_ref();
        let destination = destination.as_ref();
        let start_time = Instant::now();

        debug!("Copying directory: {:?} -> {:?}", source, destination);

        // Check if source exists and is a directory
        if !self.exists(source).await {
            return Err(Error::directory_not_found(source));
        }

        let metadata = self.metadata(source).await?;
        if !metadata.is_dir() {
            return Err(Error::other(format!("{:?} is not a directory", source)));
        }

        // Create destination directory
        fs::create_dir_all(destination).await?;

        let mut total_stats = CopyStats {
            directories_created: 1,
            ..CopyStats::default()
        };

        // Walk through source directory
        let mut entries = fs::read_dir(source).await?;
        while let Some(entry) = entries.next_entry().await? {
            let entry_path = entry.path();
            let relative_path = entry_path.strip_prefix(source)
                .map_err(|_| Error::other("Failed to get relative path"))?;
            let dest_path = destination.join(relative_path);

            let entry_metadata = entry.metadata().await?;

            if entry_metadata.is_file() {
                match self.copy_file(&entry_path, &dest_path).await {
                    Ok(stats) => total_stats.add(&stats),
                    Err(e) => {
                        error!("Failed to copy file {:?}: {}", entry_path, e);
                        total_stats.errors += 1;
                    }
                }
            } else if entry_metadata.is_dir() {
                match Box::pin(self.copy_directory(&entry_path, &dest_path)).await {
                    Ok(stats) => total_stats.add(&stats),
                    Err(e) => {
                        error!("Failed to copy directory {:?}: {}", entry_path, e);
                        total_stats.errors += 1;
                    }
                }
            }
        }

        total_stats.duration = start_time.elapsed();
        total_stats.calculate_speed();

        info!("Directory copied successfully: {:?}", destination);
        Ok(total_stats)
    }

    /// Check if a path exists
    async fn exists<P: AsRef<Path>>(&self, path: P) -> bool {
        fs::metadata(path).await.is_ok()
    }

    /// Get file metadata
    async fn metadata<P: AsRef<Path>>(&self, path: P) -> Result<std::fs::Metadata> {
        fs::metadata(path).await.map_err(Error::from)
    }
}

impl EACopy {
    /// Copy multiple files concurrently
    pub async fn copy_files_batch<P: AsRef<Path> + Send, Q: AsRef<Path> + Send>(
        &self,
        file_pairs: Vec<(P, Q)>,
    ) -> Result<CopyStats> {
        use futures::future::join_all;

        let mut total_stats = CopyStats::new();
        let start_time = Instant::now();

        // Create tasks for concurrent copying
        let tasks: Vec<_> = file_pairs
            .into_iter()
            .map(|(src, dst)| {
                let eacopy = EACopy::with_config(self.config.clone());
                async move {
                    eacopy.copy_file(src, dst).await
                }
            })
            .collect();

        // Execute all tasks concurrently
        let results = join_all(tasks).await;

        // Aggregate results
        for result in results {
            match result {
                Ok(stats) => total_stats.add(&stats),
                Err(_) => total_stats.errors += 1,
            }
        }

        total_stats.duration = start_time.elapsed();
        total_stats.calculate_speed();
        total_stats.concurrent_operations = self.config.thread_count as u64;

        Ok(total_stats)
    }

    /// Copy directory using walkdir for better performance on large directories
    pub async fn copy_directory_walkdir<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        source: P,
        destination: Q,
    ) -> Result<CopyStats> {
        let source = source.as_ref();
        let destination = destination.as_ref();
        let start_time = Instant::now();

        debug!("Copying directory with walkdir: {:?} -> {:?}", source, destination);

        // Check if source exists and is a directory
        if !self.exists(source).await {
            return Err(Error::directory_not_found(source));
        }

        // Create destination directory
        fs::create_dir_all(destination).await?;

        let mut total_stats = CopyStats {
            directories_created: 1,
            ..CopyStats::default()
        };

        // Collect all files first
        let mut file_pairs = Vec::new();
        let mut dir_count = 0u64;

        for entry in WalkDir::new(source).into_iter().filter_map(|e| e.ok()) {
            let entry_path = entry.path();
            let relative_path = entry_path.strip_prefix(source)
                .map_err(|_| Error::other("Failed to get relative path"))?;
            let dest_path = destination.join(relative_path);

            if entry.file_type().is_dir() {
                // Create directory
                if let Err(e) = tokio::fs::create_dir_all(&dest_path).await {
                    warn!("Failed to create directory {:?}: {}", dest_path, e);
                    total_stats.errors += 1;
                } else {
                    dir_count += 1;
                }
            } else if entry.file_type().is_file() {
                file_pairs.push((entry_path.to_path_buf(), dest_path));
            }
        }

        total_stats.directories_created = dir_count;

        // Copy files in batches to control memory usage
        let batch_size = self.config.thread_count.max(1);
        for chunk in file_pairs.chunks(batch_size) {
            let batch_stats = self.copy_files_batch(chunk.to_vec()).await?;
            total_stats.add(&batch_stats);
        }

        total_stats.duration = start_time.elapsed();
        total_stats.calculate_speed();

        info!("Directory copied successfully: {:?}", destination);
        Ok(total_stats)
    }
    /// Internal implementation for copying a file
    async fn copy_file_impl<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        source: P,
        destination: Q,
        file_size: u64,
    ) -> Result<u64> {
        let source = source.as_ref();
        let destination = destination.as_ref();

        let source_file = fs::File::open(source).await?;
        let dest_file = fs::File::create(destination).await?;

        // Use buffered readers/writers for better performance
        let mut source_reader = BufReader::with_capacity(self.config.buffer_size, source_file);
        let mut dest_writer = BufWriter::with_capacity(self.config.buffer_size, dest_file);

        let mut buffer = vec![0u8; self.config.buffer_size];
        let mut total_bytes = 0u64;
        let start_time = Instant::now();
        let mut last_progress_update = Instant::now();

        loop {
            let bytes_read = source_reader.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }

            dest_writer.write_all(&buffer[..bytes_read]).await?;
            total_bytes += bytes_read as u64;

            // Report progress if callback is set and enough time has passed
            let now = Instant::now();
            if let Some(callback) = &self.progress_callback {
                if now.duration_since(last_progress_update) >= Duration::from_millis(100) {
                    let elapsed = start_time.elapsed();
                    let speed = if elapsed.as_secs_f64() > 0.0 {
                        total_bytes as f64 / elapsed.as_secs_f64()
                    } else {
                        0.0
                    };

                    let eta = if speed > 0.0 && total_bytes < file_size {
                        let remaining_bytes = file_size - total_bytes;
                        Some(Duration::from_secs_f64(remaining_bytes as f64 / speed))
                    } else {
                        None
                    };

                    let progress = ProgressInfo {
                        current_file: source.to_path_buf(),
                        current_bytes: total_bytes,
                        current_total: file_size,
                        total_bytes: total_bytes,
                        total_size: file_size,
                        files_processed: 0,
                        total_files: 1,
                        speed,
                        eta,
                    };

                    callback(&progress);
                    last_progress_update = now;
                }
            }
        }

        dest_writer.flush().await?;
        Ok(total_bytes)
    }

    /// Preserve file metadata from source to destination
    async fn preserve_metadata<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        source: P,
        destination: Q,
    ) -> Result<()> {
        let source_metadata = fs::metadata(source).await?;

        // Set file times
        if let Ok(modified) = source_metadata.modified() {
            if let Ok(accessed) = source_metadata.accessed() {
                let modified_time = filetime::FileTime::from_system_time(modified);
                let accessed_time = filetime::FileTime::from_system_time(accessed);

                filetime::set_file_times(destination.as_ref(), accessed_time, modified_time)
                    .map_err(|e| Error::other(format!("Failed to set file times: {}", e)))?;
            }
        }

        // Set permissions on Unix systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = source_metadata.permissions();
            fs::set_permissions(destination, permissions).await?;
        }

        Ok(())
    }

    /// Copy with network server acceleration
    pub async fn copy_with_server<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        source: P,
        destination: Q,
        _server_addr: &str,
        _port: u16,
    ) -> Result<CopyStats> {
        // This will be implemented in the network module
        // For now, fall back to local copy
        warn!("Network copy not yet implemented, falling back to local copy");
        self.copy_file(source, destination).await
    }

    /// Perform delta copy using a reference file
    pub async fn delta_copy<P: AsRef<Path>, Q: AsRef<Path>, R: AsRef<Path>>(
        &self,
        source: P,
        destination: Q,
        _reference: R,
    ) -> Result<CopyStats> {
        // This will be implemented in the delta module
        // For now, fall back to regular copy
        warn!("Delta copy not yet implemented, falling back to regular copy");
        self.copy_file(source, destination).await
    }
}
