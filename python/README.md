# FerroCP - High-Performance File Copying Library

[![PyPI version](https://badge.fury.io/py/ferrocp.svg)](https://badge.fury.io/py/ferrocp)
[![Python versions](https://img.shields.io/pypi/pyversions/ferrocp.svg)](https://pypi.org/project/ferrocp/)
[![License](https://img.shields.io/pypi/l/ferrocp.svg)](https://github.com/loonghao/FerroCP/blob/main/LICENSE)
[![Build Status](https://github.com/loonghao/FerroCP/actions/workflows/ci.yml/badge.svg)](https://github.com/loonghao/FerroCP/actions/workflows/ci.yml)

FerroCP is a high-performance file copying library. The copy engine is written in Rust and
exposed to Python through PyO3 bindings, with a `shutil`-compatible API for easy migration.

This is the README for the Python package (`python/`). For the CLI, the Rust crates, and the
VFX platform matrix, see the [repository README](../README.md).

## Features

- **Shutil-compatible API** - `ferrocp.copy`, `ferrocp.copy2` and `ferrocp.copytree` are drop-in shaped aliases
- **Async copy engine** - every copy helper is a coroutine and must be awaited
- **Progress reporting** - a callback receives the final counters when an operation completes
- **Copy options** - `verify`, `preserve_timestamps`, `preserve_permissions`, `buffer_size` and more
- **Cross-platform** - Windows, macOS and Linux
- **Type hints** - the extension module ships a `.pyi` stub and a `py.typed` marker

### Not implemented yet

Read this before designing against the API:

- **`ferrocp.move()` raises `NotImplementedError`.** A move must copy before removing the
  source, and the copy path offers no atomicity or rollback guarantee, so deleting the source
  after a partial copy would lose data. Use `shutil.move()` until a verified move primitive
  exists.
- **Compression is not applied.** `enable_compression` is accepted and forwarded to the I/O
  layer, which has no compressor yet. `compression_level` must stay `0`; any other value is
  rejected.
- **`num_threads` must stay `0`.** The engine sizes its own thread pool, so any other value
  is rejected rather than ignored.
- **Directory sync is not wired up.** `SyncEngine` and `SyncOptions` exist but expose no
  working sync operations; the CLI `sync` subcommand also exits with an error.

Behaviour that *is* guaranteed is written down in
[docs/COPY_SEMANTICS.md](../docs/COPY_SEMANTICS.md); anything not listed there is undefined.
Error classification and the Python exception mapping live in
[docs/ERROR_MODEL.md](../docs/ERROR_MODEL.md).

## Installation

**No `ferrocp` distribution is published to PyPI yet.** Install from source:

```bash
# Clone the repository
git clone https://github.com/loonghao/FerroCP.git
cd FerroCP

# Install development dependencies
uv sync --group all

# Build the Python extension module into the current environment
uv run maturin develop --release

# Or build a wheel
uv run maturin build --release
```

### Requirements

- **Python 3.9 or newer** (the extension is built with `abi3-py39`)
- **Rust toolchain** (install from [rustup.rs](https://rustup.rs/))
- **64-bit system** (Windows, macOS or Linux)

## Quick Start

**All copy helpers are async and must be awaited** - including the `shutil`-style aliases
`copy`, `copy2` and `copytree`. Calling them without `await` only returns a coroutine.

```python
import asyncio
import ferrocp

async def main():
    # shutil-style aliases
    await ferrocp.copy("source.txt", "destination.txt")
    await ferrocp.copy2("source.txt", "destination.txt")
    await ferrocp.copytree("source_dir", "destination_dir")

    # Or the explicit helpers
    result = await ferrocp.copy_file("source.txt", "destination.txt")
    print(result.success, result.bytes_copied, result.duration_seconds)

asyncio.run(main())
```

### Copy options

`CopyOptions` accepts exactly these keyword arguments (defaults in parentheses):
`mode` (`"auto"`), `overwrite` (`"always"`), `overwrite_callback` (`None`),
`preserve_timestamps` (`True`), `preserve_permissions` (`True`), `follow_symlinks` (`False`),
`enable_compression` (`False`), `compression_level` (`0`), `buffer_size` (`65536`),
`num_threads` (`0`) and `verify` (`False`).

`buffer_size` is honoured by the copy path and must be a power of two between 4 KiB and
64 MiB. `num_threads` and `compression_level` must stay `0` - see *Not implemented yet*.

```python
import asyncio
import ferrocp

async def main():
    options = ferrocp.CopyOptions(
        verify=True,
        preserve_timestamps=True,
        preserve_permissions=True,
    )
    result = await ferrocp.copy_file("large_dataset.zip", "backup/dataset.zip", options=options)
    print(f"Copied {result.bytes_copied} bytes in {result.duration_seconds:.2f}s")

asyncio.run(main())
```

### Results

`CopyResult` carries `success`, `bytes_copied`, `files_copied`, `duration_seconds`,
`transfer_rate` and `error_message`.

```python
result = await ferrocp.copy_file("src.txt", "dst.txt")
print(f"Success: {result.success}")
print(f"Bytes copied: {result.bytes_copied}")
print(f"Duration: {result.duration_seconds:.2f}s")
print(f"Transfer rate: {result.transfer_rate:.2f} B/s")
```

### Progress reporting

`copy_file` and `copy_directory` accept a `progress_callback`. The callback is invoked after
the operation finishes with the final counters; `total_bytes`, `total_files` and `percentage`
are not populated by the current implementation.

```python
import asyncio
import ferrocp

async def main():
    def on_progress(progress):
        print(f"Copied {progress.bytes_copied} bytes in {progress.files_copied} files")

    await ferrocp.copy_file("large.bin", "backup.bin", progress_callback=on_progress)

asyncio.run(main())
```

### Asynchronous operations

For an operation handle you can poll or cancel, use `copy_file_async`:

```python
import asyncio
import ferrocp

async def main():
    operation = await ferrocp.copy_file_async("source.txt", "dest.txt")
    success = await operation.wait()
    print(f"Copy completed: {success}")

asyncio.run(main())
```

`AsyncOperation` exposes `id`, `is_running()`, `get_progress()`, `cancel()` and `wait()`.
`create_async_manager()` returns an `AsyncManager` for tracking several operations at once
(`get_active_operations()`, `get_operation()`, `cancel_all()`).

## API Reference

### Functions

| Function | Description |
|----------|-------------|
| `copy_file(source, destination, options=None, progress_callback=None)` | Copy a single file |
| `copy_directory(source, destination, options=None, progress_callback=None)` | Copy a directory tree |
| `copy(src, dst)` | Shutil-style alias of `copy_file` |
| `copy2(src, dst)` | Shutil-style alias of `copy_file` |
| `copytree(src, dst)` | Shutil-style alias of `copy_directory` |
| `quick_copy(source, destination)` | Copy with default options |
| `copy_with_verification(source, destination, progress_callback=None)` | Copy then verify |
| `copy_with_compression(source, destination, progress_callback=None)` | Copy with `enable_compression=True` |
| `copy_file_async(source, destination, options=None)` | Copy and return an `AsyncOperation` |
| `create_async_manager()` | Return an `AsyncManager` |
| `move(src, dst)` | Raises `NotImplementedError` - see *Not implemented yet* |

### Classes

- `CopyEngine` - the engine behind the module-level helpers (`copy_file`,
  `copy_directory`, `copy_file_async`, `get_statistics`, `get_features`, `is_busy`,
  `get_async_manager`)
- `CopyOptions` - per-operation configuration
- `CopyResult` - outcome of a copy
- `Progress` - counters handed to the progress callback
- `AsyncOperation` / `AsyncManager` - async operation handles
- `SyncOptions` / `SyncEngine` - present but not yet wired to a working sync
- `NetworkConfig` / `NetworkClient` - present but not yet wired to a working transfer

### Exceptions

All errors derive from `FerrocpError`: `IoError`, `CopyError`, `NetworkError`, `SyncError`
and `ConfigError`. See [docs/ERROR_MODEL.md](../docs/ERROR_MODEL.md) for how failures are
classified.

### Utilities

- `get_version()` - the package version
- `configure_logging(level="INFO")` - attach a stream handler to the `ferrocp` logger
- `get_features()` / `get_statistics()` - feature flags and global counters from the engine

## Performance

No measured numbers are published yet. The figures in the repository README are **targets**,
not measurements. To produce your own numbers, use the benchmark suite documented in
[benchmarks/README.md](../benchmarks/README.md):

```bash
uv sync --group testing
uv run nox -s benchmark          # run all benchmarks
uv run nox -s benchmark_compare  # compare against other tools
uv run nox -s codspeed           # CodSpeed benchmarks
```

The implementation leans on zero-copy I/O where the OS supports it, adaptive buffering based
on device characteristics, and a thread pool the engine sizes itself.

## Development

### Building from source

```bash
git clone https://github.com/loonghao/FerroCP.git
cd FerroCP
uv sync --group all

# Development build (fast, unoptimised)
uv run maturin develop

# Release build
uv run maturin develop --release
```

### Running tests

```bash
uv run nox -s test          # run the test suite
uv run nox -s test-3.11     # run against a specific Python version
uv run nox -s lint          # lint
uv run nox -s lint_fix      # lint with autofixes
```

### Documentation

```bash
uv run nox -s docs_only     # fast build, no Rust compilation
uv run nox -s docs          # full build with API docs
uv run nox -s docs_serve    # live-reloading dev server
```

Built documentation is published at [ferrocp.readthedocs.io](https://ferrocp.readthedocs.io/).

## License

FerroCP is distributed under the terms of the [Apache License 2.0](../LICENSE).

## Contributing

Contributions are welcome. Please read [CONTRIBUTING.md](../CONTRIBUTING.md) for the
development setup, code style, testing and pull-request process. In short: fork the
repository, branch from `main`, keep the test suite and linters green, and use
[Conventional Commits](https://www.conventionalcommits.org/) for your commit messages.

## Support

- [Documentation](https://ferrocp.readthedocs.io/)
- [Issue tracker](https://github.com/loonghao/FerroCP/issues)
- [Discussions](https://github.com/loonghao/FerroCP/discussions)

## Acknowledgments

- Built with [PyO3](https://pyo3.rs/) for Python-Rust interoperability
- Built with [maturin](https://github.com/PyO3/maturin) for packaging
- Async runtime by [Tokio](https://tokio.rs/)
- Inspired by [robocopy](https://docs.microsoft.com/en-us/windows-server/administration/windows-commands/robocopy) and [rsync](https://rsync.samba.org/)
