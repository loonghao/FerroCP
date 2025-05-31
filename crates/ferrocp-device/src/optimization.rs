//! Device-specific optimization strategies for FerroCP
//!
//! This module provides optimization strategies based on device characteristics
//! and performance analysis results.

use crate::analyzer::DevicePerformance;
use ferrocp_types::{DeviceType, Result, ZeroCopyMethod};
use std::time::Duration;
use tracing::{debug, info};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Optimization strategy for a specific device
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct OptimizationStrategy {
    /// Recommended buffer size for I/O operations
    pub buffer_size: usize,
    /// Number of concurrent operations
    pub concurrency_level: u32,
    /// Whether to use zero-copy operations
    pub use_zero_copy: bool,
    /// Preferred zero-copy method
    pub zero_copy_method: ZeroCopyMethod,
    /// Whether to use memory mapping
    pub use_memory_mapping: bool,
    /// Whether to use direct I/O
    pub use_direct_io: bool,
    /// Read-ahead size in bytes
    pub read_ahead_size: usize,
    /// Write-behind cache size in bytes
    pub write_cache_size: usize,
    /// I/O timeout duration
    pub io_timeout: Duration,
    /// Whether to use compression
    pub use_compression: bool,
    /// Compression threshold (files larger than this will be compressed)
    pub compression_threshold: u64,
}

/// Device optimizer that generates optimization strategies
#[derive(Debug)]
pub struct DeviceOptimizer {
    /// Whether to enable aggressive optimizations
    aggressive_mode: bool,
    /// Maximum memory usage for optimizations (in bytes)
    max_memory_usage: u64,
}

impl DeviceOptimizer {
    /// Create a new device optimizer
    pub fn new() -> Self {
        Self {
            aggressive_mode: false,
            max_memory_usage: 512 * 1024 * 1024, // 512MB default
        }
    }

    /// Create a device optimizer with aggressive optimizations enabled
    pub fn with_aggressive_mode(max_memory_usage: u64) -> Self {
        Self {
            aggressive_mode: true,
            max_memory_usage,
        }
    }

    /// Generate optimization strategy for a device type
    pub fn optimize_for_device(&self, device_type: DeviceType) -> Result<OptimizationStrategy> {
        debug!(
            "Generating optimization strategy for device type: {:?}",
            device_type
        );

        let strategy = match device_type {
            DeviceType::SSD => self.optimize_for_ssd(),
            DeviceType::HDD => self.optimize_for_hdd(),
            DeviceType::Network => self.optimize_for_network(),
            DeviceType::RamDisk => self.optimize_for_ramdisk(),
            DeviceType::Unknown => self.optimize_for_unknown(),
        };

        info!("Generated optimization strategy: {:?}", strategy);
        Ok(strategy)
    }

    /// Generate optimization strategy based on performance analysis
    pub fn optimize_for_performance(
        &self,
        performance: &DevicePerformance,
    ) -> Result<OptimizationStrategy> {
        debug!("Generating optimization strategy based on performance analysis");

        let buffer_size = if performance.sequential_read_speed > 1000.0 {
            // High-speed device (likely SSD or RAM disk)
            // Based on performance testing: 512KB is optimal for SSD preread
            if self.aggressive_mode {
                2 * 1024 * 1024  // Reduced from 4MB for better performance
            } else {
                512 * 1024  // Optimal size proven by testing (387.82 MiB/s vs 298.10 MiB/s)
            }
        } else if performance.sequential_read_speed > 200.0 {
            // Medium-speed device (likely good SSD)
            if self.aggressive_mode {
                1024 * 1024  // Reduced from 2MB
            } else {
                512 * 1024  // Keep optimal 512KB
            }
        } else {
            // Lower-speed device (likely HDD or network)
            if self.aggressive_mode {
                256 * 1024
            } else {
                64 * 1024
            }
        };

        let concurrency_level = if performance.random_read_iops > 10000.0 {
            if self.aggressive_mode {
                performance.queue_depth * 2
            } else {
                performance.queue_depth
            }
        } else {
            std::cmp::min(performance.queue_depth, 8)
        };

        let use_zero_copy = performance.sequential_read_speed > 100.0 && performance.supports_trim;
        let use_memory_mapping = performance.sequential_read_speed > 200.0;
        let use_direct_io = performance.sequential_read_speed > 500.0 && self.aggressive_mode;

        let strategy = OptimizationStrategy {
            buffer_size,
            concurrency_level,
            use_zero_copy,
            zero_copy_method: self.select_zero_copy_method(),
            use_memory_mapping,
            use_direct_io,
            read_ahead_size: buffer_size,  // Match buffer_size for optimal preread (512KB for SSD)
            write_cache_size: buffer_size * 2,  // Reduced from 4x for better memory efficiency
            io_timeout: Duration::from_secs(if performance.average_latency > 5000.0 {
                30
            } else {
                10
            }),
            use_compression: performance.sequential_write_speed < 50.0, // Use compression for slow devices
            compression_threshold: 1024 * 1024,                         // 1MB
        };

        info!(
            "Generated performance-based optimization strategy: {:?}",
            strategy
        );
        Ok(strategy)
    }

