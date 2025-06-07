//! Intelligent device detection and optimization for FerroCP
//!
//! This crate provides comprehensive device detection and optimization capabilities
//! for the FerroCP file copy system. It includes:
//!
//! - **Cross-platform device detection**: Identify storage device types (SSD, HDD, Network, etc.)
//! - **Windows ReFS CoW support**: Detect and utilize Copy-on-Write capabilities
//! - **Performance analysis**: Analyze storage device characteristics for optimization
//! - **Device-specific strategies**: Provide optimal settings based on device type
//!
//! # Features
//!
//! - `serde` (default): Enable serialization support for device information
//!
//! # Examples
//!
//! ```rust
//! use ferrocp_device::{DeviceDetector, DeviceAnalyzer};
//! use ferrocp_types::{DeviceType, DeviceDetector as DeviceDetectorTrait};
//!
//! # tokio_test::block_on(async {
//! let detector = DeviceDetector::new();
//! let device_type = detector.detect_device_type("/path/to/file").await?;
//!
//! let analyzer = DeviceAnalyzer::new();
//! let performance = analyzer.analyze_device_performance(&device_type).await?;
//! # Ok::<(), ferrocp_types::Error>(())
//! # });
//! ```

#![deny(missing_docs)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::module_name_repetitions)]

pub mod analyzer;
pub mod cache;
pub mod detector;
pub mod optimization;
pub mod performance;

#[cfg(windows)]
pub mod windows;

#[cfg(unix)]
pub mod unix;

// Re-export main types
pub use analyzer::{DeviceAnalyzer, DevicePerformance, FileSystemInfo};
pub use cache::{
    create_shared_cache, create_shared_cache_with_config, DeviceCache, DeviceCacheConfig,
    SharedDeviceCache,
};
pub use detector::DeviceDetector;
pub use optimization::{DeviceOptimizer, OptimizationStrategy};
pub use performance::{Bottleneck, DeviceComparison, DeviceInfo, PerformanceAnalyzer};
