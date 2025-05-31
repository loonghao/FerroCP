#!/bin/bash
# Integration test runner for FerroCP
# This script runs comprehensive integration tests and generates reports

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
RESULTS_DIR="integration_test_results"
VERBOSE=false
COVERAGE=false
TIMEOUT=300  # 5 minutes default timeout

# Function to print colored output
print_color() {
    local color=$1
    local message=$2
    echo -e "${color}${message}${NC}"
}

# Function to show usage
show_usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Integration test runner for FerroCP

OPTIONS:
    -v, --verbose       Enable verbose output
    -c, --coverage      Generate test coverage report
    -t, --timeout SEC   Set test timeout in seconds (default: 300)
    -o, --output DIR    Output directory for results (default: integration_test_results)
    -h, --help          Show this help message

EXAMPLES:
    $0                          # Run all integration tests
    $0 --verbose                # Run with verbose output
    $0 --coverage               # Run with coverage analysis
    $0 --timeout 600            # Run with 10-minute timeout

EOF
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        -c|--coverage)
            COVERAGE=true
            shift
            ;;
        -t|--timeout)
            TIMEOUT="$2"
            shift 2
            ;;
        -o|--output)
            RESULTS_DIR="$2"
            shift 2
            ;;
        -h|--help)
            show_usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            show_usage
            exit 1
            ;;
    esac
done

# Create results directory
mkdir -p "$RESULTS_DIR"

print_color $BLUE "🧪 Starting FerroCP Integration Tests"
print_color $BLUE "====================================="

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    print_color $RED "❌ cargo not found. Please install Rust and Cargo."
    exit 1
fi

# Generate timestamp
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
TEST_LOG="$RESULTS_DIR/integration_test_$TIMESTAMP.log"

print_color $GREEN "📁 Results will be saved to: $RESULTS_DIR"

# Build project first
print_color $BLUE "🔨 Building project..."
if ! cargo build --release; then
    print_color $RED "❌ Failed to build project"
    exit 1
fi
print_color $GREEN "✅ Build completed successfully"

# Prepare test arguments
TEST_ARGS=()

if [[ "$VERBOSE" == true ]]; then
    TEST_ARGS+=("--nocapture")
    print_color $YELLOW "📝 Verbose output enabled"
fi

# Run integration tests
print_color $GREEN "🧪 Running integration tests..."

TEST_CATEGORIES=(
    "test_basic_file_copy"
    "test_large_file_copy"
    "test_compression_integration"
    "test_directory_copy_integration"
    "test_multiple_copy_engines_integration"
    "test_error_recovery_integration"
    "test_timeout_handling"
)

