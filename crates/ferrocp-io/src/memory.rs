//! Memory monitoring and management for FerroCP I/O operations

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Memory monitor that tracks usage patterns and provides optimization recommendations
#[derive(Debug)]
pub struct MemoryMonitor {
    /// Historical memory usage samples
    usage_history: Arc<Mutex<VecDeque<MemoryUsageSample>>>,
    /// Maximum number of samples to keep
    max_samples: usize,
    /// Memory usage thresholds
    thresholds: MemoryThresholds,
    /// Start time for monitoring
    start_time: Instant,
}

/// A single memory usage sample
#[derive(Debug, Clone)]
pub struct MemoryUsageSample {
    /// Timestamp of the sample
    pub timestamp: Instant,
    /// Memory usage in bytes
    pub memory_used: u64,
    /// Number of active allocations
    pub active_allocations: u64,
    /// Memory efficiency percentage
    pub efficiency: f64,
}

/// Memory usage thresholds for different alert levels
#[derive(Debug, Clone)]
pub struct MemoryThresholds {
    /// Warning threshold (percentage of max memory)
    pub warning_threshold: f64,
    /// Critical threshold (percentage of max memory)
    pub critical_threshold: f64,
    /// Maximum allowed memory in bytes
    pub max_memory_bytes: u64,
    /// Minimum efficiency threshold
    pub min_efficiency: f64,
}

impl Default for MemoryThresholds {
    fn default() -> Self {
        Self {
            warning_threshold: 70.0,
            critical_threshold: 85.0,
            max_memory_bytes: 512 * 1024 * 1024, // 512MB default
            min_efficiency: 60.0,
        }
    }
}

/// Memory monitoring alerts
#[derive(Debug, Clone, PartialEq)]
pub enum MemoryAlert {
    /// Memory usage is normal
    Normal,
    /// Memory usage is approaching limits
    Warning {
        /// Current memory usage percentage
        usage_percent: f64,
        /// Alert message
        message: String,
    },
    /// Memory usage is critical
    Critical {
        /// Current memory usage percentage
        usage_percent: f64,
        /// Alert message
        message: String,
    },
    /// Memory efficiency is low
    LowEfficiency {
        /// Current efficiency percentage
        efficiency: f64,
        /// Alert message
        message: String,
    },
}

impl MemoryMonitor {
    /// Create a new memory monitor
    pub fn new(thresholds: MemoryThresholds) -> Self {
        Self {
            usage_history: Arc::new(Mutex::new(VecDeque::new())),
            max_samples: 1000, // Keep last 1000 samples
            thresholds,
            start_time: Instant::now(),
        }
    }

    /// Create a memory monitor with default thresholds
    pub fn with_default_thresholds() -> Self {
        Self::new(MemoryThresholds::default())
    }

    /// Record a memory usage sample
    pub fn record_usage(&self, memory_used: u64, active_allocations: u64, efficiency: f64) {
        let sample = MemoryUsageSample {
            timestamp: Instant::now(),
            memory_used,
            active_allocations,
            efficiency,
        };

        let mut history = self.usage_history.lock().unwrap();
        history.push_back(sample);

        // Keep only the most recent samples
        while history.len() > self.max_samples {
            history.pop_front();
        }
    }

    /// Check current memory status and return any alerts
    pub fn check_memory_status(&self, current_usage: u64, efficiency: f64) -> MemoryAlert {
        let usage_percent =
            (current_usage as f64 / self.thresholds.max_memory_bytes as f64) * 100.0;

        // Check efficiency first
        if efficiency < self.thresholds.min_efficiency {
            return MemoryAlert::LowEfficiency {
                efficiency,
                message: format!(
                    "Memory efficiency is {:.1}%, below threshold of {:.1}%",
                    efficiency, self.thresholds.min_efficiency
                ),
            };
        }

        // Check memory usage thresholds
        if usage_percent >= self.thresholds.critical_threshold {
            MemoryAlert::Critical {
                usage_percent,
                message: format!(
                    "Memory usage is {:.1}%, exceeding critical threshold of {:.1}%",
                    usage_percent, self.thresholds.critical_threshold
                ),
            }
        } else if usage_percent >= self.thresholds.warning_threshold {
            MemoryAlert::Warning {
                usage_percent,
                message: format!(
                    "Memory usage is {:.1}%, approaching warning threshold of {:.1}%",
                    usage_percent, self.thresholds.warning_threshold
                ),
            }
        } else {
            MemoryAlert::Normal
        }
    }

    /// Get memory usage statistics over a time period
    pub fn get_usage_stats(&self, duration: Duration) -> MemoryUsageStats {
        let history = self.usage_history.lock().unwrap();
        let now = Instant::now();
        let cutoff_time = now.checked_sub(duration).unwrap_or(self.start_time);

        let recent_samples: Vec<_> = history
            .iter()
            .filter(|sample| sample.timestamp >= cutoff_time)
            .cloned()
            .collect();

        if recent_samples.is_empty() {
            return MemoryUsageStats::default();
        }

        let total_samples = recent_samples.len() as f64;
        let avg_memory =
            recent_samples.iter().map(|s| s.memory_used).sum::<u64>() as f64 / total_samples;
        let avg_efficiency =
            recent_samples.iter().map(|s| s.efficiency).sum::<f64>() / total_samples;
        let max_memory = recent_samples
            .iter()
            .map(|s| s.memory_used)
            .max()
            .unwrap_or(0);
        let min_memory = recent_samples
            .iter()
            .map(|s| s.memory_used)
            .min()
            .unwrap_or(0);

        MemoryUsageStats {
            avg_memory_used: avg_memory as u64,
            max_memory_used: max_memory,
            min_memory_used: min_memory,
            avg_efficiency,
            sample_count: recent_samples.len(),
            duration_seconds: duration.as_secs_f64(),
        }
    }

