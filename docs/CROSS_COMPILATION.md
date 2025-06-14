# Cross-Compilation with cargo-zigbuild

FerroCP uses [cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild) for optimized cross-compilation across VFX Platform supported targets. This provides better cross-compilation experience compared to traditional methods.

## Why cargo-zigbuild?

- **Simplified Setup**: No need for complex cross-compilation toolchains
- **Better Compatibility**: Zig's built-in cross-compilation capabilities
- **Consistent Results**: Same binary output across different build environments
- **VFX Platform Support**: Optimized for VFX industry standard targets

## Supported Targets

FerroCP supports all VFX Platform CY2025 targets:

| Target | Platform | Architecture | Notes |
|--------|----------|--------------|-------|
| `x86_64-unknown-linux-gnu` | Linux | x86_64 | VFX Platform CY2025 |
| `aarch64-unknown-linux-gnu` | Linux | ARM64 | Modern VFX workflows |
| `x86_64-apple-darwin` | macOS | x86_64 | macOS 12.0+ |
| `aarch64-apple-darwin` | macOS | ARM64 | Apple Silicon |
| `x86_64-pc-windows-msvc` | Windows | x86_64 | VS 2022 v17.6+ |

## Quick Start

### Prerequisites

1. **Rust toolchain** (latest stable)
2. **Python 3.8+** (for zig installation)

### Build All Targets

```bash
# Using Makefile (recommended)
make build-cross

# Using PowerShell script directly
powershell -ExecutionPolicy Bypass -File scripts/zigbuild-cross.ps1

# Using shell script directly (Linux/macOS)
./scripts/zigbuild-cross.sh
```

### Build Release Versions

```bash
# Using Makefile
make build-cross-release

# Using PowerShell script
powershell -ExecutionPolicy Bypass -File scripts/zigbuild-cross.ps1 -Release

# Using shell script
./scripts/zigbuild-cross.sh --release
```

### Build Specific Target

```bash
# Using Makefile
make build-cross-target TARGET=x86_64-unknown-linux-gnu

# Using PowerShell script
powershell -ExecutionPolicy Bypass -File scripts/zigbuild-cross.ps1 -Target x86_64-unknown-linux-gnu

# Using shell script
./scripts/zigbuild-cross.sh --target x86_64-unknown-linux-gnu
```

## Script Options

### PowerShell Script (`scripts/zigbuild-cross.ps1`)

```powershell
# Build all targets in debug mode
./scripts/zigbuild-cross.ps1

# Build all targets in release mode
./scripts/zigbuild-cross.ps1 -Release

# Build specific target
./scripts/zigbuild-cross.ps1 -Target "x86_64-unknown-linux-gnu"

# Verbose output
./scripts/zigbuild-cross.ps1 -Verbose

# Clean before building
./scripts/zigbuild-cross.ps1 -Clean
```

### Shell Script (`scripts/zigbuild-cross.sh`)

```bash
# Build all targets in debug mode
./scripts/zigbuild-cross.sh

# Build all targets in release mode
./scripts/zigbuild-cross.sh --release

# Build specific target
./scripts/zigbuild-cross.sh --target x86_64-unknown-linux-gnu

# Verbose output
./scripts/zigbuild-cross.sh --verbose

# Clean before building
./scripts/zigbuild-cross.sh --clean

# Show help
./scripts/zigbuild-cross.sh --help
```

## CI/CD Integration

The GoReleaser configuration automatically uses cargo-zigbuild for all cross-compilation builds:

```yaml
# .goreleaser.yml
before:
  hooks:
    - cargo install --locked cargo-zigbuild
    - pip install ziglang
    # ... other setup

builds:
  - hooks:
      pre: cargo zigbuild --bin ferrocp --target x86_64-unknown-linux-gnu --release
```

## Troubleshooting

### Common Issues

1. **zig not found**
   ```bash
   pip install ziglang
   ```

2. **cargo-zigbuild not found**
   ```bash
   cargo install --locked cargo-zigbuild
   ```

3. **Target not added**
   ```bash
   rustup target add <target-triple>
   ```

### Performance Tips

- Use `--release` for production builds
- Use `--clean` if you encounter linking issues
- Enable verbose output (`-v` or `--verbose`) for debugging

## Comparison with Traditional Cross-Compilation

| Method | Setup Complexity | Build Speed | Compatibility | Maintenance |
|--------|------------------|-------------|---------------|-------------|
| cargo-zigbuild | Low | Fast | High | Low |
| cross | Medium | Medium | Medium | Medium |
| Manual toolchains | High | Fast | Low | High |

## Advanced Usage

### Custom Zig Version

```bash
# Install specific zig version
pip install ziglang==0.11.0
```

### Environment Variables

```bash
# Disable ASM optimizations for compatibility
export BLAKE3_NO_ASM=1

# Custom deployment target for macOS
export MACOSX_DEPLOYMENT_TARGET=12.0
```

### Integration with Other Tools

cargo-zigbuild works seamlessly with:
- GoReleaser
- GitHub Actions
- Docker builds
- Local development

## References

- [cargo-zigbuild GitHub](https://github.com/rust-cross/cargo-zigbuild)
- [Zig Cross-Compilation](https://ziglang.org/learn/overview/#cross-compiling-is-a-first-class-use-case)
- [VFX Platform](https://vfxplatform.com/)
