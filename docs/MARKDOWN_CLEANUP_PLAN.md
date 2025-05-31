# Markdown 文件整理计划

## 概述

项目中有大量散落的 Markdown 文件，需要进行整理以提高可维护性和可读性。

## 当前文件分析

### 📁 根目录文件 (需要整理)

#### 🗑️ 建议删除的文件
- `任务清单.md` - 临时文件，应该删除
- `benchmark_results_512kb_optimization.md` - 过时的基准测试结果
- `CI_CD_OPTIMIZATION_REPORT.md` - 过时的CI/CD报告
- `PERFORMANCE_BENCHMARK_OPTIMIZATION_REPORT.md` - 重复的性能报告
- `PERFORMANCE_BENCHMARKS.md` - 与 `PERFORMANCE_REPORT.md` 重复
- `performance_optimization_summary.md` - 过时的性能总结
- `WORKSPACE.md` - 不必要的工作空间文档

#### 📂 建议移动的文件
- `PERFORMANCE_REPORT.md` → `docs/performance/PERFORMANCE_REPORT.md`

### 📁 保留在根目录的文件
- `README.md` - 主要项目说明
- `README_zh.md` - 中文项目说明
- `CHANGELOG.md` - 变更日志
- `CONTRIBUTING.md` - 贡献指南

### 📁 docs/ 目录 (已整理)
- `CI_CLEANUP_PLAN.md` ✅
- `clippy-cross-platform.md` ✅
- `codspeed-setup.md` ✅
- `device-cache-enhancement.md` ✅
- `engine-selector-optimization.md` ✅
- `GORELEASER.md` ✅
- `index.md` ✅
- `MACOS_BUILD_FIX.md` ✅
- `PGO_BUILD.md` ✅
- `SKIP_EXISTING_OPTIMIZATION.md` ✅
- `TESTING_IMPLEMENTATION_REPORT.md` ✅
- `TESTING.md` ✅

### 📁 .github/ 目录 (已整理)
- `CODE_OF_CONDUCT.md` ✅
- `SECURITY.md` ✅
- `pull_request_template.md` ✅
- `release-template.md` ✅
- `ISSUE_TEMPLATE/` ✅
- `workflows/README.md` ✅

### 📁 子目录文件 (已整理)
- `benchmarks/README.md` ✅
- `crates/ferrocp-tests/README.md` ✅
- `python/README.md` ✅
- `web/README.md` ✅

## 整理计划

### 阶段 1: 创建新的目录结构

```
docs/
├── performance/          # 性能相关文档
├── development/          # 开发相关文档
├── ci-cd/               # CI/CD 相关文档
├── architecture/        # 架构设计文档
└── guides/              # 用户指南
```

### 阶段 2: 移动和重组文件

#### 性能文档
- `PERFORMANCE_REPORT.md` → `docs/performance/PERFORMANCE_REPORT.md`

#### CI/CD 文档
- `docs/CI_CLEANUP_PLAN.md` → `docs/ci-cd/CI_CLEANUP_PLAN.md`
- `docs/GORELEASER.md` → `docs/ci-cd/GORELEASER.md`
- `docs/codspeed-setup.md` → `docs/ci-cd/codspeed-setup.md`

#### 开发文档
- `docs/TESTING.md` → `docs/development/TESTING.md`
- `docs/TESTING_IMPLEMENTATION_REPORT.md` → `docs/development/TESTING_IMPLEMENTATION_REPORT.md`
- `docs/MACOS_BUILD_FIX.md` → `docs/development/MACOS_BUILD_FIX.md`
- `docs/PGO_BUILD.md` → `docs/development/PGO_BUILD.md`
- `docs/clippy-cross-platform.md` → `docs/development/clippy-cross-platform.md`

#### 架构文档
- `docs/device-cache-enhancement.md` → `docs/architecture/device-cache-enhancement.md`
- `docs/engine-selector-optimization.md` → `docs/architecture/engine-selector-optimization.md`
- `docs/SKIP_EXISTING_OPTIMIZATION.md` → `docs/architecture/SKIP_EXISTING_OPTIMIZATION.md`

### 阶段 3: 删除过时文件

