#!/bin/bash
# Test Coverage Script for ferrocp
# This script runs comprehensive tests and generates coverage reports

set -e

# Default values
OUTPUT_DIR="target/coverage"
HTML=false
XML=false
JSON=false
PROPERTY_TESTS=false
FUZZ_TESTS=false
ERROR_TESTS=false
ALL=false

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_color() {
    local color=$1
    local message=$2
    echo -e "${color}${message}${NC}"
}

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --output-dir)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        --html)
            HTML=true
            shift
            ;;
        --xml)
            XML=true
            shift
            ;;
        --json)
            JSON=true
            shift
            ;;
        --property-tests)
            PROPERTY_TESTS=true
            shift
            ;;
        --fuzz-tests)
            FUZZ_TESTS=true
            shift
            ;;
        --error-tests)
            ERROR_TESTS=true
            shift
            ;;
        --all)
            ALL=true
            shift
            ;;
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo "Options:"
            echo "  --output-dir DIR    Output directory for coverage reports (default: target/coverage)"
            echo "  --html             Generate HTML coverage report"
            echo "  --xml              Generate XML coverage report"
            echo "  --json             Generate JSON coverage report"
            echo "  --property-tests   Run property tests"
            echo "  --fuzz-tests       Run fuzz tests"
            echo "  --error-tests      Run error handling tests"
            echo "  --all              Run all tests and generate all report formats"
            echo "  --help             Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Check if cargo-tarpaulin is installed
if ! command_exists cargo-tarpaulin; then
    print_color $YELLOW "Installing cargo-tarpaulin..."
    cargo install cargo-tarpaulin
fi

# Create output directory
mkdir -p "$OUTPUT_DIR"

print_color $BLUE "🧪 Running ferrocp test coverage analysis..."

# Build tarpaulin command
TARPAULIN_ARGS=(
    "tarpaulin"
    "--workspace"
    "--timeout" "120"
    "--output-dir" "$OUTPUT_DIR"
)

# Add output formats
OUTPUT_FORMATS=()
if [[ "$HTML" == true ]] || [[ "$ALL" == true ]]; then
    OUTPUT_FORMATS+=("Html")
fi
if [[ "$XML" == true ]] || [[ "$ALL" == true ]]; then
    OUTPUT_FORMATS+=("Xml")
fi
if [[ "$JSON" == true ]] || [[ "$ALL" == true ]]; then
    OUTPUT_FORMATS+=("Json")
fi

