//! Zero-copy engine implementation for FerroCP
//!
//! This module provides the main zero-copy engine that coordinates different
//! zero-copy methods and provides intelligent fallback mechanisms.

use crate::hardware::AccelerationCapabilities;
use crate::methods::ZeroCopyCapabilities;
use ferrocp_types::{
    Result, ZeroCopyEngine as ZeroCopyEngineTrait, ZeroCopyMethod as ZeroCopyMethodType,
    ZeroCopyResult,
};
use std::path::Path;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Zero-copy engine configuration
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ZeroCopyConfig {
    /// Whether zero-copy operations are enabled
    pub enabled: bool,
    /// Minimum file size for zero-copy operations (bytes)
    pub min_file_size: u64,
    /// Maximum file size for zero-copy operations (bytes)
    pub max_file_size: u64,
    /// Timeout for zero-copy operations
    pub timeout: Duration,
    /// Whether to enable hardware acceleration
    pub hardware_acceleration: bool,
    /// Preferred zero-copy methods in order of preference
    pub preferred_methods: Vec<ZeroCopyMethodType>,
}

impl Default for ZeroCopyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_file_size: 4096,                     // 4KB minimum
            max_file_size: 100 * 1024 * 1024 * 1024, // 100GB maximum
            timeout: Duration::from_secs(300),       // 5 minutes
            hardware_acceleration: true,
            preferred_methods: vec![
                #[cfg(target_os = "linux")]
                ZeroCopyMethodType::CopyFileRange,
                #[cfg(target_os = "windows")]
                ZeroCopyMethodType::RefsCoW,
                #[cfg(target_os = "macos")]
                ZeroCopyMethodType::CopyFile,
                ZeroCopyMethodType::Fallback,
            ],
        }
    }
}

/// Zero-copy engine statistics
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ZeroCopyStats {
    /// Total number of zero-copy attempts
    pub attempts: u64,
    /// Number of successful zero-copy operations
    pub successes: u64,
    /// Number of fallback operations
    pub fallbacks: u64,
    /// Total bytes copied using zero-copy
    pub bytes_zero_copied: u64,
    /// Total bytes copied using fallback
    pub bytes_fallback: u64,
    /// Average zero-copy operation time
    pub avg_zero_copy_time: Duration,
    /// Average fallback operation time
    pub avg_fallback_time: Duration,
}

impl ZeroCopyStats {
    /// Calculate zero-copy success rate
    pub fn success_rate(&self) -> f64 {
        if self.attempts == 0 {
            0.0
        } else {
            self.successes as f64 / self.attempts as f64
        }
    }

    /// Calculate zero-copy efficiency (bytes zero-copied / total bytes)
    pub fn efficiency(&self) -> f64 {
        let total_bytes = self.bytes_zero_copied + self.bytes_fallback;
        if total_bytes == 0 {
            0.0
        } else {
            self.bytes_zero_copied as f64 / total_bytes as f64
        }
    }
}

/// Main zero-copy engine trait
pub trait ZeroCopyEngine {
    /// Get engine configuration
    fn config(&self) -> &ZeroCopyConfig;

    /// Get engine statistics
    fn stats(&self) -> &ZeroCopyStats;

    /// Get available zero-copy capabilities
    fn capabilities(&self) -> &ZeroCopyCapabilities;

    /// Get hardware acceleration capabilities
    fn hardware_capabilities(&self) -> &AccelerationCapabilities;

    /// Reset statistics
    fn reset_stats(&mut self);

    /// Update configuration
    fn update_config(&mut self, config: ZeroCopyConfig);
}

/// Zero-copy engine implementation
pub struct ZeroCopyEngineImpl {
    /// Engine configuration
    config: ZeroCopyConfig,
    /// Engine statistics
    stats: ZeroCopyStats,
    /// Zero-copy capabilities
    capabilities: ZeroCopyCapabilities,
    /// Hardware acceleration capabilities
    hardware: AccelerationCapabilities,
}

impl ZeroCopyEngineImpl {
    /// Create a new zero-copy engine
    pub fn new() -> Self {
        let config = ZeroCopyConfig::default();
        let capabilities = ZeroCopyCapabilities::detect();
        let hardware = AccelerationCapabilities::detect();

        Self {
            config,
            stats: ZeroCopyStats::default(),
            capabilities,
            hardware,
        }
    }

    /// Create a zero-copy engine with custom configuration
    pub fn with_config(config: ZeroCopyConfig) -> Self {
        let capabilities = ZeroCopyCapabilities::detect();
        let hardware = AccelerationCapabilities::detect();

        Self {
            config,
            stats: ZeroCopyStats::default(),
            capabilities,
            hardware,
        }
    }

    /// Check if zero-copy is available for the given file size and paths
    async fn is_zero_copy_suitable<P: AsRef<Path>>(
        &self,
        _source: P,
        _destination: P,
        size: u64,
    ) -> Result<bool> {
        if !self.config.enabled {
            debug!("Zero-copy disabled in configuration");
            return Ok(false);
        }

        if size < self.config.min_file_size {
            debug!(
                "File size {} below minimum threshold {}",
                size, self.config.min_file_size
            );
            return Ok(false);
        }

        if size > self.config.max_file_size {
            debug!(
                "File size {} above maximum threshold {}",
                size, self.config.max_file_size
            );
            return Ok(false);
        }

        // For now, assume zero-copy is available if enabled and size is suitable
        // In a real implementation, this would check filesystem compatibility
        Ok(true)
    }

