# CI Optimization Guide

This document explains how FerroCP's CI system is optimized to reduce queue times and improve development efficiency.

## 🚀 CI Strategy Overview

### Problem: macOS Runner Queue Issues

GitHub Actions macOS runners often have long queue times, especially during peak hours. This can significantly slow down development and testing cycles.

### Solution: Conditional and Separate Workflows

We've implemented a multi-tier CI strategy:

1. **Fast Core Testing** - Linux and Windows (always run)
2. **Conditional macOS Testing** - Only when needed
3. **VFX Platform Validation** - Comprehensive testing for releases

## 🔧 Workflow Structure

### Primary Workflows

| Workflow | File | Purpose | Triggers |
|----------|------|---------|----------|
| **Tests** | `test.yml` | Core functionality testing | All PRs, pushes |
| **macOS Tests** | `test-macos.yml` | macOS-specific testing | Conditional |
| **VFX Platform** | `vfx-platform-test.yml` | VFX compatibility | Manual, releases |

### Conditional Execution Rules

#### macOS Testing Triggers

macOS tests run automatically when:

- **Push to main/develop** - Always run for release branches
- **Manual trigger** - `workflow_dispatch` with options
- **PR with labels** - `test-macos`, `all-platforms`, `vfx-platform`
- **PR title keywords** - `[macos]`, `[all-platforms]`, `[vfx]`
- **macOS-specific changes** - Files containing `macos`, `darwin`, `apple`

#### Skipping macOS Tests

macOS tests are skipped for:

- Draft PRs (unless explicitly requested)
- Documentation-only changes
- Linux/Windows-specific changes
- Minor fixes that don't affect core functionality

## 🏷️ PR Labels for CI Control

### Available Labels

| Label | Effect | Use Case |
|-------|--------|----------|
| `test-macos` | Force macOS testing | macOS-specific changes |
| `all-platforms` | Test all platforms | Cross-platform changes |
| `vfx-platform` | Run VFX Platform tests | VFX compatibility validation |
| `skip-ci` | Skip non-essential tests | Documentation updates |
| `performance` | Run performance benchmarks | Performance-related changes |

### How to Use Labels

1. **Add labels when creating PR**:
   ```
   Title: Fix macOS compilation issue
   Labels: test-macos
   ```

2. **Add labels to existing PR**:
   - Go to PR page
   - Click "Labels" in the right sidebar
   - Select appropriate labels

3. **Use title keywords** (alternative to labels):
   ```
   [macos] Fix compilation on Apple Silicon
   [all-platforms] Update cross-platform file handling
   [vfx] Add VFX Platform CY2026 support
   ```

## ⚡ Performance Optimization Tips

### For Contributors

1. **Use specific labels** - Only trigger needed tests
2. **Draft PRs** - Use draft status for work-in-progress
3. **Descriptive titles** - Include platform keywords when relevant
4. **Small, focused PRs** - Easier to test and review

### For Maintainers

1. **Review CI needs** - Before merging, ensure appropriate testing
2. **Manual triggers** - Use `workflow_dispatch` for ad-hoc testing
3. **Release testing** - Run full VFX Platform tests before releases

## 🎬 VFX Platform Testing

### When to Run VFX Platform Tests

- **Before releases** - Ensure VFX compatibility
- **Major changes** - Cross-platform or performance updates
- **VFX-specific features** - New VFX workflow optimizations
- **Platform updates** - When updating supported platforms

### Manual VFX Platform Testing

```bash
# Trigger via GitHub UI
# Go to Actions → VFX Platform Compatibility Tests → Run workflow

# Or use GitHub CLI
gh workflow run vfx-platform-test.yml
```

## 📊 CI Performance Metrics

### Target Times

| Test Type | Target Duration | Actual (Typical) | Notes |
|-----------|----------------|------------------|-------|
| Linux Tests | < 5 minutes | ~3-4 minutes | Ubuntu 22.04, native + cross-compiled |
| Windows Tests | < 8 minutes | ~5-7 minutes | Windows Server 2022 |
| macOS Tests | < 15 minutes | ~10-20 minutes* | Conditional execution |
| VFX Platform | < 30 minutes | ~20-40 minutes* | Comprehensive validation |

*Queue time dependent

**Ubuntu 22.04 Migration**: We upgraded from Ubuntu 20.04 (retiring 2025-04-15) to Ubuntu 22.04, which provides better VFX Platform compatibility with glibc 2.35 and gcc 11.2+.

### Queue Time Optimization

1. **Avoid peak hours** - US/EU business hours have longer queues
2. **Use specific runners** - `macos-12` often faster than `macos-latest`
3. **Conditional execution** - Only run when necessary
4. **Parallel jobs** - Use matrix strategy efficiently

## 🔍 Troubleshooting

### Common Issues

1. **macOS tests not running**
   - Check PR labels
   - Verify title keywords
   - Ensure not a draft PR

2. **Long queue times**
   - Consider using labels to skip unnecessary tests
   - Run tests during off-peak hours
   - Use manual triggers for urgent testing

3. **Test failures**
   - Check platform-specific logs
   - Verify VFX Platform compatibility
   - Review recent changes for platform-specific issues

### Getting Help

- **CI Issues**: Check [GitHub Actions status](https://www.githubstatus.com/)
- **Platform Issues**: Review [VFX Platform docs](https://vfxplatform.com/)
- **Project Issues**: Open an issue with `ci` label

## 📝 Best Practices

### For Development

1. **Test locally first** - Use `cargo test` before pushing
2. **Use appropriate labels** - Help CI make smart decisions
3. **Monitor CI results** - Fix issues promptly
4. **Consider impact** - Think about which platforms are affected

### For Reviews

1. **Check CI status** - Ensure all required tests pass
2. **Verify platform coverage** - Confirm appropriate testing
3. **Performance impact** - Consider benchmark results
4. **VFX compatibility** - Validate VFX Platform compliance

---

*This optimization strategy helps maintain fast development cycles while ensuring comprehensive testing for VFX Platform compatibility.*
