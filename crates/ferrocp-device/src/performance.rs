//! Device performance analysis and display utilities

use crate::analyzer::{DeviceAnalyzer, DevicePerformance};
use crate::detector::DeviceDetector as DeviceDetectorImpl;
use ferrocp_types::{DeviceDetector as DeviceDetectorTrait, DeviceType, Result};
use std::path::Path;
use tracing::debug;

/// Extended device information including performance characteristics
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    /// Device type (SSD, HDD, Network, etc.)
    pub device_type: DeviceType,
    /// Performance characteristics
    pub performance: DevicePerformance,
    /// Filesystem type
    pub filesystem: String,
    /// Total space in bytes
    pub total_space: u64,
    /// Available space in bytes
    pub available_space: u64,
    /// Optimal buffer size for this device
    pub optimal_buffer_size: usize,
}

impl DeviceInfo {
    /// Get a human-readable description of the device type
    pub fn device_type_description(&self) -> &'static str {
        match self.device_type {
            DeviceType::SSD => "Solid State Drive (SSD)",
            DeviceType::HDD => "Hard Disk Drive (HDD)",
            DeviceType::Network => "Network Storage",
            DeviceType::RamDisk => "RAM Disk",
            DeviceType::Unknown => "Unknown Storage",
        }
    }

    /// Get theoretical read speed in MB/s
    pub fn theoretical_read_speed_mbps(&self) -> f64 {
        self.performance.sequential_read_speed
    }

    /// Get theoretical write speed in MB/s
    pub fn theoretical_write_speed_mbps(&self) -> f64 {
        self.performance.sequential_write_speed
    }

    /// Format space information
    pub fn format_space_info(&self) -> String {
        let total_gb = self.total_space as f64 / (1024.0 * 1024.0 * 1024.0);
        let available_gb = self.available_space as f64 / (1024.0 * 1024.0 * 1024.0);
        let used_percent =
            ((self.total_space - self.available_space) as f64 / self.total_space as f64) * 100.0;

        format!(
            "{:.1} GB total, {:.1} GB available ({:.1}% used)",
            total_gb, available_gb, used_percent
        )
    }

    /// Get optimal buffer size description
    pub fn format_buffer_size(&self) -> String {
        let size_kb = self.optimal_buffer_size / 1024;
        if size_kb >= 1024 {
            format!("{} MB", size_kb / 1024)
        } else {
            format!("{} KB", size_kb)
        }
    }
}

/// Device performance analyzer with enhanced capabilities
pub struct PerformanceAnalyzer {
    detector: DeviceDetectorImpl,
    analyzer: DeviceAnalyzer,
}

impl PerformanceAnalyzer {
    /// Create a new performance analyzer
    pub fn new() -> Self {
        Self {
            detector: DeviceDetectorImpl::new(),
            analyzer: DeviceAnalyzer::new(),
        }
    }

    /// Analyze device information for a given path
    pub async fn analyze_device<P: AsRef<Path> + Send + Sync>(
        &self,
        path: P,
    ) -> Result<DeviceInfo> {
        let path = path.as_ref();
        debug!("Analyzing device for path: {}", path.display());

        // Detect device type
        let device_type = self.detector.detect_device_type_cached(path).await?;
        debug!("Detected device type: {:?}", device_type);

        // Analyze performance characteristics
        let performance = self
            .analyzer
            .analyze_device_performance(&device_type)
            .await?;
        debug!("Device performance: {:?}", performance);

        // Get filesystem information
        let fs_info = self.analyzer.get_filesystem_info(path).await?;
        debug!("Filesystem info: {:?}", fs_info);

        // Get optimal buffer size
        let optimal_buffer_size =
            DeviceDetectorTrait::get_optimal_buffer_size(&self.detector, device_type);

        Ok(DeviceInfo {
            device_type,
            performance,
            filesystem: fs_info.fs_type,
            total_space: fs_info.total_space,
            available_space: fs_info.available_space,
            optimal_buffer_size,
        })
    }

    /// Compare two devices and provide optimization recommendations
    pub fn compare_devices(
        &self,
        source: &DeviceInfo,
        destination: &DeviceInfo,
    ) -> DeviceComparison {
        let bottleneck =
            if source.theoretical_read_speed_mbps() < destination.theoretical_write_speed_mbps() {
                Bottleneck::Source
            } else {
                Bottleneck::Destination
            };

        let expected_speed = source
            .theoretical_read_speed_mbps()
            .min(destination.theoretical_write_speed_mbps());

        DeviceComparison {
            bottleneck,
            expected_speed_mbps: expected_speed,
            source_info: source.clone(),
            destination_info: destination.clone(),
        }
    }
}

impl Default for PerformanceAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Device comparison result
#[derive(Debug, Clone)]
pub struct DeviceComparison {
    /// Which device is the bottleneck
    pub bottleneck: Bottleneck,
    /// Expected transfer speed in MB/s
    pub expected_speed_mbps: f64,
    /// Source device information
    pub source_info: DeviceInfo,
    /// Destination device information
    pub destination_info: DeviceInfo,
}

impl DeviceComparison {
    /// Get a description of the bottleneck
    pub fn bottleneck_description(&self) -> String {
        match self.bottleneck {
            Bottleneck::Source => format!(
                "Source {} (read speed: {:.0} MB/s)",
                self.source_info.device_type_description(),
                self.source_info.theoretical_read_speed_mbps()
            ),
            Bottleneck::Destination => format!(
                "Destination {} (write speed: {:.0} MB/s)",
                self.destination_info.device_type_description(),
                self.destination_info.theoretical_write_speed_mbps()
            ),
        }
    }

    /// Get optimization recommendations
    pub fn get_recommendations(&self) -> Vec<String> {
        let mut recommendations = Vec::new();

        match (
            &self.source_info.device_type,
            &self.destination_info.device_type,
        ) {
            (DeviceType::HDD, DeviceType::SSD) => {
                recommendations
                    .push("Consider using larger buffer sizes for HDD source".to_string());
                recommendations
                    .push("Sequential access patterns will be most efficient".to_string());
            }
            (DeviceType::SSD, DeviceType::HDD) => {
                recommendations.push("Destination HDD may be the bottleneck".to_string());
                recommendations.push("Consider defragmenting destination if possible".to_string());
            }
            (DeviceType::Network, _) | (_, DeviceType::Network) => {
                recommendations
                    .push("Network transfer - latency may affect small files".to_string());
                recommendations
                    .push("Consider compression for better network utilization".to_string());
            }
            _ => {
                recommendations.push("Both devices have similar characteristics".to_string());
            }
        }

        recommendations
    }
}

/// Identifies which device is the performance bottleneck
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bottleneck {
    /// Source device (read speed) is the bottleneck
    Source,
    /// Destination device (write speed) is the bottleneck
    Destination,
}
