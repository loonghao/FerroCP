//! Compression module for py-eacopy
//!
//! This module provides adaptive compression functionality using zstd.
//! It can automatically adjust compression levels based on network conditions
//! and provides streaming compression for large files.

use crate::config::CompressionConfig;
use crate::error::{Error, Result};
use async_compression::tokio::write::ZstdEncoder;
use async_compression::tokio::bufread::ZstdDecoder;
use async_compression::Level;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::io::{AsyncRead, AsyncWrite, AsyncReadExt, AsyncWriteExt, BufReader};
use tracing::{debug, info};
// Note: Dictionary support would require direct zstd bindings
// use zstd::dict::{EncoderDictionary, DecoderDictionary};

/// Network monitor for adaptive compression
#[derive(Debug)]
pub struct NetworkMonitor {
    /// Current network throughput (bytes per second)
    throughput: AtomicU64,
    /// Peak throughput observed
    peak_throughput: AtomicU64,
    /// Average throughput over time
    avg_throughput: AtomicU64,
    /// Last measurement time
    last_measurement: Mutex<Instant>,
    /// Throughput history for calculating moving average
    throughput_history: Mutex<Vec<(Instant, f64)>>,
    /// Maximum history size
    max_history_size: usize,
}

impl NetworkMonitor {
    /// Create a new network monitor
    pub fn new() -> Self {
        Self {
            throughput: AtomicU64::new(0),
            peak_throughput: AtomicU64::new(0),
            avg_throughput: AtomicU64::new(0),
            last_measurement: Mutex::new(Instant::now()),
            throughput_history: Mutex::new(Vec::new()),
            max_history_size: 100, // Keep last 100 measurements
        }
    }

    /// Update throughput measurement with smoothing
    pub fn update_throughput(&self, bytes: u64, duration: Duration) {
        if duration.as_secs_f64() > 0.0 {
            let current_throughput = bytes as f64 / duration.as_secs_f64();
            let current_throughput_u64 = current_throughput as u64;

            // Update current throughput
            self.throughput.store(current_throughput_u64, Ordering::Relaxed);

            // Update peak throughput
            let current_peak = self.peak_throughput.load(Ordering::Relaxed);
            if current_throughput_u64 > current_peak {
                self.peak_throughput.store(current_throughput_u64, Ordering::Relaxed);
            }

            // Update history and calculate moving average
            let now = Instant::now();
            let mut history = self.throughput_history.lock().unwrap();
            history.push((now, current_throughput));

            // Keep only recent measurements (last 60 seconds)
            let cutoff_time = now - Duration::from_secs(60);
            history.retain(|(time, _)| *time > cutoff_time);

            // Limit history size
            if history.len() > self.max_history_size {
                let excess = history.len() - self.max_history_size;
                history.drain(0..excess);
            }

            // Calculate moving average
            if !history.is_empty() {
                let avg = history.iter().map(|(_, throughput)| *throughput).sum::<f64>() / history.len() as f64;
                self.avg_throughput.store(avg as u64, Ordering::Relaxed);
            }

            // Update last measurement time
            *self.last_measurement.lock().unwrap() = now;
        }
    }

    /// Get current throughput
    pub fn get_throughput(&self) -> f64 {
        self.throughput.load(Ordering::Relaxed) as f64
    }

    /// Get peak throughput
    pub fn get_peak_throughput(&self) -> f64 {
        self.peak_throughput.load(Ordering::Relaxed) as f64
    }

    /// Get average throughput
    pub fn get_avg_throughput(&self) -> f64 {
        self.avg_throughput.load(Ordering::Relaxed) as f64
    }

    /// Get smoothed throughput for compression decisions
    pub fn get_smoothed_throughput(&self) -> f64 {
        let current = self.get_throughput();
        let average = self.get_avg_throughput();

        // Use weighted average: 70% current, 30% historical average
        if average > 0.0 {
            current * 0.7 + average * 0.3
        } else {
            current
        }
    }

    /// Check if network is stable (low variance in throughput)
    pub fn is_network_stable(&self) -> bool {
        let history = self.throughput_history.lock().unwrap();
        if history.len() < 5 {
            return false;
        }

        let values: Vec<f64> = history.iter().map(|(_, throughput)| *throughput).collect();
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let variance = values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / values.len() as f64;
        let std_dev = variance.sqrt();

        // Network is stable if standard deviation is less than 20% of mean
        std_dev < mean * 0.2
    }

