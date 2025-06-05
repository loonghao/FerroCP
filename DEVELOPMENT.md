# FerroCP Development Guide

This document provides guidelines and tools for developing FerroCP.

## Quick Start

### 1. Setup Development Environment

```bash
# Clone the repository
git clone https://github.com/loonghao/ferrocp.git
cd ferrocp

# Set up development environment (installs git hooks, checks tools)
make setup
```

### 2. Development Workflow

Before committing any code, always run:

```bash
# Check code formatting
make format-check

# Or fix formatting automatically
make format

# Run all pre-commit checks
make pre-commit
```

## Code Quality Requirements

### Formatting

All code must be properly formatted using `rustfmt`:

```bash
# Check formatting (required before commit)
cargo fmt --all -- --check

# Fix formatting
cargo fmt --all
```

### Linting

Code must pass clippy checks:

```bash
# Run clippy
cargo clippy --workspace --exclude ferrocp-python --all-targets --all-features -- -D warnings
```

### Testing

All tests must pass:

```bash
# Run Rust tests (fast)
cargo test --workspace --exclude ferrocp-python

# Run all tests including Python
make test
```

## Available Commands

### Development Setup
- `make setup` - Set up development environment and git hooks
- `make install` - Install Python dependencies

### Code Quality
- `make format` - Format all code
- `make format-check` - Check code formatting (required before commit)
- `make rust-lint` - Run Rust linting
- `make rust-fix` - Fix Rust code issues automatically
- `make pre-commit` - Run all pre-commit checks

### Testing
- `make test` - Run all tests
- `make test-rust` - Run Rust tests only (faster)

### Building
- `make build` - Build Python extension
- `make benchmark` - Run performance benchmarks

## Git Hooks

The development environment includes a pre-commit hook that automatically:

1. ✅ Checks code formatting
2. ✅ Verifies compilation
3. ✅ Runs clippy checks
4. ⚠️ Warns about TODO/FIXME comments
5. ⚠️ Warns about debug prints

### Bypassing Hooks

If you need to commit without running hooks (not recommended):

```bash
git commit --no-verify
```

## Platform-Specific Notes

### Linux

The project includes special handling for Linux environments:

- Sets `BLAKE3_NO_ASM=1` to avoid assembly compatibility issues
- Uses gold linker to avoid `ld` issues: `RUSTFLAGS="-C link-arg=-fuse-ld=gold"`

### Windows

Use PowerShell scripts for formatting:

```powershell
# Check formatting
.\scripts\format-check.ps1 -Check

# Fix formatting
.\scripts\format-check.ps1 -Fix
```

## CI/CD

### GitHub Actions

The CI pipeline runs:

1. **Format Check** - Ensures all code is properly formatted
2. **Tests** - Runs all tests on Linux
3. **Build** - Builds Python extension

### Local CI Simulation

Test your changes locally before pushing:

```bash
# Run the same checks as CI
make format-check
make rust-lint
make test-rust
```

## Troubleshooting

### Common Issues

#### 1. Format Check Fails

```bash
error: rustfmt failed to format
```

**Solution**: Run `make format` to fix formatting issues.

#### 2. Linux Linker Issues

```bash
collect2: fatal error: cannot find 'ld'
```

**Solution**: The CI configuration handles this automatically. For local development:

```bash
export RUSTFLAGS="-C link-arg=-fuse-ld=gold"
export BLAKE3_NO_ASM=1
```

#### 3. Blake3 Assembly Issues

```bash
error: failed to run custom build command for `blake3`
```

**Solution**: Set the environment variable:

```bash
export BLAKE3_NO_ASM=1
```

### Getting Help

1. Check this development guide
2. Look at the Makefile for available commands
3. Check CI logs for similar issues
4. Ask in project discussions

## Best Practices

### Before Committing

1. ✅ Run `make format-check`
2. ✅ Run `make rust-lint`
3. ✅ Run `make test-rust`
4. ✅ Write meaningful commit messages
5. ✅ Keep commits focused and atomic

### Code Style

- Follow Rust standard formatting (enforced by rustfmt)
- Write clear, self-documenting code
- Add tests for new functionality
- Update documentation when needed

### Performance

- Run benchmarks for performance-critical changes
- Use `make benchmark-quick` for development
- Check performance regression with `make benchmark-compare`

## Environment Variables

- `BLAKE3_NO_ASM=1` - Disable Blake3 assembly optimizations (required on some Linux systems)
- `RUSTFLAGS="-C link-arg=-fuse-ld=gold"` - Use gold linker on Linux
- `VERBOSE=1` - Enable verbose output for debugging

## Tools and Dependencies

### Required
- Rust (latest stable)
- Cargo
- Git

### Automatically Installed
- rustfmt (code formatting)
- clippy (linting)

### Optional
- uv (Python package manager)
- nox (Python testing)

---

For more information, see the main [README.md](README.md) file.
