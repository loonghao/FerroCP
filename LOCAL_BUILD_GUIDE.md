# 🏗️ FerroCP本地构建和测试指南

## 🚀 快速开始

### 1. 环境检查
```bash
# 检查必要工具
rustc --version
cargo --version
python --version

# 检查uv包管理器
uv --version

# 如果没有uv，安装它
pip install uv
```

### 2. 快速构建测试
```bash
# 查看所有可用的nox会话
nox -l

# 快速构建和基本测试
nox -s build

# 验证构建的wheel
nox -s verify_build
```

## 🧪 测试会话

### 基础测试
```bash
# 运行Python测试（单个版本）
nox -s test-3.11

# 运行所有Python版本测试
nox -s test

# 运行代码检查
nox -s lint
```

### 性能测试
```bash
# 运行基础基准测试
nox -s benchmark

# 运行比较基准测试
nox -s benchmark_compare

# 运行CodSpeed基准测试（本地）
nox -s codspeed

# 运行所有CodSpeed基准测试
nox -s codspeed_all
```

### 高级构建
```bash
# PGO优化构建
nox -s build_pgo

# 多平台wheel构建
nox -s build_wheels
```

## 🔧 问题排查

### 构建失败
如果遇到构建问题，nox会自动尝试：
1. 检查构建环境
2. 配置合适的编译器和链接器
3. 使用fallback策略重试

### Blake3兼容性问题
如果遇到Blake3相关错误：
```bash
# 设置环境变量
export BLAKE3_NO_ASM=1
export CC=gcc
export CXX=g++

# 然后重新构建
nox -s build
```

### 异步API问题
如果基准测试出现"no running event loop"错误，这是正常的，我们已经在代码中处理了这个问题。

## 📊 测试数据生成

### 生成基准测试数据
```bash
# 创建测试数据目录
mkdir -p benchmarks/data/test_files

# 生成测试数据
python benchmarks/data/generate_test_data.py --output-dir benchmarks/data/test_files
```

## 🎯 推荐的本地测试流程

### 开发时的快速验证
```bash
# 1. 快速构建和基本验证
nox -s build
nox -s verify_build

# 2. 运行基础测试
nox -s test-3.11

# 3. 运行基准测试
nox -s benchmark
```

### 完整验证流程
```bash
# 1. 代码检查
nox -s lint

# 2. 完整测试
nox -s test

# 3. 性能基准测试
nox -s benchmark
nox -s codspeed

# 4. 覆盖率分析
nox -s coverage_all
```

### 性能优化验证
```bash
# 1. PGO优化构建
nox -s build_pgo

# 2. 验证优化效果
nox -s verify_build

# 3. 性能对比测试
nox -s benchmark_compare
```

## 📝 输出文件位置

### 构建产物
- **Wheels**: `target/wheels/`
- **多平台wheels**: `wheelhouse/`

### 测试结果
- **基准测试结果**: `benchmarks/results/`
- **覆盖率报告**: `coverage/`
- **HTML覆盖率**: `coverage/index.html`

### 日志和调试
- **构建日志**: nox会显示详细的构建过程
- **测试日志**: pytest输出包含详细信息

## 🚨 常见问题

### 1. 编译器问题
```bash
# Linux
sudo apt-get install build-essential

# macOS
xcode-select --install

# Windows
# 安装Visual Studio Build Tools
```

### 2. Rust工具链问题
```bash
# 更新Rust
rustup update

# 检查工具链
rustup show
```

### 3. Python环境问题
```bash
# 重新同步依赖
uv sync --group testing --group build

# 清理缓存
uv cache clean
```

## 💡 性能测试建议

1. **先运行基础测试**确保功能正常
2. **使用PGO构建**获得最佳性能
3. **运行多次基准测试**确保结果稳定
4. **比较不同配置**的性能差异

---

**开始测试**: `nox -s build && nox -s verify_build`  
**完整验证**: `nox -s test && nox -s benchmark`  
**性能优化**: `nox -s build_pgo && nox -s benchmark_compare`
