# FerroCP 文档导航

欢迎来到 FerroCP 文档中心！这里包含了项目的所有技术文档，按功能分类组织。

## 📚 文档目录

### 🚀 快速开始
- [项目介绍](../README.md) - 项目概述和特性介绍
- [中文说明](../README_zh.md) - 中文版项目说明
- [变更日志](../CHANGELOG.md) - 版本更新记录
- [贡献指南](../CONTRIBUTING.md) - 如何参与项目开发

### 🏗️ 开发文档
- [测试指南](development/TESTING.md) - 测试框架和测试策略
- [测试实现报告](development/TESTING_IMPLEMENTATION_REPORT.md) - 测试系统实现详情
- [macOS 构建修复](development/MACOS_BUILD_FIX.md) - macOS 平台构建问题解决方案
- [PGO 构建指南](development/PGO_BUILD.md) - Profile-Guided Optimization 构建
- [Clippy 跨平台配置](development/clippy-cross-platform.md) - 跨平台 Clippy 配置

### 🔧 CI/CD 文档
- [GoReleaser 配置](ci-cd/GORELEASER.md) - 自动化发布配置详解
- [CI 清理计划](ci-cd/CI_CLEANUP_PLAN.md) - CI/CD 工作流优化计划
- [CodSpeed 设置](ci-cd/codspeed-setup.md) - 性能监控服务配置

### 🏛️ 架构设计
- [设备缓存增强](architecture/device-cache-enhancement.md) - 设备缓存优化设计
- [引擎选择器优化](architecture/engine-selector-optimization.md) - 复制引擎选择策略
- [跳过现有文件优化](architecture/SKIP_EXISTING_OPTIMIZATION.md) - 文件跳过逻辑优化

### 📊 性能分析
- [性能报告](performance/PERFORMANCE_REPORT.md) - 详细的性能测试报告
- [基准测试](../benchmarks/README.md) - 基准测试框架和结果

### 📖 其他资源
- [工作流说明](../.github/workflows/README.md) - GitHub Actions 工作流文档
- [安全政策](../.github/SECURITY.md) - 安全漏洞报告指南
- [行为准则](../.github/CODE_OF_CONDUCT.md) - 社区行为准则

## 🗂️ 文档结构

```
docs/
├── README.md                    # 本文档 - 文档导航
├── index.md                     # 文档首页
├── MARKDOWN_CLEANUP_PLAN.md     # 文档整理计划
├── architecture/                # 架构设计文档
│   ├── device-cache-enhancement.md
│   ├── engine-selector-optimization.md
│   └── SKIP_EXISTING_OPTIMIZATION.md
├── ci-cd/                       # CI/CD 相关文档
│   ├── CI_CLEANUP_PLAN.md
│   ├── codspeed-setup.md
│   └── GORELEASER.md
├── development/                 # 开发相关文档
│   ├── clippy-cross-platform.md
│   ├── MACOS_BUILD_FIX.md
│   ├── PGO_BUILD.md
│   ├── TESTING.md
│   └── TESTING_IMPLEMENTATION_REPORT.md
├── guides/                      # 用户指南 (待完善)
└── performance/                 # 性能分析文档
    └── PERFORMANCE_REPORT.md
```

## 🎯 文档维护

### 文档更新原则
1. **及时性**: 代码变更时同步更新相关文档
2. **准确性**: 确保文档内容与实际实现一致
3. **完整性**: 重要功能都应有对应文档
4. **可读性**: 使用清晰的结构和语言

### 文档分类标准
- **architecture/**: 系统设计、架构决策、技术方案
- **development/**: 开发环境、构建、测试、调试
- **ci-cd/**: 持续集成、持续部署、自动化流程
- **performance/**: 性能测试、优化报告、基准测试
- **guides/**: 用户指南、教程、最佳实践

### 贡献文档
如果您想为文档做出贡献：

1. 查看 [贡献指南](../CONTRIBUTING.md)
2. 确定文档应该放在哪个分类下
3. 遵循现有文档的格式和风格
4. 提交 Pull Request 进行审查

## 📞 获取帮助

如果您在使用过程中遇到问题：

1. 首先查看相关文档
2. 搜索 [GitHub Issues](https://github.com/loonghao/FerroCP/issues)
3. 如果问题未解决，请创建新的 Issue
4. 对于安全相关问题，请查看 [安全政策](../.github/SECURITY.md)

## 🔄 文档版本

本文档结构于 2024年12月 重新整理，旨在提供更好的文档组织和用户体验。

---

**注意**: 如果您发现文档中的错误或过时信息，请通过 GitHub Issues 报告或直接提交 Pull Request 修复。