    /// Get optimization recommendations based on usage patterns
    pub fn get_optimization_recommendations(&self) -> Vec<String> {
        let mut recommendations = Vec::new();
        // Use a reasonable time window that won't cause overflow in tests
        let analysis_duration = std::cmp::min(
            Duration::from_secs(300), // 5 minutes max
            self.monitoring_duration().saturating_add(Duration::from_secs(1)) // At least current duration + 1s
        );
        let recent_stats = self.get_usage_stats(analysis_duration);

        if recent_stats.avg_efficiency < 70.0 {
            recommendations.push(format!(
                "Consider reducing buffer pool sizes. Current efficiency: {:.1}%",
                recent_stats.avg_efficiency
            ));
        }

        if recent_stats.max_memory_used > self.thresholds.max_memory_bytes * 8 / 10 {
            recommendations.push(
                "Memory usage is consistently high. Consider increasing cleanup frequency."
                    .to_string(),
            );
        }

        let usage_variance = if recent_stats.max_memory_used > recent_stats.min_memory_used {
            (recent_stats.max_memory_used - recent_stats.min_memory_used) as f64
                / recent_stats.avg_memory_used as f64
        } else {
            0.0
        };

        if usage_variance > 0.5 {
            recommendations.push(
                "High memory usage variance detected. Consider implementing adaptive buffer sizing.".to_string()
            );
        }

        if recommendations.is_empty() {
            recommendations.push("Memory usage patterns are optimal.".to_string());
        }

        recommendations
    }

    /// Clear all monitoring history
    pub fn clear_history(&self) {
        self.usage_history.lock().unwrap().clear();
    }

    /// Get the total monitoring duration
    pub fn monitoring_duration(&self) -> Duration {
        Instant::now() - self.start_time
    }
}

/// Statistics about memory usage over a time period
#[derive(Debug, Clone, Default)]
pub struct MemoryUsageStats {
    /// Average memory used in bytes
    pub avg_memory_used: u64,
    /// Maximum memory used in bytes
    pub max_memory_used: u64,
    /// Minimum memory used in bytes
    pub min_memory_used: u64,
    /// Average efficiency percentage
    pub avg_efficiency: f64,
    /// Number of samples in the statistics
    pub sample_count: usize,
    /// Duration covered by the statistics in seconds
    pub duration_seconds: f64,
}

impl MemoryUsageStats {
    /// Calculate memory usage trend (positive = increasing, negative = decreasing)
    pub fn usage_trend(&self) -> f64 {
        if self.min_memory_used == 0 {
            return 0.0;
        }
        (self.max_memory_used as f64 - self.min_memory_used as f64) / self.min_memory_used as f64
    }

    /// Check if memory usage is stable (low variance)
    pub fn is_stable(&self) -> bool {
        self.usage_trend().abs() < 0.2 // Less than 20% variance
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_memory_monitor_creation() {
        let monitor = MemoryMonitor::with_default_thresholds();
        assert_eq!(monitor.monitoring_duration().as_secs(), 0);
    }

    #[test]
    fn test_memory_usage_recording() {
        let monitor = MemoryMonitor::with_default_thresholds();

        monitor.record_usage(1024 * 1024, 10, 85.0);
        monitor.record_usage(2048 * 1024, 15, 80.0);

        let stats = monitor.get_usage_stats(Duration::from_secs(1));
        assert_eq!(stats.sample_count, 2);
        assert!(stats.avg_memory_used > 0);
    }

    #[test]
    fn test_memory_alerts() {
        let thresholds = MemoryThresholds {
            warning_threshold: 50.0,
            critical_threshold: 80.0,
            max_memory_bytes: 1024 * 1024, // 1MB
            min_efficiency: 70.0,
        };

        let monitor = MemoryMonitor::new(thresholds);

        // Normal usage
        let alert = monitor.check_memory_status(256 * 1024, 85.0); // 25% usage
        assert_eq!(alert, MemoryAlert::Normal);

        // Warning level
        let alert = monitor.check_memory_status(600 * 1024, 85.0); // 60% usage
        assert!(matches!(alert, MemoryAlert::Warning { .. }));

        // Critical level
        let alert = monitor.check_memory_status(900 * 1024, 85.0); // 90% usage
        assert!(matches!(alert, MemoryAlert::Critical { .. }));

        // Low efficiency
        let alert = monitor.check_memory_status(256 * 1024, 50.0); // Low efficiency
        assert!(matches!(alert, MemoryAlert::LowEfficiency { .. }));
    }

    #[test]
    fn test_optimization_recommendations() {
        let monitor = MemoryMonitor::with_default_thresholds();

        // Record some usage patterns
        monitor.record_usage(1024 * 1024, 10, 50.0); // Low efficiency
        thread::sleep(Duration::from_millis(10));
        monitor.record_usage(2048 * 1024, 15, 55.0);

        let recommendations = monitor.get_optimization_recommendations();
        assert!(!recommendations.is_empty());
        assert!(recommendations.iter().any(|r| r.contains("efficiency")));
    }
}
