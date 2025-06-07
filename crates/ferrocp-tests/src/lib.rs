//! FerroCP Performance Testing Suite
//!
//! This crate provides comprehensive integration tests and performance benchmarks
//! for the FerroCP project, including a unified benchmark management framework.

#![warn(missing_docs)]
#![warn(clippy::all)]

/// Comprehensive benchmark suite module
///
/// This module provides a unified framework for managing and executing
/// performance benchmarks across all FerroCP components.
pub mod comprehensive_benchmark_suite {
    pub use crate::benchmark_framework::*;
}

/// Concurrency testing utilities
///
/// This module provides utilities for testing concurrent performance,
/// resource contention, and thread scalability.
pub mod concurrency_utils;

/// Unified test utilities
///
/// This module provides common utilities used across all test files
/// to ensure consistency and reduce code duplication.
pub mod test_utils;

/// Internal benchmark framework implementation
mod benchmark_framework {
    use criterion::Criterion;
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use std::time::Duration;

    /// Configuration for the benchmark suite
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct BenchmarkConfig {
        /// Sample size for benchmarks
        pub sample_size: usize,
        /// Measurement time per benchmark
        pub measurement_time: Duration,
        /// Warm-up time before measurements
        pub warm_up_time: Duration,
        /// Whether to enable profiling
        pub enable_profiling: bool,
        /// Output directory for results
        pub output_dir: String,
        /// Baseline comparison file
        pub baseline_file: Option<String>,
    }

    impl Default for BenchmarkConfig {
        fn default() -> Self {
            Self {
                sample_size: 100,
                measurement_time: Duration::from_secs(10),
                warm_up_time: Duration::from_secs(3),
                enable_profiling: false,
                output_dir: "target/criterion".to_string(),
                baseline_file: None,
            }
        }
    }

    /// Baseline performance metrics for comparison
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct BaselineMetrics {
        /// Module name
        pub module_name: String,
        /// Benchmark name
        pub benchmark_name: String,
        /// Expected throughput (operations per second)
        pub expected_throughput: f64,
        /// Expected latency (nanoseconds)
        pub expected_latency: f64,
        /// Memory usage (bytes)
        pub memory_usage: u64,
        /// Timestamp when baseline was recorded
        pub timestamp: String,
    }

    /// Trait for modular benchmark implementations
    pub trait ModuleBenchmark {
        /// Get the name of this benchmark module
        fn name(&self) -> &str;

        /// Run all benchmarks for this module
        fn run_benchmarks(&self, c: &mut Criterion);

        /// Get baseline metrics for this module
        fn get_baseline_metrics(&self) -> Vec<BaselineMetrics>;

        /// Get module-specific configuration
        fn get_config(&self) -> BenchmarkConfig {
            BenchmarkConfig::default()
        }
    }

    /// Main benchmark suite manager
    pub struct BenchmarkSuite {
        /// Configuration for the suite
        config: BenchmarkConfig,
        /// Registered benchmark modules
        modules: Vec<Box<dyn ModuleBenchmark>>,
        /// Baseline metrics storage
        baselines: HashMap<String, Vec<BaselineMetrics>>,
    }

    impl BenchmarkSuite {
        /// Create a new benchmark suite with default configuration
        pub fn new() -> Self {
            Self {
                config: BenchmarkConfig::default(),
                modules: Vec::new(),
                baselines: HashMap::new(),
            }
        }

        /// Create a new benchmark suite with custom configuration
        pub fn with_config(config: BenchmarkConfig) -> Self {
            Self {
                config,
                modules: Vec::new(),
                baselines: HashMap::new(),
            }
        }

        /// Register a benchmark module
        pub fn register_module(&mut self, module: Box<dyn ModuleBenchmark>) {
            let module_name = module.name().to_string();
            let baselines = module.get_baseline_metrics();
            self.baselines.insert(module_name, baselines);
            self.modules.push(module);
        }

