//! Intelligent engine selector for optimal copy performance
//!
//! This module provides the EngineSelector component that intelligently chooses
//! the best copy engine based on file size, device characteristics, and performance
//! requirements. This is the key component for solving small file performance issues.

use ferrocp_device::DeviceOptimizer;
use ferrocp_io::{BufferedCopyEngine, CopyOptions, MicroFileCopyEngine, ParallelCopyEngine};
use ferrocp_types::{DeviceType, Error, Result};
use ferrocp_zerocopy::ZeroCopyEngineImpl;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs;
use tracing::{debug, info};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// File size thresholds for engine selection (optimized for better small file performance)
const MICRO_FILE_THRESHOLD: u64 = 4096; // 4KB - use MicroFileCopyEngine (increased from 1KB)
const SMALL_FILE_THRESHOLD: u64 = 16384; // 16KB - use sync BufferedCopyEngine (increased from 4KB)
const ZEROCOPY_THRESHOLD: u64 = 64 * 1024; // 64KB - enable zero-copy for larger files
const PARALLEL_THRESHOLD: u64 = 50 * 1024 * 1024; // 50MB - use parallel processing for very large files

/// Engine selection strategy configuration
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct EngineSelectionConfig {
    /// Enable intelligent engine selection
    pub enabled: bool,
    /// Micro file threshold (files smaller than this use MicroFileCopyEngine)
    pub micro_file_threshold: u64,
    /// Small file threshold (files smaller than this use sync mode)
    pub small_file_threshold: u64,
    /// Zero-copy threshold (files larger than this may use zero-copy)
    pub zerocopy_threshold: u64,
    /// Parallel processing threshold (files larger than this may use parallel processing)
    pub parallel_threshold: u64,
    /// Enable device-specific optimizations
    pub enable_device_optimization: bool,
    /// Enable performance monitoring
    pub enable_performance_monitoring: bool,
    /// Enable dynamic threshold adjustment based on performance history
    pub enable_dynamic_thresholds: bool,
    /// Minimum samples required before adjusting thresholds
    pub min_samples_for_adjustment: u64,
    /// Performance improvement threshold for adjusting thresholds (percentage)
    pub performance_improvement_threshold: f64,
}

impl Default for EngineSelectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            micro_file_threshold: MICRO_FILE_THRESHOLD,
            small_file_threshold: SMALL_FILE_THRESHOLD,
            zerocopy_threshold: ZEROCOPY_THRESHOLD,
            parallel_threshold: PARALLEL_THRESHOLD,
            enable_device_optimization: true,
            enable_performance_monitoring: true,
            enable_dynamic_thresholds: true,
            min_samples_for_adjustment: 100,
            performance_improvement_threshold: 5.0, // 5% improvement required
        }
    }
}

/// Performance history for a specific file size range and engine type
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PerformanceHistory {
    /// Number of samples
    pub sample_count: u64,
    /// Average throughput in bytes per second
    pub avg_throughput_bps: f64,
    /// Average copy time in nanoseconds
    pub avg_copy_time_ns: u64,
    /// Best throughput observed
    pub best_throughput_bps: f64,
    /// Last update timestamp
    pub last_updated: std::time::SystemTime,
}

impl Default for PerformanceHistory {
    fn default() -> Self {
        Self {
            sample_count: 0,
            avg_throughput_bps: 0.0,
            avg_copy_time_ns: 0,
            best_throughput_bps: 0.0,
            last_updated: std::time::SystemTime::now(),
        }
    }
}

/// Performance summary for threshold adjustment analysis
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PerformanceSummary {
    /// Performance history for micro files
    pub micro_file_performance: PerformanceHistory,
    /// Performance history for small files
    pub small_file_performance: PerformanceHistory,
    /// Performance history for large files
    pub large_file_performance: PerformanceHistory,
    /// Current micro file threshold
    pub current_micro_threshold: u64,
    /// Current small file threshold
    pub current_small_threshold: u64,
    /// Total number of engine selections
    pub total_selections: u64,
    /// Number of threshold adjustments made
    pub threshold_adjustments: u64,
}

/// Engine selection statistics
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct EngineSelectionStats {
    /// Total number of engine selections
    pub total_selections: u64,
    /// Number of times MicroFileCopyEngine was selected
    pub micro_engine_selections: u64,
    /// Number of times sync BufferedCopyEngine was selected
    pub sync_buffered_selections: u64,
    /// Number of times async BufferedCopyEngine was selected
    pub async_buffered_selections: u64,
    /// Number of times zero-copy was enabled
    pub zerocopy_enabled_selections: u64,
    /// Average selection time in nanoseconds
    pub avg_selection_time_ns: u64,
    /// Performance history for micro files (< micro_file_threshold)
    pub micro_file_performance: PerformanceHistory,
    /// Performance history for small files (micro_file_threshold to small_file_threshold)
    pub small_file_performance: PerformanceHistory,
    /// Performance history for large files (> small_file_threshold)
    pub large_file_performance: PerformanceHistory,
    /// Number of threshold adjustments made
    pub threshold_adjustments: u64,
}

impl EngineSelectionStats {
    /// Calculate micro engine usage percentage
    pub fn micro_engine_percentage(&self) -> f64 {
        if self.total_selections == 0 {
            0.0
        } else {
            self.micro_engine_selections as f64 / self.total_selections as f64 * 100.0
        }
    }

    /// Calculate zero-copy usage percentage
    pub fn zerocopy_percentage(&self) -> f64 {
        if self.total_selections == 0 {
            0.0
        } else {
            self.zerocopy_enabled_selections as f64 / self.total_selections as f64 * 100.0
        }
    }

