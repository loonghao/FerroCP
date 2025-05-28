//! Device performance analysis for FerroCP
//!
//! This module provides functionality to analyze storage device performance
//! characteristics and provide optimization recommendations.

use ferrocp_types::{DeviceType, Error, Result};
use std::path::Path;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// File system information
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FileSystemInfo {
    /// File system type (NTFS, ext4, APFS, etc.)
    pub fs_type: String,
    /// Total space in bytes
    pub total_space: u64,
    /// Available space in bytes
    pub available_space: u64,
    /// Block size in bytes
    pub block_size: u64,
    /// Whether the filesystem supports Copy-on-Write
    pub supports_cow: bool,
    /// Whether the filesystem supports sparse files
    pub supports_sparse: bool,
    /// Whether the filesystem supports compression
    pub supports_compression: bool,
    /// Whether the filesystem supports deduplication
    pub supports_dedup: bool,
}

/// Device performance characteristics
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DevicePerformance {
    /// Sequential read speed in MB/s
    pub sequential_read_speed: f64,
    /// Sequential write speed in MB/s
    pub sequential_write_speed: f64,
    /// Random read IOPS
    pub random_read_iops: f64,
    /// Random write IOPS
    pub random_write_iops: f64,
    /// Average latency in microseconds
    pub average_latency: f64,
    /// Optimal I/O size in bytes
    pub optimal_io_size: usize,
    /// Whether the device supports TRIM/UNMAP
    pub supports_trim: bool,
    /// Device queue depth
    pub queue_depth: u32,
}

/// Device analyzer for performance testing and optimization
#[derive(Debug)]
pub struct DeviceAnalyzer {
    /// Test file size for performance benchmarks
    test_file_size: u64,
    /// Number of test iterations
    test_iterations: u32,
}

impl DeviceAnalyzer {
    /// Create a new device analyzer
    pub fn new() -> Self {
        Self {
            test_file_size: 1024 * 1024, // 1MB default test size
            test_iterations: 3,
        }
    }

    /// Create a device analyzer with custom test parameters
    pub fn with_test_params(test_file_size: u64, test_iterations: u32) -> Self {
        Self {
            test_file_size,
            test_iterations,
        }
    }

    /// Analyze device performance characteristics
    pub async fn analyze_device_performance(
        &self,
        device_type: &DeviceType,
    ) -> Result<DevicePerformance> {
        debug!("Analyzing performance for device type: {:?}", device_type);

        // For now, return estimated performance based on device type
        // In a real implementation, this would perform actual benchmarks
        let performance = match device_type {
            DeviceType::SSD => DevicePerformance {
                sequential_read_speed: 500.0,
                sequential_write_speed: 450.0,
                random_read_iops: 50000.0,
                random_write_iops: 40000.0,
                average_latency: 100.0,
                optimal_io_size: 1024 * 1024,
                supports_trim: true,
                queue_depth: 32,
            },
            DeviceType::HDD => DevicePerformance {
                sequential_read_speed: 120.0,
                sequential_write_speed: 100.0,
                random_read_iops: 150.0,
                random_write_iops: 120.0,
                average_latency: 8000.0,
                optimal_io_size: 64 * 1024,
                supports_trim: false,
                queue_depth: 4,
            },
            DeviceType::Network => DevicePerformance {
                sequential_read_speed: 100.0,
                sequential_write_speed: 80.0,
                random_read_iops: 100.0,
                random_write_iops: 80.0,
                average_latency: 10000.0,
                optimal_io_size: 8 * 1024,
                supports_trim: false,
                queue_depth: 1,
            },
            DeviceType::RamDisk => DevicePerformance {
                sequential_read_speed: 2000.0,
                sequential_write_speed: 1800.0,
                random_read_iops: 200000.0,
                random_write_iops: 180000.0,
                average_latency: 10.0,
                optimal_io_size: 4 * 1024 * 1024,
                supports_trim: false,
                queue_depth: 64,
            },
            DeviceType::Unknown => DevicePerformance {
                sequential_read_speed: 100.0,
                sequential_write_speed: 80.0,
                random_read_iops: 1000.0,
                random_write_iops: 800.0,
                average_latency: 1000.0,
                optimal_io_size: 256 * 1024,
                supports_trim: false,
                queue_depth: 8,
            },
        };

        info!("Device performance analysis complete: {:?}", performance);
        Ok(performance)
    }

