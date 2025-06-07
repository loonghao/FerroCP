//! JSON output structures for FerroCP CLI

use ferrocp_device::{DeviceComparison, DeviceInfo};
use ferrocp_types::{CopyStats, DeviceType};
use serde::{Deserialize, Serialize};

/// Complete JSON output for copy operations
#[derive(Debug, Serialize, Deserialize)]
pub struct CopyResultJson {
    /// Operation metadata
    pub metadata: OperationMetadata,
    /// Source device information
    pub source_device: DeviceInfoJson,
    /// Destination device information
    pub destination_device: DeviceInfoJson,
    /// Performance analysis
    pub performance_analysis: PerformanceAnalysisJson,
    /// Copy statistics
    pub copy_stats: CopyStatsJson,
    /// Overall result
    pub result: OperationResult,
}

/// Operation metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct OperationMetadata {
    /// FerroCP version
    pub version: String,
    /// Operation type
    pub operation: String,
    /// Timestamp when operation started
    pub timestamp: String,
    /// Source path
    pub source_path: String,
    /// Destination path
    pub destination_path: String,
}

/// Device information in JSON format
#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceInfoJson {
    /// Device type
    pub device_type: DeviceTypeJson,
    /// Device type description
    pub device_type_description: String,
    /// Filesystem type
    pub filesystem: String,
    /// Total space in bytes
    pub total_space_bytes: u64,
    /// Available space in bytes
    pub available_space_bytes: u64,
    /// Space usage percentage
    pub space_usage_percent: f64,
    /// Theoretical read speed in MB/s
    pub theoretical_read_speed_mbps: f64,
    /// Theoretical write speed in MB/s
    pub theoretical_write_speed_mbps: f64,
    /// Optimal buffer size in bytes
    pub optimal_buffer_size_bytes: usize,
    /// Technical details
    pub technical_details: TechnicalDetailsJson,
}

/// Device type in JSON format
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviceTypeJson {
    Ssd,
    Hdd,
    Network,
    Ramdisk,
    Unknown,
}

impl From<DeviceType> for DeviceTypeJson {
    fn from(device_type: DeviceType) -> Self {
        match device_type {
            DeviceType::SSD => DeviceTypeJson::Ssd,
            DeviceType::HDD => DeviceTypeJson::Hdd,
            DeviceType::Network => DeviceTypeJson::Network,
            DeviceType::RamDisk => DeviceTypeJson::Ramdisk,
            DeviceType::Unknown => DeviceTypeJson::Unknown,
        }
    }
}

/// Technical device details
#[derive(Debug, Serialize, Deserialize)]
pub struct TechnicalDetailsJson {
    /// Random read IOPS
    pub random_read_iops: f64,
    /// Random write IOPS
    pub random_write_iops: f64,
    /// Average latency in microseconds
    pub average_latency_us: f64,
    /// Queue depth
    pub queue_depth: u32,
    /// TRIM support
    pub supports_trim: bool,
}

/// Performance analysis in JSON format
#[derive(Debug, Serialize, Deserialize)]
pub struct PerformanceAnalysisJson {
    /// Expected transfer speed in MB/s
    pub expected_speed_mbps: f64,
    /// Performance bottleneck
    pub bottleneck: BottleneckJson,
    /// Optimization recommendations
    pub recommendations: Vec<String>,
}

/// Performance bottleneck information
#[derive(Debug, Serialize, Deserialize)]
pub struct BottleneckJson {
    /// Which device is the bottleneck
    pub device: String, // "source" or "destination"
    /// Bottleneck description
    pub description: String,
    /// Limiting speed in MB/s
    pub limiting_speed_mbps: f64,
}

/// Copy statistics in JSON format
#[derive(Debug, Serialize, Deserialize)]
pub struct CopyStatsJson {
    /// Number of files copied
    pub files_copied: u64,
    /// Number of directories created
    pub directories_created: u64,
    /// Total bytes copied
    pub bytes_copied: u64,
    /// Number of files skipped
    pub files_skipped: u64,
    /// Number of errors
    pub errors: u64,
    /// Duration in seconds
    pub duration_seconds: f64,
    /// Actual transfer rate in MB/s
    pub actual_transfer_rate_mbps: f64,
    /// Number of zero-copy operations
    pub zerocopy_operations: u64,
    /// Zero-copy efficiency percentage
    pub zerocopy_efficiency_percent: f64,
    /// Performance efficiency compared to expected
    pub performance_efficiency_percent: f64,
}

/// Overall operation result
#[derive(Debug, Serialize, Deserialize)]
pub struct OperationResult {
    /// Whether the operation was successful
    pub success: bool,
    /// Result message
    pub message: String,
    /// Performance rating
    pub performance_rating: PerformanceRating,
}

/// Performance rating
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PerformanceRating {
    Excellent, // >= 90%
    Good,      // >= 70%
    Fair,      // >= 50%
    Poor,      // < 50%
}

