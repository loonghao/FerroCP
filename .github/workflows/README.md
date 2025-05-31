# GitHub Actions 工作流

本项目使用优化的 GitHub Actions 工作流，确保高效的构建、测试、基准测试和发布流程。

## 目录结构

```
.github/
└── workflows/            # GitHub Actions 工作流
    ├── test.yml          # 主测试工作流（包含代码覆盖率）
    ├── goreleaser.yml    # GoReleaser 自动化发布工作流
    ├── benchmark.yml     # 性能基准测试工作流（Python + Rust）
    ├── codspeed.yml      # CodSpeed 性能监控工作流
    ├── test-pgo.yml      # PGO 优化测试工作流
    ├── bumpversion.yml   # 版本更新工作流
    ├── docs.yml          # 文档构建工作流
    ├── issue-translator.yml # 问题翻译工作流
    └── shared-setup.yml  # 共享的 Rust 环境设置组件
```

## 工作流说明

### 主测试工作流 (`test.yml`)

主测试工作流在代码推送和 Pull Request 时自动运行，执行以下任务：

- **代码质量检查**: 运行 Rust 代码格式化检查和 Clippy 静态分析
- **单元测试**: 运行 Rust 和 Python 测试套件（不包含构建步骤）
- **代码覆盖率**: 收集 Python 和 Rust 代码覆盖率并上传到 Codecov
- **安全审计**: 运行 cargo audit 检查安全漏洞

### GoReleaser 自动化发布工作流 (`goreleaser.yml`)

GoReleaser 工作流在创建版本标签时自动运行，提供完整的发布自动化：

- **交叉编译**: 自动构建多平台二进制文件（Linux x86_64/ARM64, macOS x86_64/ARM64, Windows x86_64）
- **包管理器集成**: 自动更新 Homebrew tap 和 Scoop bucket
- **Docker 镜像**: 自动构建和发布 Docker 镜像到 GitHub Container Registry
- **GitHub Release**: 自动创建 GitHub Release 并上传所有资产
- **校验和生成**: 自动生成 SHA256 校验和文件
- **变更日志**: 自动生成结构化的变更日志

### 性能基准测试工作流 (`benchmark.yml`)

综合性能基准测试工作流，支持多种基准测试类型：

- **Python 基准测试**: 使用 pytest-benchmark 运行 Python 性能测试
- **Rust 基准测试**: 使用 cargo bench 运行 Rust 性能测试
- **比较基准测试**: 与其他工具（如 robocopy）进行性能对比
- **回归分析**: 自动检测性能回归并生成报告
- **可视化报告**: 生成交互式性能图表和趋势分析

### CodSpeed 性能监控工作流 (`codspeed.yml`)

专业的性能监控和回归检测工作流：

- **PGO 优化构建**: 使用 Profile-Guided Optimization 构建优化版本
- **持续性能监控**: 集成 CodSpeed 服务进行性能跟踪
- **自动回归检测**: 检测性能回归并提供详细分析
- **并行基准测试**: 支持分片并行执行以提高效率

### PGO 优化测试工作流 (`test-pgo.yml`)

Profile-Guided Optimization 测试工作流：

- **PGO 构建测试**: 验证 PGO 优化构建的正确性
- **性能验证**: 确保 PGO 优化版本的功能完整性
- **多平台支持**: 在不同平台上测试 PGO 构建

### 版本更新工作流 (`bumpversion.yml`)

当代码推送到主分支时自动运行，执行以下任务：

- 使用 commitizen 自动更新版本号
- 根据提交消息生成 changelog
- 提交版本更新和 changelog 到仓库

### 文档构建工作流 (`docs.yml`)

在文档或源代码更改时自动运行，执行以下任务：

- 构建项目文档
- 将文档部署到 GitHub Pages

### 问题翻译工作流 (`issue-translator.yml`)

自动翻译非英文 issue 和评论：

- 检测非英文内容
- 自动翻译为英文
- 添加翻译后的内容到 issue 中

## 优化特点

1. **功能整合**: 将重复功能整合到统一工作流中，减少维护复杂度
   - 代码覆盖率功能整合到主测试工作流
   - Python 和 Rust 基准测试统一管理

2. **多平台支持**: 全面的跨平台兼容性
   - 支持 Ubuntu、macOS、Windows 三大平台
   - 特殊处理 macOS 上的 ring 库编译问题

3. **Python 版本支持**: 支持 Python 3.9 到 3.12 版本

4. **性能优化**: 多层次的性能测试和优化
   - 常规基准测试和性能回归检测
   - PGO (Profile-Guided Optimization) 优化构建
   - CodSpeed 持续性能监控

5. **自动化程度高**:
   - 自动版本管理和 changelog 生成
   - 自动发布到 PyPI 和 GitHub Releases
   - 自动性能回归检测和报告

