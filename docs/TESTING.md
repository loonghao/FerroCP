# FerroCP Testing Guide

This document describes the comprehensive testing strategy implemented for the FerroCP project, including property-based testing, fuzz testing, error handling tests, and coverage analysis.

## 🧪 Test Categories

### 1. Unit Tests
Traditional unit tests covering individual functions and modules across all crates.

**Status**: ✅ Complete (123 tests + 10 doc tests)
- All core crates have comprehensive unit test coverage
- Tests cover happy path scenarios and basic edge cases

### 2. Property-Based Tests
Property-based tests using the `proptest` crate to verify invariants across a wide range of inputs.

**Location**: `crates/ferrocp-io/src/property_tests.rs`
**Status**: ✅ Complete (6 property tests)

**Tests Include**:
- `test_adaptive_buffer_size_constraints`: Verifies buffer size constraints for different device types
- `test_buffer_adaptation_properties`: Tests buffer adaptation behavior with various performance metrics
- `test_smart_buffer_statistics`: Validates statistics consistency in SmartBuffer
- `test_buffer_pool_properties`: Tests BufferPool behavior under various conditions
- `fuzz_memory_mapped_file`: Fuzz tests memory-mapped file operations
- `fuzz_adaptive_buffer_operations`: Fuzz tests buffer operations with random data

**Run Command**:
```bash
cargo test --package ferrocp-io property_tests
```

### 3. Fuzz Tests
Fuzz testing for compression algorithms to ensure robustness with malformed or edge-case data.

**Location**: `crates/ferrocp-compression/src/fuzz_tests.rs`
**Status**: ✅ Complete (7 fuzz tests + stress tests)

**Tests Include**:
- `fuzz_compression_roundtrip`: Tests compression/decompression roundtrip with various data patterns
- `fuzz_adaptive_compressor`: Tests adaptive compressor with different data types
- `fuzz_compression_engine`: Tests compression engine with various configurations
- `fuzz_decompression_with_malformed_data`: Tests decompression with corrupted data
- `fuzz_extreme_parameters`: Tests with extreme compression parameters
- `fuzz_sequential_compression`: Tests sequential compression operations
- `stress_test_large_data`: Stress tests with large data sets (up to 10MB)
- `stress_test_rapid_cycles`: Tests rapid compression/decompression cycles

**Run Command**:
```bash
cargo test --package ferrocp-compression fuzz_tests
```

### 4. Error Handling Tests
Comprehensive error handling tests to ensure graceful failure and recovery.

**Locations**: 
- `crates/ferrocp-io/src/error_tests.rs` (12 tests)
- `crates/ferrocp-compression/src/error_tests.rs` (10 tests)

**Status**: ✅ Complete (22 error handling tests)

**ferrocp-io Error Tests**:
- Invalid path errors
- Permission denied scenarios
- Corrupted memory-mapped files
- Buffer pool edge cases
- Adaptive buffer edge cases
- Disk space errors
- Concurrent access errors
- Error context preservation
- Error recovery scenarios
- Malformed data handling
- Device-specific errors
- Resource cleanup under error conditions

**ferrocp-compression Error Tests**:
- Invalid compression data
- Corrupted compression headers
- Truncated compression data
- Extremely large data handling
- Algorithm-specific error conditions
- Adaptive compressor errors
- Memory pressure scenarios
- Error recovery and state consistency
- Timeout scenarios
- Malformed algorithm configurations
- Resource cleanup under error conditions

**Run Commands**:
```bash
cargo test --package ferrocp-io error_tests
cargo test --package ferrocp-compression error_tests
```

## 🛠️ Test Tools and Scripts

### Test Runner Script
A PowerShell script to run specific test categories:

```powershell
# Run all new test suites
.\scripts\run-tests.ps1

# Run specific test categories
.\scripts\run-tests.ps1 -PropertyTests
.\scripts\run-tests.ps1 -FuzzTests
.\scripts\run-tests.ps1 -ErrorTests
.\scripts\run-tests.ps1 -All
```

### Coverage Analysis
Scripts for generating test coverage reports:

**PowerShell (Windows)**:
```powershell
.\scripts\test-coverage.ps1 --all --html
```

**Bash (Linux/macOS)**:
```bash
./scripts/test-coverage.sh --all --html
```

**Coverage Configuration**:
- Tool: `cargo-tarpaulin`
- Output formats: HTML, XML, JSON
- Timeout: 120 seconds
- Excludes: test files, benchmarks, target directory

## 📊 Coverage Goals

### Current Status
- **Unit Tests**: 100% pass rate (123/123 + 10 doc tests)
- **Property Tests**: 100% pass rate (6/6 tests)
- **Fuzz Tests**: 100% pass rate (7+ tests)
- **Error Tests**: 100% pass rate (22/22 tests)

### Coverage Targets
- **Line Coverage**: >90% (target)
- **Branch Coverage**: >85% (target)
- **Function Coverage**: >95% (target)

### Quality Metrics
- **Excellent**: ≥90% coverage 🎉
- **Good**: ≥80% coverage ✅
- **Acceptable**: ≥70% coverage ⚠️
- **Needs Improvement**: <70% coverage ❌

## 🚀 Running Tests

### Quick Test Commands

```bash
# Run all tests
cargo test --workspace

# Run tests for specific crate
cargo test --package ferrocp-io
cargo test --package ferrocp-compression

# Run specific test categories
cargo test --package ferrocp-io property_tests
cargo test --package ferrocp-compression fuzz_tests
cargo test --package ferrocp-io error_tests
cargo test --package ferrocp-compression error_tests

# Run tests with verbose output
cargo test --package ferrocp-io property_tests --verbose
```

### Performance Testing
```bash
# Run tests in release mode for better performance
cargo test --package ferrocp-io property_tests --release
cargo test --package ferrocp-compression fuzz_tests --release
```

## 🔧 Test Configuration

### Dependencies
The following testing dependencies are configured in workspace:

```toml
[workspace.dependencies]
proptest = "1.4"           # Property-based testing
arbitrary = "1.3"          # Arbitrary data generation
tokio-test = "0.4"         # Async testing utilities
rstest = "0.18"            # Parameterized testing
tempfile = "3.8"           # Temporary file utilities
```

### Test Features
- **Property-based testing**: Using `proptest` for generating test cases
- **Fuzz testing**: Custom fuzz tests for compression algorithms
- **Async testing**: Using `tokio-test` for async test utilities
- **Parameterized testing**: Using `rstest` for data-driven tests
- **Temporary resources**: Using `tempfile` for safe test isolation

## 📈 Continuous Improvement

### Next Steps
1. **Increase Coverage**: Target >90% line coverage across all crates
2. **Performance Benchmarks**: Add performance regression tests
3. **Integration Tests**: Add cross-crate integration tests
4. **Platform Testing**: Ensure tests pass on all target platforms
5. **CI Integration**: Integrate coverage reporting into CI/CD pipeline

### Monitoring
- Regular coverage reports generation
- Performance regression detection
- Test execution time monitoring
- Flaky test identification and resolution

---

*Last updated: 2024-12-19*