    /// Reset all measurements
    pub fn reset(&self) {
        self.throughput.store(0, Ordering::Relaxed);
        self.peak_throughput.store(0, Ordering::Relaxed);
        self.avg_throughput.store(0, Ordering::Relaxed);
        *self.last_measurement.lock().unwrap() = Instant::now();
        self.throughput_history.lock().unwrap().clear();
    }
}

/// Adaptive compression engine
pub struct CompressionEngine {
    /// Configuration
    config: CompressionConfig,
    /// Network monitor for adaptive compression
    network_monitor: Arc<NetworkMonitor>,
    /// Compression statistics
    stats: Arc<Mutex<CompressionEngineStats>>,
}

/// Result of a compression operation
#[derive(Debug, Clone)]
pub struct CompressionResult {
    /// Original size before compression
    pub original_size: u64,
    /// Size after compression
    pub compressed_size: u64,
    /// Compression ratio (compressed_size / original_size)
    pub compression_ratio: f64,
    /// Compression level used
    pub compression_level: i32,
    /// Time taken for compression
    pub duration: Duration,
    /// Whether compression was actually applied
    pub was_compressed: bool,
}

impl CompressionResult {
    /// Get compression savings in bytes
    pub fn bytes_saved(&self) -> u64 {
        if self.original_size > self.compressed_size {
            self.original_size - self.compressed_size
        } else {
            0
        }
    }