6. **资源优化**:
   - 智能缓存策略减少构建时间
   - 条件执行避免不必要的资源消耗
   - 并行执行提高 CI 效率

## 使用方法

### 日常开发
- **自动测试**: 创建 Pull Request 时自动运行测试和代码检查
- **性能基准测试**: 使用 `workflow_dispatch` 手动触发特定类型的基准测试
- **代码覆盖率**: 每次 push 和 PR 都会自动收集和报告代码覆盖率

### 发布流程
- **发布新版本**: 创建版本标签（如 v1.0.0）并推送到远端
- **自动化发布**: GoReleaser 自动处理整个发布流程
  - 交叉编译所有平台的二进制文件
  - 创建压缩包和校验和
  - 生成变更日志
  - 创建 GitHub Release
  - 更新包管理器（Homebrew, Scoop）
  - 构建和发布 Docker 镜像

### 性能监控
- **定期基准测试**: 每周自动运行完整的性能基准测试
- **性能回归检测**: PR 中自动检测性能变化
- **趋势分析**: 长期性能趋势跟踪和分析

## 故障排除

### macOS 构建问题

如果在 macOS 上遇到 ring 库编译错误：

1. 检查是否应用了 macOS 特定的环境变量设置
2. 确认 `RING_PREGENERATE_ASM=1` 和 `CARGO_CFG_TARGET_FEATURE=` 环境变量
3. 验证 `MACOSX_DEPLOYMENT_TARGET=10.15` 设置
4. 查看 [macOS 构建修复文档](../docs/MACOS_BUILD_FIX.md) 获取详细信息

### 性能基准测试问题

如果基准测试失败或结果异常：

1. 检查测试数据是否正确生成
2. 验证 Rust 和 Python 环境是否正确设置
3. 确认 criterion 和 pytest-benchmark 依赖是否安装
4. 查看基准测试日志中的详细错误信息

### 代码覆盖率问题

如果代码覆盖率收集失败：

1. 确认 `CODECOV_TOKEN` 环境变量已正确设置
2. 检查 cargo-tarpaulin 是否成功安装
3. 验证 Python 和 Rust 测试是否正常运行
4. 查看覆盖率文件是否正确生成

### 文档构建问题

如果在访问文档时遇到 404 错误：

1. 确保 GitHub Pages 在仓库设置中已启用
2. 检查源是否设置为 `gh-pages` 分支
3. 等待几分钟，让 GitHub Pages 在工作流完成后部署
4. 验证 `gh-pages` 分支是否包含预期内容

### 一般工作流问题

如果工作流运行失败：

1. 检查工作流日志中的错误消息
2. 确保所有依赖项都正确指定
3. 验证仓库设置中是否配置了所需的 secrets
4. 尝试在本地运行失败的步骤进行调试
5. 检查是否有平台特定的问题（特别是 macOS）

## 环境变量

### 必需的 Secrets

- `PERSONAL_ACCESS_TOKEN`: 具有所需权限的 GitHub 令牌，用于版本更新和发布
- `CODECOV_TOKEN`: Codecov 令牌，用于上传代码覆盖率报告
- `CODSPEED_TOKEN`: CodSpeed 服务令牌，用于性能监控

### 自动设置的环境变量

以下环境变量在 macOS 构建中自动设置以解决 ring 库编译问题：

- `CC=clang`: 使用 clang 编译器
- `CXX=clang++`: 使用 clang++ 编译器
- `MACOSX_DEPLOYMENT_TARGET=10.15`: 设置最低 macOS 版本
- `RING_PREGENERATE_ASM=1`: 强制 ring 使用预编译汇编
- `CARGO_CFG_TARGET_FEATURE=`: 禁用 CPU 特性检测

### 性能优化环境变量

- `CARGO_TERM_COLOR=always`: 启用彩色输出
- `RUST_BACKTRACE=1`: 启用详细错误回溯
- `RUSTFLAGS`: 根据需要设置的 Rust 编译标志

## 工作流统计

当前项目包含 **9 个** GitHub Actions 工作流文件：

- 3 个核心工作流（测试、GoReleaser发布、基准测试）
- 3 个专业工作流（CodSpeed、PGO、文档）
- 3 个辅助工作流（版本管理、问题翻译、共享组件）

## 最近更新

- **2024年12月**: 使用 GoReleaser 替换复杂的手动发布工作流
- **2024年12月**: 简化测试工作流，移除重复的构建步骤
- **2024年12月**: 添加共享的 Rust 环境设置组件
- **2024年12月**: 整合代码覆盖率功能到主测试工作流
- **2024年12月**: 整合 Python 和 Rust 基准测试到统一工作流
- **2024年12月**: 添加 macOS ring 库编译修复
