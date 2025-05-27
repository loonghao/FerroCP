# py-eacopy

<div align="center">

[![PyPI version](https://badge.fury.io/py/py-eacopy.svg)](https://badge.fury.io/py/py-eacopy)
[![Build Status](https://github.com/loonghao/py-eacopy/workflows/Build%20and%20Release/badge.svg)](https://github.com/loonghao/py-eacopy/actions)
[![Documentation Status](https://readthedocs.org/projects/py-eacopy/badge/?version=latest)](https://py-eacopy.readthedocs.io/en/latest/?badge=latest)
[![Python Version](https://img.shields.io/pypi/pyversions/py-eacopy.svg)](https://pypi.org/project/py-eacopy/)
[![License](https://img.shields.io/github/license/loonghao/py-eacopy.svg)](https://github.com/loonghao/py-eacopy/blob/main/LICENSE)
[![Downloads](https://static.pepy.tech/badge/py-eacopy)](https://pepy.tech/project/py-eacopy)
[![Code style: black](https://img.shields.io/badge/code%20style-black-000000.svg)](https://github.com/psf/black)
[![Ruff](https://img.shields.io/badge/ruff-enabled-brightgreen)](https://github.com/astral-sh/ruff)

**⚠️ WORK IN PROGRESS ⚠️**
This project is currently under active development and not yet ready for production use.

</div>

Python bindings for EACopy, a high-performance file copy tool developed by Electronic Arts. This package provides direct access to EACopy's C++ functionality, offering superior performance for file copying operations.

## Features

- High-performance file copying with direct C++ bindings
- API compatible with Python's `shutil` module
- Support for EACopyService for accelerated network file transfers
- Cross-platform compatibility (Windows native, with fallbacks for other platforms)
- Multi-threaded file operations

## Installation

```bash
pip install py-eacopy
```

Or with Poetry:

```bash
poetry add py-eacopy
```

## Usage

```python
import eacopy

# Copy a file (similar to shutil.copy)
eacopy.copy("source.txt", "destination.txt")

# Copy a file with metadata (similar to shutil.copy2)
eacopy.copy2("source.txt", "destination.txt")

# Copy a directory tree (similar to shutil.copytree)
eacopy.copytree("source_dir", "destination_dir")

# Use EACopyService for accelerated network transfers
eacopy.copy_with_server("source_dir", "destination_dir", "server_address", port=31337)

# Configure global settings
eacopy.config.thread_count = 8  # Use 8 threads for copying
eacopy.config.compression_level = 5  # Use compression level 5 for network transfers
```

## Development

### Setup

```bash
# Clone the repository with submodules
git clone https://github.com/loonghao/py-eacopy.git
cd py-eacopy
git submodule update --init --recursive

# Install dependencies with Poetry
poetry install
```

### Building from Source

This project uses scikit-build-core to build the C++ extensions:

```bash
# Install build dependencies
pip install scikit-build-core pybind11 cmake

# Build the package
python -m pip install -e .
```

### Testing

```bash
# Run tests with nox
nox -s pytest

# Run linting
nox -s lint

# Fix linting issues
nox -s lint_fix
```

### Documentation

```bash
# Build documentation
nox -s docs

# Serve documentation with live reloading
nox -s docs-serve
```

## Performance Benchmarks

py-eacopy includes comprehensive performance benchmarks and continuous performance monitoring:

### Local Benchmarking

```bash
# Install benchmark dependencies
pip install -e ".[benchmark]"

# Run all benchmarks
uvx nox -s benchmark

# Run quick benchmarks (for development)
make benchmark-quick

# Run comparison benchmarks vs standard tools
uvx nox -s benchmark_compare

# Generate performance profiles
python scripts/profile.py --test-type file_copy --profiler all
```

### Continuous Performance Monitoring with CodSpeed

This project uses [CodSpeed](https://codspeed.io/) for continuous performance monitoring:

```bash
# Run CodSpeed benchmarks locally
make codspeed

# Run all CodSpeed benchmarks
make codspeed-all
```

CodSpeed automatically:
- 🔍 Detects performance regressions in pull requests
- 📊 Provides detailed performance analysis and visualizations
- 📈 Tracks performance trends over time
- ✅ Integrates seamlessly with our GitHub Actions CI

### Benchmark Results

Current performance targets:
- **Small files (< 1MB)**: > 100 MB/s
- **Large files (> 10MB)**: > 500 MB/s
- **vs shutil**: 2-5x faster for large files
- **vs robocopy**: Competitive performance (within 20%)

See [benchmarks/README.md](benchmarks/README.md) for detailed benchmarking documentation.

## Dependencies

- [EACopy](https://github.com/electronicarts/EACopy) - High-performance file copy tool by Electronic Arts
- [pybind11](https://github.com/pybind/pybind11) - C++11 Python bindings

## License

BSD-3-Clause (same as EACopy)

## CI/CD Configuration

This project uses GitHub Actions for CI/CD with the following workflows:

- **Build and Test**: Tests the package on multiple Python versions and operating systems.
- **Release**: Builds and publishes wheels to PyPI when a new release is created.
- **Documentation**: Builds and deploys documentation to GitHub Pages.

The release workflow uses cibuildwheel to build platform-specific wheels with the C++ extensions properly compiled for each platform.

### Release Process

To create a new release:

1. Update the version in `pyproject.toml` and `src/eacopy/__version__.py`
2. Update the `CHANGELOG.md` with the new version and changes
3. Commit and push the changes
4. Create a new tag with the version number (e.g., `0.1.0`)
5. Push the tag to GitHub

```bash
# Example release process
git add pyproject.toml src/eacopy/__version__.py CHANGELOG.md
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
