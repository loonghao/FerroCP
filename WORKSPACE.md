# FerroCP Workspace Architecture

This document describes the modern multi-crate architecture of FerroCP.

## 🏗️ Workspace Structure

FerroCP is organized as a Cargo workspace with the following crates:

### Core Foundation
- **`ferrocp-types`** - Core type system, error handling, and shared data structures
- **`ferrocp-config`** - Configuration management with validation and hot-reload

### Engine Components  
- **`ferrocp-io`** - High-performance async I/O engine with smart buffering
- **`ferrocp-device`** - Intelligent device detection and platform optimization
- **`ferrocp-zerocopy`** - Next-generation zero-copy operations with hardware acceleration
- **`ferrocp-compression`** - Adaptive compression system with ML optimization
- **`ferrocp-network`** - Modern network communication (QUIC/HTTP3)
- **`ferrocp-sync`** - Intelligent incremental synchronization

### Integration Layer
- **`ferrocp-engine`** - Main copy engine integrating all components
- **`ferrocp-cli`** - Modern CLI tool with interactive interface
- **`ferrocp-python`** - Python API bindings with async support

## 🚀 Key Features

### Modern Architecture
- **Workspace-based**: Clean separation of concerns
- **Type-safe**: Leverages Rust's type system for safety
- **Async-first**: Built on modern async/await patterns
- **Zero-copy**: Optimized for maximum performance

### Development Experience
- **Fast builds**: Optimized compilation with incremental builds
- **Rich tooling**: Comprehensive linting, formatting, and testing
- **CI/CD ready**: GitHub Actions integration
- **Documentation**: Auto-generated docs with examples

### Performance Optimizations
- **Profile-guided optimization**: PGO support for maximum performance
- **Target-specific**: CPU-specific optimizations
- **Memory efficient**: Smart memory management and pooling
- **Hardware acceleration**: Leverages modern CPU and I/O features

## 🛠️ Development Commands

### Quick Start
```bash
# Check all crates
cargo dev-check

# Run all tests
cargo dev-test

# Build everything
cargo build-all

# Run CLI tool
cargo run --package ferrocp-cli
```

### Testing
```bash
# Unit tests
cargo test-unit

# Integration tests  
cargo test-integration

# Benchmarks
cargo bench-all
```

### Documentation
```bash
# Generate and open docs
cargo doc-all

# Include private items
cargo doc-private
```

## 📦 Crate Dependencies

```
ferrocp-types (foundation)
├── ferrocp-config
├── ferrocp-io
├── ferrocp-device  
├── ferrocp-zerocopy
├── ferrocp-compression
├── ferrocp-network
├── ferrocp-sync
└── ferrocp-engine (integration)
    ├── ferrocp-cli
    └── ferrocp-python
```

## 🎯 Design Principles

1. **Modularity**: Each crate has a single, well-defined responsibility
2. **Performance**: Zero-cost abstractions and optimal algorithms
3. **Safety**: Memory safety without garbage collection
4. **Portability**: Cross-platform support with platform-specific optimizations
5. **Extensibility**: Plugin system and configurable components

## 🔧 Build Profiles

- **`dev`**: Fast compilation for development
- **`dev-fast`**: Optimized development builds
- **`release`**: Production builds with full optimization
- **`release-pgo`**: Profile-guided optimization builds
- **`bench`**: Benchmark builds with debug info

## 📈 Performance Targets

- **Throughput**: >10 GB/s on modern NVMe storage
- **Latency**: <1ms for small file operations
- **Memory**: <100MB baseline memory usage
- **CPU**: Efficient multi-core utilization

## 🧪 Testing Strategy

- **Unit tests**: Each crate has comprehensive unit tests
- **Integration tests**: Cross-crate functionality testing
- **Benchmarks**: Performance regression testing
- **Property tests**: Fuzz testing for edge cases
- **Platform tests**: Multi-platform CI testing
