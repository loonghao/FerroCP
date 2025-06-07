# Future Language Bindings for FerroCP

This document outlines the architecture and design for future language bindings for FerroCP, including Python and C++ support.

## Architecture Overview

```text
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Python API    │    │    C++ API      │    │   C API         │
│   (PyO3)        │    │   (Modern C++)  │    │   (Direct)      │
└─────────────────┘    └─────────────────┘    └─────────────────┘
          │                       │                       │
          └───────────────────────┼───────────────────────┘
                                  │
                        ┌─────────▼─────────┐
                        │   FerroCP FFI     │
                        │   (C-ABI Layer)   │
                        └─────────┬─────────┘
                                  │
                        ┌─────────▼─────────┐
                        │  FerroCP Engine   │
                        │  (Core Logic)     │
                        └───────────────────┘
```

## Current Status

### ✅ Completed
- **Core Engine**: Fully functional Rust implementation
- **CLI Interface**: Complete command-line tool
- **C-ABI Layer**: `ferrocp-ffi` crate with C-compatible interface
- **C Headers**: Complete C header file for integration
- **C++ Wrapper**: Modern C++ wrapper with RAII and exceptions

### 🚧 In Progress
- **JSON Output**: Implemented for automated testing and integration

### 📋 Planned
- **Python Bindings**: PyO3-based Python package
- **C++ Library**: Compiled C++ library with CMake support
- **Documentation**: Comprehensive API documentation for all languages

## C-ABI Interface Design

The `ferrocp-ffi` crate provides a stable C-ABI interface that serves as the foundation for all language bindings:

### Key Features
- **Memory Safety**: Proper ownership and lifetime management
- **Error Handling**: C-style error codes with detailed error information
- **Thread Safety**: Safe to call from multiple threads
- **Async Support**: Handles async operations transparently
- **Callback Support**: Progress and error callbacks for real-time feedback

### Core Functions
```c
// Library management
int ferrocp_init(void);
void ferrocp_cleanup(void);
const char* ferrocp_version(void);

// Engine management
ferrocp_engine_handle_t ferrocp_engine_create(void);
void ferrocp_engine_destroy(ferrocp_engine_handle_t handle);

// Copy operations
int ferrocp_copy(ferrocp_engine_handle_t handle, 
                 const ferrocp_copy_request_t* request,
                 ferrocp_result_t* result);

// Device information
int ferrocp_get_device_info(const char* path, 
                           ferrocp_device_info_t* device_info);
```

## Python Bindings Design

### Package Structure
```
ferrocp/
├── __init__.py          # Main package interface
├── engine.py            # Engine class
├── types.py             # Type definitions and enums
├── exceptions.py        # Exception classes
├── callbacks.py         # Callback support
├── utils.py             # Utility functions
└── _ferrocp.pyd         # Native extension module
```

### Python API Example
```python
import ferrocp

# Initialize library
ferrocp.init()

try:
    # Create engine
    engine = ferrocp.Engine()
    
    # Create copy request
    request = ferrocp.CopyRequest(
        source="source_dir",
        destination="dest_dir",
        mode=ferrocp.CopyMode.COPY,
        compress=True,
        preserve_metadata=True
    )
    
    # Execute copy with progress callback
    def progress_callback(percent, bytes_copied, total_bytes, current_file):
        print(f"Progress: {percent:.1f}% - {current_file}")
    
    stats = engine.copy(request, progress_callback=progress_callback)
    print(f"Copied {stats.files_copied} files ({stats.bytes_copied} bytes)")
    
    # Get device information
    device_info = ferrocp.get_device_info("/path/to/check")
    print(f"Device type: {device_info.device_type}")
    print(f"Read speed: {device_info.read_speed_mbps} MB/s")

finally:
    ferrocp.cleanup()
```

### Python Features
- **Pythonic API**: Following Python conventions and best practices
- **Type Hints**: Full type annotation support
- **Async Support**: Optional asyncio integration
- **Context Managers**: RAII-style resource management
- **Exception Handling**: Python exceptions mapped from C error codes
- **Progress Callbacks**: Real-time progress reporting
- **JSON Integration**: Direct support for JSON output format

## C++ Library Design

### Header Structure
```cpp
#include <ferrocp/ferrocp.hpp>

// Modern C++ API with RAII, exceptions, and STL
namespace ferrocp {
    class Library;      // RAII library management
    class Engine;       // RAII engine management
    class CopyRequest;  // Request builder pattern
    struct CopyStats;   // Statistics
    struct DeviceInfo;  // Device information
}
```