    /// Get file system information for a given path
    pub async fn get_filesystem_info<P: AsRef<Path>>(&self, path: P) -> Result<FileSystemInfo> {
        let path = path.as_ref();
        debug!("Getting filesystem info for path: {}", path.display());

        #[cfg(windows)]
        {
            return self.get_filesystem_info_windows(path).await;
        }

        #[cfg(unix)]
        {
            return self.get_filesystem_info_unix(path).await;
        }

        #[cfg(not(any(windows, unix)))]
        {
            warn!("Filesystem info not implemented for this platform");
            Ok(FileSystemInfo {
                fs_type: "unknown".to_string(),
                total_space: 0,
                available_space: 0,
                block_size: 4096,
                supports_cow: false,
                supports_sparse: false,
                supports_compression: false,
                supports_dedup: false,
            })
        }
    }

    /// Perform a quick I/O benchmark
    pub async fn benchmark_io<P: AsRef<Path>>(&self, path: P) -> Result<DevicePerformance> {
        let path = path.as_ref();
        debug!("Benchmarking I/O performance for path: {}", path.display());

        let start_time = Instant::now();

        // Create a temporary test file
        let test_file = path.join("ferrocp_benchmark_test.tmp");
        let test_data = vec![0u8; self.test_file_size as usize];

        let mut total_write_time = Duration::ZERO;
        let mut total_read_time = Duration::ZERO;

        for i in 0..self.test_iterations {
            debug!("Benchmark iteration {}/{}", i + 1, self.test_iterations);

            // Write test
            let write_start = Instant::now();
            tokio::fs::write(&test_file, &test_data)
                .await
                .map_err(|e| Error::device_detection(format!("Write benchmark failed: {}", e)))?;
            total_write_time += write_start.elapsed();

            // Read test
            let read_start = Instant::now();
            let _read_data = tokio::fs::read(&test_file)
                .await
                .map_err(|e| Error::device_detection(format!("Read benchmark failed: {}", e)))?;
            total_read_time += read_start.elapsed();
        }

        // Clean up test file
        if let Err(e) = tokio::fs::remove_file(&test_file).await {
            warn!("Failed to remove benchmark test file: {}", e);
        }

        let avg_write_time = total_write_time / self.test_iterations;
        let avg_read_time = total_read_time / self.test_iterations;

        let write_speed =
            (self.test_file_size as f64) / avg_write_time.as_secs_f64() / (1024.0 * 1024.0);
        let read_speed =
            (self.test_file_size as f64) / avg_read_time.as_secs_f64() / (1024.0 * 1024.0);

        let performance = DevicePerformance {
            sequential_read_speed: read_speed,
            sequential_write_speed: write_speed,
            random_read_iops: 1000.0, // Placeholder
            random_write_iops: 800.0, // Placeholder
            average_latency: (avg_read_time.as_micros() + avg_write_time.as_micros()) as f64 / 2.0,
            optimal_io_size: self.test_file_size as usize,
            supports_trim: false, // Would need platform-specific detection
            queue_depth: 8,
        };

        let total_time = start_time.elapsed();
        info!(
            "I/O benchmark completed in {:?}: {:?}",
            total_time, performance
        );

        Ok(performance)
    }

    /// Set the test file size for benchmarks
    pub fn set_test_file_size(&mut self, size: u64) {
        self.test_file_size = size;
    }

    /// Set the number of test iterations
    pub fn set_test_iterations(&mut self, iterations: u32) {
        self.test_iterations = iterations;
    }
}

impl Default for DeviceAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_device_analyzer_creation() {
        let analyzer = DeviceAnalyzer::new();
        assert_eq!(analyzer.test_file_size, 1024 * 1024);
        assert_eq!(analyzer.test_iterations, 3);
    }

    #[tokio::test]
    async fn test_custom_test_params() {
        let analyzer = DeviceAnalyzer::with_test_params(2048, 5);
        assert_eq!(analyzer.test_file_size, 2048);
        assert_eq!(analyzer.test_iterations, 5);
    }

    #[tokio::test]
    async fn test_performance_analysis() {
        let analyzer = DeviceAnalyzer::new();

        let ssd_perf = analyzer
            .analyze_device_performance(&DeviceType::SSD)
            .await
            .unwrap();
        assert!(ssd_perf.sequential_read_speed > 0.0);
        assert!(ssd_perf.supports_trim);

        let hdd_perf = analyzer
            .analyze_device_performance(&DeviceType::HDD)
            .await
            .unwrap();
        assert!(hdd_perf.sequential_read_speed > 0.0);
        assert!(!hdd_perf.supports_trim);
    }

    #[tokio::test]
    async fn test_parameter_setters() {
        let mut analyzer = DeviceAnalyzer::new();

        analyzer.set_test_file_size(4096);
        assert_eq!(analyzer.test_file_size, 4096);

        analyzer.set_test_iterations(10);
        assert_eq!(analyzer.test_iterations, 10);
    }
}
