//! Compression performance benchmarking
//!
//! This module provides comprehensive benchmarking tools for comparing
//! compression algorithms and measuring performance characteristics.

use crate::adaptive::DataType;
use crate::algorithms::{Algorithm, AlgorithmImpl};
use ferrocp_types::{CompressionAlgorithm, Error, Result};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{debug, info};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Benchmark result for a single compression test
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BenchmarkResult {
    /// Algorithm used
    pub algorithm: CompressionAlgorithm,
    /// Compression level used
    pub level: u8,
    /// Data type tested
    pub data_type: DataType,
    /// Original data size
    pub original_size: u64,
    /// Compressed data size
    pub compressed_size: u64,
    /// Compression time
    pub compression_time: Duration,
    /// Decompression time
    pub decompression_time: Duration,
    /// Compression ratio (compressed/original)
    pub compression_ratio: f64,
    /// Compression speed (bytes/second)
    pub compression_speed: f64,
    /// Decompression speed (bytes/second)
    pub decompression_speed: f64,
    /// Memory usage during compression (estimated)
    pub memory_usage: u64,
}

impl BenchmarkResult {
    /// Calculate compression efficiency score
    pub fn efficiency_score(&self) -> f64 {
        // Weighted score: 40% ratio, 30% compression speed, 30% decompression speed
        let ratio_score = 1.0 - self.compression_ratio;
        let comp_speed_score = (self.compression_speed / 10_000_000.0).min(1.0); // Normalize to 10MB/s
        let decomp_speed_score = (self.decompression_speed / 50_000_000.0).min(1.0); // Normalize to 50MB/s

        0.4 * ratio_score + 0.3 * comp_speed_score + 0.3 * decomp_speed_score
    }

    /// Calculate space savings in bytes
    pub fn space_savings(&self) -> u64 {
        self.original_size.saturating_sub(self.compressed_size)
    }

    /// Calculate space savings percentage
    pub fn space_savings_percent(&self) -> f64 {
        if self.original_size == 0 {
            0.0
        } else {
            (self.space_savings() as f64 / self.original_size as f64) * 100.0
        }
    }
}

/// Comprehensive benchmark suite for compression algorithms
#[derive(Debug)]
pub struct CompressionBenchmark {
    /// Test data sets for different data types
    test_data: HashMap<DataType, Vec<u8>>,
    /// Benchmark results
    results: Vec<BenchmarkResult>,
}

impl CompressionBenchmark {
    /// Create a new benchmark suite
    pub fn new() -> Self {
        let mut benchmark = Self {
            test_data: HashMap::new(),
            results: Vec::new(),
        };

        benchmark.generate_test_data();
        benchmark
    }

    /// Add custom test data for a specific data type
    pub fn add_test_data(&mut self, data_type: DataType, data: Vec<u8>) {
        self.test_data.insert(data_type, data);
    }

    /// Run benchmarks for all algorithms and data types
    pub async fn run_all_benchmarks(&mut self) -> Result<()> {
        info!("Starting comprehensive compression benchmarks");

        let algorithms = AlgorithmImpl::all_algorithms();

        for algorithm in algorithms {
            for (data_type, test_data) in &self.test_data {
                if test_data.is_empty() {
                    continue;
                }

                let algo_impl = AlgorithmImpl::create(algorithm);
                let levels = self.get_test_levels(algorithm);

                for level in levels {
                    match self
                        .benchmark_algorithm(&*algo_impl, *data_type, test_data, level)
                        .await
                    {
                        Ok(result) => {
                            debug!(
                                "Benchmark completed: {:?} level {} on {:?} - ratio: {:.3}, speed: {:.0} MB/s",
                                algorithm, level, data_type, result.compression_ratio,
                                result.compression_speed / 1_000_000.0
                            );
                            self.results.push(result);
                        }
                        Err(e) => {
                            debug!(
                                "Benchmark failed for {:?} level {} on {:?}: {}",
                                algorithm, level, data_type, e
                            );
                        }
                    }
                }
            }
        }

        info!("Completed {} benchmark tests", self.results.len());
        Ok(())
    }

    /// Run benchmark for a specific algorithm and data type
    pub async fn benchmark_algorithm(
        &self,
        algorithm: &dyn Algorithm,
        data_type: DataType,
        test_data: &[u8],
        level: u8,
    ) -> Result<BenchmarkResult> {
        let original_size = test_data.len() as u64;

        // Compression benchmark
        let compression_start = Instant::now();
        let compressed_data = algorithm.compress(test_data, level)?;
        let compression_time = compression_start.elapsed();

        let compressed_size = compressed_data.len() as u64;
        let compression_ratio = compressed_size as f64 / original_size as f64;
        let compression_speed = original_size as f64 / compression_time.as_secs_f64();

        // Decompression benchmark
        let decompression_start = Instant::now();
        let decompressed_data = algorithm.decompress(&compressed_data)?;
        let decompression_time = decompression_start.elapsed();

        // Verify data integrity
        if decompressed_data != test_data {
            return Err(Error::compression(
                "Data integrity check failed after decompression",
            ));
        }

        let decompression_speed = compressed_size as f64 / decompression_time.as_secs_f64();

        // Estimate memory usage (simplified)
        let memory_usage = (original_size + compressed_size) * 2; // Rough estimate

        Ok(BenchmarkResult {
            algorithm: algorithm.algorithm_type(),
            level,
            data_type,
            original_size,
            compressed_size,
            compression_time,
            decompression_time,
            compression_ratio,
            compression_speed,
            decompression_speed,
            memory_usage,
        })
    }

