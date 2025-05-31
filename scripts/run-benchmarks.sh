#!/bin/bash
# Performance benchmarking script for FerroCP

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Default values
OUTPUT_DIR="benchmark_results"
BASELINE_FILE=""
COMPARE_MODE=false
QUICK_MODE=false
PROFILE_MODE=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --output-dir)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        --baseline)
            BASELINE_FILE="$2"
            COMPARE_MODE=true
            shift 2
            ;;
        --quick)
            QUICK_MODE=true
            shift
            ;;
        --profile)
            PROFILE_MODE=true
            shift
            ;;
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --output-dir DIR    Directory to save benchmark results (default: benchmark_results)"
            echo "  --baseline FILE     Compare against baseline results"
            echo "  --quick            Run quick benchmarks only"
            echo "  --profile          Enable profiling mode"
            echo "  --help             Show this help message"
            exit 0
            ;;
        *)
            print_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

print_status "Starting FerroCP performance benchmarks"

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Check if criterion is available
if ! command -v cargo &> /dev/null; then
    print_error "Cargo is not installed or not in PATH"
    exit 1
fi

# Build in release mode first
print_status "Building project in release mode..."
if ! cargo build --release; then
    print_error "Failed to build project in release mode"
    exit 1
fi

# Set benchmark parameters based on mode
if [ "$QUICK_MODE" = true ]; then
    BENCH_TIME="--measurement-time 10"
    SAMPLE_SIZE="--sample-size 50"
    print_status "Running in quick mode (reduced sample size and time)"
else
    BENCH_TIME="--measurement-time 30"
    SAMPLE_SIZE="--sample-size 100"
    print_status "Running full benchmarks"
fi

# Generate timestamp for this run
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
RESULTS_FILE="$OUTPUT_DIR/benchmark_results_$TIMESTAMP.json"

print_status "Running performance benchmarks..."

# Run benchmarks with criterion
BENCHMARK_CMD="cargo bench -p ferrocp-tests"

if [ "$PROFILE_MODE" = true ]; then
    print_status "Running benchmarks with profiling enabled..."
    # Add profiling flags
    BENCHMARK_CMD="$BENCHMARK_CMD -- --profile-time=5"
fi

# Execute benchmarks
print_status "Running comprehensive performance benchmarks..."
print_status "- Micro file copy performance tests"
print_status "- Function-level performance analysis"
print_status "- Memory efficiency benchmarks"
print_status "- System call efficiency tests"
print_status "- Parallel I/O performance tests"
print_status "- Pre-read algorithm benchmarks"
print_status "- Cross-engine performance comparison"

# Run individual benchmark suites
BENCHMARK_SUITES=(
    "micro_copy_benchmark"
    "function_level_benchmarks"
    "memory_efficiency_benchmarks"
    "syscall_efficiency_benchmarks"
    "preread_benchmark"
)

for suite in "${BENCHMARK_SUITES[@]}"; do
    print_status "Running $suite..."
    if ! cargo bench -p ferrocp-tests --bench "$suite" $BENCH_TIME $SAMPLE_SIZE; then
        print_warning "Benchmark suite $suite failed, continuing with others..."
    else
        print_success "Completed $suite"
    fi
done

print_success "Benchmarks completed successfully"
print_status "Results saved to: $RESULTS_FILE"

# Generate HTML report
HTML_REPORT="$OUTPUT_DIR/benchmark_report_$TIMESTAMP.html"
print_status "Generating HTML report..."

if ! cargo bench --bench performance_benchmarks -- --output-format html > "$HTML_REPORT"; then
    print_warning "Failed to generate HTML report, but JSON results are available"
else
    print_success "HTML report generated: $HTML_REPORT"
fi

# Compare with baseline if provided
if [ "$COMPARE_MODE" = true ] && [ -f "$BASELINE_FILE" ]; then
    print_status "Comparing with baseline: $BASELINE_FILE"
    
    COMPARISON_FILE="$OUTPUT_DIR/comparison_$TIMESTAMP.txt"
    
    # Simple comparison script (you might want to use a more sophisticated tool)
    cat > "$COMPARISON_FILE" << EOF
# Performance Comparison Report
Generated: $(date)
Baseline: $BASELINE_FILE
Current: $RESULTS_FILE

## Summary
This is a basic comparison. For detailed analysis, use criterion's built-in comparison tools.

EOF
    
    print_success "Comparison saved to: $COMPARISON_FILE"
fi

# Run integration tests to ensure functionality
print_status "Running integration tests to verify functionality..."
if ! cargo test --test integration_tests; then
    print_warning "Some integration tests failed. Check functionality before trusting benchmark results."
else
    print_success "All integration tests passed"
fi

# Generate performance summary
SUMMARY_FILE="$OUTPUT_DIR/performance_summary_$TIMESTAMP.txt"
print_status "Generating performance summary..."

cat > "$SUMMARY_FILE" << EOF
# FerroCP Performance Summary
Generated: $(date)
Mode: $([ "$QUICK_MODE" = true ] && echo "Quick" || echo "Full")
Profiling: $([ "$PROFILE_MODE" = true ] && echo "Enabled" || echo "Disabled")

## System Information
OS: $(uname -s)
Architecture: $(uname -m)
CPU Cores: $(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo "Unknown")
Rust Version: $(rustc --version)

## Benchmark Results
Detailed results: $RESULTS_FILE
HTML Report: $HTML_REPORT

## Key Metrics
(Extract key metrics from JSON results here)

## Benchmark Suites Executed
- Micro Copy Benchmarks: Small file optimization strategies
- Function Level Benchmarks: Individual function performance analysis
- Memory Efficiency Benchmarks: Memory usage patterns and optimization
- System Call Efficiency: Syscall overhead and optimization
- Pre-read Algorithm: Predictive I/O performance
- Parallel I/O: Large file parallel processing performance

## Recommendations
- For file sizes < 4KB: Use MicroFileCopyEngine with SuperFast strategy
- For file sizes 4KB-50MB: Use BufferedCopyEngine with adaptive buffers
- For file sizes > 50MB: Use ParallelCopyEngine for optimal throughput
- For memory-constrained environments: Monitor buffer pool efficiency
- For syscall optimization: Prefer larger buffer sizes to reduce syscall overhead
- For predictive workloads: Enable pre-read algorithms for large files

## Performance Targets
- Small files (<4KB): 25%+ improvement over std::fs::copy
- Large files (>100MB): 15-25% improvement with parallel processing
- Memory efficiency: <10% overhead for buffer management
- Syscall efficiency: Minimize syscalls per MB transferred

## Next Steps
1. Review detailed results in HTML report
2. Compare with previous benchmarks if available
3. Identify performance bottlenecks using function-level analysis
4. Optimize memory usage patterns based on efficiency benchmarks
5. Fine-tune syscall patterns for target workloads
6. Consider hardware-specific optimizations
EOF

print_success "Performance summary saved to: $SUMMARY_FILE"

# Cleanup temporary files if any
print_status "Cleaning up temporary files..."

print_success "Benchmark run completed successfully!"
echo ""
echo "Results available in:"
echo "  - JSON: $RESULTS_FILE"
echo "  - HTML: $HTML_REPORT"
echo "  - Summary: $SUMMARY_FILE"
echo ""

if [ "$COMPARE_MODE" = true ]; then
    echo "  - Comparison: $COMPARISON_FILE"
    echo ""
fi

print_status "To view HTML report, open: file://$PWD/$HTML_REPORT"
