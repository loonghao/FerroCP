//! Concurrency testing utilities for FerroCP
//!
//! This module provides utility functions and structures for testing
//! concurrent performance, resource contention, and thread scalability.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Thread performance metrics collector
#[derive(Debug, Clone)]
pub struct ThreadMetrics {
    pub thread_id: usize,
    pub operations_completed: Arc<AtomicUsize>,
    pub bytes_processed: Arc<AtomicUsize>,
    pub errors_encountered: Arc<AtomicUsize>,
    pub start_time: Instant,
    pub last_operation_time: Arc<std::sync::Mutex<Instant>>,
}

impl ThreadMetrics {
    pub fn new(thread_id: usize) -> Self {
        let now = Instant::now();
        Self {
            thread_id,
            operations_completed: Arc::new(AtomicUsize::new(0)),
            bytes_processed: Arc::new(AtomicUsize::new(0)),
            errors_encountered: Arc::new(AtomicUsize::new(0)),
            start_time: now,
            last_operation_time: Arc::new(std::sync::Mutex::new(now)),
        }
    }

    pub fn record_operation(&self, bytes: usize) {
        self.operations_completed.fetch_add(1, Ordering::Relaxed);
        self.bytes_processed.fetch_add(bytes, Ordering::Relaxed);
        if let Ok(mut last_time) = self.last_operation_time.lock() {
            *last_time = Instant::now();
        }
    }

    pub fn record_error(&self) {
        self.errors_encountered.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_throughput_mbps(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            let mb_processed = self.bytes_processed.load(Ordering::Relaxed) as f64 / (1024.0 * 1024.0);
            mb_processed / elapsed
        } else {
            0.0
        }
    }

    pub fn get_operations_per_second(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            self.operations_completed.load(Ordering::Relaxed) as f64 / elapsed
        } else {
            0.0
        }
    }

    pub fn get_error_rate(&self) -> f64 {
        let total_ops = self.operations_completed.load(Ordering::Relaxed);
        let errors = self.errors_encountered.load(Ordering::Relaxed);
        if total_ops > 0 {
            errors as f64 / total_ops as f64
        } else {
            0.0
        }
    }
}

/// Aggregated performance metrics for multiple threads
#[derive(Debug)]
pub struct AggregatedMetrics {
    pub thread_metrics: Vec<ThreadMetrics>,
    pub total_duration: Duration,
}

impl AggregatedMetrics {
    pub fn new(thread_metrics: Vec<ThreadMetrics>, total_duration: Duration) -> Self {
        Self {
            thread_metrics,
            total_duration,
        }
    }

    pub fn total_operations(&self) -> usize {
        self.thread_metrics
            .iter()
            .map(|m| m.operations_completed.load(Ordering::Relaxed))
            .sum()
    }

    pub fn total_bytes_processed(&self) -> usize {
        self.thread_metrics
            .iter()
            .map(|m| m.bytes_processed.load(Ordering::Relaxed))
            .sum()
    }

    pub fn total_errors(&self) -> usize {
        self.thread_metrics
            .iter()
            .map(|m| m.errors_encountered.load(Ordering::Relaxed))
            .sum()
    }

    pub fn average_throughput_mbps(&self) -> f64 {
        let total_mb = self.total_bytes_processed() as f64 / (1024.0 * 1024.0);
        total_mb / self.total_duration.as_secs_f64()
    }

    pub fn thread_efficiency(&self) -> f64 {
        if self.thread_metrics.is_empty() {
            return 0.0;
        }

        let single_thread_throughput = self.thread_metrics[0].get_throughput_mbps();
        let actual_throughput = self.average_throughput_mbps();
        let expected_throughput = single_thread_throughput * self.thread_metrics.len() as f64;

        if expected_throughput > 0.0 {
            actual_throughput / expected_throughput
        } else {
            0.0
        }
    }

    pub fn scalability_factor(&self) -> f64 {
        if self.thread_metrics.is_empty() {
            return 0.0;
        }

        let operations_per_thread = self.total_operations() as f64 / self.thread_metrics.len() as f64;
        operations_per_thread
    }

    pub fn load_balance_coefficient(&self) -> f64 {
        if self.thread_metrics.is_empty() {
            return 1.0;
        }

        let operations: Vec<f64> = self.thread_metrics
            .iter()
            .map(|m| m.operations_completed.load(Ordering::Relaxed) as f64)
            .collect();

        let mean = operations.iter().sum::<f64>() / operations.len() as f64;
        let variance = operations
            .iter()
            .map(|&x| (x - mean).powi(2))
            .sum::<f64>() / operations.len() as f64;

        let std_dev = variance.sqrt();
        
        // Coefficient of variation (lower is better for load balance)
        if mean > 0.0 {
            1.0 - (std_dev / mean).min(1.0)
        } else {
            1.0
        }
    }
}

/// Resource contention detector
#[derive(Debug)]
pub struct ContentionDetector {
    pub lock_wait_times: Arc<std::sync::Mutex<Vec<Duration>>>,
    pub contention_events: Arc<AtomicUsize>,
    pub total_lock_attempts: Arc<AtomicUsize>,
}