impl CopyResultJson {
    /// Create a new CopyResultJson from components
    pub fn new(
        source_path: String,
        destination_path: String,
        source_device: &DeviceInfo,
        destination_device: &DeviceInfo,
        comparison: &DeviceComparison,
        stats: &CopyStats,
    ) -> Self {
        let actual_speed_mbps = stats.transfer_rate() / 1024.0 / 1024.0;
        let efficiency = if comparison.expected_speed_mbps > 0.0 {
            (actual_speed_mbps / comparison.expected_speed_mbps) * 100.0
        } else {
            0.0
        };

        let performance_rating = match efficiency {
            e if e >= 90.0 => PerformanceRating::Excellent,
            e if e >= 70.0 => PerformanceRating::Good,
            e if e >= 50.0 => PerformanceRating::Fair,
            _ => PerformanceRating::Poor,
        };

        let success = stats.errors == 0;
        let message = if success {
            if efficiency >= 90.0 {
                "Copy completed successfully with excellent performance".to_string()
            } else if efficiency >= 60.0 {
                "Copy completed successfully with good performance".to_string()
            } else {
                "Copy completed successfully but performance was lower than expected".to_string()
            }
        } else {
            format!("Copy completed with {} errors", stats.errors)
        };

        Self {
            metadata: OperationMetadata {
                version: env!("CARGO_PKG_VERSION").to_string(),
                operation: "copy".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                source_path,
                destination_path,
            },
            source_device: DeviceInfoJson::from_device_info(source_device),
            destination_device: DeviceInfoJson::from_device_info(destination_device),
            performance_analysis: PerformanceAnalysisJson::from_comparison(comparison),
            copy_stats: CopyStatsJson::from_copy_stats(stats, efficiency),
            result: OperationResult {
                success,
                message,
                performance_rating,
            },
        }
    }
}

impl DeviceInfoJson {
    /// Create DeviceInfoJson from DeviceInfo
    pub fn from_device_info(device_info: &DeviceInfo) -> Self {
        let space_usage_percent = if device_info.total_space > 0 {
            ((device_info.total_space - device_info.available_space) as f64
                / device_info.total_space as f64)
                * 100.0
        } else {
            0.0
        };

        Self {
            device_type: device_info.device_type.into(),
            device_type_description: device_info.device_type_description().to_string(),
            filesystem: device_info.filesystem.clone(),
            total_space_bytes: device_info.total_space,
            available_space_bytes: device_info.available_space,
            space_usage_percent,
            theoretical_read_speed_mbps: device_info.theoretical_read_speed_mbps(),
            theoretical_write_speed_mbps: device_info.theoretical_write_speed_mbps(),
            optimal_buffer_size_bytes: device_info.optimal_buffer_size,
            technical_details: TechnicalDetailsJson {
                random_read_iops: device_info.performance.random_read_iops,
                random_write_iops: device_info.performance.random_write_iops,
                average_latency_us: device_info.performance.average_latency,
                queue_depth: device_info.performance.queue_depth,
                supports_trim: device_info.performance.supports_trim,
            },
        }
    }
}

impl PerformanceAnalysisJson {
    /// Create PerformanceAnalysisJson from DeviceComparison
    pub fn from_comparison(comparison: &DeviceComparison) -> Self {
        let bottleneck = match comparison.bottleneck {
            ferrocp_device::Bottleneck::Source => BottleneckJson {
                device: "source".to_string(),
                description: format!(
                    "Source {} limits read speed",
                    comparison.source_info.device_type_description()
                ),
                limiting_speed_mbps: comparison.source_info.theoretical_read_speed_mbps(),
            },
            ferrocp_device::Bottleneck::Destination => BottleneckJson {
                device: "destination".to_string(),
                description: format!(
                    "Destination {} limits write speed",
                    comparison.destination_info.device_type_description()
                ),
                limiting_speed_mbps: comparison.destination_info.theoretical_write_speed_mbps(),
            },
        };

        Self {
            expected_speed_mbps: comparison.expected_speed_mbps,
            bottleneck,
            recommendations: comparison.get_recommendations(),
        }
    }
}

impl CopyStatsJson {
    /// Create CopyStatsJson from CopyStats
    pub fn from_copy_stats(stats: &CopyStats, efficiency: f64) -> Self {
        Self {
            files_copied: stats.files_copied,
            directories_created: stats.directories_created,
            bytes_copied: stats.bytes_copied,
            files_skipped: stats.files_skipped,
            errors: stats.errors,
            duration_seconds: stats.duration.as_secs_f64(),
            actual_transfer_rate_mbps: stats.transfer_rate() / 1024.0 / 1024.0,
            zerocopy_operations: stats.zerocopy_operations,
            zerocopy_efficiency_percent: stats.zerocopy_efficiency() * 100.0,
            performance_efficiency_percent: efficiency,
        }
    }
}
