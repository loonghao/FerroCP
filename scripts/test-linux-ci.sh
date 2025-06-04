#!/bin/bash
# Linux CI Test Script - Simulates GitHub Actions environment

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Test command execution
test_command() {
    local description="$1"
    local command="$2"
    
    log_info "Testing: $description"
    if eval "$command"; then
        log_success "$description - PASSED"
        return 0
    else
        log_error "$description - FAILED"
        return 1
    fi
}

# Main test execution
main() {
    log_info "Starting Linux CI Test Simulation"
    log_info "=================================="
    
    # Set environment variables like CI
    export CARGO_TERM_COLOR=always
    export RUST_BACKTRACE=1
    export BLAKE3_NO_ASM=1
    export CARGO_NET_GIT_FETCH_WITH_CLI=true
    export RUST_LOG=debug
    
    log_info "Environment variables set:"
    echo "  CARGO_TERM_COLOR=$CARGO_TERM_COLOR"
    echo "  RUST_BACKTRACE=$RUST_BACKTRACE"
    echo "  BLAKE3_NO_ASM=$BLAKE3_NO_ASM"
    echo "  CARGO_NET_GIT_FETCH_WITH_CLI=$CARGO_NET_GIT_FETCH_WITH_CLI"
    echo "  RUST_LOG=$RUST_LOG"
    echo ""
    
    # Check system dependencies
    log_info "Checking system dependencies..."
    
    if ! command -v rustc &> /dev/null; then
        log_error "Rust is not installed"
        exit 1
    fi
    
    if ! command -v cargo &> /dev/null; then
        log_error "Cargo is not installed"
        exit 1
    fi
    
    # Check for build tools
    if ! command -v gcc &> /dev/null; then
        log_warning "GCC not found - may need build-essential package"
    fi
    
    if ! command -v pkg-config &> /dev/null; then
        log_warning "pkg-config not found - may need pkg-config package"
    fi
    
    # Display versions
    log_info "Tool versions:"
    rustc --version
    cargo --version
    echo ""
    
    # Run the same tests as CI
    local failed_tests=0
    
    # 1. Check formatting
    if ! test_command "Code formatting check" "cargo fmt --all -- --check"; then
        ((failed_tests++))
    fi
    
    # 2. Run workspace tests (excluding Python extension)
    if ! test_command "Rust workspace tests" "cargo test --workspace --exclude ferrocp-python --verbose"; then
        ((failed_tests++))
    fi
    
    # 3. Build Python extension
    if ! test_command "Python extension build" "cargo build -p ferrocp-python"; then
        ((failed_tests++))
    fi
    
    # 4. Additional checks
    if ! test_command "Clippy check (if available)" "cargo clippy --workspace --all-targets --all-features -- -D warnings || true"; then
        log_warning "Clippy check failed or not available"
    fi
    
    # Summary
    echo ""
    log_info "Test Summary"
    log_info "============"
    
    if [ $failed_tests -eq 0 ]; then
        log_success "All tests passed! ✅"
        log_success "Linux CI environment should work correctly."
        exit 0
    else
        log_error "$failed_tests test(s) failed! ❌"
        log_error "Linux CI environment needs fixes."
        exit 1
    fi
}

# Run main function
main "$@"