    /// Update performance history for a file size category
    pub fn update_performance_history(
        &mut self,
        file_size: u64,
        bytes_copied: u64,
        copy_time_ns: u64,
        micro_threshold: u64,
        small_threshold: u64,
    ) {
        let throughput_bps = if copy_time_ns > 0 {
            (bytes_copied as f64 * 1_000_000_000.0) / copy_time_ns as f64
        } else {
            0.0
        };

        let history = if file_size <= micro_threshold {
            &mut self.micro_file_performance
        } else if file_size <= small_threshold {
            &mut self.small_file_performance
        } else {
            &mut self.large_file_performance
        };

        // Update running averages
        if history.sample_count == 0 {
            history.avg_throughput_bps = throughput_bps;
            history.avg_copy_time_ns = copy_time_ns;
            history.best_throughput_bps = throughput_bps;
        } else {
            let count = history.sample_count as f64;
            history.avg_throughput_bps =
                (history.avg_throughput_bps * count + throughput_bps) / (count + 1.0);
            history.avg_copy_time_ns = (history.avg_copy_time_ns * history.sample_count
                + copy_time_ns)
                / (history.sample_count + 1);
            history.best_throughput_bps = history.best_throughput_bps.max(throughput_bps);
        }

        history.sample_count += 1;
        history.last_updated = std::time::SystemTime::now();
    }

    /// Check if we have enough samples to consider threshold adjustment
    pub fn should_consider_threshold_adjustment(&self, min_samples: u64) -> bool {
        self.micro_file_performance.sample_count >= min_samples
            && self.small_file_performance.sample_count >= min_samples
    }

    /// Get performance improvement potential for threshold adjustment
    pub fn get_threshold_adjustment_recommendation(
        &self,
        current_micro_threshold: u64,
        current_small_threshold: u64,
    ) -> Option<(u64, u64)> {
        if !self.should_consider_threshold_adjustment(50) {
            return None;
        }

        // Enhanced heuristic based on performance ratios and statistical significance
        let micro_throughput = self.micro_file_performance.avg_throughput_bps;
        let small_throughput = self.small_file_performance.avg_throughput_bps;
        let large_throughput = self.large_file_performance.avg_throughput_bps;

        // Calculate performance ratios
        let micro_vs_small_ratio = if small_throughput > 0.0 {
            micro_throughput / small_throughput
        } else {
            1.0
        };

        let small_vs_large_ratio = if large_throughput > 0.0 {
            small_throughput / large_throughput
        } else {
            1.0
        };

        // More sophisticated threshold adjustment logic
        let mut new_micro_threshold = current_micro_threshold;
        let mut new_small_threshold = current_small_threshold;

        // Adjust micro threshold based on performance comparison
        if micro_vs_small_ratio > 1.25 && self.micro_file_performance.sample_count >= 100 {
            // Micro engine is significantly better, expand its range
            new_micro_threshold = (current_micro_threshold * 3 / 2)
                .min(current_small_threshold / 2)
                .min(8192); // Cap at 8KB
        } else if micro_vs_small_ratio < 0.8 && self.small_file_performance.sample_count >= 100 {
            // Small file sync is better, reduce micro threshold
            new_micro_threshold = (current_micro_threshold * 3 / 4).max(1024); // Minimum 1KB
        }

        // Adjust small threshold based on performance comparison
        if small_vs_large_ratio > 1.15 && self.small_file_performance.sample_count >= 100 {
            // Small file sync is better than large file async, expand its range
            new_small_threshold = (current_small_threshold * 5 / 4).min(32768); // Cap at 32KB
        } else if small_vs_large_ratio < 0.85 && self.large_file_performance.sample_count >= 100 {
            // Large file async is better, reduce small threshold
            new_small_threshold = (current_small_threshold * 3 / 4).max(new_micro_threshold * 2);
            // Ensure small > micro
        }

        // Only recommend changes if they're significant (>= 10% change)
        let micro_change_percent = ((new_micro_threshold as f64 - current_micro_threshold as f64)
            / current_micro_threshold as f64)
            .abs()
            * 100.0;
        let small_change_percent = ((new_small_threshold as f64 - current_small_threshold as f64)
            / current_small_threshold as f64)
            .abs()
            * 100.0;

        if micro_change_percent >= 10.0 || small_change_percent >= 10.0 {
            Some((new_micro_threshold, new_small_threshold))
        } else {
            None
        }
    }
}

/// Engine selection result
#[derive(Debug, Clone)]
pub struct EngineSelection {
    /// Selected engine type
    pub engine_type: EngineType,
    /// Whether to use synchronous mode
    pub use_sync_mode: bool,
    /// Whether zero-copy is enabled
    pub zerocopy_enabled: bool,
    /// Recommended copy options
    pub copy_options: CopyOptions,
    /// Selection reasoning for debugging
    pub reasoning: String,
}

/// Engine type enumeration
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum EngineType {
    /// Micro file copy engine for files < 1KB
    MicroFile,
    /// Buffered copy engine for general use
    Buffered,
    /// Zero-copy engine for large files
    ZeroCopy,
    /// Parallel copy engine for very large files
    Parallel,
}