    /// Optimize strategy for SSD devices
    fn optimize_for_ssd(&self) -> OptimizationStrategy {
        OptimizationStrategy {
            buffer_size: if self.aggressive_mode {
                2 * 1024 * 1024
            } else {
                1024 * 1024
            },
            concurrency_level: if self.aggressive_mode { 64 } else { 32 },
            use_zero_copy: true,
            zero_copy_method: self.select_zero_copy_method(),
            use_memory_mapping: true,
            use_direct_io: self.aggressive_mode,
            read_ahead_size: 2 * 1024 * 1024,
            write_cache_size: 4 * 1024 * 1024,
            io_timeout: Duration::from_secs(10),
            use_compression: false,
            compression_threshold: 10 * 1024 * 1024,
        }
    }

    /// Optimize strategy for HDD devices
    fn optimize_for_hdd(&self) -> OptimizationStrategy {
        OptimizationStrategy {
            buffer_size: if self.aggressive_mode {
                256 * 1024
            } else {
                64 * 1024
            },
            concurrency_level: if self.aggressive_mode { 8 } else { 4 },
            use_zero_copy: true,
            zero_copy_method: self.select_zero_copy_method(),
            use_memory_mapping: false,
            use_direct_io: false,
            read_ahead_size: 128 * 1024,
            write_cache_size: 512 * 1024,
            io_timeout: Duration::from_secs(30),
            use_compression: true,
            compression_threshold: 1024 * 1024,
        }
    }

    /// Optimize strategy for network devices
    fn optimize_for_network(&self) -> OptimizationStrategy {
        OptimizationStrategy {
            buffer_size: if self.aggressive_mode {
                32 * 1024
            } else {
                8 * 1024
            },
            concurrency_level: if self.aggressive_mode { 4 } else { 2 },
            use_zero_copy: false,
            zero_copy_method: ZeroCopyMethod::Fallback,
            use_memory_mapping: false,
            use_direct_io: false,
            read_ahead_size: 16 * 1024,
            write_cache_size: 64 * 1024,
            io_timeout: Duration::from_secs(60),
            use_compression: true,
            compression_threshold: 64 * 1024,
        }
    }

    /// Optimize strategy for RAM disk devices
    fn optimize_for_ramdisk(&self) -> OptimizationStrategy {
        OptimizationStrategy {
            buffer_size: if self.aggressive_mode {
                8 * 1024 * 1024
            } else {
                4 * 1024 * 1024
            },
            concurrency_level: if self.aggressive_mode { 128 } else { 64 },
            use_zero_copy: true,
            zero_copy_method: self.select_zero_copy_method(),
            use_memory_mapping: true,
            use_direct_io: false,
            read_ahead_size: 8 * 1024 * 1024,
            write_cache_size: 16 * 1024 * 1024,
            io_timeout: Duration::from_secs(5),
            use_compression: false,
            compression_threshold: 100 * 1024 * 1024,
        }
    }