    /// Get compression savings as percentage
    pub fn savings_percentage(&self) -> f64 {
        if self.original_size > 0 {
            (self.bytes_saved() as f64 / self.original_size as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Get compression throughput in MB/s
    pub fn throughput_mbps(&self) -> f64 {
        if self.duration.as_secs_f64() > 0.0 {
            (self.original_size as f64 / (1024.0 * 1024.0)) / self.duration.as_secs_f64()
        } else {
            0.0
        }
    }
}

/// Detailed compression engine statistics
#[derive(Debug, Default, Clone)]
pub struct CompressionEngineStats {
    /// Total bytes processed (uncompressed)
    pub total_bytes_processed: u64,
    /// Total bytes after compression
    pub total_bytes_compressed: u64,
    /// Number of compression operations
    pub compression_operations: u64,
    /// Number of decompression operations
    pub decompression_operations: u64,
    /// Total compression time
    pub total_compression_time: Duration,
    /// Total decompression time
    pub total_decompression_time: Duration,
    /// Best compression ratio achieved
    pub best_compression_ratio: f64,
    /// Worst compression ratio achieved
    pub worst_compression_ratio: f64,
    /// Average compression level used
    pub avg_compression_level: f64,
    /// Number of adaptive level changes
    pub adaptive_level_changes: u64,
}

impl CompressionEngine {
    /// Create a new compression engine
    pub fn new(config: CompressionConfig) -> Self {
        Self {
            config,
            network_monitor: Arc::new(NetworkMonitor::new()),
            stats: Arc::new(Mutex::new(CompressionEngineStats::default())),
        }
    }

    // Note: Dictionary support would require direct zstd bindings
    // For now, we'll focus on adaptive compression without dictionaries

    /// Get network monitor
    pub fn network_monitor(&self) -> Arc<NetworkMonitor> {
        self.network_monitor.clone()
    }

    /// Get compression statistics
    pub fn get_detailed_stats(&self) -> CompressionEngineStats {
        self.stats.lock().unwrap().clone()
    }

    /// Reset compression statistics
    pub fn reset_stats(&self) {
        *self.stats.lock().unwrap() = CompressionEngineStats::default();
        self.network_monitor.reset();
    }

    /// Determine optimal compression level based on network conditions
    pub fn adaptive_compression_level(&self, _network_speed: f64) -> i32 {
        if !self.config.adaptive {
            return self.config.level;
        }

        // Use smoothed throughput for more stable decisions
        let smoothed_speed = self.network_monitor.get_smoothed_throughput();
        let is_stable = self.network_monitor.is_network_stable();

        // Adaptive algorithm with multiple thresholds and stability consideration
        const VERY_HIGH_SPEED: f64 = 1000.0 * 1024.0 * 1024.0; // 1 GB/s
        const HIGH_SPEED: f64 = 100.0 * 1024.0 * 1024.0;       // 100 MB/s
        const MEDIUM_SPEED: f64 = 10.0 * 1024.0 * 1024.0;      // 10 MB/s
        const LOW_SPEED: f64 = 1.0 * 1024.0 * 1024.0;          // 1 MB/s

        let base_level = if smoothed_speed > VERY_HIGH_SPEED {
            // Very fast network: minimal compression for maximum speed
            std::cmp::max(1, self.config.level - 3)
        } else if smoothed_speed > HIGH_SPEED {
            // Fast network: prioritize speed over compression ratio
            std::cmp::max(1, self.config.level - 2)
        } else if smoothed_speed > MEDIUM_SPEED {
            // Medium network: balanced approach
            self.config.level
        } else if smoothed_speed > LOW_SPEED {
            // Slow network: prioritize compression ratio
            std::cmp::min(22, self.config.level + 2)
        } else {
            // Very slow network: maximum compression
            std::cmp::min(22, self.config.level + 4)
        };

        // Adjust based on network stability
        let adjusted_level = if is_stable {
            // Stable network: can use slightly higher compression
            std::cmp::min(22, base_level + 1)
        } else {
            // Unstable network: use slightly lower compression for safety
            std::cmp::max(1, base_level - 1)
        };

        // Update statistics if level changed
        let mut stats = self.stats.lock().unwrap();
        if adjusted_level != self.config.level {
            stats.adaptive_level_changes += 1;
        }

        // Update average compression level
        let total_ops = stats.compression_operations + 1;
        stats.avg_compression_level = (stats.avg_compression_level * (total_ops - 1) as f64 + adjusted_level as f64) / total_ops as f64;

        debug!("Adaptive compression: speed={:.2} MB/s, stable={}, level={}",
               smoothed_speed / (1024.0 * 1024.0), is_stable, adjusted_level);

        adjusted_level
    }

    /// Compress data stream with detailed statistics
    pub async fn compress_stream<R, W>(&self, reader: R, writer: W, file_size: u64) -> Result<CompressionResult>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        let start_time = Instant::now();

        if !self.config.enabled || file_size < self.config.min_file_size {
            // No compression, just copy
            let bytes_written = self.copy_stream(reader, writer).await?;
            return Ok(CompressionResult {
                original_size: file_size,
                compressed_size: bytes_written,
                compression_ratio: 1.0,
                compression_level: 0,
                duration: start_time.elapsed(),
                was_compressed: false,
            });
        }

        let network_speed = self.network_monitor.get_smoothed_throughput();
        let compression_level = self.adaptive_compression_level(network_speed);

        debug!("Using compression level {} (network speed: {:.2} MB/s)",
               compression_level, network_speed / (1024.0 * 1024.0));

        // Create encoder
        let mut encoder = ZstdEncoder::with_quality(writer, Level::Precise(compression_level));

        let bytes_written = self.copy_stream_with_progress(reader, &mut encoder, file_size).await?;
        encoder.shutdown().await?;

        let duration = start_time.elapsed();
        let compression_ratio = if file_size > 0 {
            bytes_written as f64 / file_size as f64
        } else {
            1.0
        };

        // Update statistics
        {
            let mut stats = self.stats.lock().unwrap();
            stats.total_bytes_processed += file_size;
            stats.total_bytes_compressed += bytes_written;
            stats.compression_operations += 1;
            stats.total_compression_time += duration;

            // Update best/worst compression ratios
            if stats.compression_operations == 1 {
                stats.best_compression_ratio = compression_ratio;
                stats.worst_compression_ratio = compression_ratio;
            } else {
                stats.best_compression_ratio = stats.best_compression_ratio.min(compression_ratio);
                stats.worst_compression_ratio = stats.worst_compression_ratio.max(compression_ratio);
            }
        }

        info!("Compressed {} bytes to {} bytes (ratio: {:.3}, level: {}, time: {:?})",
              file_size, bytes_written, compression_ratio, compression_level, duration);

        Ok(CompressionResult {
            original_size: file_size,
            compressed_size: bytes_written,
            compression_ratio,
            compression_level,
            duration,
            was_compressed: true,
        })
    }

    /// Decompress data stream with detailed statistics
    pub async fn decompress_stream<R, W>(&self, reader: R, writer: W) -> Result<CompressionResult>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        let start_time = Instant::now();

        // Create buffered reader for better performance
        let buf_reader = BufReader::new(reader);

        // Create decoder
        let mut decoder = ZstdDecoder::new(buf_reader);

        let bytes_written = self.copy_stream(&mut decoder, writer).await?;
        let duration = start_time.elapsed();

        // Update statistics
        {
            let mut stats = self.stats.lock().unwrap();
            stats.decompression_operations += 1;
            stats.total_decompression_time += duration;
        }

        info!("Decompressed {} bytes in {:?}", bytes_written, duration);

        Ok(CompressionResult {
            original_size: 0, // Unknown for decompression
            compressed_size: bytes_written,
            compression_ratio: 1.0, // Unknown for decompression
            compression_level: 0,
            duration,
            was_compressed: false, // This was decompression
        })
    }

    /// Copy data from reader to writer
    async fn copy_stream<R, W>(&self, mut reader: R, mut writer: W) -> Result<u64>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        let mut buffer = vec![0u8; 64 * 1024]; // 64KB buffer
        let mut total_bytes = 0u64;

        loop {
            let bytes_read = reader.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }

            writer.write_all(&buffer[..bytes_read]).await?;
            total_bytes += bytes_read as u64;
        }

