# ferrocp

<div align="center">

[![PyPI version](https://badge.fury.io/py/ferrocp.svg)](https://badge.fury.io/py/ferrocp)
[![Build Status](https://github.com/loonghao/ferrocp/workflows/Build%20and%20Release/badge.svg)](https://github.com/loonghao/ferrocp/actions)
[![Documentation Status](https://readthedocs.org/projects/ferrocp/badge/?version=latest)](https://ferrocp.readthedocs.io/en/latest/?badge=latest)
[![Python Version](https://img.shields.io/pypi/pyversions/ferrocp.svg)](https://pypi.org/project/ferrocp/)
[![License](https://img.shields.io/github/license/loonghao/ferrocp.svg)](https://github.com/loonghao/ferrocp/blob/main/LICENSE)
[![Downloads](https://static.pepy.tech/badge/ferrocp)](https://pepy.tech/project/ferrocp)
[![Code style: black](https://img.shields.io/badge/code%20style-black-000000.svg)](https://github.com/psf/black)
[![Ruff](https://img.shields.io/badge/ruff-enabled-brightgreen)](https://github.com/astral-sh/ruff)

**⚠️ 开发中项目 ⚠️**
本项目目前正在积极开发中，尚未准备好用于生产环境。

</div>

Python绑定库，用于Electronic Arts开发的高性能文件复制工具EACopy。该包提供对EACopy的C++功能的直接访问，为文件复制操作提供卓越的性能。

## 特性

- 通过直接C++绑定实现高性能文件复制
- API与Python的`shutil`模块兼容
- 支持EACopyService加速网络文件传输
- 跨平台兼容性（Windows原生支持，其他平台提供回退方案）
- 多线程文件操作

## 安装

```bash
pip install ferrocp
```

或者使用 Poetry:

```bash
poetry add ferrocp
```

## 使用方法

```python
import ferrocp

# 复制文件（类似于shutil.copy）
ferrocp.copy("source.txt", "destination.txt")

# 复制文件及其元数据（类似于shutil.copy2）
ferrocp.copy2("source.txt", "destination.txt")

# 复制目录树（类似于shutil.copytree）
ferrocp.copytree("source_dir", "destination_dir")

# 使用网络服务加速传输
ferrocp.copy_with_server("source_dir", "destination_dir", "server_address", port=31337)

# 配置全局设置
ferrocp.config.thread_count = 8  # 使用8个线程进行复制
ferrocp.config.compression_level = 5  # 网络传输使用压缩级别5
```

## 开发

### 环境设置

```bash
# 克隆仓库并初始化子模块
git clone https://github.com/loonghao/ferrocp.git
cd ferrocp
git submodule update --init --recursive

# 使用 Poetry 安装依赖
poetry install
```

### 从源码构建

本项目使用scikit-build-core构建C++扩展：

```bash
# 安装构建依赖
pip install scikit-build-core pybind11 cmake

# 构建包
python -m pip install -e .
```

### 测试

```bash
# 使用 nox 运行测试
nox -s pytest

# 运行代码检查
nox -s lint

# 修复代码风格问题
nox -s lint_fix
```

### 文档

```bash
# 构建文档
nox -s docs

# 启动带有实时重载功能的文档服务器
nox -s docs-serve
```

## 依赖

- [EACopy](https://github.com/electronicarts/EACopy) - Electronic Arts开发的高性能文件复制工具
- [pybind11](https://github.com/pybind/pybind11) - C++11 Python绑定

## 许可证

BSD-3-Clause（与EACopy相同）

## CI/CD 配置

本项目使用 GitHub Actions 进行 CI/CD，包含以下工作流：

- **构建和测试**：在多个 Python 版本和操作系统上测试包。
- **发布**：在创建新版本时构建并发布wheel包到 PyPI。
- **文档**：构建文档并部署到 GitHub Pages。

发布工作流使用 cibuildwheel 为每个平台构建带有正确编译的C++扩展的特定wheel包。

### 发布流程

创建新版本的步骤：

1. 更新 `pyproject.toml` 和 `python/ferrocp/__version__.py` 中的版本号
2. 更新 `CHANGELOG.md`，添加新版本和变更内容
3. 提交并推送更改
4. 创建一个带有版本号的新标签（例如 `0.1.0`）
5. 将标签推送到 GitHub

```bash
# 发布流程示例
git add pyproject.toml python/ferrocp/__version__.py CHANGELOG.md
git commit -m "Release 0.1.0"
git tag 0.1.0
git push && git push --tags
```

## 贡献

欢迎贡献！请随时提交Pull Request。

1. Fork 仓库
2. 创建您的特性分支 (`git checkout -b feature/amazing-feature`)
3. 提交您的更改 (`git commit -m 'Add some amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 打开一个 Pull Request
