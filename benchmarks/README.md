# Performance Benchmarks

This directory contains comprehensive performance benchmarks for py-eacopy, including both local benchmarking tools and continuous performance monitoring with CodSpeed.

## Quick Start

### Prerequisites

Install **hyperfine** (command-line benchmarking tool):

```bash
# Ubuntu/Debian
sudo apt install hyperfine

# macOS
brew install hyperfine

# Windows (using Chocolatey)
choco install hyperfine

# Or download from: https://github.com/sharkdp/hyperfine/releases
```

### Install Dependencies

```bash
# Install benchmark dependencies using uv (recommended)
uv sync --group testing

# Or using pip
pip install -e ".[testing]"
```

### Run Basic Benchmarks

```bash
# Run all benchmarks
uv run nox -s benchmark

# Run comparison benchmarks only
uv run nox -s benchmark_compare

# Run CodSpeed benchmarks
uv run nox -s codspeed

# Run specific benchmark groups
uv run pytest benchmarks/ --benchmark-only --benchmark-group=file_copy_sizes
```

## Benchmark Categories

### 1. Core Performance Tests (`test_performance.py`)

- **File Copy Sizes**: Test copying files of different sizes (1KB to 100MB)
- **Thread Count Performance**: Compare performance with different thread counts
- **Compression Levels**: Test impact of different compression levels
- **Buffer Sizes**: Optimize buffer size for different scenarios
- **Directory Copy**: Test directory tree copying performance
- **Memory Usage**: Monitor memory consumption during operations
- **Zero-Copy Performance**: Test zero-copy optimization effectiveness

### 2. Comparison Tests (`test_comparison.py`)

- **vs Python shutil**: Compare against standard library
- **vs Windows robocopy**: Compare against system tools (Windows only)
- **Throughput Analysis**: Detailed throughput measurements

### 3. CodSpeed Benchmarks (`test_codspeed.py`)

Optimized benchmarks for continuous performance monitoring:
- **Deterministic**: Use fixed patterns for reproducible results
- **Focused**: Each benchmark tests a specific performance aspect
- **CI-Optimized**: Designed for stable CI execution
- **Regression Detection**: Automatically detect performance regressions

```bash
# Run CodSpeed benchmarks locally
uv run nox -s codspeed

# Run all CodSpeed benchmarks
uv run nox -s codspeed_all

# Test CodSpeed integration
uv run pytest benchmarks/test_codspeed.py --codspeed -v
```

### 4. Rust Benchmarks (`../benches/copy_benchmarks.rs`)

Low-level Rust benchmarks using the Criterion framework:

```bash
# Run Rust benchmarks
cargo bench

# Generate HTML reports
cargo bench -- --output-format html
```

## Profiling and Analysis

### Generate Test Data

```bash
# Generate test files for benchmarks
uv run python benchmarks/data/generate_test_data.py --output-dir benchmarks/data/test_files
```

### Performance Profiling

```bash
# Use nox profiling session (recommended)
uv run nox -s profile

# Or run profiling tools directly:
# py-spy record -o profile.svg -- python your_script.py
# memory_profiler: python -m memory_profiler your_script.py
# cProfile: python -m cProfile -o profile.prof your_script.py
```

### Flamegraph Generation

For Rust code profiling:

```bash
# Install flamegraph
cargo install flamegraph

# Generate flamegraph (Linux/macOS)
cargo flamegraph --bench copy_benchmarks

# On Windows, use perf alternative
cargo bench --bench copy_benchmarks
```

## Benchmark Results

Results are saved in `benchmarks/results/`:

- `benchmark.json`: pytest-benchmark results
- `*.speedscope`: py-spy profiles (view at https://www.speedscope.app/)
- `*.prof`: cProfile results
- `*.txt`: Memory profiling results

### Viewing Results

```bash
# View benchmark history
pytest-benchmark compare benchmarks/results/benchmark.json

# Generate HTML report
pytest benchmarks/ --benchmark-only --benchmark-autosave --benchmark-save-data
```

## Performance Targets

### Throughput Targets

- **Small files (< 1MB)**: > 100 MB/s
- **Large files (> 10MB)**: > 500 MB/s
- **Directory operations**: > 50 files/s

### Memory Usage Targets

- **Peak memory**: < 100MB for files up to 1GB
- **Memory efficiency**: < 1% of file size for large files

### Comparison Targets

- **vs shutil**: 2-5x faster for large files
- **vs robocopy**: Competitive performance (within 20%)

## Continuous Performance Monitoring

### CI Integration

Benchmarks run automatically in CI to detect performance regressions:

```yaml
# .github/workflows/benchmark.yml
- name: Run benchmarks
  run: uv run nox -s benchmark

- name: Compare with baseline
  run: uv run pytest-benchmark compare baseline.json current.json
```

### Performance Regression Detection

```bash
# Save baseline
uv run pytest benchmarks/ --benchmark-only --benchmark-save=baseline

# Compare against baseline
uv run pytest benchmarks/ --benchmark-only --benchmark-compare=baseline
```

## Optimization Guidelines

### For File Copy Performance

1. **Buffer Size**: Use 8MB+ for large files, 64KB for small files
2. **Thread Count**: Optimal is usually 2-4x CPU cores
3. **Compression**: Use level 1-3 for network transfers
4. **Zero-Copy**: Enable for local file operations

### For Directory Copy Performance

1. **Batch Small Files**: Group small files for better efficiency
2. **Parallel Processing**: Use multiple threads for large directories
3. **Skip Empty Directories**: Optimize directory traversal

### For Memory Efficiency

1. **Streaming**: Use streaming for files > 100MB
2. **Buffer Management**: Reuse buffers to reduce allocations
3. **Async I/O**: Use async operations to reduce blocking

## Troubleshooting

### Common Issues

1. **Permission Errors**: Run with appropriate permissions
2. **Disk Space**: Ensure sufficient space for test files
3. **Antivirus**: May interfere with file operations

### Debug Performance Issues

```bash
# Enable debug logging
RUST_LOG=debug uv run nox -s profile

# Use verbose benchmarks
uv run pytest benchmarks/ --benchmark-only -v

# Check system resources
uv run python -c "import psutil; print(psutil.virtual_memory(), psutil.disk_usage('.'))"
```

## Contributing

When adding new benchmarks:

1. Follow existing naming conventions
2. Add appropriate markers (`@pytest.mark.benchmark`)
3. Include comparison with standard tools
4. Document expected performance characteristics
5. Update this README with new benchmark descriptions

### Benchmark Best Practices

- Use realistic test data sizes
- Include both best-case and worst-case scenarios
- Test on different hardware configurations
- Measure multiple metrics (time, memory, throughput)
- Compare against relevant baselines
