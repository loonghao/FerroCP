# CodSpeed异步API修复报告

## 🚨 问题描述

在CodSpeed基准测试中遇到"RuntimeError: no running event loop"错误：

```
>       engine.copy_file(str(medium_test_file), str(dest), options)
E       RuntimeError: no running event loop

benchmarks/test_codspeed.py:127: RuntimeError
```

## 🔍 问题根因

### 1. API使用错误
**核心问题**：基准测试代码混用了同步和异步API

**技术细节**：
- `ferrocp.copy()` 和 `ferrocp.copy_file()` 是同步函数
- `engine.copy_file()` 是异步方法，需要在事件循环中运行
- CodSpeed基准测试环境不支持异步函数

### 2. 参数传递方式错误
**问题**：尝试直接向同步函数传递配置参数
- `ferrocp.copy(src, dst, compression_level=3)` - 不支持
- `ferrocp.copy(src, dst, num_threads=4)` - 不支持

**正确方式**：使用`CopyOptions`对象配置参数

## 🔧 修复方案

### 1. 统一使用同步API

**修改前（错误的异步调用）**：
```python
@pytest.mark.benchmark
def test_copy_with_compression(medium_test_file, temp_dir):
    dest = temp_dir / get_unique_filename("compressed_dest")
    engine = ferrocp.CopyEngine()
    options = ferrocp.CopyOptions()
    options.compression_level = 3
    options.enable_compression = True
    engine.copy_file(str(medium_test_file), str(dest), options)  # 异步方法！
    assert dest.exists()
```

**修改后（正确的同步调用）**：
```python
@pytest.mark.benchmark
def test_copy_with_compression(medium_test_file, temp_dir):
    dest = temp_dir / get_unique_filename("compressed_dest")
    # 使用同步API和CopyOptions
    options = ferrocp.CopyOptions()
    options.compression_level = 3
    options.enable_compression = True
    ferrocp.copy_file(str(medium_test_file), str(dest), options=options)
    assert dest.exists()
```

### 2. 正确配置多线程选项

**修改前**：
```python
def test_copy_multi_thread(large_test_file, temp_dir):
    dest = temp_dir / get_unique_filename("multi_thread_dest")
    engine = ferrocp.CopyEngine()
    options = ferrocp.CopyOptions()
    options.num_threads = 4
    engine.copy_file(str(large_test_file), str(dest), options)  # 异步方法！
```

**修改后**：
```python
def test_copy_multi_thread(large_test_file, temp_dir):
    dest = temp_dir / get_unique_filename("multi_thread_dest")
    # 使用同步API
    options = ferrocp.CopyOptions()
    options.num_threads = 4
    ferrocp.copy_file(str(large_test_file), str(dest), options=options)
```

## 📋 修改文件

- `benchmarks/test_codspeed.py` - 修复异步API调用问题

## ✅ 修复效果

### 1. API一致性
- ✅ 所有基准测试使用同步API
- ✅ 避免事件循环相关错误
- ✅ 与CodSpeed环境兼容

### 2. 功能完整性
- ✅ 压缩功能基准测试正常工作
- ✅ 多线程功能基准测试正常工作
- ✅ 保持原有的性能测试覆盖

### 3. 测试稳定性
- ✅ 消除"RuntimeError: no running event loop"错误
- ✅ 基准测试结果可重现
- ✅ CodSpeed监控正常工作

## 🧪 验证方法

### 本地测试
```bash
# 运行CodSpeed基准测试
pytest benchmarks/test_codspeed.py --benchmark-only

# 运行特定的基准测试
pytest benchmarks/test_codspeed.py::test_copy_with_compression --benchmark-only
pytest benchmarks/test_codspeed.py::test_copy_multi_thread --benchmark-only
```

### CI验证
观察CodSpeed工作流中的：
1. 基准测试执行成功
2. 没有异步相关错误
3. 性能数据正确收集

## 📝 技术说明

### FerroCP API层次

1. **同步API**：
   - `ferrocp.copy_file()` - 同步文件复制
   - `ferrocp.copy()` - shutil兼容的同步复制
   - 适用于基准测试和简单脚本

2. **异步API**：
   - `engine.copy_file()` - 异步文件复制
   - 需要事件循环环境
   - 适用于异步应用程序

### CopyOptions配置

正确的配置方式：
```python
options = ferrocp.CopyOptions()
options.compression_level = 3
options.enable_compression = True
options.num_threads = 4
options.buffer_size = 64 * 1024
```

### 基准测试最佳实践

- 使用同步API避免事件循环复杂性
- 通过CopyOptions配置功能选项
- 确保测试的可重现性和稳定性

---

**修复完成时间**：2025年1月27日  
**影响范围**：CodSpeed基准测试、性能监控  
**向后兼容性**：✅ 完全兼容现有功能
