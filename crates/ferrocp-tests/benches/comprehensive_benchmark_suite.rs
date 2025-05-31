//! Comprehensive Benchmark Suite for FerroCP
//!
//! This module provides a unified framework for managing and executing
//! all performance benchmarks across the FerroCP project.

use criterion::{criterion_group, criterion_main, Criterion};
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
            // *c = std::mem::take(c).with_profiler(...);
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
    pub fn load_baselines(&mut self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
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

/// Copy engine benchmark module
pub struct CopyEngineBenchmark;

impl ModuleBenchmark for CopyEngineBenchmark {
    fn name(&self) -> &str {
        "copy_engine"
    }
    
    fn run_benchmarks(&self, c: &mut Criterion) {
        use ferrocp_io::{BufferedCopyEngine, CopyEngine};
        use tempfile::TempDir;
        use tokio::runtime::Runtime;
        
        let rt = Runtime::new().unwrap();
        
        // File copy benchmark
        c.bench_function("copy_engine_1mb", |b| {
            b.iter(|| {
                let temp_dir = TempDir::new().unwrap();
                let source = temp_dir.path().join("source.dat");
                let dest = temp_dir.path().join("dest.dat");
                
                // Create 1MB test file
                let data = vec![0u8; 1024 * 1024];
                std::fs::write(&source, data).unwrap();
                
                rt.block_on(async {
                    let mut engine = BufferedCopyEngine::new();
                    let result = engine.copy_file(&source, &dest).await.unwrap();
                    criterion::black_box(result);
                })
            })
        });
    }
    
    fn get_baseline_metrics(&self) -> Vec<BaselineMetrics> {
        vec![
            BaselineMetrics {
                module_name: "copy_engine".to_string(),
                benchmark_name: "copy_engine_1mb".to_string(),
                expected_throughput: 100.0, // MB/s
                expected_latency: 10_000_000.0, // 10ms
                memory_usage: 1024 * 1024, // 1MB
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    .to_string(),
            },
        ]
    }
}

/// Main benchmark function that sets up and runs all benchmarks
fn comprehensive_benchmarks(c: &mut Criterion) {
    let mut suite = BenchmarkSuite::new();
    
    // Register all benchmark modules
    suite.register_module(Box::new(CopyEngineBenchmark));
    
    // Run all benchmarks
    suite.run_all_benchmarks(c);
}

criterion_group!(benches, comprehensive_benchmarks);
criterion_main!(benches);
