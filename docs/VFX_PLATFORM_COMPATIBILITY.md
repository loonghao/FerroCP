# VFX Platform Compatibility

FerroCP is designed to be fully compatible with the [VFX Reference Platform](https://vfxplatform.com/) standards, ensuring seamless integration into professional VFX and animation pipelines.

## Supported Platforms

### Current VFX Platform Support (CY2025)

| Platform | Architecture | Minimum Version | VFX Platform Standard |
|----------|-------------|-----------------|----------------------|
| **Linux** | x86_64 | glibc 2.28+, gcc 11.2.1+ | ✅ CY2025 Compatible |
| **Linux** | ARM64 | glibc 2.28+, gcc 11.2.1+ | ✅ Modern VFX Workflows |
| **macOS** | x86_64 | macOS 12.0+ | ✅ CY2025 Compatible |
| **macOS** | ARM64 (Apple Silicon) | macOS 14.0+ | ✅ CY2025+ Compatible |
| **Windows** | x86_64 | VS 2022 v17.6+, SDK 10.0.20348+ | ✅ CY2025 Compatible |

### Future VFX Platform Support (CY2026 Draft)

| Platform | Architecture | Planned Version | VFX Platform Standard |
|----------|-------------|-----------------|----------------------|
| **Linux** | x86_64 | glibc 2.34+, gcc 14.2+ | 🔄 CY2026 Draft |
| **macOS** | x86_64/ARM64 | macOS 14.0+ | 🔄 CY2026 Draft |
| **Windows** | x86_64 | VS 2022 v17.6+ | 🔄 CY2026 Draft |

## Technical Specifications

### Compiler Requirements

- **Linux**: GCC 11.2.1+ with new libstdc++ ABI (`_GLIBCXX_USE_CXX11_ABI=1`)
- **macOS**: Xcode with deployment target support for specified macOS versions
- **Windows**: Visual Studio 2022 v17.6+ with Windows SDK 10.0.20348+

### Runtime Dependencies

- **Python**: 3.11.x (CY2025), 3.13.x (CY2026 Draft)
- **C++ Standard**: C++17 (CY2025), C++20 (CY2026 Draft)
- **Rust**: 1.75+ (MSRV for modern async/await features)

### Performance Optimizations

FerroCP includes VFX-specific optimizations:

- **Large File Handling**: Optimized for typical VFX file sizes (GB+ assets)
- **Network Transfer**: Efficient handling of render farm distributions
- **Compression**: Support for VFX-friendly compression algorithms
- **Memory Management**: Optimized for high-memory VFX workstations

## Integration Guidelines

### Studio Pipeline Integration

1. **Render Farm Deployment**: Use Linux x86_64 binaries for compute nodes
2. **Artist Workstations**: Use platform-specific binaries (macOS/Windows)
3. **Cloud Workflows**: ARM64 support for cost-effective cloud instances

### Software Compatibility

FerroCP is tested for compatibility with:

- **DCC Applications**: Maya, Houdini, Nuke, Blender
- **Render Engines**: Arnold, V-Ray, RenderMan, Cycles
- **Pipeline Tools**: OpenPype, Shotgun/Flow Production Tracking

## Testing and Validation

### Continuous Integration

Our CI pipeline tests on:

- **Ubuntu 22.04** (glibc 2.35, gcc 11.2+) - **Exceeds VFX Platform CY2025 requirements**
  - Native x86_64 builds
  - Cross-compiled ARM64 builds using `gcc-aarch64-linux-gnu`
- **macOS 12+** (Intel) and **macOS 14+** (Apple Silicon)
- **Windows Server 2022** (VS 2022, Windows SDK 10.0.20348+)

**Note**: We upgraded from Ubuntu 20.04 to Ubuntu 22.04 due to the [scheduled retirement](https://github.com/actions/runner-images/issues/11101) on 2025-04-15. Ubuntu 22.04 provides even better VFX Platform compatibility with newer toolchain versions.

### Performance Benchmarks

Regular performance testing ensures:

- **Throughput**: Maintains high transfer speeds across all platforms
- **Reliability**: Consistent behavior in VFX production environments
- **Resource Usage**: Efficient memory and CPU utilization

## Version Support Policy

Following VFX Platform guidance:

- **Current Year + 3 Previous Years**: Full support and updates
- **Older Versions**: Best-effort compatibility, no new features
- **Breaking Changes**: Only with major VFX Platform transitions

## Getting Help

For VFX Platform specific issues:

1. Check our [GitHub Issues](https://github.com/loonghao/ferrocp/issues)
2. Review [VFX Platform Documentation](https://vfxplatform.com/)
3. Contact the VFX community via [vfx-platform-discuss](https://groups.google.com/g/vfx-platform-discuss)

---

*This document is updated regularly to reflect the latest VFX Platform standards and FerroCP compatibility status.*
