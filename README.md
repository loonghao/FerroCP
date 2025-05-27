# FerroCP

<div align="center">

[![PyPI version](https://badge.fury.io/py/ferrocp.svg)](https://badge.fury.io/py/ferrocp)
[![Build Status](https://github.com/loonghao/FerroCP/workflows/Build%20and%20Release/badge.svg)](https://github.com/loonghao/FerroCP/actions)
[![Documentation Status](https://readthedocs.org/projects/ferrocp/badge/?version=latest)](https://ferrocp.readthedocs.io/en/latest/?badge=latest)
[![Python Version](https://img.shields.io/pypi/pyversions/ferrocp.svg)](https://pypi.org/project/ferrocp/)
[![License](https://img.shields.io/github/license/loonghao/FerroCP.svg)](https://github.com/loonghao/FerroCP/blob/main/LICENSE)
[![Downloads](https://static.pepy.tech/badge/ferrocp)](https://pepy.tech/project/ferrocp)
[![Ruff](https://img.shields.io/badge/ruff-enabled-brightgreen)](https://github.com/astral-sh/ruff)
[![CodSpeed](https://img.shields.io/badge/CodSpeed-performance%20monitoring-blue)](https://codspeed.io/loonghao/FerroCP)

**🚀 High-Performance File Copying Tool**
*Built with Rust for Maximum Speed and Reliability*

[中文文档](README_zh.md) | [Documentation](https://ferrocp.readthedocs.io/) | [Benchmarks](benchmarks/README.md)

</div>

**FerroCP** (Iron Copy) is a high-performance, cross-platform file copying tool written in Rust with Python bindings. Designed from the ground up for speed and reliability, FerroCP delivers **2-5x faster** file operations compared to standard Python tools while maintaining a familiar, easy-to-use API.

## ✨ Key Features

### 🚀 **Performance First**
- **2-5x faster** than Python's `shutil` for large files
- **Native Rust implementation** with zero-copy optimizations
- **Multi-threaded operations** with automatic CPU detection
- **Memory efficient** with configurable buffer sizes

### 🔧 **Developer Friendly**
- **Drop-in replacement** for Python's `shutil` module
- **Familiar API** - no learning curve required
- **Type hints** and comprehensive documentation
- **Modern tooling** with maturin and uv support

### 🌍 **Cross-Platform Excellence**
- **Windows, Linux, macOS** native support
- **Consistent performance** across all platforms
- **Platform-specific optimizations** automatically applied
- **Unicode filename support** with proper encoding handling

### 📊 **Production Ready**
- **Comprehensive testing** with 95%+ code coverage
- **Continuous performance monitoring** with CodSpeed
- **Memory leak detection** and resource management
- **Error handling** with detailed diagnostics

## 📦 Installation

### Quick Start (Recommended)

```bash
# Install from PyPI (when available)
pip install ferrocp

# Or install with uv (faster)
uv add ferrocp
```

### Development Installation

```bash
# Clone the repository
git clone https://github.com/loonghao/FerroCP.git
cd FerroCP

# Install with uv (recommended)
uv sync --group all
uv run maturin develop --release

# Or with traditional pip
pip install -e ".[dev,test,benchmark]"
```

### Requirements

- **Python 3.9+** (3.11+ recommended for best performance)
- **Rust toolchain** (automatically installed by maturin if needed)
- **64-bit system** (Windows, Linux, macOS)

## 🚀 Quick Start

### Basic Usage (Drop-in Replacement)

```python
import ferrocp

# Replace shutil.copy with ferrocp.copy - same API, better performance!
ferrocp.copy("source.txt", "destination.txt")

# Copy with metadata preservation (like shutil.copy2)
ferrocp.copy2("source.txt", "destination.txt")

# Copy entire directory trees (like shutil.copytree)
ferrocp.copytree("source_dir", "destination_dir")
```

### Advanced Configuration

```python
import ferrocp

# Create a configured copier instance
copier = ferrocp.EACopy(
    thread_count=8,           # Use 8 threads for parallel operations
    buffer_size=8*1024*1024,  # 8MB buffer for large files
    compression_level=3,      # Compression for network transfers
    verify_integrity=True     # Verify file integrity after copy
)

# High-performance file copying
copier.copy_file("large_dataset.zip", "backup/dataset.zip")

# Batch operations with progress tracking
files_to_copy = [
    ("data1.bin", "backup/data1.bin"),
    ("data2.bin", "backup/data2.bin"),
    ("data3.bin", "backup/data3.bin"),
]

for src, dst in files_to_copy:
    result = copier.copy_file(src, dst)
    print(f"Copied {result.bytes_copied} bytes in {result.duration:.2f}s")
```

### Command Line Interface

```bash
# Basic file copy
ferrocp copy source.txt destination.txt

# Copy with options
ferrocp copy --threads 8 --verbose large_file.zip backup/

# Directory synchronization
ferrocp copy --mirror source_dir/ destination_dir/

# Show help
ferrocp --help
```

## 📊 Performance Comparison

FerroCP consistently outperforms standard Python file operations:

| Operation | File Size | FerroCP | shutil | Speedup |
|-----------|-----------|---------|--------|---------|
| **Single File** | 1 KB | 87.8 μs | 290 μs | **3.3x faster** |
| **Single File** | 1 MB | 227.5 μs | 1.9 ms | **8.5x faster** |
| **Single File** | 10 MB | 2.4 ms | 12.5 ms | **5.2x faster** |
| **Single File** | 100 MB | 24 ms | 125 ms | **5.2x faster** |
| **Directory Tree** | 1000 files | 1.2 s | 4.8 s | **4.0x faster** |

### Real-World Scenarios

```python
import time
import ferrocp
import shutil

# Benchmark: Copy a 1GB file
start = time.time()
ferrocp.copy("large_file.bin", "backup.bin")
ferrocp_time = time.time() - start

start = time.time()
shutil.copy("large_file.bin", "backup_shutil.bin")
shutil_time = time.time() - start

print(f"FerroCP: {ferrocp_time:.2f}s")
print(f"shutil:  {shutil_time:.2f}s")
print(f"Speedup: {shutil_time/ferrocp_time:.1f}x faster")
```

*Benchmarks run on Windows 11, Intel i7-12700K, NVMe SSD. Results may vary by system.*

## 🔬 Development

### Prerequisites

- **Python 3.9+** (3.11+ recommended)
- **Rust toolchain** (install from [rustup.rs](https://rustup.rs/))
- **uv** (recommended, install from [uv docs](https://docs.astral.sh/uv/))

### Development Setup

```bash
# Clone the repository
git clone https://github.com/loonghao/FerroCP.git
cd FerroCP

# Install all development dependencies
uv sync --group all

# Or install specific dependency groups
uv sync --group testing    # Testing tools (pytest, coverage)
uv sync --group linting    # Code quality (ruff, mypy)
uv sync --group docs       # Documentation (sphinx, mkdocs)
uv sync --group benchmark  # Performance testing tools
```

### Building from Source

This project uses **maturin** to build Rust extensions:

```bash
# Development build with Python bindings (fast, for testing)
uv run maturin develop --features python

# Release build with Python bindings (optimized)
uv run maturin develop --release --features python

# Build wheel packages for Python
uv run maturin build --release --features python

# Build standalone CLI tool (no Python dependencies)
cargo build --release --bin ferrocp
```

**Note**: The CLI tool (`ferrocp.exe`) is built without Python dependencies and can run independently. The Python module requires the `python` feature to be enabled.

### Testing

```bash
# Run tests
uv run nox -s test

# Run tests for specific Python version
uv run nox -s test-3.11

# Run linting
uv run nox -s lint

# Fix linting issues automatically
uv run nox -s lint_fix
```

### Documentation

```bash
# Build documentation
uv run nox -s docs

# Serve documentation with live reloading
uv run nox -s docs_serve
```

## Performance Benchmarks

ferrocp includes comprehensive performance benchmarks and continuous performance monitoring:

### Prerequisites for Benchmarking

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

### Local Benchmarking

```bash
# Install benchmark dependencies
uv sync --group testing

# Run all benchmarks
uv run nox -s benchmark

# Run comparison benchmarks vs standard tools
uv run nox -s benchmark_compare

# Generate performance profiles
uv run nox -s profile
```

### Profile-Guided Optimization (PGO) Builds

For maximum performance, use PGO-optimized builds:

```bash
# Build with PGO optimization (takes longer but ~10-15% faster)
uv run nox -s build_pgo

# Regular optimized build
uv run nox -s build

# Verify build works correctly
uv run nox -s verify_build
```

### Continuous Performance Monitoring with CodSpeed

This project uses [CodSpeed](https://codspeed.io/) for continuous performance monitoring:

```bash
# Run CodSpeed benchmarks locally
uv run nox -s codspeed

# Run all CodSpeed benchmarks
uv run nox -s codspeed_all
```

CodSpeed automatically:
- 🔍 Detects performance regressions in pull requests
- 📊 Provides detailed performance analysis and visualizations
- 📈 Tracks performance trends over time
- ✅ Integrates seamlessly with our GitHub Actions CI
- 🚀 Uses PGO-optimized builds for accurate performance measurement

### Benchmark Results

Current performance targets:
- **Small files (< 1MB)**: > 100 MB/s
- **Large files (> 10MB)**: > 500 MB/s
- **vs shutil**: 2-5x faster for large files
- **vs robocopy**: Competitive performance (within 20%)
- **PGO builds**: Additional 10-15% performance improvement

See [benchmarks/README.md](benchmarks/README.md) for detailed benchmarking documentation.

## Dependencies

### Core Dependencies
- [Rust](https://www.rust-lang.org/) - Systems programming language for high-performance extensions
- [PyO3](https://pyo3.rs/) - Rust bindings for Python
- [maturin](https://github.com/PyO3/maturin) - Build tool for Rust-based Python extensions

### Development Dependencies
- [uv](https://docs.astral.sh/uv/) - Fast Python package manager
- [nox](https://nox.thea.codes/) - Flexible test automation
- [ruff](https://github.com/astral-sh/ruff) - Fast Python linter and formatter
- [pytest](https://pytest.org/) - Testing framework
- [CodSpeed](https://codspeed.io/) - Continuous performance monitoring

### Benchmarking Tools
- [hyperfine](https://github.com/sharkdp/hyperfine) - Command-line benchmarking tool
- [pytest-benchmark](https://pytest-benchmark.readthedocs.io/) - Python benchmarking plugin

## License

BSD-3-Clause

## CI/CD Configuration

This project uses GitHub Actions for CI/CD with the following workflows:

- **Build and Test**: Tests the package on multiple Python versions and operating systems.
- **Release**: Builds and publishes wheels to PyPI when a new release is created.
- **Documentation**: Builds and deploys documentation to GitHub Pages.

The release workflow uses cibuildwheel to build platform-specific wheels with the Rust extensions properly compiled for each platform.

### Release Process

To create a new release:

1. Update the version in `pyproject.toml` and `python/ferrocp/__version__.py`
2. Update the `CHANGELOG.md` with the new version and changes
3. Commit and push the changes
4. Create a new tag with the version number (e.g., `0.1.0`)
5. Push the tag to GitHub

```bash
# Example release process
git add pyproject.toml python/ferrocp/__version__.py CHANGELOG.md
git commit -m "Release 0.1.0"
git tag 0.1.0
git push && git push --tags
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request