### C++ API Example
```cpp
#include <ferrocp/ferrocp.hpp>
#include <iostream>

int main() {
    try {
        // RAII library initialization
        ferrocp::Library lib;
        
        // Create engine
        ferrocp::Engine engine;
        
        // Create copy request
        ferrocp::CopyRequest request("source", "destination");
        request.mode = ferrocp::CopyMode::Copy;
        request.compress = true;
        request.preserve_metadata = true;
        
        // Execute copy with lambda callback
        auto stats = engine.copy_with_progress(request,
            [](double percent, uint64_t copied, uint64_t total, const std::string& file) {
                std::cout << "Progress: " << percent << "% - " << file << std::endl;
            }
        );
        
        std::cout << "Copied " << stats.files_copied << " files" << std::endl;
        
        // Get device information
        auto device_info = ferrocp::get_device_info("/path/to/check");
        std::cout << "Device type: " << static_cast<int>(device_info.device_type) << std::endl;
        
    } catch (const ferrocp::Exception& e) {
        std::cerr << "FerroCP error: " << e.what() << std::endl;
        return 1;
    }
    
    return 0;
}
```

### C++ Features
- **Modern C++17/20**: Using latest C++ features
- **RAII**: Automatic resource management
- **Exception Safety**: Strong exception safety guarantees
- **STL Integration**: Using standard containers and algorithms
- **Template Support**: Generic programming support
- **CMake Integration**: Easy integration with CMake projects

## Build System Integration

### Cargo Features
```toml
[features]
default = ["cli"]
cli = ["ferrocp-cli"]
ffi = ["ferrocp-ffi"]
python = ["ferrocp-ffi", "pyo3"]
cpp = ["ferrocp-ffi", "cxx"]
all-bindings = ["python", "cpp"]
```

### CMake Support (Future)
```cmake
find_package(FerroCP REQUIRED)
target_link_libraries(my_app FerroCP::ferrocp)
```

### Python Package (Future)
```bash
pip install ferrocp
```

## Testing Strategy

### Unit Tests
- **C-ABI Tests**: Test all FFI functions directly
- **Python Tests**: pytest-based test suite
- **C++ Tests**: Google Test or Catch2-based tests
- **Integration Tests**: Cross-language integration tests

### Performance Tests
- **Benchmark Suite**: Consistent benchmarks across all languages
- **Memory Tests**: Memory usage and leak detection
- **Concurrency Tests**: Multi-threaded safety tests

### CI/CD Integration
- **Multi-language Builds**: Build and test all bindings
- **Cross-platform Testing**: Windows, Linux, macOS
- **Performance Regression**: Automated performance monitoring

## Documentation Plan

### API Documentation
- **C API**: Doxygen-generated documentation
- **Python API**: Sphinx-generated documentation with examples
- **C++ API**: Doxygen-generated documentation
- **Tutorials**: Step-by-step guides for each language

### Examples
- **Basic Usage**: Simple copy operations
- **Advanced Features**: Compression, verification, callbacks
- **Integration Examples**: Real-world usage scenarios
- **Performance Optimization**: Best practices for each language

## Migration Path

### Phase 1: Foundation (Current)
- ✅ Core engine implementation
- ✅ C-ABI interface
- ✅ C/C++ headers
- ✅ Basic testing

### Phase 2: Python Bindings
- 🔄 Re-enable ferrocp-python crate
- 🔄 PyO3 integration
- 🔄 Python package structure
- 🔄 Python-specific tests

### Phase 3: C++ Library
- 📋 CMake build system
- 📋 C++ library compilation
- 📋 Package manager integration (vcpkg, Conan)
- 📋 C++ examples and documentation

### Phase 4: Advanced Features
- 📋 Async Python API
- 📋 C++ coroutine support
- 📋 Advanced callback mechanisms
- 📋 Performance optimization

## Compatibility Guarantees

### ABI Stability
- **C-ABI**: Stable interface with versioning
- **Semantic Versioning**: Clear compatibility promises
- **Deprecation Policy**: Gradual migration for breaking changes

### Platform Support
- **Windows**: Full support for all bindings
- **Linux**: Full support for all bindings
- **macOS**: Full support for all bindings
- **Architecture**: x86_64, ARM64 support

This architecture ensures that FerroCP can be easily integrated into projects using any of these languages while maintaining performance, safety, and ease of use.
