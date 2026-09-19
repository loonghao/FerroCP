# FerroCP

FerroCP is a high-performance, cross-platform file copying tool written in Rust
with Python bindings. It ships a Rust CLI (`ferrocp`) and a Python package
(`ferrocp`) with a `shutil`-compatible API.

## Installation

No distribution is published yet: the package is not on PyPI and no prebuilt CLI
archive is attached to a GitHub release. Build from source:

```bash
git clone https://github.com/loonghao/FerroCP.git
cd FerroCP
uv sync --group all
uv run maturin develop --release   # Python package
cargo build --release --bin ferrocp  # standalone Rust CLI
```

Requires Python 3.9+ and a Rust toolchain.

## Quick Start

```python
import asyncio
import ferrocp

async def main():
    result = await ferrocp.copy_file("source.txt", "destination.txt")
    print(result.success, result.bytes_copied, result.duration_seconds)

asyncio.run(main())
```

All copy helpers are asynchronous and must be awaited, including the
`shutil`-style aliases `copy`, `copy2` and `copytree`.

## Documentation

- [Project overview](../README.md) (also available in [Chinese](../README_zh.md))
- [Installation](source/installation.rst)
- [Usage](source/usage.rst)
- [API reference](source/api.rst)
- [Performance benchmarks](../benchmarks/README.md)
- [JSON output](JSON_OUTPUT.md)

## License

Apache-2.0. See the [LICENSE](https://github.com/loonghao/FerroCP/blob/main/LICENSE)
file for details.