```bash
# 删除过时和重复的文件
rm 任务清单.md
rm benchmark_results_512kb_optimization.md
rm CI_CD_OPTIMIZATION_REPORT.md
rm PERFORMANCE_BENCHMARK_OPTIMIZATION_REPORT.md
rm PERFORMANCE_BENCHMARKS.md
rm performance_optimization_summary.md
rm WORKSPACE.md
```

### 阶段 4: 更新文档索引

创建 `docs/README.md` 作为文档导航：

```markdown
# FerroCP 文档

## 📚 文档导航

### 🚀 快速开始
- [项目介绍](../README.md)
- [安装指南](guides/installation.md)
- [使用指南](guides/usage.md)

### 🏗️ 开发
- [开发环境设置](development/setup.md)
- [测试指南](development/TESTING.md)
- [构建指南](development/building.md)

### 🔧 CI/CD
- [GoReleaser 配置](ci-cd/GORELEASER.md)
- [CI 清理计划](ci-cd/CI_CLEANUP_PLAN.md)
- [性能监控设置](ci-cd/codspeed-setup.md)

### 🏛️ 架构
- [设备缓存增强](architecture/device-cache-enhancement.md)
- [引擎选择器优化](architecture/engine-selector-optimization.md)
- [跳过现有文件优化](architecture/SKIP_EXISTING_OPTIMIZATION.md)

### 📊 性能
- [性能报告](performance/PERFORMANCE_REPORT.md)
- [基准测试](../benchmarks/README.md)
```

## 实施步骤

### 1. 创建新目录结构
```bash
mkdir -p docs/{performance,development,ci-cd,architecture,guides}
```

### 2. 移动文件
```bash
# 性能文档
mv PERFORMANCE_REPORT.md docs/performance/

# CI/CD 文档
mv docs/CI_CLEANUP_PLAN.md docs/ci-cd/
mv docs/GORELEASER.md docs/ci-cd/
mv docs/codspeed-setup.md docs/ci-cd/

# 开发文档
mv docs/TESTING.md docs/development/
mv docs/TESTING_IMPLEMENTATION_REPORT.md docs/development/
mv docs/MACOS_BUILD_FIX.md docs/development/
mv docs/PGO_BUILD.md docs/development/
mv docs/clippy-cross-platform.md docs/development/

# 架构文档
mv docs/device-cache-enhancement.md docs/architecture/
mv docs/engine-selector-optimization.md docs/architecture/
mv docs/SKIP_EXISTING_OPTIMIZATION.md docs/architecture/
```

### 3. 删除过时文件
```bash
rm 任务清单.md
rm benchmark_results_512kb_optimization.md
rm CI_CD_OPTIMIZATION_REPORT.md
rm PERFORMANCE_BENCHMARK_OPTIMIZATION_REPORT.md
rm PERFORMANCE_BENCHMARKS.md
rm performance_optimization_summary.md
rm WORKSPACE.md
```

### 4. 创建文档索引
```bash
# 创建主文档索引
cat > docs/README.md << 'EOF'
# FerroCP 文档导航
...
EOF
```

## 预期结果

### 整理前
- 根目录: 8个散落的 .md 文件
- docs/: 12个文件混合在一起
- 总计: 20个需要整理的文档文件

### 整理后
- 根目录: 4个核心文件 (README.md, README_zh.md, CHANGELOG.md, CONTRIBUTING.md)
- docs/performance/: 1个文件
- docs/development/: 5个文件
- docs/ci-cd/: 3个文件
- docs/architecture/: 3个文件
- docs/guides/: 待创建
- 删除: 7个过时文件

## 好处

### 🎯 更好的组织结构
- 按功能分类的清晰目录结构
- 易于查找相关文档
- 减少根目录混乱

### 📚 改善文档体验
- 统一的文档导航
- 逻辑清晰的文档层次
- 更好的可维护性

### 🧹 减少维护负担
- 删除过时和重复文档
- 集中管理相关文档
- 更容易保持文档同步

## 风险评估

### ⚠️ 潜在风险
- 移动文件可能破坏现有链接
- 删除文件可能丢失有用信息

### 🛡️ 缓解措施
- 在移动前检查所有内部链接
- 备份要删除的文件内容
- 更新所有相关的链接引用
- 创建重定向或说明文件
