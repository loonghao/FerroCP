# Profile-Guided Optimization (PGO) Builds

This document explains how to use Profile-Guided Optimization (PGO) to build high-performance versions of ferrocp.

## What is PGO?

Profile-Guided Optimization (PGO) is a compiler optimization technique that uses runtime profiling data to optimize code generation. The process involves:

1. **Profile Generation**: Build the code with instrumentation to collect runtime data
2. **Profile Collection**: Run the instrumented code with representative workloads
3. **Profile-Guided Compilation**: Rebuild the code using the collected profile data for optimization

PGO typically provides 5-15% performance improvements for CPU-intensive applications.

## Prerequisites

- **Rust toolchain** with LLVM tools
- **uv** package manager
- **maturin** for building Rust extensions

## Building with PGO

### Automated PGO Build

The easiest way to build with PGO is using the nox session:

```bash
# Build with PGO optimization (recommended)
uv run nox -s build_pgo

# Or using make
make build-pgo
```

This will automatically:
1. Build an instrumented version
2. Run representative workloads to collect profile data
3. Rebuild with optimizations based on the profile data

### Manual PGO Build

For more control over the PGO process:

```bash
# Step 1: Build with profile generation
RUSTFLAGS="-Cprofile-generate=./pgo-data" uv run maturin build --release

# Step 2: Install and run workloads
pip install target/wheels/*.whl --force-reinstall
python -c "
import ferrocp
import tempfile
from pathlib import Path

# Run representative file operations
with tempfile.TemporaryDirectory() as temp_dir:
    temp_path = Path(temp_dir)
    source_dir = temp_path / 'source'
    dest_dir = temp_path / 'dest'
    source_dir.mkdir()
    dest_dir.mkdir()
    
    # Create test files
    for i in range(100):
        test_file = source_dir / f'test_{i}.txt'
        test_file.write_text(f'Test content {i}' * 1000)
    
    # Run copy operations
    eacopy = ferrocp.EACopy()
    for i in range(50):
        eacopy.copy_file(
            source_dir / f'test_{i}.txt',
            dest_dir / f'test_{i}.txt'
        )
"

# Step 3: Merge profile data
llvm-profdata merge -o merged.profdata pgo-data/*.profraw

# Step 4: Build with profile use
RUSTFLAGS="-Cprofile-use=./merged.profdata" uv run maturin build --release
```

## PGO in CI/CD

### GitHub Actions Integration

PGO builds are automatically used in our CI/CD pipeline:

```yaml
# .github/workflows/codspeed.yml
- name: Build PGO-optimized project
  uses: ./.github/actions/build-pgo-wheel
  with:
    interpreter: python3.11
    rust-toolchain: stable
```

### CodSpeed Integration

Our CodSpeed benchmarks use PGO-optimized builds for accurate performance measurement:

```bash
# CodSpeed automatically uses PGO builds
uv run nox -s codspeed
```

## Performance Benefits

### Expected Improvements

- **File copy operations**: 5-10% faster
- **Directory traversal**: 8-15% faster
- **Compression/decompression**: 10-20% faster
- **Overall throughput**: 5-15% improvement

### Benchmark Comparison

```bash
# Compare PGO vs regular build
uv run nox -s build        # Regular optimized build
uv run nox -s build_pgo    # PGO optimized build
uv run nox -s benchmark    # Run benchmarks to compare
```

## Troubleshooting

### Common Issues

1. **LLVM tools not found**
   ```bash
   # Install Rust with complete toolchain
   rustup component add llvm-tools-preview
   ```

2. **Profile data collection fails**
   ```bash
   # Ensure sufficient disk space and permissions
   mkdir -p pgo-data
   chmod 755 pgo-data
   ```

3. **Build fails with profile use**
   ```bash
   # Check profile data exists and is valid
   ls -la pgo-data/
   llvm-profdata show merged.profdata
   ```

### Debug Mode

Enable debug logging to troubleshoot PGO builds:

```bash
RUST_LOG=debug uv run nox -s build_pgo
```

## Best Practices

### Profile Data Collection

1. **Representative Workloads**: Use realistic file sizes and operations
2. **Diverse Scenarios**: Include different file types and directory structures
3. **Sufficient Coverage**: Run enough operations to collect meaningful data

### Build Environment

1. **Consistent Environment**: Use the same environment for profile collection and final build
2. **Clean State**: Start with a clean build directory
3. **Resource Availability**: Ensure sufficient disk space for profile data

### Performance Validation

1. **Benchmark Before/After**: Always compare performance with regular builds
2. **Multiple Runs**: Run benchmarks multiple times for statistical significance
3. **Real-World Testing**: Test with actual use cases, not just synthetic benchmarks

## Integration with Development Workflow

### Local Development

```bash
# Quick development cycle
make dev-setup          # Install dependencies and build
make dev-test           # Run tests and linting
make dev-benchmark      # Quick performance check

# Performance optimization cycle
make build-pgo          # Build with PGO
make benchmark          # Full performance testing
make verify-build       # Verify functionality
```

### Release Process

1. **Development**: Use regular builds for fast iteration
2. **Performance Testing**: Use PGO builds for accurate benchmarking
3. **Release**: Create PGO-optimized wheels for distribution

## Advanced Configuration

### Custom Profile Collection

Create custom workloads for specific use cases:

```python
# custom_profile_workload.py
import ferrocp
import tempfile
from pathlib import Path

def collect_profile_data():
    """Custom workload for profile data collection."""
    eacopy = ferrocp.EACopy()
    
    # Your specific use case scenarios
    # - Large file operations
    # - Network transfers
    # - Compression scenarios
    # - etc.
    
    pass

if __name__ == "__main__":
    collect_profile_data()
```

### Cargo Profile Configuration

The project includes PGO-specific Cargo profiles:

```toml
# Cargo.toml
[profile.release-pgo]
inherits = "release"
lto = "fat"
codegen-units = 1
panic = "abort"
strip = true
```

Use with:

```bash
cargo build --profile release-pgo
```

## References

- [Rust PGO Documentation](https://doc.rust-lang.org/rustc/profile-guided-optimization.html)
- [LLVM PGO Guide](https://llvm.org/docs/HowToBuildWithPGO.html)
- [PyO3 Performance Tips](https://pyo3.rs/latest/performance.html)
