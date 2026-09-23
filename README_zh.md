# FerroCP

<div align="center">

[![CI](https://github.com/loonghao/FerroCP/actions/workflows/ci.yml/badge.svg)](https://github.com/loonghao/FerroCP/actions/workflows/ci.yml)
[![Release](https://github.com/loonghao/FerroCP/workflows/Release/badge.svg)](https://github.com/loonghao/FerroCP/actions)
[![Python Version](https://img.shields.io/badge/python-3.9%20%7C%203.10%20%7C%203.11%20%7C%203.12-blue)](https://github.com/loonghao/FerroCP)
[![License](https://img.shields.io/github/license/loonghao/FerroCP.svg)](https://github.com/loonghao/FerroCP/blob/main/LICENSE)
[![Ruff](https://img.shields.io/badge/ruff-enabled-brightgreen)](https://github.com/astral-sh/ruff)
[![CodSpeed](https://img.shields.io/badge/CodSpeed-performance%20monitoring-blue)](https://codspeed.io/loonghao/FerroCP)

**🚀 高性能文件复制工具**
*使用 Rust 构建，追求最大速度和可靠性*

[English Documentation](README.md) | [文档](https://ferrocp.readthedocs.io/) | [性能测试](benchmarks/README.md)

</div>

**FerroCP** (Iron Copy) 是一个使用 Rust 编写的高性能跨平台文件复制工具，提供 Python 绑定。它包含一个 Rust CLI（`ferrocp`）和一个提供 `shutil` 兼容 API 的 Python 包（`ferrocp`）。

## ✨ 功能

### 🚀 **已实现**
- **原生 Rust 实现**，零拷贝优化（`crates/ferrocp-zerocopy`）
- **异步复制引擎**，支持进度回调（`ferrocp.CopyEngine`、`ferrocp.copy_file`）
- **`shutil` 兼容辅助函数**：`copy`、`copy2`、`copytree`
- **设备感知复制**：分析设备类型、文件系统、理论速度与最佳缓冲区大小
- **`copy` 子命令支持 `--json`**，便于自动化和基准测试
- **复制路径真正生效的选项**：`verify`、`preserve_timestamps`、`preserve_permissions`、`enable_compression`
- **VFX 平台兼容**，遵循 [VFX Reference Platform](https://vfxplatform.com/) 标准

### ⚠️ **尚未实现（已按 `main` 分支代码逐条核对）**
- **`move` 直接抛出 `NotImplementedError`**：`ferrocp.move()` 会显式拒绝执行，而不再假装可用。
  移动操作必须先完成复制再删除源路径，但 FerroCP 的复制辅助函数永远不会完成——引擎的调度器分发循环没有启动。
  此前的实现跳过 `await` 就删除源路径，导致静默数据丢失；改为等待则只会永久挂起。
  在引擎分发路径修复之前，请改用 `shutil.move()`。
- **`ferrocp sync`、`ferrocp verify`、`ferrocp config`** 可被解析，但只打印占位信息；其逻辑在 `crates/ferrocp-cli/src/main.rs` 中仍是 `TODO`
- **已接收但未接入引擎的 CLI 选项**：`ferrocp copy` 的 `--threads`、`--compression-level`、`--zero-copy`
- **可设置但被复制路径忽略的 `CopyOptions` 字段**：`mode`、`overwrite`、`buffer_size`、`num_threads`、`follow_symlinks`、`compression_level`（`crates/ferrocp-python/src/copy.rs` 只读取 `verify`、`preserve_timestamps`、`preserve_permissions` 和 `enable_compression`）
- **排除/包含模式**只存在于 CLI，Python API 的 `CopyOptions` 没有提供对应字段
- **`cargo install` 不可用**：没有 crate 发布到 crates.io

## 📦 安装

### ⚠️ 暂无已发布的包

**目前没有任何 `ferrocp` 发行包。** Python 包**尚未发布到 PyPI**，GitHub Release 也没有附带预编译 CLI 压缩包，因此请从源码安装：

```bash
# 克隆仓库
git clone https://github.com/loonghao/FerroCP.git
cd FerroCP

# 安装开发依赖
uv sync --group all

# 在当前环境中构建 Python 扩展模块
uv run maturin develop --release

# 或构建 wheel
uv run maturin build --release

# 构建独立的 Rust CLI（不依赖 Python）
cargo build --release --bin ferrocp
```

### 系统要求

- **Python 3.9+**（`nox` 默认解释器为 3.11；扩展模块使用 `abi3-py39` 构建）
- **Rust 工具链**（从 [rustup.rs](https://rustup.rs/) 安装）
- **64 位系统**（Windows、Linux、macOS）

## 🚀 Python API

所有复制辅助函数都是**异步**的，必须 `await` —— 包括 `shutil` 风格的别名
`copy`、`copy2` 和 `copytree`。不加 `await` 调用只会得到一个协程对象。

### 基本用法

```python
import asyncio
import ferrocp

async def main():
    # shutil 风格别名（异步辅助函数的包装）
    await ferrocp.copy("source.txt", "destination.txt")
    await ferrocp.copy2("source.txt", "destination.txt")
    await ferrocp.copytree("source_dir", "destination_dir")

    # 或使用显式的辅助函数
    result = await ferrocp.copy_file("source.txt", "destination.txt")
    print(result.success, result.bytes_copied, result.duration_seconds)

asyncio.run(main())
```

### 配置

`CopyOptions` 提供以下关键字参数（括号中为默认值）：`mode`（`"auto"`）、
`overwrite`（`"prompt"`）、`preserve_timestamps`（`True`）、`preserve_permissions`（`True`）、
`follow_symlinks`（`False`）、`enable_compression`（`False`）、`compression_level`（`6`）、
`buffer_size`（`65536`）、`num_threads`（`0`）和 `verify`（`False`）。

其中只有 `verify`、`preserve_timestamps`、`preserve_permissions` 和 `enable_compression`
会真正改变复制行为，其余字段可设置但会被忽略。

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
    print(f"复制了 {result.bytes_copied} 字节，耗时 {result.duration_seconds:.2f}s")

asyncio.run(main())
```

`CopyResult` 包含 `bytes_copied`、`files_copied`、`duration_seconds`、`transfer_rate`、
`success` 和 `error_message`。

### 进度回调

`copy_file` 与 `copy_directory` 接受 `progress_callback`。回调在操作结束后被调用一次，
传入最终计数；当前实现不会填充 `total_bytes`、`total_files` 和 `percentage`。

```python
import asyncio
import ferrocp

async def main():
    def on_progress(progress):
        print(f"已复制 {progress.bytes_copied} 字节 / {progress.files_copied} 个文件")

    await ferrocp.copy_file("large.bin", "backup.bin", progress_callback=on_progress)

asyncio.run(main())
```

## 🖥️ 命令行界面

存在两个不同的 `ferrocp` 命令：

| 命令 | 来源 | 安装方式 |
|------|------|----------|
| Rust CLI（`copy`、`sync`、`verify`、`device`、`config`） | `crates/ferrocp-cli` | `cargo build --release --bin ferrocp` |
| Python CLI（`copy`、`copy_with_server`、`benchmark`） | `python/ferrocp/cli.py`（click） | `maturin develop` / `maturin build` |

### Rust CLI

全局选项（必须位于子命令**之前**）：`-d/--debug`、`-q/--quiet`、`-v/--verbose`、
`-c/--config <PATH>`、`-V/--version`。

```bash
# 复制文件
ferrocp copy source.txt destination.txt

# verbose 是全局选项，需要放在子命令之前
ferrocp --verbose copy --threads 8 large_file.zip backup/

# 镜像目录（等价于 robocopy /MIR）
ferrocp copy --mirror source_dir/ destination_dir/

# 结构化输出，便于自动化（只有 copy 子命令支持 --json）
ferrocp copy source_dir/ destination_dir/ --json

# 显示帮助
ferrocp --help
ferrocp copy --help
```

`ferrocp copy` 选项：

| 选项 | 说明 |
|------|------|
| `-m, --mode <MODE>` | `all`（默认）、`newer`、`different`、`mirror` |
| `-t, --threads <THREADS>` | 可接收，但尚未接入引擎 |
| `--compress` | 启用压缩 |
| `--compression-level <LEVEL>` | 0-22，默认 `6`；可接收，但尚未接入引擎 |
| `--zero-copy` | 启用零拷贝操作；可接收，但尚未接入引擎 |
| `--mirror` | 镜像模式，覆盖 `--mode` |
| `--exclude <PATTERN>` / `--include <PATTERN>` | 可重复的模式参数 |
| `--json` | 输出 JSON 结果文档 |

> `sync`、`verify` 和 `config` 可被解析，但目前只打印占位信息；其真实逻辑在
> `crates/ferrocp-cli/src/main.rs` 中尚未实现。

### Python CLI

由 `ferrocp` 控制台脚本（`python/ferrocp/cli.py`）提供：

```bash
ferrocp --version
ferrocp --verbose copy SOURCE DESTINATION --threads 4 --buffer-size 8388608 --compression 0
ferrocp copy_with_server SOURCE DESTINATION --server HOST --port 8080
ferrocp benchmark
```

## 📊 性能

目前没有公布实测数据。下表是**目标值**，不是实测结果：

| 操作 | 文件大小 | 目标 FerroCP | shutil | 目标提升 |
|------|----------|--------------|--------|----------|
| **单文件** | 1 KB | < 100 μs | 290 μs | **3x+ 更快** |
| **单文件** | 1 MB | < 300 μs | 1.9 ms | **6x+ 更快** |
| **单文件** | 10 MB | < 5 ms | 12.5 ms | **2.5x+ 更快** |
| **单文件** | 100 MB | < 50 ms | 125 ms | **2.5x+ 更快** |
| **目录树** | 1000 文件 | < 2 s | 4.8 s | **2x+ 更快** |

要得到自己的实测数据，请使用 [benchmarks/README.md](benchmarks/README.md) 中的基准测试套件：

```bash
uv sync --group testing
uv run nox -s benchmark          # 运行全部基准测试
uv run nox -s benchmark_compare  # 与其他工具对比
uv run nox -s codspeed           # CodSpeed 基准测试
```

## 🔬 开发

### 前置要求

- **Python 3.9+**（推荐 3.11+）
- **Rust 工具链**（从 [rustup.rs](https://rustup.rs/) 安装）
- **uv**（推荐，从 [uv docs](https://docs.astral.sh/uv/) 安装）

### 开发设置

```bash
# 克隆仓库
git clone https://github.com/loonghao/FerroCP.git
cd FerroCP

# 安装所有开发依赖
uv sync --group all

# 或安装特定依赖组
uv sync --group testing    # 测试工具（pytest、coverage、pytest-benchmark、pytest-codspeed）
uv sync --group linting    # 代码质量（ruff、mypy）
uv sync --group docs       # 文档（sphinx、pydata-sphinx-theme、myst-parser）
uv sync --group build      # 打包（build、twine、cibuildwheel）
```

### 从源码构建

本项目使用 **maturin** 构建 Rust 扩展：

```bash
# 开发构建（快速，用于测试）
uv run maturin develop

# 发布构建（优化）
uv run maturin develop --release

# 构建 Python wheel 包
uv run maturin build --release

# 构建独立 CLI 工具（无 Python 依赖）
cargo build --release --bin ferrocp
```

**注意**：CLI 工具（`ferrocp.exe`）构建时不依赖 Python，可独立运行。项目不存在 `python`
Cargo 特性 —— Python 扩展模块由 `pyproject.toml` 中 `[tool.maturin]` 指定的
`crates/ferrocp-python` 构建。

### 测试

```bash
# 运行测试
uv run nox -s test

# 运行特定 Python 版本的测试
uv run nox -s test-3.11

# 运行代码检查
uv run nox -s lint

# 自动修复代码风格问题
uv run nox -s lint_fix
```

### 文档

FerroCP 提供多种文档构建方式，满足不同需求：

```bash
# 快速文档构建（CI 优化，无需 Rust 编译）
uv run nox -s docs_only

# 完整文档构建，包含 API 文档（需要 Rust 编译）
uv run nox -s docs

# 启动带实时重载的文档服务器（开发模式）
uv run nox -s docs_serve

# 使用 make 直接构建（最小依赖）
cd docs && make html
```

#### 文档构建选项

- **`docs_only`**: 快速构建，适用于 CI 环境，独立于 Rust 编译（约 2-3 分钟）
- **`docs`**: 完整构建，包含 API 文档（需要 maturin，约 10-15 分钟）
- **`docs_serve`**: 开发服务器，支持实时重载和 API 文档
- **`make html`**: 直接 Sphinx 构建，最小依赖

#### 故障排除

如果遇到构建问题：

```bash
# 清理构建产物
cd docs && make clean

# 验证 Sphinx 配置
cd docs && python -c "import sys; sys.path.append('source'); import conf; print('✅ 配置正常')"

# 检查依赖
pip install sphinx>=7.0.0 pydata-sphinx-theme>=0.14.1
```

## CI 与 VFX 平台

仓库当前在 `.github/workflows/` 中包含五个 GitHub Actions 工作流：

| 工作流 | 用途 |
|--------|------|
| `ci.yml` | 合并门禁：Linux / Windows / macOS 三平台 Rust fmt、clippy、test，Python lint，Python 扩展模块构建，以及 `cargo deny` / `cargo audit` 依赖门禁 |
| `cargo-lock.yml` | 依赖可复现门禁：`cargo metadata --locked` 在 manifest 改动而 `Cargo.lock` 未同步更新时失败 |
| `release-please.yml` | 根据 conventional commits 创建/更新 release PR |
| `goreleaser.yml` | 交叉编译 CLI 二进制并附加到 release |
| `test-goreleaser.yml` | 校验 GoReleaser 配置 |

CI 门禁覆盖整个 workspace 与全部 target
（`cargo check --workspace --all-targets --all-features`），而不是单个包或单个二进制。
本地检查可以使用 `scripts/` 下的辅助脚本（例如 `scripts/local-ci-check.ps1`、
`scripts/quick-ci-check.ps1` 和 `scripts/run-tests.ps1`）。

目前没有独立的 VFX 平台工作流。上方的 VFX 徽章描述的是项目目标，而非已验证的测试结论；
VFX 场景的覆盖（大文件资产复制、平台相关的元数据处理）由常规的三平台测试矩阵承担，
而不是单独的工作流。

如需详细的 VFX 平台兼容性信息，请参阅 [docs/VFX_PLATFORM_COMPATIBILITY.md](docs/VFX_PLATFORM_COMPATIBILITY.md)。

## 依赖

### 核心依赖
- [Rust](https://www.rust-lang.org/) - 高性能扩展的系统编程语言
- [PyO3](https://pyo3.rs/) - Python 的 Rust 绑定
- [maturin](https://github.com/PyO3/maturin) - 基于 Rust 的 Python 扩展构建工具

### 开发依赖
- [uv](https://docs.astral.sh/uv/) - 快速 Python 包管理器
- [nox](https://nox.thea.codes/) - 灵活的测试自动化
- [ruff](https://github.com/astral-sh/ruff) - 快速 Python 代码检查和格式化工具
- [pytest](https://pytest.org/) - 测试框架
- [CodSpeed](https://codspeed.io/) - 持续性能监控

## 许可证

Apache-2.0

## 贡献

欢迎贡献！请随时提交 Pull Request。

1. Fork 仓库
2. 创建您的特性分支 (`git checkout -b feature/amazing-feature`)
3. 提交您的更改 (`git commit -m 'Add some amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 打开一个 Pull Request
