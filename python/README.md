# FerroCP - High-Performance File Copying Library

[![PyPI version](https://badge.fury.io/py/ferrocp.svg)](https://badge.fury.io/py/ferrocp)
[![Python versions](https://img.shields.io/pypi/pyversions/ferrocp.svg)](https://pypi.org/project/ferrocp/)
[![License](https://img.shields.io/pypi/l/ferrocp.svg)](https://github.com/loonghao/py-eacopy/blob/main/LICENSE)
[![Build Status](https://github.com/loonghao/py-eacopy/workflows/CI/badge.svg)](https://github.com/loonghao/py-eacopy/actions)

FerroCP is a high-performance file copying library written in Rust with Python bindings. It provides fast, efficient file operations with zero-copy optimization, asynchronous support, and advanced features like network transfers and directory synchronization.

## 🚀 Features

- **Zero-Copy Optimization** - Maximum performance with minimal memory overhead
- **Asynchronous Operations** - Non-blocking file operations with cancellation support
- **Progress Monitoring** - Real-time progress reporting with speed and ETA
- **Network Transfers** - Remote file copying with resume capability
- **Directory Synchronization** - Bidirectional sync with conflict resolution
- **Cross-Platform** - Works on Windows, macOS, and Linux
- **Type Safety** - Full type hints for better IDE support
- **Memory Efficient** - Adaptive buffering based on device characteristics

## 📦 Installation

```bash
pip install ferrocp
```

### Requirements

- Python 3.8 or higher
- Windows, macOS, or Linux

## 🔧 Quick Start

### Basic File Copying

```python
import ferrocp

# Simple file copy
ferrocp.copy_file("source.txt", "destination.txt")

# Copy with options
options = ferrocp.CopyOptions(
    verify=True,
    preserve_timestamps=True,
    enable_compression=True
)
ferrocp.copy_file("source.txt", "dest.txt", options=options)
```

### Progress Monitoring

```python
def progress_callback(progress):
    print(f"Progress: {progress.percentage:.1f}% "
          f"Speed: {progress.speed_mbps:.2f} MB/s")

ferrocp.copy_file(
    "large_file.bin", 
    "backup.bin",
    progress_callback=progress_callback
)
```

### Asynchronous Operations

```python
import asyncio

async def async_copy():
    # Start async copy
    operation = await ferrocp.copy_file_async("source.txt", "dest.txt")
    
    # Monitor progress
    while await operation.is_running():
        progress = await operation.get_progress()
        if progress:
            print(f"Progress: {progress * 100:.1f}%")
        await asyncio.sleep(0.1)
    
    # Wait for completion
    success = await operation.wait()
    print(f"Copy completed: {success}")

asyncio.run(async_copy())
```

### Directory Operations

```python
# Copy entire directory
options = ferrocp.CopyOptions(recursive=True)
ferrocp.copy_directory("source_dir", "dest_dir", options)

# Directory synchronization
sync_options = ferrocp.SyncOptions(
    bidirectional=True,
    delete_orphaned=True
)
ferrocp.sync_directories("dir1", "dir2", sync_options)
```

### Network Transfers

```python
# Setup network client
config = ferrocp.NetworkConfig(
    host="192.168.1.100",
    port=8080,
    timeout=30.0
)
client = ferrocp.NetworkClient(config)

# Transfer file
client.transfer_file("local.txt", "remote.txt")
```

## 📚 API Reference

### Core Classes

#### `CopyEngine`
Main engine for file operations.

```python
engine = ferrocp.CopyEngine()
result = engine.copy_file("src.txt", "dst.txt")
```

#### `CopyOptions`
Configuration for copy operations.

```python
options = ferrocp.CopyOptions(
    verify=True,                    # Verify copied data
    preserve_timestamps=True,       # Keep original timestamps
    preserve_permissions=True,      # Keep file permissions
    enable_compression=False,       # Use compression
    buffer_size=65536,             # Buffer size in bytes
    max_retries=3,                 # Retry attempts
    retry_delay=1.0,               # Delay between retries
    overwrite=True,                # Overwrite existing files
    recursive=False                # Recursive directory copy
)
```

#### `CopyResult`
Result information from copy operations.

```python
result = ferrocp.copy_file("src.txt", "dst.txt")
print(f"Success: {result.success}")
print(f"Bytes copied: {result.bytes_copied}")
print(f"Duration: {result.duration_ms}ms")
print(f"Throughput: {result.throughput_mbps:.2f} MB/s")
```

### Convenience Functions

- `copy_file(source, destination, options=None, progress_callback=None)` - Copy a single file
- `copy_directory(source, destination, options=None, progress_callback=None)` - Copy a directory
- `quick_copy(source, destination)` - Fast copy with default settings
- `copy_with_verification(source, destination, progress_callback=None)` - Copy with verification
- `copy_with_compression(source, destination, progress_callback=None)` - Copy with compression

### Asynchronous API

- `copy_file_async(source, destination, options=None)` - Async file copy
- `AsyncOperation` - Handle for async operations
- `AsyncManager` - Manager for multiple async operations

## 🎯 Performance

FerroCP is designed for maximum performance:

- **Zero-copy I/O** where supported by the operating system
- **Adaptive buffering** based on storage device characteristics
- **Parallel processing** for directory operations
- **Memory-mapped files** for large file operations
- **SIMD optimizations** for data verification

### Benchmarks

| Operation | FerroCP | shutil | Speedup |
|-----------|---------|--------|---------|
| Large file (1GB) | 2.1s | 8.7s | 4.1x |
| Many small files (10k) | 3.2s | 12.1s | 3.8x |
| Network transfer | 1.8s | 6.2s | 3.4x |

*Benchmarks run on Windows 10, NVMe SSD, 16GB RAM*

## 🛠️ Development

### Building from Source

```bash
# Clone the repository
git clone https://github.com/loonghao/py-eacopy.git
cd py-eacopy

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build the Python package
cd python
pip install maturin
maturin develop
```

### Running Tests

```bash
# Install test dependencies
pip install -e ".[test]"

# Run tests
pytest tests/

# Run benchmarks
pytest tests/test_benchmarks.py -m benchmark
```

## 📄 License

This project is licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

## 🤝 Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## 📞 Support

- 📖 [Documentation](https://ferrocp.readthedocs.io)
- 🐛 [Issue Tracker](https://github.com/loonghao/py-eacopy/issues)
- 💬 [Discussions](https://github.com/loonghao/py-eacopy/discussions)

## 🙏 Acknowledgments

- Built with [PyO3](https://pyo3.rs/) for Python-Rust interoperability
- Inspired by [robocopy](https://docs.microsoft.com/en-us/windows-server/administration/windows-commands/robocopy) and [rsync](https://rsync.samba.org/)
- Uses [Tokio](https://tokio.rs/) for async runtime