# Default to HTML if no format specified
if [[ ${#OUTPUT_FORMATS[@]} -eq 0 ]]; then
    OUTPUT_FORMATS=("Html")
fi

# Add output formats to command
for format in "${OUTPUT_FORMATS[@]}"; do
    TARPAULIN_ARGS+=("--out" "$format")
done

# Test categories
TEST_CATEGORIES=()
if [[ "$PROPERTY_TESTS" == true ]] || [[ "$ALL" == true ]]; then
    TEST_CATEGORIES+=("property_tests")
fi
if [[ "$FUZZ_TESTS" == true ]] || [[ "$ALL" == true ]]; then
    TEST_CATEGORIES+=("fuzz_tests")
fi
if [[ "$ERROR_TESTS" == true ]] || [[ "$ALL" == true ]]; then
    TEST_CATEGORIES+=("error_tests")
fi

# Run coverage analysis
if [[ ${#TEST_CATEGORIES[@]} -eq 0 ]] || [[ "$ALL" == true ]]; then
    print_color $GREEN "Running all tests with coverage..."
    
    # Run comprehensive test coverage
    cargo "${TARPAULIN_ARGS[@]}" \
        --all-features \
        --exclude-files "*/tests/*" \
        --exclude-files "*/benches/*"
else
    # Run specific test categories
    for category in "${TEST_CATEGORIES[@]}"; do
        print_color $GREEN "Running $category with coverage..."
        cargo "${TARPAULIN_ARGS[@]}" --test "$category"
    done
fi

# Run property tests separately if requested
if [[ "$PROPERTY_TESTS" == true ]] || [[ "$ALL" == true ]]; then
    print_color $GREEN "Running property tests for ferrocp-io..."
    cargo test --package ferrocp-io property_tests --release
fi

# Run fuzz tests separately if requested  
if [[ "$FUZZ_TESTS" == true ]] || [[ "$ALL" == true ]]; then
    print_color $GREEN "Running fuzz tests for ferrocp-compression..."
    cargo test --package ferrocp-compression fuzz_tests --release
fi

# Run error handling tests separately if requested
if [[ "$ERROR_TESTS" == true ]] || [[ "$ALL" == true ]]; then
    print_color $GREEN "Running error handling tests..."
    cargo test --package ferrocp-io error_tests --release
    cargo test --package ferrocp-compression error_tests --release
fi

# Generate summary report
print_color $BLUE "\n📊 Test Coverage Summary"
print_color $BLUE "========================="

if [[ -f "$OUTPUT_DIR/tarpaulin-report.html" ]]; then
    print_color $GREEN "✅ HTML coverage report: $OUTPUT_DIR/tarpaulin-report.html"
fi

if [[ -f "$OUTPUT_DIR/cobertura.xml" ]]; then
    print_color $GREEN "✅ XML coverage report: $OUTPUT_DIR/cobertura.xml"
fi

if [[ -f "$OUTPUT_DIR/tarpaulin-report.json" ]]; then
    print_color $GREEN "✅ JSON coverage report: $OUTPUT_DIR/tarpaulin-report.json"
fi

# Parse coverage percentage from HTML report if available
if [[ -f "$OUTPUT_DIR/tarpaulin-report.html" ]]; then
    if command_exists grep && command_exists sed; then
        COVERAGE_PERCENT=$(grep -o 'Coverage: [0-9]*\.?[0-9]*%' "$OUTPUT_DIR/tarpaulin-report.html" | sed 's/Coverage: \([0-9]*\.?[0-9]*\)%/\1/')
        
        if [[ -n "$COVERAGE_PERCENT" ]]; then
            print_color $GREEN "📈 Overall Coverage: ${COVERAGE_PERCENT}%"
            
            # Coverage quality assessment
            if (( $(echo "$COVERAGE_PERCENT >= 90" | bc -l) )); then
                print_color $GREEN "🎉 Excellent coverage!"
            elif (( $(echo "$COVERAGE_PERCENT >= 80" | bc -l) )); then
                print_color $GREEN "✅ Good coverage"
            elif (( $(echo "$COVERAGE_PERCENT >= 70" | bc -l) )); then
                print_color $YELLOW "⚠️  Acceptable coverage"
            else
                print_color $RED "❌ Coverage needs improvement"
            fi
        fi
    fi
fi

print_color $BLUE "\n🎯 Test Categories Completed:"
if [[ "$PROPERTY_TESTS" == true ]] || [[ "$ALL" == true ]]; then
    print_color $GREEN "  ✅ Property Tests (ferrocp-io)"
fi
if [[ "$FUZZ_TESTS" == true ]] || [[ "$ALL" == true ]]; then
    print_color $GREEN "  ✅ Fuzz Tests (ferrocp-compression)"
fi
if [[ "$ERROR_TESTS" == true ]] || [[ "$ALL" == true ]]; then
    print_color $GREEN "  ✅ Error Handling Tests"
fi

print_color $GREEN "\n🏁 Coverage analysis completed successfully!"

# Open HTML report if available and on macOS
if [[ "$HTML" == true ]] && [[ -f "$OUTPUT_DIR/tarpaulin-report.html" ]] && [[ "$OSTYPE" == "darwin"* ]]; then
    print_color $BLUE "Opening coverage report in browser..."
    open "$OUTPUT_DIR/tarpaulin-report.html"
fi