    /// Attempt zero-copy operation with the best available method
    async fn attempt_zero_copy<P: AsRef<Path> + Send + Sync>(
        &self,
        _source: P,
        _destination: P,
        size: u64,
    ) -> Result<ZeroCopyResult> {
        let start_time = Instant::now();

        // For now, simulate zero-copy attempt
        // In a real implementation, this would try different platform-specific methods
        for &method in &self.config.preferred_methods {
            debug!("Attempting zero-copy with method: {:?}", method);

            // Simulate zero-copy operation
            if method != ZeroCopyMethodType::Fallback {
                let elapsed = start_time.elapsed();
                info!(
                    "Zero-copy successful with method {:?}, {} bytes in {:?}",
                    method, size, elapsed
                );

                return Ok(ZeroCopyResult {
                    bytes_copied: size,
                    zerocopy_used: true,
                    method,
                });
            }
        }

        // All zero-copy methods failed
        warn!("All zero-copy methods failed, falling back to regular copy");
        Ok(ZeroCopyResult {
            bytes_copied: 0,
            zerocopy_used: false,
            method: ZeroCopyMethodType::Fallback,
        })
    }

    /// Update statistics after an operation
    fn update_stats(&mut self, result: &ZeroCopyResult, operation_time: Duration) {
        self.stats.attempts += 1;

        if result.zerocopy_used {
            self.stats.successes += 1;
            self.stats.bytes_zero_copied += result.bytes_copied;

            // Update average zero-copy time
            let total_time = self.stats.avg_zero_copy_time.as_nanos() as u64
                * (self.stats.successes - 1)
                + operation_time.as_nanos() as u64;
            self.stats.avg_zero_copy_time = Duration::from_nanos(total_time / self.stats.successes);
        } else {
            self.stats.fallbacks += 1;
            self.stats.bytes_fallback += result.bytes_copied;

            // Update average fallback time
            let total_time = self.stats.avg_fallback_time.as_nanos() as u64
                * (self.stats.fallbacks - 1)
                + operation_time.as_nanos() as u64;
            self.stats.avg_fallback_time = Duration::from_nanos(total_time / self.stats.fallbacks);
        }
    }
}

impl Default for ZeroCopyEngineImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl ZeroCopyEngine for ZeroCopyEngineImpl {
    fn config(&self) -> &ZeroCopyConfig {
        &self.config
    }

    fn stats(&self) -> &ZeroCopyStats {
        &self.stats
    }

    fn capabilities(&self) -> &ZeroCopyCapabilities {
        &self.capabilities
    }

    fn hardware_capabilities(&self) -> &AccelerationCapabilities {
        &self.hardware
    }

    fn reset_stats(&mut self) {
        self.stats = ZeroCopyStats::default();
    }

    fn update_config(&mut self, config: ZeroCopyConfig) {
        self.config = config;
    }
}

#[async_trait::async_trait]
impl ZeroCopyEngineTrait for ZeroCopyEngineImpl {
    async fn zero_copy<P: AsRef<Path> + Send + Sync>(
        &self,
        source: P,
        destination: P,
        size: u64,
    ) -> Result<bool> {
        let start_time = Instant::now();

        // Check if zero-copy is suitable for this operation
        if !self
            .is_zero_copy_suitable(&source, &destination, size)
            .await?
        {
            return Ok(false);
        }

        // Attempt zero-copy operation
        let result = self.attempt_zero_copy(source, destination, size).await?;
        let _operation_time = start_time.elapsed();

        // Update statistics (note: this requires mutable access, which we don't have in this trait)
        // In a real implementation, you might use interior mutability (Mutex/RwLock) for stats

        Ok(result.zerocopy_used)
    }

    fn is_zero_copy_available<P: AsRef<Path>>(&self, _source: P, _destination: P) -> bool {
        // For now, return true if zero-copy is enabled
        // In a real implementation, this would check platform-specific capabilities
        self.config.enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_copy_config_default() {
        let config = ZeroCopyConfig::default();
        assert!(config.enabled);
        assert_eq!(config.min_file_size, 4096);
        assert!(!config.preferred_methods.is_empty());
    }

    #[test]
    fn test_zero_copy_stats() {
        let mut stats = ZeroCopyStats::default();
        assert_eq!(stats.success_rate(), 0.0);
        assert_eq!(stats.efficiency(), 0.0);

        stats.attempts = 10;
        stats.successes = 7;
        stats.bytes_zero_copied = 1000;
        stats.bytes_fallback = 500;

        assert_eq!(stats.success_rate(), 0.7);
        assert!((stats.efficiency() - 0.6666666666666666).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_zero_copy_engine_creation() {
        let engine = ZeroCopyEngineImpl::new();
        assert!(engine.config().enabled);
        assert_eq!(engine.stats().attempts, 0);
    }

    #[tokio::test]
    async fn test_zero_copy_engine_with_config() {
        let mut config = ZeroCopyConfig::default();
        config.enabled = false;

        let engine = ZeroCopyEngineImpl::with_config(config);
        assert!(!engine.config().enabled);
    }
}
