# macOS Build Fix Documentation

## 问题描述

在 macOS CI/CD 环境中，`ring` 库编译时出现 CPU 特性检测断言失败的错误：

```
error[E0080]: evaluation of constant value failed
--> /Users/runner/.cargo/registry/src/index.crates.io-1949f9423d5b5f7f/ring-0.17.14/src/cpu/arm/darwin.rs:44:5
|
44 |     assert!((CAPS_STATIC & MIN_STATIC_FEATURES) == MIN_STATIC_FEATURES);
|     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
|     the evaluated program panicked at 'assertion failed: (CAPS_STATIC & MIN_STATIC_FEATURES) == MIN_STATIC_FEATURES'
```

## 根本原因

1. **构建缓存损坏**: 之前的构建可能留下了损坏的中间文件
2. **依赖版本冲突**: 不同版本的依赖库可能产生冲突  
3. **编译器版本兼容性**: Rust 1.86.0 可能与某些缓存的构建产物不兼容
4. **macOS 特定的编译环境问题**: ring 库在 macOS 上需要特定的编译器配置

## 解决方案

### 1. CI/CD 工作流修复

#### 修改的文件：
- `.github/workflows/test.yml`
- `.github/workflows/rust-benchmarks.yml` 
- `.github/workflows/release.yml`

#### 添加的步骤：
```yaml
# Clean build cache on macOS to avoid ring compilation issues
- name: Clean build cache (macOS)
  if: matrix.os == 'macos-latest'
  run: cargo clean

# Set macOS-specific environment variables for ring compilation
- name: Set macOS build environment
  if: matrix.os == 'macos-latest'
  run: |
    echo "CC=clang" >> $GITHUB_ENV
    echo "CXX=clang++" >> $GITHUB_ENV
    echo "MACOSX_DEPLOYMENT_TARGET=10.15" >> $GITHUB_ENV
```

### 2. Cargo 配置优化

#### 修改文件：`.cargo/config.toml`

添加 macOS 特定的 rustflags：
```toml
[target.x86_64-apple-darwin]
rustflags = [
    "-C", "target-cpu=native",
    "-C", "link-arg=-Wl,-rpath,@loader_path"
]

[target.aarch64-apple-darwin]
rustflags = [
    "-C", "target-cpu=native", 
    "-C", "link-arg=-Wl,-rpath,@loader_path"
]
```

### 3. 环境变量配置

为 macOS 构建设置特定的环境变量：
- `MACOSX_DEPLOYMENT_TARGET=10.15`: 确保向后兼容性
- `CC=clang`: 使用系统 clang 编译器
- `CXX=clang++`: 使用系统 clang++ 编译器

## 验证结果

### 本地验证
- ✅ Windows 构建正常
- ✅ 清理缓存后重新构建成功
- ✅ 所有 crate 编译通过

### CI/CD 验证
- ✅ 修复推送到 `refactor-cross-platform-api` 分支
- ✅ 提交哈希: `026103f`
- ⏳ 等待 CI/CD 管道验证

## 影响的依赖库

主要影响的库：
- `ring v0.17.14` - 加密库，被 rustls 使用
- `ring v0.16.20` - 旧版本，被其他依赖使用
- `rustls v0.21.12` - TLS 库，依赖 ring
- `quinn` - QUIC 协议实现，依赖 rustls

## 最佳实践

### 1. 平台特定配置
- 使用条件编译和平台特定的配置
- 在 CI/CD 中为不同平台设置不同的环境变量

### 2. 构建缓存管理
- 在遇到编译问题时，优先清理构建缓存
- 为不同平台使用不同的缓存键

### 3. 依赖管理
- 定期更新依赖库到稳定版本
- 监控依赖库的平台兼容性

## 相关链接

- [Ring 库 GitHub Issues](https://github.com/briansmith/ring/issues)
- [Rust Cross-compilation Guide](https://rust-lang.github.io/rustup/cross-compilation.html)
- [macOS Development Environment Setup](https://developer.apple.com/documentation/xcode)

## 维护说明

如果将来再次遇到类似问题：

1. 首先尝试清理构建缓存：`cargo clean`
2. 检查 ring 库的版本兼容性
3. 验证 macOS 环境变量设置
4. 考虑更新 Rust 工具链版本

---

**修复日期**: 2024年12月
**修复作者**: Long Hao <hal.long@outlook.com>
**相关分支**: refactor-cross-platform-api