    /// Get benchmark results
    pub fn results(&self) -> &[BenchmarkResult] {
        &self.results
    }

    /// Get best algorithm for specific criteria
    pub fn best_for_ratio(&self, data_type: Option<DataType>) -> Option<&BenchmarkResult> {
        self.filter_results(data_type).min_by(|a, b| {
            a.compression_ratio
                .partial_cmp(&b.compression_ratio)
                .unwrap()
        })
    }

    /// Get best algorithm for compression speed
    pub fn best_for_speed(&self, data_type: Option<DataType>) -> Option<&BenchmarkResult> {
        self.filter_results(data_type).max_by(|a, b| {
            a.compression_speed
                .partial_cmp(&b.compression_speed)
                .unwrap()
        })
    }

    /// Get best algorithm for overall efficiency
    pub fn best_for_efficiency(&self, data_type: Option<DataType>) -> Option<&BenchmarkResult> {
        self.filter_results(data_type).max_by(|a, b| {
            a.efficiency_score()
                .partial_cmp(&b.efficiency_score())
                .unwrap()
        })
    }

    /// Generate comparison report
    pub fn generate_report(&self) -> String {
        let mut report = String::new();
        report.push_str("# Compression Benchmark Report\n\n");

        // Summary statistics
        report.push_str("## Summary\n");
        report.push_str(&format!("Total tests: {}\n", self.results.len()));

        if let Some(best_ratio) = self.best_for_ratio(None) {
            report.push_str(&format!(
                "Best compression ratio: {:.3} ({:?} level {})\n",
                best_ratio.compression_ratio, best_ratio.algorithm, best_ratio.level
            ));
        }

        if let Some(best_speed) = self.best_for_speed(None) {
            report.push_str(&format!(
                "Best compression speed: {:.0} MB/s ({:?} level {})\n",
                best_speed.compression_speed / 1_000_000.0,
                best_speed.algorithm,
                best_speed.level
            ));
        }

        if let Some(best_efficiency) = self.best_for_efficiency(None) {
            report.push_str(&format!(
                "Best overall efficiency: {:.3} ({:?} level {})\n",
                best_efficiency.efficiency_score(),
                best_efficiency.algorithm,
                best_efficiency.level
            ));
        }

        // Per-algorithm analysis
        report.push_str("\n## Algorithm Analysis\n");
        let algorithms = AlgorithmImpl::all_algorithms();

        for algorithm in algorithms {
            let algo_results: Vec<_> = self
                .results
                .iter()
                .filter(|r| r.algorithm == algorithm)
                .collect();

            if algo_results.is_empty() {
                continue;
            }

            report.push_str(&format!("\n### {:?}\n", algorithm));

            let avg_ratio: f64 = algo_results
                .iter()
                .map(|r| r.compression_ratio)
                .sum::<f64>()
                / algo_results.len() as f64;
            let avg_speed: f64 = algo_results
                .iter()
                .map(|r| r.compression_speed)
                .sum::<f64>()
                / algo_results.len() as f64;

            report.push_str(&format!("Average compression ratio: {:.3}\n", avg_ratio));
            report.push_str(&format!(
                "Average compression speed: {:.0} MB/s\n",
                avg_speed / 1_000_000.0
            ));

            if let Some(best) = algo_results.iter().max_by(|a, b| {
                a.efficiency_score()
                    .partial_cmp(&b.efficiency_score())
                    .unwrap()
            }) {
                report.push_str(&format!(
                    "Best efficiency score: {:.3} (level {})\n",
                    best.efficiency_score(),
                    best.level
                ));
            }
        }

        report
    }

    /// Clear all results
    pub fn clear_results(&mut self) {
        self.results.clear();
    }

    /// Filter results by data type
    fn filter_results(
        &self,
        data_type: Option<DataType>,
    ) -> impl Iterator<Item = &BenchmarkResult> {
        self.results
            .iter()
            .filter(move |r| data_type.map_or(true, |dt| r.data_type == dt))
    }

    /// Get test levels for an algorithm
    fn get_test_levels(&self, algorithm: CompressionAlgorithm) -> Vec<u8> {
        let algo_impl = AlgorithmImpl::create(algorithm);
        let max_level = algo_impl.max_level();
        let default_level = algo_impl.default_level();

        if max_level <= 1 {
            vec![default_level]
        } else {
            // Test default, min, max, and a few intermediate levels
            let mut levels = vec![1, default_level, max_level];
            if max_level > 5 {
                levels.push(max_level / 2);
            }
            levels.sort_unstable();
            levels.dedup();
            levels
        }
    }

