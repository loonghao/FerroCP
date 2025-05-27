# CodSpeed Integration Setup

This document explains how to set up CodSpeed continuous performance monitoring for the py-eacopy project.

## What is CodSpeed?

CodSpeed is a professional continuous performance monitoring platform that:
- Automatically detects performance regressions in pull requests
- Provides detailed performance analysis and visualizations
- Tracks performance trends over time
- Integrates seamlessly with GitHub Actions

## Setup Instructions

### 1. Repository Setup

1. **Enable CodSpeed for the repository:**
   - Go to [CodSpeed Settings](https://codspeed.io/settings)
   - Install the CodSpeed GitHub App
   - Select the py-eacopy repository
   - Enable monitoring for the repository

2. **Add CodSpeed Token:**
   - Copy the `CODSPEED_TOKEN` from repository settings
   - Add it as a GitHub repository secret named `CODSPEED_TOKEN`

### 2. Local Development

Install CodSpeed dependencies:

```bash
# Install benchmark dependencies including CodSpeed
pip install -e ".[benchmark]"

# Run CodSpeed benchmarks locally (for testing)
make codspeed

# Run all CodSpeed benchmarks
make codspeed-all

# Using nox directly
uvx nox -s codspeed
uvx nox -s codspeed_all
```

### 3. Benchmark Structure

#### Core Benchmarks (`benchmarks/test_codspeed.py`)

These benchmarks are specifically optimized for CodSpeed:

- **File Copy Benchmarks**: Small (1KB), Medium (1MB), Large (10MB)
- **Compression Benchmarks**: Different compression levels
- **Threading Benchmarks**: Single vs multi-threaded performance
- **Directory Operations**: Small and medium directory copying
- **Buffer Optimization**: Different buffer sizes
- **Comparison Baselines**: Against shutil for regression detection

#### Benchmark Design Principles

1. **Deterministic**: Use fixed seeds and patterns for reproducible results
2. **Focused**: Each benchmark tests a specific performance aspect
3. **Stable**: Avoid external dependencies that could affect timing
4. **Meaningful**: Test real-world usage patterns

### 4. CI Integration

The CodSpeed workflow (`.github/workflows/codspeed.yml`) includes:

#### Main Job (`benchmarks`)
- Runs on all PRs and main branch pushes
- Executes core CodSpeed benchmarks
- Provides PR comments with performance analysis

#### Comprehensive Job (`comprehensive-benchmarks`)
- Runs only on main branch pushes
- Executes all benchmarks for complete analysis
- Avoids overwhelming PR checks

#### Parallel Job (`parallel-benchmarks`)
- Available for manual triggering
- Demonstrates sharded benchmark execution
- Useful for large benchmark suites

### 5. Performance Metrics

CodSpeed tracks several key metrics:

#### Primary Metrics
- **Execution Time**: Wall-clock time for operations
- **Throughput**: MB/s for file operations
- **CPU Usage**: Processor utilization
- **Memory Usage**: Peak memory consumption

#### Regression Detection
- **Threshold**: 5% performance degradation triggers alerts
- **Statistical Significance**: Uses statistical analysis to filter noise
- **Trend Analysis**: Long-term performance trend tracking

### 6. Best Practices

#### Writing CodSpeed Benchmarks

```python
import pytest

@pytest.mark.benchmark
def test_core_operation():
    """Benchmark a core operation - keep it simple and focused."""
    # Setup (not timed)
    data = create_test_data(1024)
    
    # The actual operation being benchmarked
    result = py_eacopy.copy(source, dest)
    
    # Verification (not timed)
    assert result.success
```

#### Benchmark Naming
- Use descriptive names: `test_copy_large_file_codspeed`
- Include size/configuration: `test_copy_with_compression_level_6`
- Group related benchmarks: `test_copy_*`, `test_directory_*`

#### Data Management
- Use deterministic test data for reproducible results
- Keep test data sizes reasonable for CI performance
- Clean up resources properly

### 7. Interpreting Results

#### PR Comments
CodSpeed automatically comments on PRs with:
- Performance comparison vs base branch
- Regression/improvement detection
- Links to detailed analysis

#### Dashboard
The CodSpeed dashboard provides:
- Historical performance trends
- Detailed benchmark results
- Performance regression analysis
- Comparison across different commits

### 8. Troubleshooting

#### Common Issues

1. **Benchmarks not running:**
   - Check that `pytest-codspeed` is installed
   - Verify `@pytest.mark.benchmark` markers are present
   - Ensure GitHub token is correctly configured

2. **Inconsistent results:**
   - Use deterministic test data
   - Avoid external dependencies in benchmarks
   - Check for resource contention

3. **Missing performance data:**
   - Verify CodSpeed action is running
   - Check GitHub Actions logs for errors
   - Ensure repository is enabled in CodSpeed settings

#### Debug Commands

```bash
# Test benchmarks locally
pytest benchmarks/test_codspeed.py --codspeed -v

# Check benchmark discovery
pytest benchmarks/test_codspeed.py --collect-only

# Run with verbose output
uvx nox -s codspeed -- -v -s
```

### 9. Advanced Configuration

#### Custom Benchmark Groups

```python
# Group benchmarks by functionality
@pytest.mark.benchmark(group="file_operations")
def test_file_copy():
    pass

@pytest.mark.benchmark(group="directory_operations")
def test_directory_copy():
    pass
```

#### Parameterized Benchmarks

```python
@pytest.mark.benchmark
@pytest.mark.parametrize("size", [1024, 1024*1024, 10*1024*1024])
def test_copy_various_sizes(size):
    # Benchmark different file sizes
    pass
```

#### Environment Variables

```yaml
# In GitHub Actions
- name: Run benchmarks
  uses: CodSpeedHQ/action@v3
  env:
    RUST_LOG: info
    PY_EACOPY_BUFFER_SIZE: 1048576
  with:
    run: uv run pytest benchmarks/test_codspeed.py --codspeed
    token: ${{ secrets.CODSPEED_TOKEN }}
```

### 10. Integration with Development Workflow

#### Pre-commit Checks
```bash
# Add to pre-commit hooks
make codspeed
```

#### Release Process
1. Run comprehensive benchmarks before release
2. Review performance trends in CodSpeed dashboard
3. Document any significant performance changes

#### Performance Goals
- Maintain or improve performance with each release
- Target specific throughput goals (e.g., >500 MB/s for large files)
- Monitor memory usage and CPU efficiency

## Resources

- [CodSpeed Documentation](https://docs.codspeed.io/)
- [pytest-codspeed Plugin](https://pypi.org/project/pytest-codspeed/)
- [CodSpeed GitHub Action](https://github.com/CodSpeedHQ/action)
- [Example: pydantic-core](https://codspeed.io/pydantic/pydantic-core)
