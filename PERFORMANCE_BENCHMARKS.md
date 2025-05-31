# FerroCP Performance Benchmarking Suite

## Overview

This document describes the comprehensive performance benchmarking suite implemented for FerroCP. The suite provides automated performance testing, regression detection, and continuous monitoring capabilities.

## 🎯 Objectives

- **Performance Measurement**: Accurately measure core operation performance
- **Regression Detection**: Automatically detect performance regressions in CI/CD
- **Trend Analysis**: Track performance trends over time
- **Cross-Platform Testing**: Ensure consistent performance across platforms
- **Optimization Guidance**: Provide data-driven optimization recommendations

## 📊 Components

### 1. Integration Tests (`crates/ferrocp-tests/tests/`)

**Purpose**: Verify functional correctness and basic performance expectations

**Tests Included**:
- `test_basic_file_copy`: Basic file copying functionality
- `test_compression_integration`: Compression/decompression operations

**Key Features**:
- Real-world scenario testing
- Error handling verification
- Cross-component integration validation

### 2. Performance Benchmarks (`crates/ferrocp-tests/benches/`)

**Purpose**: Detailed performance measurement and analysis

**Benchmarks Included**:
- **Compression Performance**: Measures compression and decompression throughput
  - Data sizes: 64KB, 1MB
  - Metrics: Throughput (MiB/s), latency
  - Algorithms: Zstd (primary)

**Current Results** (example):
```
compression/compress/65536     [35.7 MiB/s - 37.0 MiB/s]
compression/decompress/65536   [29.9 MiB/s - 34.5 MiB/s]
compression/compress/1048576   [361.9 MiB/s - 387.6 MiB/s]
compression/decompress/1048576 [372.5 MiB/s - 395.9 MiB/s]
```

### 3. Automation Scripts

**Bash Script** (`scripts/run-benchmarks.sh`):
- Unix/Linux/macOS support
- Comprehensive benchmark execution
- Result analysis and reporting

**Windows Batch Script** (`scripts/run-benchmarks.bat`):
- Windows support
- Same functionality as bash script
- Native Windows command integration

**Features**:
- Quick mode for CI environments
- Baseline comparison
- Profiling support
- Automated report generation

### 4. CI/CD Integration

**GitHub Actions Workflow** (`.github/workflows/rust-benchmarks.yml`):
- Multi-platform testing (Ubuntu, Windows, macOS)
- Automated benchmark execution
- Performance regression detection
- Artifact collection and storage

**Workflow Features**:
- Daily scheduled runs
- PR-triggered performance checks
- Trend analysis
- Automated alerts for regressions

## 🚀 Usage

### Running Tests Locally

```bash
# Run all integration tests
cargo test -p ferrocp-tests --test integration_tests

# Run all benchmarks
cargo bench -p ferrocp-tests

# Run with custom script (Unix/Linux/macOS)
./scripts/run-benchmarks.sh --quick

# Run with custom script (Windows)
scripts\run-benchmarks.bat --quick
```

### CI/CD Integration

The benchmarks automatically run:
- **On every PR**: Performance regression check
- **Daily**: Full benchmark suite with trend analysis
- **On demand**: Manual workflow dispatch

### Interpreting Results

**Throughput Metrics**:
- Higher values indicate better performance
- Measured in MiB/s (Mebibytes per second)
- Compare against baseline for regression detection

**Latency Metrics**:
- Lower values indicate better performance
- Measured in milliseconds
- Important for interactive applications

## 📈 Performance Characteristics

### Current Performance Profile

**Compression (Zstd)**:
- Small files (64KB): ~36 MiB/s compression, ~32 MiB/s decompression
- Large files (1MB): ~375 MiB/s compression, ~384 MiB/s decompression
- Performance scales well with data size

**File Operations**:
- Basic file copy: Functional and tested
- Error handling: Robust and verified
- Cross-platform: Consistent behavior

### Performance Expectations

**Target Performance**:
- File copy: >100 MiB/s for large files
- Compression: >50 MiB/s for typical workloads
- Memory usage: Bounded and predictable
- Latency: <10ms for small operations

## 🔧 Configuration

### Benchmark Parameters

**Standard Mode**:
- Measurement time: 30 seconds
- Sample size: 100 iterations
- Full statistical analysis

**Quick Mode**:
- Measurement time: 10 seconds
- Sample size: 50 iterations
- Suitable for CI environments

### Customization

**Environment Variables**:
- `RUST_LOG=debug`: Enable debug logging
- Custom benchmark parameters via command line

**Configuration Files**:
- Criterion configuration in `Cargo.toml`
- GitHub Actions workflow parameters

## 📋 Monitoring and Alerts

### Regression Detection

**Automatic Detection**:
- >5% performance degradation triggers alerts
- Statistical significance testing
- Baseline comparison with historical data

**Manual Review**:
- Detailed HTML reports generated
- JSON data for programmatic analysis
- Trend visualization (planned)

### Continuous Monitoring

**Daily Benchmarks**:
- Full performance suite execution
- Historical data collection
- Trend analysis and reporting

**Performance Tracking**:
- Key metrics dashboard (planned)
- Performance regression history
- Optimization opportunity identification

## 🛠️ Development Guidelines

### Adding New Benchmarks

1. **Create benchmark function** in `benches/performance_benchmarks.rs`
2. **Add to criterion group** in the same file
3. **Update CI workflow** if needed
4. **Document expected performance** in this file

### Performance Testing Best Practices

1. **Use realistic data sizes** for benchmarks
2. **Include both small and large data sets**
3. **Test error conditions** in integration tests
4. **Document performance expectations**
5. **Monitor for regressions** in CI/CD

### Optimization Workflow

1. **Establish baseline** with current benchmarks
2. **Implement optimization**
3. **Run benchmarks** to measure impact
4. **Compare results** with baseline
5. **Document improvements** and update expectations

## 🔮 Future Enhancements

### Planned Features

- [ ] **Property-based testing** with proptest
- [ ] **Fuzzing integration** for robustness testing
- [ ] **Memory profiling** benchmarks
- [ ] **Network performance** tests
- [ ] **Cross-platform comparison** analysis
- [ ] **Automated optimization** recommendations
- [ ] **Performance dashboard** with visualizations
- [ ] **Historical trend analysis**
- [ ] **Benchmark result database**
- [ ] **Performance regression bisection**

### Advanced Benchmarking

- [ ] **Micro-benchmarks** for individual functions
- [ ] **End-to-end scenarios** for real-world usage
- [ ] **Stress testing** under high load
- [ ] **Concurrency benchmarks** for parallel operations
- [ ] **Hardware-specific optimizations**

## 📚 References

- [Criterion.rs Documentation](https://docs.rs/criterion/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [FerroCP Architecture Documentation](./ARCHITECTURE.md)

## 🤝 Contributing

When contributing performance-related changes:

1. **Run benchmarks** before and after changes
2. **Document performance impact** in PR description
3. **Update benchmark expectations** if needed
4. **Add new benchmarks** for new features
5. **Follow performance testing guidelines**

For questions or suggestions about the performance testing suite, please open an issue or discussion in the repository.
