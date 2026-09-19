# FerroCP

<div align="center">

[![Build Status](https://github.com/loonghao/FerroCP/workflows/Build%20and%20Release/badge.svg)](https://github.com/loonghao/FerroCP/actions)
[![Python Version](https://img.shields.io/pypi/pyversions/ferrocp.svg)](https://pypi.org/project/ferrocp/)
[![License](https://img.shields.io/github/license/loonghao/FerroCP.svg)](https://github.com/loonghao/FerroCP/blob/main/LICENSE)
[![Ruff](https://img.shields.io/badge/ruff-enabled-brightgreen)](https://github.com/astral-sh/ruff)
[![CodSpeed](https://img.shields.io/badge/CodSpeed-performance%20monitoring-blue)](https://codspeed.io/loonghao/FerroCP)

**⚠️ 开发中项目 ⚠️**

**🚀 高性能文件复制工具**
*使用 Rust 构建，追求最大速度和可靠性*

**本项目目前正在积极开发中，尚未准备好用于生产环境。**

[English Documentation](README.md) | [文档](https://ferrocp.readthedocs.io/) | [性能测试](benchmarks/README.md)

</div>

**FerroCP** (Iron Copy) 是一个使用 Rust 编写的高性能跨平台文件复制工具，提供 Python 绑定。从零开始设计，专注于速度和可靠性，FerroCP 的目标是在保持熟悉易用 API 的同时，提供比标准 Python 工具快 **2-5 倍**的文件操作性能。

## ✨ 计划功能

### 🚀 **性能优先** (开发中)
- **目标：比 Python 的 `shutil` 快 2-5 倍**处理大文件
- **原生 Rust 实现**，零拷贝优化
- **多线程操作**，自动 CPU 检测
- **内存高效**，可配置缓冲区大小

### 🔧 **开发者友好** (计划中)
- **Python `shutil` 模块的直接替代品**
- **熟悉的 API** - 无需学习成本
- **类型提示**和全面的文档
- **现代工具链**，支持 maturin 和 uv

### 🌍 **跨平台卓越** (开发中)
- **Windows、Linux、macOS** 原生支持
- **跨平台一致性能**
- **平台特定优化**自动应用
- **Unicode 文件名支持**，正确编码处理

### 📊 **开发状态**
- **进行中** - 核心功能正在实现
- **测试框架**正在建立
- **性能基准测试**基础设施已就位
- **CI/CD 流水线**已配置用于未来发布

## 📦 安装

### ⚠️ 暂未可用

**FerroCP 目前正在开发中，暂不可安装。**

准备就绪后，将通过以下方式提供：

```bash
# 未来的 PyPI 安装（暂不可用）
pip install ferrocp

# 或使用 uv（暂不可用）
uv add ferrocp
```

### 开发安装（贡献者）

```bash
# 克隆仓库
git clone https://github.com/loonghao/FerroCP.git
cd FerroCP

# 安装开发依赖
uv sync --group all
uv run maturin develop --release

# 注意：核心功能仍在实现中
```

### 系统要求（可用时）

- **Python 3.9+**（推荐 3.11+ 以获得最佳性能）
- **Rust 工具链**（maturin 会自动安装）
- **64 位系统**（Windows、Linux、macOS）

## 🚀 计划 API（开发中）

### 基本用法（计划的直接替代）

```python
import ferrocp

# 计划的 API - 用 ferrocp.copy 替代 shutil.copy
ferrocp.copy("source.txt", "destination.txt")

# 复制文件及其元数据（类似于 shutil.copy2）
ferrocp.copy2("source.txt", "destination.txt")

# 复制目录树（类似于 shutil.copytree）
ferrocp.copytree("source_dir", "destination_dir")
```

### 高级配置（计划中）

```python
import ferrocp

# 计划的高级 API
copier = ferrocp.EACopy(
    thread_count=8,           # 使用 8 个线程进行并行操作
    buffer_size=8*1024*1024,  # 8MB 缓冲区用于大文件
    compression_level=3,      # 网络传输压缩
    verify_integrity=True     # 复制后验证文件完整性
)

# 高性能文件复制（计划中）
copier.copy_file("large_dataset.zip", "backup/dataset.zip")

# 带进度跟踪的批量操作（计划中）
files_to_copy = [
    ("data1.bin", "backup/data1.bin"),
    ("data2.bin", "backup/data2.bin"),
    ("data3.bin", "backup/data3.bin"),
]

for src, dst in files_to_copy:
    result = copier.copy_file(src, dst)
    print(f"复制了 {result.bytes_copied} 字节，耗时 {result.duration:.2f}s")
```

### 命令行界面（计划中）

```bash
# 计划的 CLI 界面
ferrocp copy source.txt destination.txt

# 带选项的复制（计划中）
ferrocp copy --threads 8 --verbose large_file.zip backup/

# 目录同步（计划中）
ferrocp copy --mirror source_dir/ destination_dir/

# 显示帮助（计划中）
ferrocp --help
```

## 📊 性能目标

FerroCP 的目标是实现以下性能指标：

| 操作 | 文件大小 | 目标 FerroCP | shutil | 目标提升 |
|------|----------|--------------|--------|----------|
| **单文件** | 1 KB | < 100 μs | 290 μs | **3x+ 更快** |
| **单文件** | 1 MB | < 300 μs | 1.9 ms | **6x+ 更快** |
| **单文件** | 10 MB | < 5 ms | 12.5 ms | **2.5x+ 更快** |
| **单文件** | 100 MB | < 50 ms | 125 ms | **2.5x+ 更快** |
| **目录树** | 1000 文件 | < 2 s | 4.8 s | **2x+ 更快** |

### 计划基准测试

```python
import time
import ferrocp  # 暂不可用
import shutil

# 未来基准测试示例
start = time.time()
ferrocp.copy("large_file.bin", "backup.bin")
ferrocp_time = time.time() - start

start = time.time()
shutil.copy("large_file.bin", "backup_shutil.bin")
shutil_time = time.time() - start

print(f"FerroCP: {ferrocp_time:.2f}s")
print(f"shutil:  {shutil_time:.2f}s")
print(f"提升: {shutil_time/ferrocp_time:.1f}x 更快")
```

*性能目标基于初步研究。实际结果将在实现完成后测量和记录。*

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
uv sync --group docs       # 文档（sphinx、mkdocs）
uv sync --group build      # 打包（build、twine、cibuildwheel）
```

### 从源码构建

本项目使用 **maturin** 构建 Rust 扩展：

```bash
# 开发构建（快速，用于测试）
uv run maturin develop --features python

# 发布构建（优化）
uv run maturin develop --release --features python

# 构建 Python wheel 包
uv run maturin build --release --features python

# 构建独立 CLI 工具（无 Python 依赖）
cargo build --release --bin ferrocp
```

**注意**：CLI 工具（`ferrocp.exe`）构建时不依赖 Python，可独立运行。Python 模块需要启用 `python` 特性。

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