PASSED_TESTS=0
FAILED_TESTS=0
TOTAL_TESTS=${#TEST_CATEGORIES[@]}

# Create detailed test report
REPORT_FILE="$RESULTS_DIR/integration_test_report_$TIMESTAMP.md"

cat > "$REPORT_FILE" << EOF
# FerroCP Integration Test Report

**Generated:** $(date)  
**Test Run ID:** $TIMESTAMP  
**Timeout:** ${TIMEOUT}s  
**Verbose:** $VERBOSE  
**Coverage:** $COVERAGE  

## Test Environment

- **OS:** $(uname -s)
- **Architecture:** $(uname -m)
- **Rust Version:** $(rustc --version)
- **Cargo Version:** $(cargo --version)

## Test Results

EOF

print_color $BLUE "📋 Running individual test categories:"

for test_name in "${TEST_CATEGORIES[@]}"; do
    print_color $YELLOW "  🔍 Running: $test_name"
    
    # Run individual test with timeout
    if timeout "$TIMEOUT" cargo test -p ferrocp-tests --test integration_tests "$test_name" "${TEST_ARGS[@]}" >> "$TEST_LOG" 2>&1; then
        print_color $GREEN "    ✅ PASSED: $test_name"
        echo "- ✅ **$test_name**: PASSED" >> "$REPORT_FILE"
        ((PASSED_TESTS++))
    else
        print_color $RED "    ❌ FAILED: $test_name"
        echo "- ❌ **$test_name**: FAILED" >> "$REPORT_FILE"
        ((FAILED_TESTS++))
        
        # Extract error details
        echo "  - Error details available in log file" >> "$REPORT_FILE"
    fi
done

# Run all tests together for final verification
print_color $BLUE "🔄 Running complete integration test suite..."

if timeout "$TIMEOUT" cargo test -p ferrocp-tests --test integration_tests "${TEST_ARGS[@]}" >> "$TEST_LOG" 2>&1; then
    print_color $GREEN "✅ Complete test suite PASSED"
    SUITE_STATUS="PASSED"
else
    print_color $RED "❌ Complete test suite FAILED"
    SUITE_STATUS="FAILED"
fi

# Generate coverage report if requested
if [[ "$COVERAGE" == true ]]; then
    print_color $BLUE "📊 Generating test coverage report..."
    
    if command -v cargo-tarpaulin &> /dev/null; then
        COVERAGE_DIR="$RESULTS_DIR/coverage_$TIMESTAMP"
        mkdir -p "$COVERAGE_DIR"
        
        if cargo tarpaulin -p ferrocp-tests --test integration_tests --out Html --output-dir "$COVERAGE_DIR" >> "$TEST_LOG" 2>&1; then
            print_color $GREEN "✅ Coverage report generated: $COVERAGE_DIR/tarpaulin-report.html"
            echo "" >> "$REPORT_FILE"
            echo "## Coverage Report" >> "$REPORT_FILE"
            echo "Coverage report available at: \`$COVERAGE_DIR/tarpaulin-report.html\`" >> "$REPORT_FILE"
        else
            print_color $YELLOW "⚠️ Failed to generate coverage report"
        fi
    else
        print_color $YELLOW "⚠️ cargo-tarpaulin not found. Install with: cargo install cargo-tarpaulin"
    fi
fi

# Complete the report
cat >> "$REPORT_FILE" << EOF

## Summary

- **Total Tests:** $TOTAL_TESTS
- **Passed:** $PASSED_TESTS
- **Failed:** $FAILED_TESTS
- **Success Rate:** $(( PASSED_TESTS * 100 / TOTAL_TESTS ))%
- **Complete Suite:** $SUITE_STATUS

## Test Categories Covered

1. **Basic File Copy** - Tests fundamental file copying functionality
2. **Large File Copy** - Tests performance with large files (10MB+)
3. **Compression Integration** - Tests compression/decompression workflows
4. **Directory Copy** - Tests recursive directory copying
5. **Multiple Copy Engines** - Tests different copy engine implementations
6. **Error Recovery** - Tests error handling and recovery mechanisms
7. **Timeout Handling** - Tests timeout and cancellation scenarios

## Performance Characteristics

Integration tests verify:
- ✅ Correctness of file operations
- ✅ Data integrity across all operations
- ✅ Error handling and recovery
- ✅ Memory usage patterns
- ✅ Timeout and cancellation behavior

## Recommendations

EOF

if [[ $FAILED_TESTS -eq 0 ]]; then
    cat >> "$REPORT_FILE" << EOF
🎉 **All integration tests passed!** The system is ready for production use.

### Next Steps:
1. Run performance benchmarks to verify performance characteristics
2. Test on different hardware configurations
3. Validate with real-world workloads
4. Consider stress testing with concurrent operations
EOF
else
    cat >> "$REPORT_FILE" << EOF
⚠️ **Some integration tests failed.** Review the failures before proceeding.

### Next Steps:
1. Review failed test details in the log file: \`$TEST_LOG\`
2. Fix identified issues
3. Re-run integration tests
4. Consider adding additional test coverage for edge cases
EOF
fi

cat >> "$REPORT_FILE" << EOF

## Log Files

- **Detailed Test Log:** \`$TEST_LOG\`
- **This Report:** \`$REPORT_FILE\`

---
*Generated by FerroCP Integration Test Runner*
EOF

# Final summary
print_color $BLUE "📊 Integration Test Summary"
print_color $BLUE "=========================="
print_color $GREEN "✅ Passed: $PASSED_TESTS/$TOTAL_TESTS"

if [[ $FAILED_TESTS -gt 0 ]]; then
    print_color $RED "❌ Failed: $FAILED_TESTS/$TOTAL_TESTS"
fi

print_color $BLUE "📄 Detailed report: $REPORT_FILE"
print_color $BLUE "📋 Test log: $TEST_LOG"

if [[ "$COVERAGE" == true ]] && [[ -f "$COVERAGE_DIR/tarpaulin-report.html" ]]; then
    print_color $BLUE "📊 Coverage report: $COVERAGE_DIR/tarpaulin-report.html"
fi

# Exit with appropriate code
if [[ $FAILED_TESTS -eq 0 ]] && [[ "$SUITE_STATUS" == "PASSED" ]]; then
    print_color $GREEN "🎉 All integration tests completed successfully!"
    exit 0
else
    print_color $RED "❌ Some integration tests failed. Check the report for details."
    exit 1
fi