/// Intelligent engine selector
pub struct EngineSelector {
    /// Configuration for engine selection
    config: EngineSelectionConfig,
    /// Device optimizer for hardware-specific optimizations
    device_optimizer: DeviceOptimizer,
    /// Engine instances
    micro_engine: Arc<tokio::sync::Mutex<MicroFileCopyEngine>>,
    buffered_engine: Arc<tokio::sync::Mutex<BufferedCopyEngine>>,
    parallel_engine: Arc<tokio::sync::Mutex<ParallelCopyEngine>>,
    zerocopy_engine: Arc<ZeroCopyEngineImpl>,
    /// Selection statistics
    stats: Arc<tokio::sync::RwLock<EngineSelectionStats>>,
}

impl EngineSelector {
    /// Create a new engine selector with default configuration
    pub fn new() -> Self {
        Self::with_config(EngineSelectionConfig::default())
    }

    /// Create a new engine selector with custom configuration
    pub fn with_config(config: EngineSelectionConfig) -> Self {
        let device_optimizer = DeviceOptimizer::new();
        let micro_engine = Arc::new(tokio::sync::Mutex::new(MicroFileCopyEngine::new()));
        let buffered_engine = Arc::new(tokio::sync::Mutex::new(BufferedCopyEngine::new()));
        let parallel_engine = Arc::new(tokio::sync::Mutex::new(ParallelCopyEngine::new()));
        let zerocopy_engine = Arc::new(ZeroCopyEngineImpl::new());
        let stats = Arc::new(tokio::sync::RwLock::new(EngineSelectionStats::default()));

        Self {
            config,
            device_optimizer,
            micro_engine,
            buffered_engine,
            parallel_engine,
            zerocopy_engine,
            stats,
        }
    }

    /// Select the optimal engine for a copy operation
    pub async fn select_optimal_engine<P: AsRef<Path> + Send>(
        &self,
        source: P,
        destination: P,
    ) -> Result<EngineSelection> {
        let start_time = std::time::Instant::now();

        if !self.config.enabled {
            // If intelligent selection is disabled, use default buffered engine
            return Ok(EngineSelection {
                engine_type: EngineType::Buffered,
                use_sync_mode: false,
                zerocopy_enabled: false,
                copy_options: CopyOptions::default(),
                reasoning: "Intelligent selection disabled".to_string(),
            });
        }

        let source_path = source.as_ref();
        let dest_path = destination.as_ref();

        // Get file size
        let file_size = self.get_file_size(source_path).await?;

        // Detect device types if optimization is enabled
        let (source_device, dest_device) = if self.config.enable_device_optimization {
            (
                self.detect_device_type(source_path).await?,
                self.detect_device_type(dest_path).await?,
            )
        } else {
            (DeviceType::Unknown, DeviceType::Unknown)
        };

        // Select engine based on file size and device characteristics
        let selection = self
            .select_engine_internal(
                file_size,
                source_device,
                dest_device,
                source_path,
                dest_path,
            )
            .await?;

        // Update statistics
        if self.config.enable_performance_monitoring {
            self.update_selection_stats(&selection, start_time.elapsed())
                .await;
        }

        debug!(
            "Selected engine: {:?} for file size {} bytes ({})",
            selection.engine_type, file_size, selection.reasoning
        );

        Ok(selection)
    }

    /// Internal engine selection logic
    async fn select_engine_internal(
        &self,
        file_size: u64,
        source_device: DeviceType,
        dest_device: DeviceType,
        source_path: &Path,
        dest_path: &Path,
    ) -> Result<EngineSelection> {
        // Step 1: Check for micro file optimization
        if file_size <= self.config.micro_file_threshold {
            return Ok(EngineSelection {
                engine_type: EngineType::MicroFile,
                use_sync_mode: true,
                zerocopy_enabled: false,
                copy_options: self.create_micro_file_options(),
                reasoning: format!("Micro file optimization for {} bytes", file_size),
            });
        }

        // Step 2: Check for small file sync optimization
        if file_size <= self.config.small_file_threshold {
            return Ok(EngineSelection {
                engine_type: EngineType::Buffered,
                use_sync_mode: true,
                zerocopy_enabled: false,
                copy_options: self.create_small_file_options(source_device, dest_device),
                reasoning: format!("Small file sync optimization for {} bytes", file_size),
            });
        }

        // Step 3: Check for parallel processing for very large files
        if file_size >= self.config.parallel_threshold {
            return Ok(EngineSelection {
                engine_type: EngineType::Parallel,
                use_sync_mode: false,
                zerocopy_enabled: false,
                copy_options: self.create_parallel_file_options(source_device, dest_device),
                reasoning: format!("Parallel processing optimization for {} bytes", file_size),
            });
        }

        // Step 4: Large file optimization with potential zero-copy
        let zerocopy_enabled = file_size >= self.config.zerocopy_threshold
            && self
                .should_use_zerocopy(source_device, dest_device, source_path, dest_path)
                .await;

        let engine_type = if zerocopy_enabled {
            EngineType::ZeroCopy
        } else {
            EngineType::Buffered
        };

        Ok(EngineSelection {
            engine_type,
            use_sync_mode: false,
            zerocopy_enabled,
            copy_options: self.create_large_file_options(
                source_device,
                dest_device,
                zerocopy_enabled,
            ),
            reasoning: format!(
                "Large file optimization for {} bytes, zero-copy: {}",
                file_size, zerocopy_enabled
            ),
        })
    }

    /// Create copy options for micro files
    fn create_micro_file_options(&self) -> CopyOptions {
        CopyOptions {
            buffer_size: Some(1024), // Small buffer for micro files
            enable_progress: false,  // Disable progress for micro files
            progress_interval: Duration::from_millis(1000),
            verify_copy: false, // Skip verification for speed
            preserve_metadata: true,
            enable_zero_copy: false, // No zero-copy for micro files
            max_retries: 1,          // Minimal retries for speed
            enable_preread: false,   // No pre-read for micro files
            preread_strategy: None,
            enable_compression: false, // No compression for micro files
            compression_level: 1,      // Minimal compression level
        }
    }