    /// Optimize strategy for unknown devices
    fn optimize_for_unknown(&self) -> OptimizationStrategy {
        OptimizationStrategy {
            buffer_size: 256 * 1024,
            concurrency_level: 8,
            use_zero_copy: false,
            zero_copy_method: ZeroCopyMethod::Fallback,
            use_memory_mapping: false,
            use_direct_io: false,
            read_ahead_size: 256 * 1024,
            write_cache_size: 512 * 1024,
            io_timeout: Duration::from_secs(30),
            use_compression: false,
            compression_threshold: 1024 * 1024,
        }
    }

    /// Select the best zero-copy method for the current platform
    fn select_zero_copy_method(&self) -> ZeroCopyMethod {
        #[cfg(target_os = "linux")]
        {
            ZeroCopyMethod::CopyFileRange
        }
        #[cfg(target_os = "macos")]
        {
            ZeroCopyMethod::CopyFile
        }
        #[cfg(target_os = "windows")]
        {
            ZeroCopyMethod::RefsCoW
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            ZeroCopyMethod::Fallback
        }
    }

    /// Enable or disable aggressive optimizations
    pub fn set_aggressive_mode(&mut self, enabled: bool) {
        self.aggressive_mode = enabled;
    }

    /// Set maximum memory usage for optimizations
    pub fn set_max_memory_usage(&mut self, max_memory: u64) {
        self.max_memory_usage = max_memory;
    }

    /// Get current aggressive mode setting
    pub fn is_aggressive_mode(&self) -> bool {
        self.aggressive_mode
    }

    /// Get current maximum memory usage setting
    pub fn max_memory_usage(&self) -> u64 {
        self.max_memory_usage
    }
}

impl Default for DeviceOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_optimizer_creation() {
        let optimizer = DeviceOptimizer::new();
        assert!(!optimizer.is_aggressive_mode());
        assert_eq!(optimizer.max_memory_usage(), 512 * 1024 * 1024);
    }

    #[test]
    fn test_aggressive_mode() {
        let optimizer = DeviceOptimizer::with_aggressive_mode(1024 * 1024 * 1024);
        assert!(optimizer.is_aggressive_mode());
        assert_eq!(optimizer.max_memory_usage(), 1024 * 1024 * 1024);
    }

    #[test]
    fn test_ssd_optimization() {
        let optimizer = DeviceOptimizer::new();
        let strategy = optimizer.optimize_for_device(DeviceType::SSD).unwrap();

        assert!(strategy.use_zero_copy);
        assert!(strategy.use_memory_mapping);
        assert_eq!(strategy.buffer_size, 1024 * 1024);
        assert_eq!(strategy.concurrency_level, 32);
    }

    #[test]
    fn test_hdd_optimization() {
        let optimizer = DeviceOptimizer::new();
        let strategy = optimizer.optimize_for_device(DeviceType::HDD).unwrap();

        assert!(strategy.use_zero_copy);
        assert!(!strategy.use_memory_mapping);
        assert!(strategy.use_compression);
        assert_eq!(strategy.buffer_size, 64 * 1024);
        assert_eq!(strategy.concurrency_level, 4);
    }

    #[test]
    fn test_network_optimization() {
        let optimizer = DeviceOptimizer::new();
        let strategy = optimizer.optimize_for_device(DeviceType::Network).unwrap();

        assert!(!strategy.use_zero_copy);
        assert!(!strategy.use_memory_mapping);
        assert!(strategy.use_compression);
        assert_eq!(strategy.buffer_size, 8 * 1024);
        assert_eq!(strategy.concurrency_level, 2);
    }

    #[test]
    fn test_ramdisk_optimization() {
        let optimizer = DeviceOptimizer::new();
        let strategy = optimizer.optimize_for_device(DeviceType::RamDisk).unwrap();

        assert!(strategy.use_zero_copy);
        assert!(strategy.use_memory_mapping);
        assert!(!strategy.use_compression);
        assert_eq!(strategy.buffer_size, 4 * 1024 * 1024);
        assert_eq!(strategy.concurrency_level, 64);
    }
}
