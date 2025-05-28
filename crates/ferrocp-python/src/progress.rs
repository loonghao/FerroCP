//! Progress tracking for Python bindings

use ferrocp_types::CopyStats;
use pyo3::prelude::*;
use std::time::Duration;

/// Python wrapper for progress information
#[pyclass(name = "Progress")]
#[derive(Debug, Clone)]
pub struct PyProgress {
    /// Total bytes to copy
    #[pyo3(get)]
    pub total_bytes: u64,
    /// Bytes copied so far
    #[pyo3(get)]
    pub bytes_copied: u64,
    /// Total files to copy
    #[pyo3(get)]
    pub total_files: u64,
    /// Files copied so far
    #[pyo3(get)]
    pub files_copied: u64,
    /// Current transfer rate (bytes per second)
    #[pyo3(get)]
    pub transfer_rate: f64,
    /// Estimated time remaining in seconds
    #[pyo3(get)]
    pub eta_seconds: Option<f64>,
    /// Progress percentage (0.0 - 100.0)
    #[pyo3(get)]
    pub percentage: f64,
    /// Current file being processed
    #[pyo3(get)]
    pub current_file: Option<String>,
}

#[pymethods]
impl PyProgress {
    /// Create a new progress instance
    #[new]
    pub fn new() -> Self {
        Self {
            total_bytes: 0,
            bytes_copied: 0,
            total_files: 0,
            files_copied: 0,
            transfer_rate: 0.0,
            eta_seconds: None,
            percentage: 0.0,
            current_file: None,
        }
    }

    /// Check if the operation is complete
    pub fn is_complete(&self) -> bool {
        self.bytes_copied >= self.total_bytes && self.files_copied >= self.total_files
    }

    /// Get formatted transfer rate string
    pub fn format_transfer_rate(&self) -> String {
        format_bytes_per_second(self.transfer_rate)
    }

    /// Get formatted ETA string
    pub fn format_eta(&self) -> String {
        match self.eta_seconds {
            Some(seconds) => format_duration(Duration::from_secs_f64(seconds)),
            None => "Unknown".to_string(),
        }
    }

    /// Get formatted bytes copied string
    pub fn format_bytes_copied(&self) -> String {
        format!(
            "{} / {}",
            format_bytes(self.bytes_copied),
            format_bytes(self.total_bytes)
        )
    }

    /// Get formatted files copied string
    pub fn format_files_copied(&self) -> String {
        format!("{} / {}", self.files_copied, self.total_files)
    }

    /// String representation
    fn __str__(&self) -> String {
        format!(
            "Progress({}%, {} files, {} bytes, {})",
            self.percentage,
            self.format_files_copied(),
            self.format_bytes_copied(),
            self.format_transfer_rate()
        )
    }

    /// Representation
    fn __repr__(&self) -> String {
        format!(
            "Progress(percentage={:.1}, files_copied={}, bytes_copied={}, transfer_rate={:.1})",
            self.percentage, self.files_copied, self.bytes_copied, self.transfer_rate
        )
    }
}

impl Default for PyProgress {
    fn default() -> Self {
        Self::new()
    }
}

impl From<CopyStats> for PyProgress {
    fn from(stats: CopyStats) -> Self {
        // Since CopyStats doesn't have total_bytes/total_files, we'll use what we have
        // For progress tracking, we'll assume 100% completion if we have any bytes copied
        let percentage = if stats.bytes_copied > 0 {
            100.0 // Assume completion since we don't have total info
        } else {
            0.0
        };

        let transfer_rate = if stats.duration.as_secs_f64() > 0.0 {
            stats.bytes_copied as f64 / stats.duration.as_secs_f64()
        } else {
            0.0
        };

        // No ETA calculation since we don't have total bytes
        let eta_seconds = None;

        Self {
            total_bytes: stats.bytes_copied, // Use bytes_copied as total for completed operations
            bytes_copied: stats.bytes_copied,
            total_files: stats.files_copied, // Use files_copied as total for completed operations
            files_copied: stats.files_copied,
            transfer_rate,
            eta_seconds,
            percentage,
            current_file: None,
        }
    }
}

/// Python callback wrapper for progress updates
pub type ProgressCallback = PyObject;

/// Call Python progress callback
pub fn call_progress_callback(
    py: Python<'_>,
    callback: &Option<ProgressCallback>,
    progress: &PyProgress,
) -> PyResult<()> {
    if let Some(callback) = callback {
        let progress_obj = Py::new(py, progress.clone())?;
        callback.call1(py, (progress_obj,))?;
    }
    Ok(())
}

/// Format bytes as human-readable string
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.1} {}", size, UNITS[unit_index])
}

/// Format bytes per second as human-readable string
fn format_bytes_per_second(bytes_per_sec: f64) -> String {
    const UNITS: &[&str] = &["B/s", "KB/s", "MB/s", "GB/s", "TB/s"];
    let mut size = bytes_per_sec;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.1} {}", size, UNITS[unit_index])
}

/// Format duration as human-readable string
fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.as_secs();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_creation() {
        let progress = PyProgress::new();
        assert_eq!(progress.total_bytes, 0);
        assert_eq!(progress.bytes_copied, 0);
        assert_eq!(progress.percentage, 0.0);
    }

    #[test]
    fn test_progress_from_stats() {
        let stats = CopyStats {
            bytes_copied: 500,
            files_copied: 5,
            duration: Duration::from_secs(1),
            ..Default::default()
        };

        let progress = PyProgress::from(stats);
        assert_eq!(progress.bytes_copied, 500);
        assert_eq!(progress.files_copied, 5);
        assert_eq!(progress.transfer_rate, 500.0);
    }

    #[test]
    fn test_format_functions() {
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1048576), "1.0 MB");

        assert_eq!(format_bytes_per_second(1024.0), "1.0 KB/s");
        assert_eq!(format_bytes_per_second(1048576.0), "1.0 MB/s");

        assert_eq!(format_duration(Duration::from_secs(65)), "1m 5s");
        assert_eq!(format_duration(Duration::from_secs(3665)), "1h 1m 5s");
    }
}