impl ContentionDetector {
    pub fn new() -> Self {
        Self {
            lock_wait_times: Arc::new(std::sync::Mutex::new(Vec::new())),
            contention_events: Arc::new(AtomicUsize::new(0)),
            total_lock_attempts: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn record_lock_attempt(&self, wait_time: Duration) {
        self.total_lock_attempts.fetch_add(1, Ordering::Relaxed);
        
        if let Ok(mut times) = self.lock_wait_times.lock() {
            times.push(wait_time);
        }

        // Consider it contention if wait time is significant
        if wait_time > Duration::from_micros(100) {
            self.contention_events.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn contention_ratio(&self) -> f64 {
        let total = self.total_lock_attempts.load(Ordering::Relaxed);
        let contentions = self.contention_events.load(Ordering::Relaxed);
        
        if total > 0 {
            contentions as f64 / total as f64
        } else {
            0.0
        }
    }

    pub fn average_wait_time(&self) -> Duration {
        if let Ok(times) = self.lock_wait_times.lock() {
            if !times.is_empty() {
                let total_nanos: u64 = times.iter().map(|d| d.as_nanos() as u64).sum();
                Duration::from_nanos(total_nanos / times.len() as u64)
            } else {
                Duration::ZERO
            }
        } else {
            Duration::ZERO
        }
    }

    pub fn max_wait_time(&self) -> Duration {
        if let Ok(times) = self.lock_wait_times.lock() {
            times.iter().max().copied().unwrap_or(Duration::ZERO)
        } else {
            Duration::ZERO
        }
    }
}

/// Memory usage tracker for concurrent operations
#[derive(Debug)]
pub struct MemoryTracker {
    pub allocations: Arc<AtomicUsize>,
    pub deallocations: Arc<AtomicUsize>,
    pub peak_usage: Arc<AtomicUsize>,
    pub current_usage: Arc<AtomicUsize>,
}

impl MemoryTracker {
    pub fn new() -> Self {
        Self {
            allocations: Arc::new(AtomicUsize::new(0)),
            deallocations: Arc::new(AtomicUsize::new(0)),
            peak_usage: Arc::new(AtomicUsize::new(0)),
            current_usage: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn record_allocation(&self, size: usize) {
        self.allocations.fetch_add(1, Ordering::Relaxed);
        let current = self.current_usage.fetch_add(size, Ordering::Relaxed) + size;
        
        // Update peak usage
        let mut peak = self.peak_usage.load(Ordering::Relaxed);
        while current > peak {
            match self.peak_usage.compare_exchange_weak(peak, current, Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => break,
                Err(x) => peak = x,
            }
        }
    }

    pub fn record_deallocation(&self, size: usize) {
        self.deallocations.fetch_add(1, Ordering::Relaxed);
        self.current_usage.fetch_sub(size, Ordering::Relaxed);
    }

    pub fn get_stats(&self) -> (usize, usize, usize, usize) {
        (
            self.allocations.load(Ordering::Relaxed),
            self.deallocations.load(Ordering::Relaxed),
            self.peak_usage.load(Ordering::Relaxed),
            self.current_usage.load(Ordering::Relaxed),
        )
    }

    pub fn memory_efficiency(&self) -> f64 {
        let (allocs, deallocs, peak, current) = self.get_stats();
        
        if allocs > 0 {
            // Efficiency based on allocation/deallocation balance and memory reuse
            let balance_factor = if allocs > 0 { deallocs as f64 / allocs as f64 } else { 0.0 };
            let reuse_factor = if peak > 0 { 1.0 - (current as f64 / peak as f64) } else { 1.0 };
            
            (balance_factor + reuse_factor) / 2.0
        } else {
            1.0
        }
    }
}

/// Performance analysis utilities
pub struct PerformanceAnalyzer;

impl PerformanceAnalyzer {
    /// Calculate optimal thread count based on performance metrics
    pub fn calculate_optimal_thread_count(metrics: &[AggregatedMetrics]) -> usize {
        if metrics.is_empty() {
            return 1;
        }

        let mut best_efficiency = 0.0;
        let mut optimal_threads = 1;

        for metric in metrics {
            let efficiency = metric.thread_efficiency();
            if efficiency > best_efficiency {
                best_efficiency = efficiency;
                optimal_threads = metric.thread_metrics.len();
            }
        }

        optimal_threads
    }

    /// Identify performance bottlenecks
    pub fn identify_bottlenecks(metrics: &AggregatedMetrics) -> Vec<String> {
        let mut bottlenecks = Vec::new();

        // Check thread efficiency
        if metrics.thread_efficiency() < 0.7 {
            bottlenecks.push("Low thread efficiency - possible resource contention".to_string());
        }

        // Check load balance
        if metrics.load_balance_coefficient() < 0.8 {
            bottlenecks.push("Poor load balancing across threads".to_string());
        }

        // Check error rate
        let error_rate = metrics.total_errors() as f64 / metrics.total_operations() as f64;
        if error_rate > 0.01 {
            bottlenecks.push(format!("High error rate: {:.2}%", error_rate * 100.0));
        }

        bottlenecks
    }

    /// Generate performance report
    pub fn generate_report(metrics: &AggregatedMetrics) -> String {
        format!(
            "Performance Report:\n\
             - Threads: {}\n\
             - Total Operations: {}\n\
             - Total Bytes: {} MB\n\
             - Average Throughput: {:.2} MB/s\n\
             - Thread Efficiency: {:.1}%\n\
             - Load Balance: {:.1}%\n\
             - Scalability Factor: {:.2}\n\
             - Error Rate: {:.3}%",
            metrics.thread_metrics.len(),
            metrics.total_operations(),
            metrics.total_bytes_processed() / (1024 * 1024),
            metrics.average_throughput_mbps(),
            metrics.thread_efficiency() * 100.0,
            metrics.load_balance_coefficient() * 100.0,
            metrics.scalability_factor(),
            (metrics.total_errors() as f64 / metrics.total_operations() as f64) * 100.0
        )
    }
}
