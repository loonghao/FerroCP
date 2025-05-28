//! High-performance file copying engine

use crate::{AdaptiveBuffer, AsyncFileReader, AsyncFileWriter, BufferPool};
use ferrocp_types::{CopyStats, DeviceType, Error, ProgressInfo, Result};
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::fs;
use tracing::{debug, info};

/// Copy options for customizing copy behavior
#[derive(Debug, Clone)]
pub struct CopyOptions {
    /// Buffer size for I/O operations
    pub buffer_size: Option<usize>,
    /// Enable progress reporting
    pub enable_progress: bool,
    /// Progress reporting interval
    pub progress_interval: Duration,
    /// Enable verification after copy
    pub verify_copy: bool,
    /// Preserve file metadata
    pub preserve_metadata: bool,
    /// Enable zero-copy optimizations
    pub enable_zero_copy: bool,
    /// Maximum number of retry attempts
    pub max_retries: u32,
}

impl Default for CopyOptions {
    fn default() -> Self {
        Self {
            buffer_size: None, // Auto-detect based on device
            enable_progress: true,
            progress_interval: Duration::from_millis(100),
            verify_copy: false,
            preserve_metadata: true,
            enable_zero_copy: true,
            max_retries: 3,
        }
    }
}

/// Trait for copy engines
#[async_trait::async_trait]
pub trait CopyEngine {
    /// Copy a single file
    async fn copy_file<P: AsRef<Path> + Send>(
        &mut self,
        source: P,
        destination: P,
    ) -> Result<CopyStats>;

    /// Copy a file with custom options
    async fn copy_file_with_options<P: AsRef<Path> + Send>(
        &mut self,
        source: P,
        destination: P,
        options: CopyOptions,
    ) -> Result<CopyStats>;

    /// Get the device type for a path
    async fn detect_device_type<P: AsRef<Path> + Send>(&self, path: P) -> Result<DeviceType>;
}

/// Buffered copy engine with adaptive buffering
#[derive(Debug)]
pub struct BufferedCopyEngine {
    buffer_pool: BufferPool,
    default_options: CopyOptions,
}

impl BufferedCopyEngine {
    /// Create a new buffered copy engine
    pub fn new() -> Self {
        Self {
            buffer_pool: BufferPool::default(),
            default_options: CopyOptions::default(),
        }
    }

    /// Create a new buffered copy engine with custom buffer pool
    pub fn with_buffer_pool(buffer_pool: BufferPool) -> Self {
        Self {
            buffer_pool,
            default_options: CopyOptions::default(),
        }
    }

    /// Set default copy options
    pub fn with_default_options(mut self, options: CopyOptions) -> Self {
        self.default_options = options;
        self
    }

    /// Copy file with detailed progress tracking
    async fn copy_file_internal<P: AsRef<Path>>(
        &mut self,
        source: P,
        destination: P,
        options: CopyOptions,
    ) -> Result<CopyStats> {
        let source_path = source.as_ref();
        let dest_path = destination.as_ref();

        info!(
            "Starting copy: {} -> {}",
            source_path.display(),
            dest_path.display()
        );

        let start_time = Instant::now();
        let mut stats = CopyStats::new();

        // Get source file metadata
        let source_metadata = fs::metadata(source_path).await.map_err(|e| Error::Io {
            message: format!("Failed to read source metadata: {}", e),
        })?;

        let file_size = source_metadata.len();
        stats.bytes_copied = 0;

        // Detect device types for optimization
        let source_device = self.detect_device_type(source_path).await?;
        let dest_device = self.detect_device_type(dest_path).await?;

        debug!(
            "Source device: {:?}, Destination device: {:?}",
            source_device, dest_device
        );

        // Determine optimal buffer size
        let buffer_size = options.buffer_size.unwrap_or_else(|| {
            Self::calculate_optimal_buffer_size(file_size, source_device, dest_device)
        });

        // Create adaptive buffer
        let mut buffer = AdaptiveBuffer::with_size(source_device, buffer_size);

        // Open source and destination files
        let mut reader = AsyncFileReader::open(source_path).await?;
        let mut writer = AsyncFileWriter::create(dest_path).await?;

        // Copy data with progress tracking
        let mut bytes_copied = 0u64;
        let mut last_progress_time = Instant::now();

        loop {
            // Read chunk
            let bytes_read = reader.read_into_buffer(&mut buffer).await?;
            if bytes_read == 0 {
                break; // EOF
            }

            // Write chunk
            let bytes_written = writer.write_from_buffer(&buffer, bytes_read).await?;
            bytes_copied += bytes_written as u64;

            // Update statistics
            stats.bytes_copied = bytes_copied;

            // Report progress if enabled
            if options.enable_progress && last_progress_time.elapsed() >= options.progress_interval
            {
                let progress = ProgressInfo {
                    current_file: source_path.to_path_buf(),
                    current_file_bytes: bytes_copied,
                    current_file_size: file_size,
                    files_processed: 1,
                    total_files: 1,
                    bytes_processed: bytes_copied,
                    total_bytes: file_size,
                    transfer_rate: bytes_copied as f64 / start_time.elapsed().as_secs_f64(),
                    eta: if bytes_copied > 0 {
                        let rate = bytes_copied as f64 / start_time.elapsed().as_secs_f64();
                        let remaining_bytes = file_size.saturating_sub(bytes_copied);
                        Some(Duration::from_secs_f64(remaining_bytes as f64 / rate))
                    } else {
                        None
                    },
                };

                debug!("Copy progress: {:.1}%", progress.current_file_progress());
                last_progress_time = Instant::now();
            }

            buffer.clear();
        }

        // Ensure all data is written
        writer.flush().await?;

        // Preserve metadata if requested
        if options.preserve_metadata {
            self.preserve_file_metadata(source_path, dest_path).await?;
        }

        // Verify copy if requested
        if options.verify_copy {
            self.verify_copy(source_path, dest_path).await?;
        }

        stats.duration = start_time.elapsed();
        stats.files_copied = 1;

        info!(
            "Copy completed: {} bytes in {:?}",
            stats.bytes_copied, stats.duration
        );
        Ok(stats)
    }

