# FerroCP

<div align="center">

[![Build Status](https://github.com/loonghao/FerroCP/workflows/Tests/badge.svg)](https://github.com/loonghao/FerroCP/actions)
[![VFX Platform](https://img.shields.io/badge/VFX%20Platform-CY2025%20Compatible-brightgreen)](https://vfxplatform.com/)
[![Python Version](https://img.shields.io/pypi/pyversions/ferrocp.svg)](https://pypi.org/project/ferrocp/)
[![License](https://img.shields.io/github/license/loonghao/FerroCP.svg)](https://github.com/loonghao/FerroCP/blob/main/LICENSE)
[![Ruff](https://img.shields.io/badge/ruff-enabled-brightgreen)](https://github.com/astral-sh/ruff)
[![CodSpeed](https://img.shields.io/badge/CodSpeed-performance%20monitoring-blue)](https://codspeed.io/loonghao/FerroCP)
[![Multi-Platform](https://img.shields.io/badge/platforms-Linux%20%7C%20macOS%20%7C%20Windows-blue)](https://github.com/loonghao/FerroCP/actions)

**⚠️ WORK IN PROGRESS ⚠️**

**🚀 High-Performance File Copying Tool**
*Built with Rust for Maximum Speed and Reliability*

**This project is currently under active development and is not ready for production use.**

[中文文档](README_zh.md) | [Documentation](https://ferrocp.readthedocs.io/) | [Benchmarks](benchmarks/README.md)

</div>

**FerroCP** (Iron Copy) is a high-performance, cross-platform file copying tool written in Rust with Python bindings. Designed from the ground up for speed and reliability, FerroCP aims to deliver **2-5x faster** file operations compared to standard Python tools while maintaining a familiar, easy-to-use API.

## ✨ Planned Features

### 🚀 **Performance First** (In Development)
- **Target: 2-5x faster** than Python's `shutil` for large files
- **Native Rust implementation** with zero-copy optimizations
- **Multi-threaded operations** with automatic CPU detection
- **Memory efficient** with configurable buffer sizes

### 🔧 **Developer Friendly** (Planned)
- **Drop-in replacement** for Python's `shutil` module
- **Familiar API** - no learning curve required
- **Type hints** and comprehensive documentation
- **Modern tooling** with maturin and uv support

### 🌍 **Cross-Platform Excellence** (In Development)
- **Windows, Linux, macOS** native support
- **VFX Platform compatibility** - follows [VFX Reference Platform](https://vfxplatform.com/) standards
- **Multi-architecture support** - x86_64 and ARM64 (Apple Silicon)
- **Consistent performance** across all platforms
- **Platform-specific optimizations** automatically applied
- **Unicode filename support** with proper encoding handling

### 📊 **Development Status**
- **Work in Progress** - Core functionality being implemented
- **Testing framework** being established
- **Performance benchmarking** infrastructure in place
- **CI/CD pipeline** configured for future releases

## 📦 Installation

### ⚠️ Not Yet Available

**FerroCP is currently under development and not available for installation.**

When ready, it will be available via:

```bash
# Future PyPI installation (not yet available)
pip install ferrocp

# Or with uv (not yet available)
uv add ferrocp
```

### Development Installation (For Contributors)

```bash
# Clone the repository
git clone https://github.com/loonghao/FerroCP.git
cd FerroCP

# Install development dependencies
uv sync --group all
uv run maturin develop --release

# Note: Core functionality is still being implemented
```

### Requirements (When Available)

- **Python 3.9+** (3.11+ recommended for best performance)
- **Rust toolchain** (automatically installed by maturin if needed)
- **64-bit system** (Windows, Linux, macOS)

## 🚀 Planned API (Under Development)

### Basic Usage (Planned Drop-in Replacement)

```python
import ferrocp

# Planned API - Replace shutil.copy with ferrocp.copy
ferrocp.copy("source.txt", "destination.txt")

# Copy with metadata preservation (like shutil.copy2)
ferrocp.copy2("source.txt", "destination.txt")

# Copy entire directory trees (like shutil.copytree)
ferrocp.copytree("source_dir", "destination_dir")
```

### Advanced Configuration (Planned)

```python
import ferrocp

# Planned advanced API
copier = ferrocp.EACopy(
    thread_count=8,           # Use 8 threads for parallel operations
    buffer_size=8*1024*1024,  # 8MB buffer for large files
    compression_level=3,      # Compression for network transfers
    verify_integrity=True     # Verify file integrity after copy
)

# High-performance file copying (planned)
copier.copy_file("large_dataset.zip", "backup/dataset.zip")

# Batch operations with progress tracking (planned)
files_to_copy = [
    ("data1.bin", "backup/data1.bin"),
    ("data2.bin", "backup/data2.bin"),
    ("data3.bin", "backup/data3.bin"),
]

for src, dst in files_to_copy:
    result = copier.copy_file(src, dst)
    print(f"Copied {result.bytes_copied} bytes in {result.duration:.2f}s")
```

### Command Line Interface (Planned)

```bash
# Planned CLI interface
ferrocp copy source.txt destination.txt

# Copy with options (planned)
ferrocp copy --threads 8 --verbose large_file.zip backup/

# Directory synchronization (planned)
ferrocp copy --mirror source_dir/ destination_dir/

# Show help (planned)
ferrocp --help
```

## 📊 Performance Goals

FerroCP aims to achieve the following performance targets:

| Operation | File Size | Target FerroCP | shutil | Target Speedup |
|-----------|-----------|----------------|--------|----------------|
| **Single File** | 1 KB | < 100 μs | 290 μs | **3x+ faster** |
| **Single File** | 1 MB | < 300 μs | 1.9 ms | **6x+ faster** |
| **Single File** | 10 MB | < 5 ms | 12.5 ms | **2.5x+ faster** |
| **Single File** | 100 MB | < 50 ms | 125 ms | **2.5x+ faster** |
| **Directory Tree** | 1000 files | < 2 s | 4.8 s | **2x+ faster** |

### Planned Benchmarking

```python
import time
import ferrocp  # Not yet available
import shutil

# Future benchmark example
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

*Performance targets based on preliminary research. Actual results will be measured and documented when implementation is complete.*

## 🎬 VFX Platform Compatibility

FerroCP is designed to be fully compatible with the [VFX Reference Platform](https://vfxplatform.com/) standards, ensuring seamless integration into professional VFX and animation pipelines.

### Supported VFX Platforms

| Platform | Architecture | VFX Platform Year | Status |
|----------|-------------|-------------------|--------|
| **Linux** | x86_64 | CY2025 (gcc 11.2.1+, glibc 2.28+) | ✅ Supported |
| **Linux** | ARM64 | Modern VFX Workflows | ✅ Supported |
| **macOS** | x86_64 | CY2025 (macOS 12.0+) | ✅ Supported |
| **macOS** | ARM64 | CY2025+ (macOS 14.0+) | ✅ Supported |
| **Windows** | x86_64 | CY2025 (VS 2022 v17.6+) | ✅ Supported |

### VFX Industry Benefits

- **🎭 Studio Pipeline Integration**: Optimized for render farm and artist workstation workflows
- **🎨 DCC Application Support**: Compatible with Maya, Houdini, Nuke, Blender, and other VFX tools
- **🖥️ Render Farm Efficiency**: High-performance file operations for distributed rendering
- **☁️ Cloud VFX Workflows**: ARM64 support for cost-effective cloud instances
- **📁 Large Asset Handling**: Optimized for typical VFX file sizes (textures, geometry, renders)

### VFX Platform Testing

Our CI pipeline includes comprehensive VFX Platform compatibility testing:

```bash
# Run VFX Platform compatibility tests
.github/workflows/vfx-platform-test.yml

# Platforms tested:
# - Ubuntu 22.04 (VFX CY2025 Linux) - Native x86_64 + Cross-compiled ARM64
# - macOS 12+ (VFX CY2025 Intel)
# - macOS 14+ (VFX CY2025+ Apple Silicon)
# - Windows 2022 (VFX CY2025 Windows)
```

**CI Infrastructure Update**: We upgraded from Ubuntu 20.04 to Ubuntu 22.04 due to the [scheduled retirement](https://github.com/actions/runner-images/issues/11101) on 2025-04-15. Ubuntu 22.04 provides even better VFX Platform compatibility with glibc 2.35 and gcc 11.2+.

For detailed VFX Platform compatibility information, see [docs/VFX_PLATFORM_COMPATIBILITY.md](docs/VFX_PLATFORM_COMPATIBILITY.md).

### CI Optimization

Our CI system is optimized to reduce queue times while maintaining comprehensive testing:

- **🚀 Fast Core Tests**: Linux and Windows tests run on every PR
- **🍎 Conditional macOS Tests**: Only run when needed (use `test-macos` label)
- **🎬 VFX Platform Tests**: Comprehensive testing for releases

**PR Labels for CI Control:**
- `test-macos` - Force macOS testing
- `all-platforms` - Test all platforms
- `vfx-platform` - Run VFX Platform validation

See [docs/CI_OPTIMIZATION.md](docs/CI_OPTIMIZATION.md) for detailed CI usage guide.

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

FerroCP provides multiple ways to build documentation depending on your needs:

```bash
# Quick documentation build (CI-optimized, no Rust compilation required)
uv run nox -s docs_only

# Full documentation build with API docs (requires Rust compilation)
uv run nox -s docs

# Serve documentation with live reloading (development mode)
uv run nox -s docs_serve

# Direct build using make (minimal dependencies)
cd docs && make html
```

#### Documentation Build Options

- **`docs_only`**: Fast build for CI environments, independent of Rust compilation (~2-3 minutes)
- **`docs`**: Complete build including API documentation (requires maturin, ~10-15 minutes)
- **`docs_serve`**: Development server with live reloading and API docs
- **`make html`**: Direct Sphinx build with minimal dependencies

#### Troubleshooting

If you encounter build issues:

```bash
# Clean build artifacts
cd docs && make clean

# Verify Sphinx configuration
cd docs && python -c "import sys; sys.path.append('source'); import conf; print('✅ Config OK')"

# Check dependencies
pip install sphinx>=7.0.0 pydata-sphinx-theme>=0.14.1
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

Apache-2.0

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
