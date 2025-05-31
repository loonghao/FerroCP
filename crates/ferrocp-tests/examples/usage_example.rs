//! Example usage of the FerroCP Benchmark Suite
//!
//! This example demonstrates how to create custom benchmark modules
//! and integrate them with the unified benchmark framework.

use criterion::{black_box, Criterion};
use ferrocp_tests::comprehensive_benchmark_suite::{
    BaselineMetrics, BenchmarkConfig, BenchmarkSuite, ModuleBenchmark,
};
use std::time::Duration;

/// Example custom benchmark module
pub struct CustomBenchmark {
    name: String,
}

impl CustomBenchmark {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

impl ModuleBenchmark for CustomBenchmark {
    fn name(&self) -> &str {
        &self.name
    }

    fn run_benchmarks(&self, c: &mut Criterion) {
        // Example benchmark: simple computation
        c.bench_function(&format!("{}_computation", self.name), |b| {
            b.iter(|| {
                let result = (0..1000).map(|i| i * i).sum::<i32>();
                black_box(result)
            })
        });

        // Example benchmark: memory allocation
        c.bench_function(&format!("{}_allocation", self.name), |b| {
            b.iter(|| {
                let vec: Vec<u8> = vec![0; 1024];
                black_box(vec)
            })
        });
    }

    fn get_baseline_metrics(&self) -> Vec<BaselineMetrics> {
        vec![
            BaselineMetrics {
                module_name: self.name.clone(),
                benchmark_name: format!("{}_computation", self.name),
                expected_throughput: 10000.0,
                expected_latency: 100_000.0, // 0.1ms
                memory_usage: 0,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    .to_string(),
            },
            BaselineMetrics {
                module_name: self.name.clone(),
                benchmark_name: format!("{}_allocation", self.name),
                expected_throughput: 1000.0,
                expected_latency: 1_000_000.0, // 1ms
                memory_usage: 1024,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    .to_string(),
            },
        ]
    }

    fn get_config(&self) -> BenchmarkConfig {
        BenchmarkConfig {
            sample_size: 50, // Smaller sample size for faster testing
            measurement_time: Duration::from_secs(5),
            warm_up_time: Duration::from_secs(1),
            enable_profiling: false,
            output_dir: "target/criterion".to_string(),
            baseline_file: Some("baseline.json".to_string()),
        }
    }
}

fn main() {
    println!("FerroCP Benchmark Suite Usage Example");
    
    // Create a custom benchmark configuration
    let config = BenchmarkConfig {
        sample_size: 50,
        measurement_time: Duration::from_secs(5),
        warm_up_time: Duration::from_secs(2),
        enable_profiling: false,
        output_dir: "target/custom_benchmarks".to_string(),
        baseline_file: Some("custom_baseline.json".to_string()),
    };

    // Create benchmark suite with custom configuration
    let mut suite = BenchmarkSuite::with_config(config);

    // Register custom benchmark modules
    suite.register_module(Box::new(CustomBenchmark::new("example_module".to_string())));
    suite.register_module(Box::new(CustomBenchmark::new("another_module".to_string())));

    // Save baseline metrics
    if let Err(e) = suite.save_baselines("examples/custom_baseline.json") {
        eprintln!("Failed to save baselines: {}", e);
    } else {
        println!("Baseline metrics saved to examples/custom_baseline.json");
    }

    // Load baseline metrics (example)
    let mut load_suite = BenchmarkSuite::new();
    if let Err(e) = load_suite.load_baselines("examples/baseline_metrics.json") {
        eprintln!("Failed to load baselines: {}", e);
    } else {
        println!("Baseline metrics loaded from examples/baseline_metrics.json");
        
        // Display loaded baselines
        if let Some(baselines) = load_suite.get_module_baselines("copy_engine") {
            println!("Copy engine baselines:");
            for baseline in baselines {
                println!("  - {}: {} ops/s", baseline.benchmark_name, baseline.expected_throughput);
            }
        }
    }

    println!("Example completed successfully!");
}