    /// Generate test data for different data types
    fn generate_test_data(&mut self) {
        // Text data
        let text_data = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(200);
        self.test_data
            .insert(DataType::Text, text_data.into_bytes());

        // Binary data (pseudo-random)
        let mut binary_data = Vec::with_capacity(10240);
        for i in 0..10240 {
            binary_data.push((i * 17 + 42) as u8);
        }
        self.test_data.insert(DataType::Binary, binary_data);

        // Highly repetitive data
        let repetitive_data = vec![0x42u8; 8192];
        self.test_data.insert(DataType::Log, repetitive_data);

        // JSON-like structured data
        let json_data =
            r#"{"name":"test","value":123,"items":["a","b","c"],"nested":{"key":"value"}}"#
                .repeat(100);
        self.test_data
            .insert(DataType::Config, json_data.into_bytes());

        // Mixed data
        let mut mixed_data = Vec::new();
        mixed_data.extend_from_slice(b"Header: Some text content\n");
        mixed_data.extend_from_slice(&[0x00, 0x01, 0x02, 0x03, 0xFF, 0xFE, 0xFD, 0xFC]);
        mixed_data.extend_from_slice(b"More text after binary data\n");
        mixed_data = mixed_data.repeat(50);
        self.test_data.insert(DataType::Unknown, mixed_data);
    }
}

impl Default for CompressionBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_benchmark_creation() {
        let benchmark = CompressionBenchmark::new();
        assert!(!benchmark.test_data.is_empty());
    }

    #[tokio::test]
    async fn test_single_algorithm_benchmark() {
        let benchmark = CompressionBenchmark::new();
        let algorithm = AlgorithmImpl::create(CompressionAlgorithm::Zstd);
        let test_data = b"Hello, world! This is test data for benchmarking.".repeat(10);

        let result = benchmark
            .benchmark_algorithm(&*algorithm, DataType::Text, &test_data, 3)
            .await
            .unwrap();

        assert_eq!(result.algorithm, CompressionAlgorithm::Zstd);
        assert_eq!(result.level, 3);
        assert_eq!(result.data_type, DataType::Text);
        assert_eq!(result.original_size, test_data.len() as u64);
        assert!(result.compression_time > Duration::ZERO);
        assert!(result.decompression_time > Duration::ZERO);
        assert!(result.compression_speed > 0.0);
        assert!(result.decompression_speed > 0.0);
    }

    #[tokio::test]
    async fn test_benchmark_results_analysis() {
        let mut benchmark = CompressionBenchmark::new();

        // Add a simple test result
        let result = BenchmarkResult {
            algorithm: CompressionAlgorithm::Zstd,
            level: 3,
            data_type: DataType::Text,
            original_size: 1000,
            compressed_size: 400,
            compression_time: Duration::from_millis(10),
            decompression_time: Duration::from_millis(5),
            compression_ratio: 0.4,
            compression_speed: 100_000.0,
            decompression_speed: 200_000.0,
            memory_usage: 2800,
        };

        benchmark.results.push(result);

        assert_eq!(benchmark.results().len(), 1);
        assert!(benchmark.best_for_ratio(None).is_some());
        assert!(benchmark.best_for_speed(None).is_some());
        assert!(benchmark.best_for_efficiency(None).is_some());
    }

    #[test]
    fn test_benchmark_result_calculations() {
        let result = BenchmarkResult {
            algorithm: CompressionAlgorithm::Zstd,
            level: 3,
            data_type: DataType::Text,
            original_size: 1000,
            compressed_size: 400,
            compression_time: Duration::from_millis(10),
            decompression_time: Duration::from_millis(5),
            compression_ratio: 0.4,
            compression_speed: 100_000.0,
            decompression_speed: 200_000.0,
            memory_usage: 2800,
        };

        assert_eq!(result.space_savings(), 600);
        assert_eq!(result.space_savings_percent(), 60.0);
        assert!(result.efficiency_score() > 0.0);
        assert!(result.efficiency_score() <= 1.0);
    }

    #[test]
    fn test_report_generation() {
        let mut benchmark = CompressionBenchmark::new();

        // Add a test result
        let result = BenchmarkResult {
            algorithm: CompressionAlgorithm::Zstd,
            level: 3,
            data_type: DataType::Text,
            original_size: 1000,
            compressed_size: 400,
            compression_time: Duration::from_millis(10),
            decompression_time: Duration::from_millis(5),
            compression_ratio: 0.4,
            compression_speed: 100_000.0,
            decompression_speed: 200_000.0,
            memory_usage: 2800,
        };

        benchmark.results.push(result);

        let report = benchmark.generate_report();
        assert!(report.contains("Compression Benchmark Report"));
        assert!(report.contains("Total tests: 1"));
        assert!(report.contains("Zstd"));
    }
}