        writer.flush().await?;
        Ok(total_bytes)
    }

    /// Copy data from reader to writer with progress tracking
    async fn copy_stream_with_progress<R, W>(&self, mut reader: R, mut writer: W, _total_size: u64) -> Result<u64>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        let mut buffer = vec![0u8; 64 * 1024]; // 64KB buffer
        let mut total_bytes = 0u64;
        let start_time = Instant::now();
        let mut last_update = start_time;

        loop {
            let bytes_read = reader.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }

            writer.write_all(&buffer[..bytes_read]).await?;
            total_bytes += bytes_read as u64;

            // Update network monitor periodically
            let now = Instant::now();
            if now.duration_since(last_update) >= Duration::from_millis(100) {
                let chunk_duration = now.duration_since(last_update);
                self.network_monitor.update_throughput(bytes_read as u64, chunk_duration);
                last_update = now;
            }
        }

        writer.flush().await?;

        // Final throughput update
        let total_duration = start_time.elapsed();
        if total_duration.as_millis() > 0 {
            self.network_monitor.update_throughput(total_bytes, total_duration);
        }

        Ok(total_bytes)
    }

    /// Check if compression is beneficial for the given file size
    pub fn should_compress(&self, file_size: u64) -> bool {
        self.config.enabled && file_size >= self.config.min_file_size
    }

    /// Get compression statistics
    pub fn get_stats(&self) -> CompressionStats {
        let detailed_stats = self.get_detailed_stats();
        let network_monitor = &self.network_monitor;

        CompressionStats {
            current_level: self.config.level,
            adaptive_enabled: self.config.adaptive,
            network_speed: network_monitor.get_throughput(),
            peak_network_speed: network_monitor.get_peak_throughput(),
            avg_network_speed: network_monitor.get_avg_throughput(),
            network_stable: network_monitor.is_network_stable(),
            total_operations: detailed_stats.compression_operations + detailed_stats.decompression_operations,
            compression_operations: detailed_stats.compression_operations,
            decompression_operations: detailed_stats.decompression_operations,
            total_bytes_processed: detailed_stats.total_bytes_processed,
            total_bytes_compressed: detailed_stats.total_bytes_compressed,
            overall_compression_ratio: if detailed_stats.total_bytes_processed > 0 {
                detailed_stats.total_bytes_compressed as f64 / detailed_stats.total_bytes_processed as f64
            } else {
                1.0
            },
            best_compression_ratio: detailed_stats.best_compression_ratio,
            worst_compression_ratio: detailed_stats.worst_compression_ratio,
            avg_compression_level: detailed_stats.avg_compression_level,
            adaptive_level_changes: detailed_stats.adaptive_level_changes,
        }
    }

    /// Estimate compression benefit for a given file size and network speed
    pub fn estimate_compression_benefit(&self, file_size: u64, network_speed: f64) -> CompressionEstimate {
        if !self.should_compress(file_size) {
            return CompressionEstimate {
                recommended: false,
                estimated_ratio: 1.0,
                estimated_time_savings: Duration::ZERO,
                estimated_bandwidth_savings: 0,
            };
        }

        let compression_level = self.adaptive_compression_level(network_speed);

        // Estimate compression ratio based on level (rough approximation)
        let estimated_ratio = match compression_level {
            1..=3 => 0.7,   // Fast compression: ~30% reduction
            4..=6 => 0.6,   // Balanced: ~40% reduction
            7..=12 => 0.5,  // Good compression: ~50% reduction
            13..=22 => 0.4, // Best compression: ~60% reduction
            _ => 0.7,
        };

        let compressed_size = (file_size as f64 * estimated_ratio) as u64;
        let bandwidth_savings = file_size - compressed_size;

        // Estimate time savings based on network speed
        let uncompressed_transfer_time = file_size as f64 / network_speed;
        let compressed_transfer_time = compressed_size as f64 / network_speed;
        let time_savings = Duration::from_secs_f64(uncompressed_transfer_time - compressed_transfer_time);

        CompressionEstimate {
            recommended: bandwidth_savings > 0 && time_savings > Duration::from_millis(100),
            estimated_ratio,
            estimated_time_savings: time_savings,
            estimated_bandwidth_savings: bandwidth_savings,
        }
    }
}