        /// Run all registered benchmarks
        pub fn run_all_benchmarks(&self, c: &mut Criterion) {
            // Configure Criterion with our settings
            self.configure_criterion(c);

            // Run benchmarks for each module
            for module in &self.modules {
                println!("Running benchmarks for module: {}", module.name());
                module.run_benchmarks(c);
            }
        }

        /// Configure Criterion with benchmark settings
        fn configure_criterion(&self, c: &mut Criterion) {
            *c = std::mem::take(c)
                .sample_size(self.config.sample_size)
                .measurement_time(self.config.measurement_time)
                .warm_up_time(self.config.warm_up_time);

            if self.config.enable_profiling {
                // Note: Profiling support can be added by enabling pprof feature
                println!("Profiling enabled (requires pprof feature)");
            }
        }

        /// Get baseline metrics for a specific module
        pub fn get_module_baselines(&self, module_name: &str) -> Option<&Vec<BaselineMetrics>> {
            self.baselines.get(module_name)
        }

        /// Save baseline metrics to file
        pub fn save_baselines(&self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
            let json = serde_json::to_string_pretty(&self.baselines)?;
            std::fs::write(file_path, json)?;
            Ok(())
        }

        /// Load baseline metrics from file
        pub fn load_baselines(
            &mut self,
            file_path: &str,
        ) -> Result<(), Box<dyn std::error::Error>> {
            let content = std::fs::read_to_string(file_path)?;
            self.baselines = serde_json::from_str(&content)?;
            Ok(())
        }
    }

    impl Default for BenchmarkSuite {
        fn default() -> Self {
            Self::new()
        }
    }

    /// Configure Criterion with standard settings for FerroCP benchmarks
    pub fn configure_criterion() -> Criterion {
        Criterion::default()
            .sample_size(100)
            .measurement_time(Duration::from_secs(10))
            .warm_up_time(Duration::from_secs(3))
            .with_plots()
    }
}

#[cfg(test)]
mod tests {
    use super::comprehensive_benchmark_suite::*;
    use criterion::Criterion;

    struct TestBenchmark;

    impl ModuleBenchmark for TestBenchmark {
        fn name(&self) -> &str {
            "test_module"
        }

        fn run_benchmarks(&self, c: &mut Criterion) {
            c.bench_function("test_benchmark", |b| b.iter(|| criterion::black_box(42)));
        }

        fn get_baseline_metrics(&self) -> Vec<BaselineMetrics> {
            vec![BaselineMetrics {
                module_name: "test_module".to_string(),
                benchmark_name: "test_benchmark".to_string(),
                expected_throughput: 1000.0,
                expected_latency: 1_000_000.0,
                memory_usage: 0,
                timestamp: "1735574400".to_string(),
            }]
        }
    }

    #[test]
    fn test_benchmark_suite_creation() {
        let suite = BenchmarkSuite::new();
        assert!(suite.get_module_baselines("nonexistent").is_none());
    }

    #[test]
    fn test_module_registration() {
        let mut suite = BenchmarkSuite::new();
        suite.register_module(Box::new(TestBenchmark));

        let baselines = suite.get_module_baselines("test_module");
        assert!(baselines.is_some());
        assert_eq!(baselines.unwrap().len(), 1);
    }

    #[test]
    fn test_baseline_serialization() {
        let mut suite = BenchmarkSuite::new();
        suite.register_module(Box::new(TestBenchmark));

        // Test saving baselines
        let temp_file = "test_baselines.json";
        suite.save_baselines(temp_file).unwrap();

        // Test loading baselines
        let mut new_suite = BenchmarkSuite::new();
        new_suite.load_baselines(temp_file).unwrap();

        let baselines = new_suite.get_module_baselines("test_module");
        assert!(baselines.is_some());

        // Cleanup
        std::fs::remove_file(temp_file).ok();
    }
}