    /// Calculate optimal buffer size based on file size and device types
    fn calculate_optimal_buffer_size(
        file_size: u64,
        source_device: DeviceType,
        dest_device: DeviceType,
    ) -> usize {
        // Base size on the slower device
        let base_size = match (source_device, dest_device) {
            (DeviceType::SSD, DeviceType::SSD) => 1024 * 1024, // 1MB
            (DeviceType::HDD, _) | (_, DeviceType::HDD) => 64 * 1024, // 64KB
            (DeviceType::Network, _) | (_, DeviceType::Network) => 128 * 1024, // 128KB
            (DeviceType::RamDisk, DeviceType::RamDisk) => 8 * 1024 * 1024, // 8MB
            _ => 256 * 1024,                                   // 256KB default
        };

        // Adjust based on file size
        if file_size < 1024 * 1024 {
            // Small files: use smaller buffer
            base_size / 4
        } else if file_size > 100 * 1024 * 1024 {
            // Large files: use larger buffer
            base_size * 2
        } else {
            base_size
        }
    }

    /// Preserve file metadata from source to destination
    async fn preserve_file_metadata<P: AsRef<Path>>(
        &self,
        source: P,
        destination: P,
    ) -> Result<()> {
        let source_metadata = fs::metadata(source.as_ref()).await.map_err(|e| Error::Io {
            message: format!("Failed to read source metadata: {}", e),
        })?;

        // Set file times
        let accessed = source_metadata
            .accessed()
            .unwrap_or_else(|_| std::time::SystemTime::now());
        let modified = source_metadata
            .modified()
            .unwrap_or_else(|_| std::time::SystemTime::now());

        filetime::set_file_times(
            destination.as_ref(),
            filetime::FileTime::from_system_time(accessed),
            filetime::FileTime::from_system_time(modified),
        )
        .map_err(|e| Error::Io {
            message: format!("Failed to set file times: {}", e),
        })?;

        // Set permissions on Unix systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = source_metadata.permissions();
            fs::set_permissions(destination.as_ref(), permissions)
                .await
                .map_err(|e| Error::Io {
                    message: format!("Failed to set permissions: {}", e),
                })?;
        }

        Ok(())
    }

    /// Verify that the copy was successful
    async fn verify_copy<P: AsRef<Path>>(&self, source: P, destination: P) -> Result<()> {
        let source_metadata = fs::metadata(source.as_ref()).await.map_err(|e| Error::Io {
            message: format!("Failed to read source metadata: {}", e),
        })?;

        let dest_metadata = fs::metadata(destination.as_ref())
            .await
            .map_err(|e| Error::Io {
                message: format!("Failed to read destination metadata: {}", e),
            })?;

        if source_metadata.len() != dest_metadata.len() {
            return Err(Error::other(format!(
                "File size mismatch: source {} bytes, destination {} bytes",
                source_metadata.len(),
                dest_metadata.len()
            )));
        }

        debug!("Copy verification successful");
        Ok(())
    }
}

#[async_trait::async_trait]
impl CopyEngine for BufferedCopyEngine {
    async fn copy_file<P: AsRef<Path> + Send>(
        &mut self,
        source: P,
        destination: P,
    ) -> Result<CopyStats> {
        self.copy_file_with_options(source, destination, self.default_options.clone())
            .await
    }

    async fn copy_file_with_options<P: AsRef<Path> + Send>(
        &mut self,
        source: P,
        destination: P,
        options: CopyOptions,
    ) -> Result<CopyStats> {
        self.copy_file_internal(source, destination, options).await
    }

    async fn detect_device_type<P: AsRef<Path> + Send>(&self, _path: P) -> Result<DeviceType> {
        // TODO: Implement actual device detection
        // For now, return SSD as default
        Ok(DeviceType::SSD)
    }
}

impl Default for BufferedCopyEngine {
    fn default() -> Self {
        Self::new()
    }
}
