# ferrocp $RELEASE_VERSION

<div align="center">

[![PyPI version](https://badge.fury.io/py/ferrocp.svg)](https://badge.fury.io/py/ferrocp)
[![Build Status](https://github.com/loonghao/ferrocp/workflows/Build%20and%20Release/badge.svg)](https://github.com/loonghao/ferrocp/actions)
[![Documentation Status](https://readthedocs.org/projects/ferrocp/badge/?version=latest)](https://ferrocp.readthedocs.io/en/latest/?badge=latest)
[![Python Version](https://img.shields.io/pypi/pyversions/ferrocp.svg)](https://pypi.org/project/ferrocp/)
[![License](https://img.shields.io/github/license/loonghao/ferrocp.svg)](https://github.com/loonghao/ferrocp/blob/main/LICENSE)
[![Downloads](https://static.pepy.tech/badge/ferrocp)](https://pepy.tech/project/ferrocp)
[![Code style: black](https://img.shields.io/badge/code%20style-black-000000.svg)](https://github.com/psf/black)
[![Ruff](https://img.shields.io/badge/ruff-enabled-brightgreen)](https://github.com/astral-sh/ruff)

**⚠️ WORK IN PROGRESS ⚠️**
This project is currently under active development and not yet ready for production use.

</div>

## 🚀 What's New

$CHANGES

For detailed release notes, see the [CHANGELOG.md](https://github.com/loonghao/ferrocp/blob/main/CHANGELOG.md).

## 📦 Installation

### Using pip

```bash
pip install ferrocp==$RELEASE_VERSION
```

### Using Poetry

```bash
poetry add ferrocp==$RELEASE_VERSION
```

### From source

```bash
git clone https://github.com/loonghao/ferrocp.git
cd ferrocp
git checkout v$RELEASE_VERSION
git submodule update --init --recursive
pip install -e .
```

## 💻 Supported Platforms

- Windows (native support)
- Linux (fallback implementation)
- macOS (fallback implementation)

## 🐍 Python Compatibility

- Python 3.8+

## ✨ Key Features

- High-performance file copying with direct C++ bindings
- API compatible with Python's `shutil` module
- Support for EACopyService for accelerated network file transfers
- Multi-threaded file operations

## 📚 Documentation

For detailed documentation, visit [https://ferrocp.readthedocs.io/](https://ferrocp.readthedocs.io/)