    /// Create copy options for small files
    fn create_small_file_options(
        &self,
        source_device: DeviceType,
        dest_device: DeviceType,
    ) -> CopyOptions {
        let buffer_size = self.calculate_small_file_buffer_size(source_device, dest_device);

        CopyOptions {
            buffer_size: Some(buffer_size),
            enable_progress: false, // Disable progress for small files
            progress_interval: Duration::from_millis(1000),
            verify_copy: false, // Skip verification for speed
            preserve_metadata: true,
            enable_zero_copy: false, // No zero-copy for small files
            max_retries: 2,
            enable_preread: false, // No pre-read for small files
            preread_strategy: None,
            enable_compression: false, // Disable compression for local copies
            compression_level: 3,      // Balanced compression level
        }
    }

    /// Create copy options for parallel processing
    fn create_parallel_file_options(
        &self,
        source_device: DeviceType,
        dest_device: DeviceType,
    ) -> CopyOptions {
        let buffer_size = self.calculate_parallel_buffer_size(source_device, dest_device);

        CopyOptions {
            buffer_size: Some(buffer_size),
            enable_progress: true,
            progress_interval: Duration::from_millis(50), // More frequent updates for parallel
            verify_copy: false,
            preserve_metadata: true,
            enable_zero_copy: false, // Parallel engine handles its own optimization
            max_retries: 3,
            enable_preread: false, // Parallel engine has its own pre-read logic
            preread_strategy: None,
            enable_compression: false, // Parallel engine doesn't use compression
            compression_level: 3,
        }
    }

    /// Create copy options for large files
    fn create_large_file_options(
        &self,
        source_device: DeviceType,
        dest_device: DeviceType,
        zerocopy_enabled: bool,
    ) -> CopyOptions {
        let buffer_size = if zerocopy_enabled {
            // Larger buffer for zero-copy operations
            self.calculate_zerocopy_buffer_size(source_device, dest_device)
        } else {
            self.calculate_large_file_buffer_size(source_device, dest_device)
        };

        CopyOptions {
            buffer_size: Some(buffer_size),
            enable_progress: true,
            progress_interval: Duration::from_millis(100),
            verify_copy: false,
            preserve_metadata: true,
            enable_zero_copy: zerocopy_enabled,
            max_retries: 3,
            enable_preread: true,      // Enable pre-read for large files
            preread_strategy: None,    // Auto-detect based on device
            enable_compression: false, // Large files use zero-copy instead of compression
            compression_level: 1,      // Fast compression if needed
        }
    }

    /// Calculate buffer size for small files
    fn calculate_small_file_buffer_size(
        &self,
        source_device: DeviceType,
        dest_device: DeviceType,
    ) -> usize {
        match (source_device, dest_device) {
            (DeviceType::SSD, DeviceType::SSD) => 16 * 1024, // 16KB
            (DeviceType::RamDisk, _) | (_, DeviceType::RamDisk) => 32 * 1024, // 32KB
            _ => 8 * 1024,                                   // 8KB default
        }
    }

    /// Calculate buffer size for large files
    fn calculate_large_file_buffer_size(
        &self,
        source_device: DeviceType,
        dest_device: DeviceType,
    ) -> usize {
        match (source_device, dest_device) {
            (DeviceType::SSD, DeviceType::SSD) => 1024 * 1024, // 1MB
            (DeviceType::RamDisk, DeviceType::RamDisk) => 4 * 1024 * 1024, // 4MB
            (DeviceType::HDD, _) | (_, DeviceType::HDD) => 256 * 1024, // 256KB
            (DeviceType::Network, _) | (_, DeviceType::Network) => 128 * 1024, // 128KB
            _ => 512 * 1024,                                   // 512KB default
        }
    }

    /// Calculate buffer size for parallel processing
    fn calculate_parallel_buffer_size(
        &self,
        source_device: DeviceType,
        dest_device: DeviceType,
    ) -> usize {
        // Parallel processing uses smaller individual buffers but more of them
        match (source_device, dest_device) {
            (DeviceType::SSD, DeviceType::SSD) => 1024 * 1024, // 1MB per chunk
            (DeviceType::RamDisk, DeviceType::RamDisk) => 2 * 1024 * 1024, // 2MB per chunk
            (DeviceType::HDD, _) | (_, DeviceType::HDD) => 512 * 1024, // 512KB per chunk
            (DeviceType::Network, _) | (_, DeviceType::Network) => 256 * 1024, // 256KB per chunk
            _ => 1024 * 1024,                                  // 1MB default
        }
    }

    /// Calculate buffer size for zero-copy operations
    fn calculate_zerocopy_buffer_size(
        &self,
        source_device: DeviceType,
        dest_device: DeviceType,
    ) -> usize {
        // Zero-copy operations can use larger buffers
        match (source_device, dest_device) {
            (DeviceType::SSD, DeviceType::SSD) => 2 * 1024 * 1024, // 2MB
            (DeviceType::RamDisk, DeviceType::RamDisk) => 8 * 1024 * 1024, // 8MB
            _ => 1024 * 1024,                                      // 1MB default
        }
    }