/// Compression statistics
#[derive(Debug, Clone)]
pub struct CompressionStats {
    /// Current compression level
    pub current_level: i32,
    /// Whether adaptive compression is enabled
    pub adaptive_enabled: bool,
    /// Current network speed (bytes per second)
    pub network_speed: f64,
    /// Peak network speed observed
    pub peak_network_speed: f64,
    /// Average network speed
    pub avg_network_speed: f64,
    /// Whether network is stable
    pub network_stable: bool,
    /// Total operations (compression + decompression)
    pub total_operations: u64,
    /// Number of compression operations
    pub compression_operations: u64,
    /// Number of decompression operations
    pub decompression_operations: u64,
    /// Total bytes processed (uncompressed)
    pub total_bytes_processed: u64,
    /// Total bytes after compression
    pub total_bytes_compressed: u64,
    /// Overall compression ratio
    pub overall_compression_ratio: f64,
    /// Best compression ratio achieved
    pub best_compression_ratio: f64,
    /// Worst compression ratio achieved
    pub worst_compression_ratio: f64,
    /// Average compression level used
    pub avg_compression_level: f64,
    /// Number of adaptive level changes
    pub adaptive_level_changes: u64,
}

/// Compression benefit estimation
#[derive(Debug, Clone)]
pub struct CompressionEstimate {
    /// Whether compression is recommended
    pub recommended: bool,
    /// Estimated compression ratio
    pub estimated_ratio: f64,
    /// Estimated time savings from compression
    pub estimated_time_savings: Duration,
    /// Estimated bandwidth savings in bytes
    pub estimated_bandwidth_savings: u64,
}

impl Default for CompressionEngine {
    fn default() -> Self {
        Self::new(CompressionConfig::default())
    }
}

/// Convenience functions for compression
pub mod ops {
    use super::*;

    /// Compress bytes using zstd
    pub fn compress_bytes(data: &[u8], level: i32) -> Result<Vec<u8>> {
        zstd::bulk::compress(data, level).map_err(Error::from)
    }

    /// Decompress bytes using zstd
    pub fn decompress_bytes(data: &[u8]) -> Result<Vec<u8>> {
        zstd::bulk::decompress(data, 1024 * 1024 * 100) // 100MB limit
            .map_err(Error::from)
    }

    /// Get compression ratio
    pub fn compression_ratio(original_size: u64, compressed_size: u64) -> f64 {
        if original_size == 0 {
            return 1.0;
        }
        compressed_size as f64 / original_size as f64
    }
}
