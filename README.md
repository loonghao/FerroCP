# FerroCP

<div align="center">

[![CI](https://github.com/loonghao/FerroCP/actions/workflows/ci.yml/badge.svg)](https://github.com/loonghao/FerroCP/actions/workflows/ci.yml)
[![Release](https://github.com/loonghao/FerroCP/workflows/Release/badge.svg)](https://github.com/loonghao/FerroCP/actions)
[![VFX Platform](https://img.shields.io/badge/VFX%20Platform-CY2025%20Compatible-brightgreen)](https://vfxplatform.com/)
[![Python Version](https://img.shields.io/badge/python-3.9%20%7C%203.10%20%7C%203.11%20%7C%203.12-blue)](https://github.com/loonghao/FerroCP)
[![License](https://img.shields.io/github/license/loonghao/FerroCP.svg)](https://github.com/loonghao/FerroCP/blob/main/LICENSE)
[![Ruff](https://img.shields.io/badge/ruff-enabled-brightgreen)](https://github.com/astral-sh/ruff)
[![CodSpeed](https://img.shields.io/badge/CodSpeed-performance%20monitoring-blue)](https://codspeed.io/loonghao/FerroCP)
[![Multi-Platform](https://img.shields.io/badge/platforms-Linux%20%7C%20macOS%20%7C%20Windows-blue)](https://github.com/loonghao/FerroCP/actions)

**🚀 High-Performance File Copying Tool**
*Built with Rust for Maximum Speed and Reliability*

[中文文档](README_zh.md) | [Documentation](https://ferrocp.readthedocs.io/) | [Benchmarks](benchmarks/README.md)

</div>

**FerroCP** (Iron Copy) is a high-performance, cross-platform file copying tool written in Rust with Python bindings. It provides a Rust CLI (`ferrocp`) and a Python package (`ferrocp`) with a `shutil`-compatible API.

## ✨ Features

### 🚀 **Implemented**
- **Native Rust implementation** with zero-copy optimizations (`crates/ferrocp-zerocopy`)
- **Async copy engine** with progress reporting (`ferrocp.CopyEngine`, `ferrocp.copy_file`)
- **Shutil-compatible helpers**: `copy`, `copy2`, `copytree`
- **Device-aware copy**: per-device analysis (type, filesystem, theoretical speeds, optimal buffer size)
- **JSON output** for the `copy` subcommand (`--json`) for automation and benchmarking
- **Options honoured by the copy path**: `verify`, `preserve_timestamps`, `preserve_permissions`, `enable_compression`
- **VFX Platform compatibility** - follows [VFX Reference Platform](https://vfxplatform.com/) standards

### ⚠️ **Not implemented (verified against the code on `main`)**
- **`move` raises `NotImplementedError`**: `ferrocp.move()` refuses to run instead of pretending to work.
  A move must finish copying before removing the source, but the FerroCP copy helpers never complete
  because the engine's scheduler dispatch loop is not started. The previous implementation skipped the
  `await`, deleted the source and silently lost data; awaiting it would hang forever instead. Use
  `shutil.move()` until the engine dispatch path is fixed.
- **`ferrocp sync`**, **`ferrocp verify`** and **`ferrocp config`** are parsed but print a placeholder "completed" message; the underlying logic is still `TODO` in `crates/ferrocp-cli/src/main.rs`
- **CLI options accepted but not wired to the engine**: `--threads`, `--compression-level` and `--zero-copy` on `ferrocp copy`
- **`CopyOptions` fields accepted but ignored by the copy path**: `mode`, `overwrite`, `buffer_size`, `num_threads`, `follow_symlinks` and `compression_level` (only `verify`, `preserve_timestamps`, `preserve_permissions` and `enable_compression` are read in `crates/ferrocp-python/src/copy.rs`)
- **Exclude/include patterns** exist on the CLI, but `CopyOptions` exposes no pattern fields in the Python API
- **`cargo install`** is not available: no crate has been published to crates.io

## 📦 Installation

### ⚠️ No published packages

**No `ferrocp` distribution is published yet.** The Python package is **not on PyPI**, and no pre-built CLI archive is attached to a GitHub release, so install from source:

```bash
# Clone the repository
git clone https://github.com/loonghao/FerroCP.git
cd FerroCP

# Install development dependencies
uv sync --group all

# Build the Python extension module in the current environment
uv run maturin develop --release

# Or build a wheel
uv run maturin build --release

# Build the standalone Rust CLI (no Python dependency)
cargo build --release --bin ferrocp
```

### Requirements

- **Python 3.9+** (3.11 is the default `nox` interpreter; the extension is built with `abi3-py39`)
- **Rust toolchain** (install from [rustup.rs](https://rustup.rs/))
- **64-bit system** (Windows, Linux, macOS)

## 🚀 Python API

All copy helpers are **async** and must be awaited — including the `shutil`-style aliases
`copy`, `copy2` and `copytree`. Calling them without `await` only returns a coroutine.

### Basic Usage

```python
import asyncio
import ferrocp

async def main():
    # shutil-style aliases (wrappers around the async helpers)
    await ferrocp.copy("source.txt", "destination.txt")
    await ferrocp.copy2("source.txt", "destination.txt")
    await ferrocp.copytree("source_dir", "destination_dir")

    # Or the explicit helpers
    result = await ferrocp.copy_file("source.txt", "destination.txt")
    print(result.success, result.bytes_copied, result.duration_seconds)

asyncio.run(main())
```

### Configuration

`CopyOptions` exposes exactly these keyword arguments (defaults in parentheses):
`mode` (`"auto"`), `overwrite` (`"prompt"`), `preserve_timestamps` (`True`),
`preserve_permissions` (`True`), `follow_symlinks` (`False`), `enable_compression` (`False`),
`compression_level` (`6`), `buffer_size` (`65536`), `num_threads` (`0`) and `verify` (`False`).

Only `verify`, `preserve_timestamps`, `preserve_permissions` and `enable_compression`
currently change the copy behaviour; the other fields are accepted but ignored.

```python
import asyncio
import ferrocp

async def main():
    options = ferrocp.CopyOptions(
        verify=True,
        preserve_timestamps=True,
        preserve_permissions=True,
        enable_compression=True,
    )
    result = await ferrocp.copy_file("large_dataset.zip", "backup/dataset.zip", options=options)
    print(f"Copied {result.bytes_copied} bytes in {result.duration_seconds:.2f}s")

asyncio.run(main())
```

A `CopyResult` carries `bytes_copied`, `files_copied`, `duration_seconds`, `transfer_rate`,
`success` and `error_message`.

### Progress Reporting

`copy_file` and `copy_directory` accept a `progress_callback`. The callback is invoked
after the operation finishes with the final counters; `total_bytes`, `total_files` and
`percentage` are not populated by the current implementation.

```python
import asyncio
import ferrocp

async def main():
    def on_progress(progress):
        print(f"Copied {progress.bytes_copied} bytes in {progress.files_copied} files")

    await ferrocp.copy_file("large.bin", "backup.bin", progress_callback=on_progress)

asyncio.run(main())
```

## 🖥️ Command Line Interface

Two different `ferrocp` commands exist:

| Command | Source | Installed by |
|---------|--------|--------------|
| Rust CLI (`copy`, `sync`, `verify`, `device`, `config`) | `crates/ferrocp-cli` | `cargo build --release --bin ferrocp` |
| Python CLI (`copy`, `copy_with_server`, `benchmark`) | `python/ferrocp/cli.py` (click) | `maturin develop` / `maturin build` |

### Rust CLI

Global options (must come **before** the subcommand): `-d/--debug`, `-q/--quiet`,
`-v/--verbose`, `-c/--config <PATH>`, `-V/--version`.

```bash
# Copy a file
ferrocp copy source.txt destination.txt

# Verbose is a global option: it goes before the subcommand
ferrocp --verbose copy --threads 8 large_file.zip backup/

# Mirror a directory (equivalent to robocopy /MIR)
ferrocp copy --mirror source_dir/ destination_dir/

# Structured output for automation (only the copy subcommand supports --json)
ferrocp copy source_dir/ destination_dir/ --json

# Show help
ferrocp --help
ferrocp copy --help
```

`ferrocp copy` options:

| Option | Description |
|--------|-------------|
| `-m, --mode <MODE>` | `all` (default), `newer`, `different`, `mirror` |
| `-t, --threads <THREADS>` | Accepted, but not wired to the engine yet |
| `--compress` | Enable compression |
| `--compression-level <LEVEL>` | 0-22, default `6`; accepted, but not wired to the engine yet |
| `--zero-copy` | Enable zero-copy operations; accepted, but not wired to the engine yet |
| `--mirror` | Mirror mode; overrides `--mode` |
| `--exclude <PATTERN>` / `--include <PATTERN>` | Repeatable patterns |
| `--json` | Emit the JSON result document |

> `sync`, `verify` and `config` are parsed but currently only print a placeholder message;
> the real logic is still unimplemented in `crates/ferrocp-cli/src/main.rs`.

### Python CLI

Provided by the `ferrocp` console script (`python/ferrocp/cli.py`):

```bash
ferrocp --version
ferrocp --verbose copy SOURCE DESTINATION --threads 4 --buffer-size 8388608 --compression 0
ferrocp copy_with_server SOURCE DESTINATION --server HOST --port 8080
ferrocp benchmark
```

## 📊 Performance

No measured numbers are published yet. The figures below are **targets**, not measurements:

| Operation | File Size | Target FerroCP | shutil | Target Speedup |
|-----------|-----------|----------------|--------|----------------|
| **Single File** | 1 KB | < 100 μs | 290 μs | **3x+ faster** |
| **Single File** | 1 MB | < 300 μs | 1.9 ms | **6x+ faster** |
| **Single File** | 10 MB | < 5 ms | 12.5 ms | **2.5x+ faster** |
| **Single File** | 100 MB | < 50 ms | 125 ms | **2.5x+ faster** |
| **Directory Tree** | 1000 files | < 2 s | 4.8 s | **2x+ faster** |

To produce your own numbers, use the benchmark suite in [benchmarks/README.md](benchmarks/README.md):

```bash
uv sync --group testing
uv run nox -s benchmark          # run all benchmarks
uv run nox -s benchmark_compare  # compare against other tools
uv run nox -s codspeed           # CodSpeed benchmarks
```

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

> These rows describe the platforms FerroCP is designed and built for. No CI workflow
> currently runs VFX Platform validation, so the “Supported” status is a project target
> rather than a verified test result.

### VFX Industry Benefits

- **🎭 Studio Pipeline Integration**: Optimized for render farm and artist workstation workflows
- **🎨 DCC Application Support**: Compatible with Maya, Houdini, Nuke, Blender, and other VFX tools
- **🖥️ Render Farm Efficiency**: High-performance file operations for distributed rendering
- **☁️ Cloud VFX Workflows**: ARM64 support for cost-effective cloud instances
- **📁 Large Asset Handling**: Optimized for typical VFX file sizes (textures, geometry, renders)

For detailed VFX Platform compatibility information, see [docs/VFX_PLATFORM_COMPATIBILITY.md](docs/VFX_PLATFORM_COMPATIBILITY.md).

### CI

The repository ships five GitHub Actions workflows in `.github/workflows/`:

| Workflow | Purpose |
|----------|---------|
| `ci.yml` | Merge gate: Rust fmt/clippy/test on Linux, Windows and macOS, Python lint, the Python extension build, and the `cargo deny` / `cargo audit` dependency gate |
| `cargo-lock.yml` | Dependency reproducibility gate: `cargo metadata --locked` fails when a manifest changed without a matching `Cargo.lock` update |
| `release-please.yml` | Opens/updates the release PR from conventional commits |
| `goreleaser.yml` | Cross-compiles and attaches CLI binaries to a release |
| `test-goreleaser.yml` | Validates the GoReleaser configuration |

The CI gate covers the whole workspace and every target
(`cargo check --workspace --all-targets --all-features`) rather than a single
package or binary. Local checks are available through the helper scripts in
`scripts/` (for example `scripts/local-ci-check.ps1`, `scripts/quick-ci-check.ps1`
and `scripts/run-tests.ps1`).

There is no separate VFX platform workflow. The VFX badge above describes a
project target, not a verified test result; VFX-shaped coverage (large asset
copies, platform-specific metadata handling) is exercised by the regular
three-platform test matrix instead of a dedicated workflow.

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
uv sync --group testing    # Testing tools (pytest, coverage, pytest-benchmark, pytest-codspeed)
uv sync --group linting    # Code quality (ruff, mypy)
uv sync --group docs       # Documentation (sphinx, pydata-sphinx-theme, myst-parser)
uv sync --group build      # Packaging (build, twine, cibuildwheel)
```

### Building from Source

This project uses **maturin** to build Rust extensions:

```bash
# Development build with Python bindings (fast, for testing)
uv run maturin develop

# Release build with Python bindings (optimized)
uv run maturin develop --release

# Build wheel packages for Python
uv run maturin build --release

# Build standalone CLI tool (no Python dependencies)
cargo build --release --bin ferrocp
```

**Note**: The CLI tool (`ferrocp.exe`) is built without Python dependencies and can run
independently. There is no `python` Cargo feature — the Python extension module is built from
`crates/ferrocp-python` as configured in `[tool.maturin]` in `pyproject.toml`.

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

### Build Verification

```bash
# Regular optimized build
uv run nox -s build

# Verify build works correctly
uv run nox -s verify_build
```

> PGO builds are not available: the `build_pgo` nox session was removed, so
> `make build-pgo` (and the `nox -s build_pgo` call behind it) currently fails.
> The approach is still documented in [docs/development/PGO_BUILD.md](docs/development/PGO_BUILD.md).

### Continuous Performance Monitoring with CodSpeed

This project uses [CodSpeed](https://codspeed.io/) for continuous performance monitoring:

```bash
# Run CodSpeed benchmarks locally
uv run nox -s codspeed

# Run all CodSpeed benchmarks
uv run nox -s codspeed_all
```

CodSpeed benchmarks can be run locally with `uv run nox -s codspeed`. Note that no GitHub
Actions workflow currently runs them, so CodSpeed does not report on pull requests.

### Benchmark Targets

Unverified targets (no measured results are published yet):
- **Small files (< 1MB)**: > 100 MB/s
- **Large files (> 10MB)**: > 500 MB/s
- **vs shutil**: 2-5x faster for large files
- **vs robocopy**: Competitive performance (within 20%)

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

This project uses GitHub Actions for releases:

- **Release Please** (`release-please.yml`): opens and updates the release PR from conventional commits.
- **Release** (`goreleaser.yml`): cross-compiles the `ferrocp` CLI binaries with GoReleaser and attaches them to the GitHub release.
- **Test GoReleaser Configuration** (`test-goreleaser.yml`): validates the GoReleaser configuration.

No workflow currently publishes wheels to PyPI or deploys documentation.

### Release Process

Releases are driven by [release-please](https://github.com/googleapis/release-please) and
conventional commits. You never edit the version or the changelog by hand.

1. Land your changes on `main` using [Conventional Commits](https://www.conventionalcommits.org/)
   (`feat:`, `fix:`, `perf:`, `docs:`, `chore:`, ...).
2. `release-please` opens (or updates) a **release PR** titled
   `chore(main): release x.y.z`. It carries the next version and the generated
   `CHANGELOG.md` entry.
3. Review the release PR and **merge it**. Merging is what cuts the release.
4. On merge, `release-please` tags `vx.y.z` and creates the GitHub Release, then
   the release workflow cross-compiles the `ferrocp` binaries with GoReleaser and
   attaches them to that release.

What makes the version move:

| Commit type | Release |
| ----------- | ------- |
| `fix:`, `perf:`, `deps:` | patch |
| `feat:` | minor |
| `BREAKING CHANGE:` footer or `!` | major |

Version sources, all updated by the release PR — never by hand:

- `Cargo.toml` → `[workspace.package] version` (every crate inherits it via
  `version.workspace = true`)
- `python/ferrocp/__version__.py`

`pyproject.toml` declares `dynamic = ["version"]`, so the wheel version comes from
`crates/ferrocp-python/Cargo.toml` and therefore also from the workspace version.

To re-publish a tag without a new release, run the **Release** workflow manually from
the Actions tab and set `ref` to the tag (for example `v0.4.1`).

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request