    /// Determine if zero-copy should be used
    async fn should_use_zerocopy(
        &self,
        source_device: DeviceType,
        dest_device: DeviceType,
        _source_path: &Path,
        _dest_path: &Path,
    ) -> bool {
        // Zero-copy is beneficial for:
        // 1. Same device type (especially SSD to SSD)
        // 2. High-performance devices
        // 3. Large files (already checked by caller)

        match (source_device, dest_device) {
            (DeviceType::SSD, DeviceType::SSD) => true,
            (DeviceType::RamDisk, DeviceType::RamDisk) => true,
            (DeviceType::Network, _) | (_, DeviceType::Network) => false, // Network doesn't benefit from zero-copy
            (DeviceType::HDD, DeviceType::HDD) => true,                   // Can still benefit
            _ => true, // Default to enabled for unknown combinations
        }
    }

    /// Get file size
    async fn get_file_size(&self, path: &Path) -> Result<u64> {
        let metadata = fs::metadata(path).await.map_err(|e| Error::Io {
            message: format!("Failed to get file metadata for {}: {}", path.display(), e),
        })?;
        Ok(metadata.len())
    }

    /// Detect device type for a path
    async fn detect_device_type(&self, path: &Path) -> Result<DeviceType> {
        // For now, use a simple heuristic
        // In a real implementation, this would use platform-specific APIs

        // Check if path is on a network drive
        if let Some(path_str) = path.to_str() {
            if path_str.starts_with("\\\\") || path_str.starts_with("//") {
                return Ok(DeviceType::Network);
            }
        }

        // Default to SSD for now
        // TODO: Implement actual device detection using platform-specific APIs
        Ok(DeviceType::SSD)
    }

    /// Update selection statistics
    async fn update_selection_stats(&self, selection: &EngineSelection, selection_time: Duration) {
        let mut stats = self.stats.write().await;

        stats.total_selections += 1;

        match selection.engine_type {
            EngineType::MicroFile => stats.micro_engine_selections += 1,
            EngineType::Buffered => {
                if selection.use_sync_mode {
                    stats.sync_buffered_selections += 1;
                } else {
                    stats.async_buffered_selections += 1;
                }
            }
            EngineType::Parallel => {
                stats.async_buffered_selections += 1; // Count as async buffered for compatibility
                                                      // TODO: Add specific parallel engine statistics
            }
            EngineType::ZeroCopy => {
                stats.async_buffered_selections += 1; // Zero-copy uses buffered engine
                if selection.zerocopy_enabled {
                    stats.zerocopy_enabled_selections += 1;
                }
            }
        }

        // Update average selection time
        let total_time_ns = stats.avg_selection_time_ns * (stats.total_selections - 1)
            + selection_time.as_nanos() as u64;
        stats.avg_selection_time_ns = total_time_ns / stats.total_selections;
    }

    /// Get a reference to the micro file copy engine
    pub async fn get_micro_engine(&self) -> Arc<tokio::sync::Mutex<MicroFileCopyEngine>> {
        self.micro_engine.clone()
    }

    /// Get a reference to the buffered copy engine
    pub async fn get_buffered_engine(&self) -> Arc<tokio::sync::Mutex<BufferedCopyEngine>> {
        self.buffered_engine.clone()
    }

    /// Get a reference to the parallel copy engine
    pub async fn get_parallel_engine(&self) -> Arc<tokio::sync::Mutex<ParallelCopyEngine>> {
        self.parallel_engine.clone()
    }

    /// Get a reference to the zero-copy engine
    pub async fn get_zerocopy_engine(&self) -> Arc<ZeroCopyEngineImpl> {
        self.zerocopy_engine.clone()
    }

    /// Get current selection statistics
    pub async fn get_stats(&self) -> EngineSelectionStats {
        self.stats.read().await.clone()
    }

    /// Reset selection statistics
    pub async fn reset_stats(&self) {
        let mut stats = self.stats.write().await;
        *stats = EngineSelectionStats::default();
    }

    /// Get current configuration
    pub fn get_config(&self) -> &EngineSelectionConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config(&mut self, config: EngineSelectionConfig) {
        self.config = config;
        info!("Engine selector configuration updated");
    }

    /// Check if intelligent selection is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Enable or disable intelligent selection
    pub fn set_enabled(&mut self, enabled: bool) {
        self.config.enabled = enabled;
        info!(
            "Intelligent engine selection {}",
            if enabled { "enabled" } else { "disabled" }
        );
    }

    /// Update performance history with copy operation results
    pub async fn update_performance_history(
        &self,
        file_size: u64,
        bytes_copied: u64,
        copy_time_ns: u64,
    ) {
        if !self.config.enable_performance_monitoring {
            return;
        }

        let mut stats = self.stats.write().await;
        stats.update_performance_history(
            file_size,
            bytes_copied,
            copy_time_ns,
            self.config.micro_file_threshold,
            self.config.small_file_threshold,
        );

        // Check if we should adjust thresholds
        if self.config.enable_dynamic_thresholds {
            if let Some((new_micro, new_small)) = stats.get_threshold_adjustment_recommendation(
                self.config.micro_file_threshold,
                self.config.small_file_threshold,
            ) {
                // Only adjust if the improvement is significant
                let micro_improvement = (new_micro as f64
                    - self.config.micro_file_threshold as f64)
                    / self.config.micro_file_threshold as f64
                    * 100.0;

                if micro_improvement.abs() >= self.config.performance_improvement_threshold {
                    stats.threshold_adjustments += 1;
                    info!(
                        "Adjusting thresholds based on performance data: micro {} -> {}, small {} -> {}",
                        self.config.micro_file_threshold, new_micro,
                        self.config.small_file_threshold, new_small
                    );

                    // Note: In a real implementation, we would need to update the config
                    // This requires making the method mutable or using interior mutability
                    debug!("Threshold adjustment recommended but not applied (requires mutable access)");
                }
            }
        }
    }

