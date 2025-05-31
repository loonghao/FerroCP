# FerroCP Performance Testing Suite

This crate contains comprehensive integration tests and performance benchmarks for FerroCP.

## Overview

The testing suite is designed to:

- **Integration Tests**: Verify that all FerroCP components work together correctly in real-world scenarios
- **Performance Benchmarks**: Measure and track performance characteristics of core operations
- **Regression Detection**: Identify performance regressions and improvements over time
- **Unified Benchmark Framework**: Modular and extensible benchmark management system

## Unified Benchmark Framework

The comprehensive benchmark suite provides a standardized framework for managing performance benchmarks across all FerroCP modules.

### Key Features

- **Modular Design**: Easy to add new benchmark modules
- **Baseline Management**: Store and compare against performance baselines
- **Configuration System**: Flexible benchmark configuration
- **Criterion Integration**: Built on the industry-standard Criterion benchmarking library
- **JSON Export/Import**: Save and load benchmark configurations and baselines

## Running Tests

### Integration Tests

Run all integration tests:

```bash
cargo test -p ferrocp-tests --test integration_tests
```

Run specific integration test:

```bash
cargo test -p ferrocp-tests --test integration_tests test_basic_file_copy
```

### Performance Benchmarks

Run all benchmarks:

```bash
cargo bench -p ferrocp-tests
```

Run specific benchmark:

```bash
cargo bench -p ferrocp-tests benchmark_compression
```

Run the unified benchmark suite:

```bash
cargo bench -p ferrocp-tests --bench comprehensive_benchmark_suite
```

### Creating Custom Benchmark Modules

To create a new benchmark module, implement the `ModuleBenchmark` trait:

```rust
use ferrocp_tests::comprehensive_benchmark_suite::{ModuleBenchmark, BaselineMetrics};
use criterion::Criterion;

pub struct MyBenchmark;

impl ModuleBenchmark for MyBenchmark {
    fn name(&self) -> &str {
        "my_module"
    }

    fn run_benchmarks(&self, c: &mut Criterion) {
        c.bench_function("my_benchmark", |b| {
            b.iter(|| {
                // Your benchmark code here
            })
        });
    }

    fn get_baseline_metrics(&self) -> Vec<BaselineMetrics> {
        // Return expected performance baselines
        vec![]
    }
}
```

### Benchmark Configuration

Create a custom configuration:

```rust
use ferrocp_tests::comprehensive_benchmark_suite::{BenchmarkConfig, BenchmarkSuite};
use std::time::Duration;

let config = BenchmarkConfig {
    sample_size: 100,
    measurement_time: Duration::from_secs(10),
    warm_up_time: Duration::from_secs(3),
    enable_profiling: false,
    output_dir: "target/criterion".to_string(),
    baseline_file: Some("baseline.json".to_string()),
};

let mut suite = BenchmarkSuite::with_config(config);
```

### Using the Benchmark Script

The project includes a comprehensive benchmarking script:

```bash
# Run full benchmarks
./scripts/run-benchmarks.sh

# Run quick benchmarks (reduced sample size)
./scripts/run-benchmarks.sh --quick

# Compare with baseline
./scripts/run-benchmarks.sh --baseline baseline_results.json

# Enable profiling
./scripts/run-benchmarks.sh --profile
```

## Test Structure

### Integration Tests

Located in `tests/integration_tests.rs`:

- **test_basic_file_copy**: Tests basic file copying functionality
- **test_compression_integration**: Tests compression and decompression operations

### Performance Benchmarks

Located in `benches/performance_benchmarks.rs`:

- **Compression Benchmarks**: Measures compression and decompression performance for different data sizes

## Benchmark Results

Current performance characteristics (example results):

### Compression Performance

| Data Size | Compression Throughput | Decompression Throughput |
|-----------|----------------------|--------------------------|
| 64KB      | ~36 MiB/s           | ~32 MiB/s               |
| 1MB       | ~375 MiB/s          | ~384 MiB/s              |

*Note: Results may vary based on hardware and system configuration*

## CI/CD Integration

### GitHub Actions

The project includes automated performance testing:

- **rust-benchmarks.yml**: Runs performance benchmarks on multiple platforms
- **Performance Regression Check**: Automatically detects significant performance changes in PRs
- **Trend Analysis**: Tracks performance trends over time

### Benchmark Artifacts

Benchmark results are automatically uploaded as artifacts:

- JSON results for programmatic analysis
- HTML reports for visual inspection
- Performance summaries with key metrics

## Performance Monitoring

### Continuous Monitoring

- Daily automated benchmark runs
- Performance trend tracking
- Regression alerts for significant changes (>5%)

### Key Metrics

The benchmarks track several important metrics:

- **Throughput**: Data processed per second (MiB/s)
- **Latency**: Time to complete operations
- **Memory Usage**: Peak memory consumption
- **CPU Utilization**: Processor usage patterns

## Development Guidelines

### Adding New Tests

1. **Integration Tests**: Add new test functions to `tests/integration_tests.rs`
2. **Benchmarks**: Add new benchmark functions to `benches/performance_benchmarks.rs`
3. **Update CI**: Modify GitHub Actions workflows if needed

### Test Data

- Use `create_test_file()` for file-based tests
- Use `create_compressible_data()` for compression tests
- Keep test data sizes reasonable for CI environments

### Performance Expectations

- **File Copy**: Should achieve >100 MiB/s for large files
- **Compression**: Should achieve >50 MiB/s for typical data
- **Memory Usage**: Should remain bounded and predictable

## Troubleshooting

### Common Issues

1. **Benchmark Timeouts**: Increase `max_wait_seconds` in benchmark configuration
2. **Memory Issues**: Reduce test data sizes or enable streaming mode
3. **Platform Differences**: Some benchmarks may show different results on different platforms

### Debug Mode

Run benchmarks with debug output:

```bash
RUST_LOG=debug cargo bench -p ferrocp-tests
```

### Profiling

Enable profiling for detailed performance analysis:

```bash
cargo bench -p ferrocp-tests --features profiling
```

## Contributing

When contributing performance-related changes:

1. Run benchmarks before and after changes
2. Document any significant performance impacts
3. Update benchmark expectations if needed
4. Consider adding new benchmarks for new features

## Dependencies

The test suite uses:

- **criterion**: For performance benchmarking
- **tokio-test**: For async test utilities
- **tempfile**: For temporary file management
- **proptest**: For property-based testing (future)

## Future Enhancements

Planned improvements:

- [ ] Property-based testing with proptest
- [ ] Fuzzing integration
- [ ] Memory profiling benchmarks
- [ ] Network performance tests
- [ ] Cross-platform performance comparison
- [ ] Automated performance regression detection
- [ ] Performance optimization recommendations
