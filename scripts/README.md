# FerroCP CI Scripts

This directory contains scripts to help ensure your code passes CI checks before pushing to the repository.

## Overview

The CI pipeline runs several checks based on `.github/workflows/ci.yml`:

1. **Format Check** - `cargo fmt --all -- --check`
2. **Clippy Check** - `cargo clippy --workspace --all-targets --all-features -- -D clippy::correctness -D clippy::suspicious -D clippy::complexity -W clippy::perf -W clippy::style`
3. **Tests** - `cargo test --workspace` (lib, integration and doc tests, `ferrocp-python` included)
4. **Build Test** - GoReleaser build simulation (`test-goreleaser.yml`, not part of `ci.yml`)
5. **Cargo.lock Check** - `cargo metadata --locked` (`cargo-lock.yml`, not part of `ci.yml`)

Benchmarks are compiled by the check above (`--all-targets`) but never executed
in CI: they are criterion harnesses, not correctness checks. Run them locally
with `cargo bench --workspace`.

## Scripts

### 🚀 Quick Check (Recommended for Development)

**Windows:**
```powershell
.\scripts\quick-ci-check.ps1
```

**Linux/macOS:**
```bash
./scripts/quick-ci-check.sh --help  # Coming soon
```

**Features:**
- Fast execution (< 2 minutes)
- Essential checks only
- Auto-fix option with `-Fix`
- Perfect for development workflow

### 🔍 Full CI Check (Before Push)

**Windows:**
```powershell
.\scripts\local-ci-check.ps1
```

**Linux/macOS:**
```bash
./scripts/local-ci-check.sh
```

**Features:**
- Complete CI simulation
- All checks that run in CI
- Detailed reporting
- Cross-platform compatible

### Options

Both scripts support these options:

| Option | Description |
|--------|-------------|
| `--skip-format` | Skip format check |
| `--skip-clippy` | Skip clippy check |
| `--skip-tests` | Skip unit and integration tests |
| `--skip-build` | Skip build test |
| `--skip-bench` | Skip benchmark tests |
| `--verbose` | Enable verbose output |
| `--fix` | Auto-fix formatting issues |
| `--help` | Show help message |

## Recommended Workflow

### 1. During Development
```powershell
# Quick check before committing
.\scripts\quick-ci-check.ps1

# Auto-fix formatting issues
.\scripts\quick-ci-check.ps1 -Fix
```

### 2. Before Pushing
```powershell
# Full CI validation
.\scripts\local-ci-check.ps1

# If issues found, fix them and re-run
.\scripts\local-ci-check.ps1 -Fix
```

### 3. Troubleshooting
```powershell
# Skip problematic checks temporarily
.\scripts\local-ci-check.ps1 --skip-bench --verbose

# Focus on specific checks
.\scripts\local-ci-check.ps1 --skip-tests --skip-build
```

## CI Environment Matching

These scripts are designed to match the CI environment as closely as possible:

- **Environment Variables**: `CARGO_TERM_COLOR=always`, `RUST_BACKTRACE=1`
- **Rust Components**: Ensures `rustfmt` and `clippy` are installed
- **Command Flags**: Uses exact same cargo commands as CI
- **Exclusions**: Excludes `ferrocp-python` crate (disabled in CI)

## Platform Differences

### Windows (PowerShell)
- Uses `.ps1` scripts
- PowerShell-specific error handling
- Windows-specific binary paths (`.exe`)

### Linux/macOS (Bash)
- Uses `.sh` scripts
- POSIX-compliant commands
- Unix-specific binary paths

### Clippy Differences
The scripts are more lenient with clippy warnings for local development to avoid cross-platform inconsistencies. CI may be stricter.

## Performance Tips

### Quick Development Cycle
```powershell
# Format only
.\scripts\quick-ci-check.ps1 -Fix

# Tests only
.\scripts\local-ci-check.ps1 --skip-format --skip-clippy --skip-build --skip-bench
```

### Full Validation
```powershell
# Complete check (may take 5-10 minutes)
.\scripts\local-ci-check.ps1 --verbose
```

## Troubleshooting

### Common Issues

1. **Format Check Fails**
   ```powershell
   .\scripts\quick-ci-check.ps1 -Fix
   ```

2. **Clippy Warnings**
   ```powershell
   cargo clippy --workspace --all-targets --all-features --fix
   ```

3. **Test Failures**
   ```powershell
   cargo test --workspace --exclude ferrocp-python --verbose
   ```

4. **Build Issues**
   ```powershell
   cargo clean
   cargo build --workspace --exclude ferrocp-python --release
   ```

### Environment Issues

1. **Missing Rust Components**
   ```bash
   rustup component add rustfmt clippy
   ```

2. **Outdated Rust**
   ```bash
   rustup update stable
   ```

3. **Cache Issues**
   ```bash
   cargo clean
   rm -rf target/
   ```

## Integration with Git Hooks

You can set up these scripts as git hooks:

### Pre-commit Hook
```bash
#!/bin/sh
# .git/hooks/pre-commit
./scripts/quick-ci-check.sh --fix
```

### Pre-push Hook
```bash
#!/bin/sh
# .git/hooks/pre-push
./scripts/local-ci-check.sh
```

## CI Status Matching

After running these scripts successfully, your code should:
- ✅ Pass format checks in CI
- ✅ Pass clippy checks in CI (with some platform variance)
- ✅ Pass unit tests in CI
- ✅ Pass integration tests in CI
- ✅ Build successfully in CI
- ✅ Pass benchmark tests in CI

This ensures a smooth CI experience and reduces failed builds.