    /// Apply threshold adjustments (requires mutable access)
    pub fn apply_threshold_adjustment(
        &mut self,
        new_micro_threshold: u64,
        new_small_threshold: u64,
    ) {
        if new_micro_threshold != self.config.micro_file_threshold
            || new_small_threshold != self.config.small_file_threshold
        {
            info!(
                "Applying threshold adjustment: micro {} -> {}, small {} -> {}",
                self.config.micro_file_threshold,
                new_micro_threshold,
                self.config.small_file_threshold,
                new_small_threshold
            );

            self.config.micro_file_threshold = new_micro_threshold;
            self.config.small_file_threshold = new_small_threshold;
        }
    }

    /// Get threshold adjustment recommendations
    pub async fn get_threshold_recommendations(&self) -> Option<(u64, u64)> {
        if !self.config.enable_dynamic_thresholds {
            return None;
        }

        let stats = self.stats.read().await;
        stats.get_threshold_adjustment_recommendation(
            self.config.micro_file_threshold,
            self.config.small_file_threshold,
        )
    }

    /// Automatically check and apply threshold adjustments if beneficial
    pub async fn auto_adjust_thresholds(&mut self) -> Result<bool> {
        if !self.config.enable_dynamic_thresholds {
            return Ok(false);
        }

        // Check if we have enough samples to make adjustments
        let stats = self.stats.read().await;
        if !stats.should_consider_threshold_adjustment(self.config.min_samples_for_adjustment) {
            return Ok(false);
        }

        // Get recommendations
        if let Some((new_micro, new_small)) = stats.get_threshold_adjustment_recommendation(
            self.config.micro_file_threshold,
            self.config.small_file_threshold,
        ) {
            // Calculate improvement potential
            let micro_improvement = ((new_micro as f64 - self.config.micro_file_threshold as f64)
                / self.config.micro_file_threshold as f64)
                .abs()
                * 100.0;
            let small_improvement = ((new_small as f64 - self.config.small_file_threshold as f64)
                / self.config.small_file_threshold as f64)
                .abs()
                * 100.0;

            // Only apply if improvement is significant
            if micro_improvement >= self.config.performance_improvement_threshold
                || small_improvement >= self.config.performance_improvement_threshold
            {
                // Drop the read lock before applying changes
                drop(stats);

                // Apply the adjustment
                self.apply_threshold_adjustment(new_micro, new_small);

                // Update statistics
                let mut stats = self.stats.write().await;
                stats.threshold_adjustments += 1;

                info!(
                    "Auto-adjusted thresholds: micro {}KB -> {}KB ({:.1}% change), small {}KB -> {}KB ({:.1}% change)",
                    self.config.micro_file_threshold / 1024, new_micro / 1024, micro_improvement,
                    self.config.small_file_threshold / 1024, new_small / 1024, small_improvement
                );

                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Get performance summary for threshold adjustment analysis
    pub async fn get_performance_summary(&self) -> PerformanceSummary {
        let stats = self.stats.read().await;

        PerformanceSummary {
            micro_file_performance: stats.micro_file_performance.clone(),
            small_file_performance: stats.small_file_performance.clone(),
            large_file_performance: stats.large_file_performance.clone(),
            current_micro_threshold: self.config.micro_file_threshold,
            current_small_threshold: self.config.small_file_threshold,
            total_selections: stats.total_selections,
            threshold_adjustments: stats.threshold_adjustments,
        }
    }
}

impl Default for EngineSelector {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for EngineSelector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EngineSelector")
            .field("config", &self.config)
            .field("device_optimizer", &self.device_optimizer)
            .field("micro_engine", &"Arc<Mutex<MicroFileCopyEngine>>")
            .field("buffered_engine", &"Arc<Mutex<BufferedCopyEngine>>")
            .field("parallel_engine", &"Arc<Mutex<ParallelCopyEngine>>")
            .field("zerocopy_engine", &"Arc<ZeroCopyEngineImpl>")
            .field("stats", &"Arc<RwLock<EngineSelectionStats>>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_engine_selector_creation() {
        let selector = EngineSelector::new();
        assert!(selector.is_enabled());

        let stats = selector.get_stats().await;
        assert_eq!(stats.total_selections, 0);
    }

    #[tokio::test]
    async fn test_micro_file_selection() {
        let temp_dir = TempDir::new().unwrap();
        let selector = EngineSelector::new();

        // Create a micro file (2KB - within new 4KB threshold)
        let source = temp_dir.path().join("micro.txt");
        fs::write(&source, "A".repeat(2048)).unwrap();

        let dest = temp_dir.path().join("dest.txt");

        let selection = selector
            .select_optimal_engine(&source, &dest)
            .await
            .unwrap();

        assert_eq!(selection.engine_type, EngineType::MicroFile);
        assert!(selection.use_sync_mode);
        assert!(!selection.zerocopy_enabled);
        assert!(selection.reasoning.contains("Micro file optimization"));
    }

    #[tokio::test]
    async fn test_small_file_selection() {
        let temp_dir = TempDir::new().unwrap();
        let selector = EngineSelector::new();

        // Create a small file (8KB - within new 16KB threshold)
        let source = temp_dir.path().join("small.txt");
        fs::write(&source, "B".repeat(8192)).unwrap();

        let dest = temp_dir.path().join("dest.txt");

        let selection = selector
            .select_optimal_engine(&source, &dest)
            .await
            .unwrap();

        assert_eq!(selection.engine_type, EngineType::Buffered);
        assert!(selection.use_sync_mode);
        assert!(!selection.zerocopy_enabled);
        assert!(selection.reasoning.contains("Small file sync optimization"));
    }

    #[tokio::test]
    async fn test_large_file_selection() {
        let temp_dir = TempDir::new().unwrap();
        let selector = EngineSelector::new();

        // Create a large file (100KB)
        let source = temp_dir.path().join("large.txt");
        fs::write(&source, "C".repeat(100 * 1024)).unwrap();

        let dest = temp_dir.path().join("dest.txt");

        let selection = selector
            .select_optimal_engine(&source, &dest)
            .await
            .unwrap();

        // Should select buffered or zero-copy engine
        assert!(matches!(
            selection.engine_type,
            EngineType::Buffered | EngineType::ZeroCopy
        ));
        assert!(!selection.use_sync_mode);
        assert!(selection.reasoning.contains("Large file optimization"));
    }

    #[tokio::test]
    async fn test_parallel_file_selection() {
        let temp_dir = TempDir::new().unwrap();
        let selector = EngineSelector::new();

        // Create a very large file (60MB - above parallel threshold)
        let source = temp_dir.path().join("very_large.txt");
        let large_content = vec![b'D'; 60 * 1024 * 1024];
        fs::write(&source, large_content).unwrap();

        let dest = temp_dir.path().join("dest.txt");

        let selection = selector
            .select_optimal_engine(&source, &dest)
            .await
            .unwrap();

        // Should select parallel engine
        assert_eq!(selection.engine_type, EngineType::Parallel);
        assert!(!selection.use_sync_mode);
        assert!(!selection.zerocopy_enabled);
        assert!(selection
            .reasoning
            .contains("Parallel processing optimization"));
    }

    #[tokio::test]
    async fn test_disabled_selection() {
        let temp_dir = TempDir::new().unwrap();
        let mut selector = EngineSelector::new();
        selector.set_enabled(false);

        let source = temp_dir.path().join("test.txt");
        fs::write(&source, "test").unwrap();

        let dest = temp_dir.path().join("dest.txt");

        let selection = selector
            .select_optimal_engine(&source, &dest)
            .await
            .unwrap();

        assert_eq!(selection.engine_type, EngineType::Buffered);
        assert!(!selection.use_sync_mode);
        assert!(!selection.zerocopy_enabled);
        assert_eq!(selection.reasoning, "Intelligent selection disabled");
    }

    #[tokio::test]
    async fn test_statistics_tracking() {
        let temp_dir = TempDir::new().unwrap();
        let selector = EngineSelector::new();

        // Create files of different sizes (updated for new thresholds)
        let micro_file = temp_dir.path().join("micro.txt");
        fs::write(&micro_file, "A".repeat(2048)).unwrap(); // 2KB - still micro

        let small_file = temp_dir.path().join("small.txt");
        fs::write(&small_file, "B".repeat(8192)).unwrap(); // 8KB - now small sync

        let dest = temp_dir.path().join("dest.txt");

        // Select engines for different files
        let _selection1 = selector
            .select_optimal_engine(&micro_file, &dest)
            .await
            .unwrap();
        let _selection2 = selector
            .select_optimal_engine(&small_file, &dest)
            .await
            .unwrap();

        let stats = selector.get_stats().await;
        assert_eq!(stats.total_selections, 2);
        assert_eq!(stats.micro_engine_selections, 1);
        assert_eq!(stats.sync_buffered_selections, 1);
        assert!(stats.avg_selection_time_ns > 0);
    }

    #[test]
    fn test_engine_selection_stats() {
        let mut stats = EngineSelectionStats::default();

        stats.total_selections = 10;
        stats.micro_engine_selections = 3;
        stats.zerocopy_enabled_selections = 2;

        assert_eq!(stats.micro_engine_percentage(), 30.0);
        assert_eq!(stats.zerocopy_percentage(), 20.0);
    }

    #[test]
    fn test_config_default() {
        let config = EngineSelectionConfig::default();
        assert!(config.enabled);
        assert_eq!(config.micro_file_threshold, MICRO_FILE_THRESHOLD);
        assert_eq!(config.small_file_threshold, SMALL_FILE_THRESHOLD);
        assert_eq!(config.zerocopy_threshold, ZEROCOPY_THRESHOLD);
        assert!(config.enable_dynamic_thresholds);
        assert_eq!(config.min_samples_for_adjustment, 100);
        assert_eq!(config.performance_improvement_threshold, 5.0);
    }

    #[test]
    fn test_performance_history_update() {
        let mut stats = EngineSelectionStats::default();

        // Update micro file performance
        stats.update_performance_history(1024, 1024, 1_000_000, 4096, 16384);
        assert_eq!(stats.micro_file_performance.sample_count, 1);
        assert!(stats.micro_file_performance.avg_throughput_bps > 0.0);

        // Update small file performance
        stats.update_performance_history(8192, 8192, 2_000_000, 4096, 16384);
        assert_eq!(stats.small_file_performance.sample_count, 1);
        assert!(stats.small_file_performance.avg_throughput_bps > 0.0);

        // Update large file performance
        stats.update_performance_history(100_000, 100_000, 10_000_000, 4096, 16384);
        assert_eq!(stats.large_file_performance.sample_count, 1);
        assert!(stats.large_file_performance.avg_throughput_bps > 0.0);
    }

    #[test]
    fn test_threshold_adjustment_recommendation() {
        let mut stats = EngineSelectionStats::default();

        // Not enough samples initially
        assert!(!stats.should_consider_threshold_adjustment(50));

        // Add enough samples with micro files performing significantly better
        for _ in 0..100 {
            // Micro files: 2048 bytes in 250,000 ns = ~8 MB/s
            stats.update_performance_history(2048, 2048, 250_000, 4096, 16384);
        }

        for _ in 0..100 {
            // Small files: 8192 bytes in 2,000,000 ns = ~4 MB/s (much slower, ratio = 2.0)
            stats.update_performance_history(8192, 8192, 2_000_000, 4096, 16384);
        }

        assert!(stats.should_consider_threshold_adjustment(50));

        // Debug the performance values
        println!(
            "Micro throughput: {}",
            stats.micro_file_performance.avg_throughput_bps
        );
        println!(
            "Small throughput: {}",
            stats.small_file_performance.avg_throughput_bps
        );

        let recommendation = stats.get_threshold_adjustment_recommendation(4096, 16384);
        assert!(
            recommendation.is_some(),
            "Should recommend threshold adjustment when micro files perform much better"
        );

        if let Some((new_micro, new_small)) = recommendation {
            assert!(new_micro > 4096); // Should increase micro threshold
            assert_eq!(new_small, 16384); // Small threshold unchanged
        }
    }

    #[tokio::test]
    async fn test_optimized_thresholds_performance() {
        let temp_dir = TempDir::new().unwrap();
        let selector = EngineSelector::new();

        // Test that files up to 4KB now use micro engine (vs old 1KB limit)
        let source_3kb = temp_dir.path().join("3kb.txt");
        fs::write(&source_3kb, "A".repeat(3072)).unwrap();

        let dest = temp_dir.path().join("dest.txt");
        let selection = selector
            .select_optimal_engine(&source_3kb, &dest)
            .await
            .unwrap();

        assert_eq!(selection.engine_type, EngineType::MicroFile);
        assert!(selection.reasoning.contains("Micro file optimization"));

        // Test that files up to 16KB now use sync mode (vs old 4KB limit)
        let source_12kb = temp_dir.path().join("12kb.txt");
        fs::write(&source_12kb, "B".repeat(12288)).unwrap();

        let selection = selector
            .select_optimal_engine(&source_12kb, &dest)
            .await
            .unwrap();

        assert_eq!(selection.engine_type, EngineType::Buffered);
        assert!(selection.use_sync_mode);
        assert!(selection.reasoning.contains("Small file sync optimization"));
    }

    #[test]
    fn test_enhanced_threshold_adjustment() {
        let mut stats = EngineSelectionStats::default();

        // Simulate micro files performing much better than small files
        for _ in 0..100 {
            // Micro files: 1KB in 500,000ns = 2MB/s
            stats.update_performance_history(1024, 1024, 500_000, 4096, 16384);
        }

        for _ in 0..100 {
            // Small files: 8KB in 8_000_000ns = 1MB/s (slower)
            stats.update_performance_history(8192, 8192, 8_000_000, 4096, 16384);
        }

        // Should recommend expanding micro threshold
        let recommendation = stats.get_threshold_adjustment_recommendation(4096, 16384);
        assert!(recommendation.is_some());

        let (new_micro, new_small) = recommendation.unwrap();
        assert!(new_micro > 4096); // Should expand micro threshold
        assert_eq!(new_small, 16384); // Small threshold should remain the same
    }

    #[test]
    fn test_threshold_adjustment_requires_sufficient_samples() {
        let mut stats = EngineSelectionStats::default();

        // Add only a few samples (less than required minimum)
        for _ in 0..10 {
            stats.update_performance_history(1024, 1024, 500_000, 4096, 16384);
            stats.update_performance_history(8192, 8192, 8_000_000, 4096, 16384);
        }

        // Should not recommend adjustment with insufficient samples
        let recommendation = stats.get_threshold_adjustment_recommendation(4096, 16384);
        assert!(recommendation.is_none());
    }

    #[tokio::test]
    async fn test_auto_threshold_adjustment() {
        let mut selector = EngineSelector::new();

        // Simulate performance data that would trigger adjustment
        for _ in 0..150 {
            selector
                .update_performance_history(1024, 1024, 500_000)
                .await; // Fast micro files
            selector
                .update_performance_history(8192, 8192, 8_000_000)
                .await; // Slower small files
        }

        // Test auto adjustment
        let adjusted = selector.auto_adjust_thresholds().await.unwrap();
        assert!(adjusted); // Should have made an adjustment

        // Verify thresholds were actually changed
        let config = selector.get_config();
        assert!(config.micro_file_threshold > 4096); // Should have expanded
    }

    #[tokio::test]
    async fn test_performance_summary() {
        let selector = EngineSelector::new();

        // Add some performance data
        for _ in 0..50 {
            selector
                .update_performance_history(1024, 1024, 1_000_000)
                .await;
            selector
                .update_performance_history(8192, 8192, 2_000_000)
                .await;
            selector
                .update_performance_history(100_000, 100_000, 10_000_000)
                .await;
        }

        let summary = selector.get_performance_summary().await;

        assert_eq!(summary.micro_file_performance.sample_count, 50);
        assert_eq!(summary.small_file_performance.sample_count, 50);
        assert_eq!(summary.large_file_performance.sample_count, 50);
        assert_eq!(summary.current_micro_threshold, 4096);
        assert_eq!(summary.current_small_threshold, 16384);
        assert!(summary.micro_file_performance.avg_throughput_bps > 0.0);
    }
}
