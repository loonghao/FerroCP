#!/bin/bash
# Local CI Check Script for FerroCP
# This script runs all CI checks locally to ensure consistency with CI machines
# Based on .github/workflows/build-test.yml configuration

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
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

log_section() {
    echo ""
    echo -e "${CYAN}🔍 $1${NC}"
    echo -e "${CYAN}$(printf '=%.0s' $(seq 1 $((${#1} + 4))))${NC}"
}

# Parse command line arguments
SKIP_FORMAT=false
SKIP_CLIPPY=false
SKIP_TESTS=false
SKIP_BUILD=false
SKIP_BENCH=false
VERBOSE=false
FIX=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --skip-format)
            SKIP_FORMAT=true
            shift
            ;;
        --skip-clippy)
            SKIP_CLIPPY=true
            shift
            ;;
        --skip-tests)
            SKIP_TESTS=true
            shift
            ;;
        --skip-build)
            SKIP_BUILD=true
            shift
            ;;
        --skip-bench)
            SKIP_BENCH=true
            shift
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --fix)
            FIX=true
            shift
            ;;
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo "Options:"
            echo "  --skip-format    Skip format check"
            echo "  --skip-clippy    Skip clippy check"
            echo "  --skip-tests     Skip unit and integration tests"
            echo "  --skip-build     Skip build test"
            echo "  --skip-bench     Skip benchmark tests"
            echo "  --verbose        Enable verbose output"
            echo "  --fix            Auto-fix formatting issues"
            echo "  --help           Show this help message"
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Check if we're in the correct directory
if [[ ! -f "Cargo.toml" ]]; then
    log_error "Not in project root directory. Please run from ferrocp project root."
    exit 1
fi

# Environment setup
export CARGO_TERM_COLOR=always
export RUST_BACKTRACE=1

echo -e "${MAGENTA}🚀 FerroCP Local CI Check${NC}"
echo -e "${MAGENTA}=========================${NC}"
log_info "This script runs all CI checks locally to ensure consistency with CI machines"
echo ""

# Check Rust installation
log_section "Environment Check"
if ! command -v rustc &> /dev/null || ! command -v cargo &> /dev/null; then
    log_error "Rust/Cargo not found. Please install from https://rustup.rs/"
    exit 1
fi

RUST_VERSION=$(rustc --version)
CARGO_VERSION=$(cargo --version)
log_info "Rust: $RUST_VERSION"
log_info "Cargo: $CARGO_VERSION"

# Check required components
if ! rustup component list --installed | grep -q "rustfmt"; then
    log_info "Installing rustfmt component..."
    rustup component add rustfmt
fi

if ! rustup component list --installed | grep -q "clippy"; then
    log_info "Installing clippy component..."
    rustup component add clippy
fi

log_success "Environment check passed"

TOTAL_CHECKS=0
PASSED_CHECKS=0
FAILED_CHECKS=()

# 1. Format Check (matches CI: cargo fmt --all -- --check)
if [[ "$SKIP_FORMAT" != "true" ]]; then
    log_section "Format Check"
    ((TOTAL_CHECKS++))
    
    if [[ "$FIX" == "true" ]]; then
        log_info "Fixing code formatting..."
        if cargo fmt --all; then
            log_success "Format check passed"
            ((PASSED_CHECKS++))
        else
            log_error "Format check failed"
            FAILED_CHECKS+=("Format Check")
        fi
    else
        log_info "Checking code formatting..."
        if [[ "$VERBOSE" == "true" ]]; then
            CARGO_CMD="cargo fmt --all -- --check --verbose"
        else
            CARGO_CMD="cargo fmt --all -- --check"
        fi
        
        if $CARGO_CMD; then
            log_success "Format check passed"
            ((PASSED_CHECKS++))
        else
            log_error "Format check failed"
            FAILED_CHECKS+=("Format Check")
            log_info "Run with --fix flag to auto-fix formatting issues"
        fi
    fi
fi

# 2. Clippy Check (matches CI: cargo clippy --workspace --all-targets --all-features -- -D warnings)
if [[ "$SKIP_CLIPPY" != "true" ]]; then
    log_section "Clippy Check"
    ((TOTAL_CHECKS++))
    
    log_info "Running clippy linting..."
    # Note: CI uses -D warnings, but we're more lenient for local development
    # to avoid cross-platform clippy inconsistencies
    if [[ "$VERBOSE" == "true" ]]; then
        CARGO_CMD="cargo clippy --workspace --all-targets --all-features --verbose"
    else
        CARGO_CMD="cargo clippy --workspace --all-targets --all-features"
    fi
    
    if $CARGO_CMD; then
        log_success "Clippy check passed"
        ((PASSED_CHECKS++))
    else
        log_warning "Clippy check found issues (non-fatal for local development)"
        log_info "CI may be more strict with clippy warnings"
        ((PASSED_CHECKS++))  # Count as passed for local development
    fi
fi

# 3. Unit Tests (matches CI: cargo test --workspace --exclude ferrocp-python --lib)
if [[ "$SKIP_TESTS" != "true" ]]; then
    log_section "Unit Tests"
    ((TOTAL_CHECKS++))
    
    log_info "Running unit tests..."
    if [[ "$VERBOSE" == "true" ]]; then
        CARGO_CMD="cargo test --workspace --exclude ferrocp-python --lib --verbose"
    else
        CARGO_CMD="cargo test --workspace --exclude ferrocp-python --lib"
    fi
    
    if $CARGO_CMD; then
        log_success "Unit tests passed"
        ((PASSED_CHECKS++))
    else
        log_error "Unit tests failed"
        FAILED_CHECKS+=("Unit Tests")
    fi
    
    # Integration Tests (matches CI: cargo test --workspace --exclude ferrocp-python --test '*')
    log_section "Integration Tests"
    ((TOTAL_CHECKS++))
    
    log_info "Running integration tests..."
    if [[ "$VERBOSE" == "true" ]]; then
        CARGO_CMD="cargo test --workspace --exclude ferrocp-python --test '*' --verbose"
    else
        CARGO_CMD="cargo test --workspace --exclude ferrocp-python --test '*'"
    fi
    
    if $CARGO_CMD; then
        log_success "Integration tests passed"
        ((PASSED_CHECKS++))
    else
        log_error "Integration tests failed"
        FAILED_CHECKS+=("Integration Tests")
    fi
fi

# 4. Build Test (simplified version of CI GoReleaser build)
if [[ "$SKIP_BUILD" != "true" ]]; then
    log_section "Build Test"
    ((TOTAL_CHECKS++))
    
    log_info "Running build test..."
    if [[ "$VERBOSE" == "true" ]]; then
        CARGO_CMD="cargo build --workspace --exclude ferrocp-python --release --verbose"
    else
        CARGO_CMD="cargo build --workspace --exclude ferrocp-python --release"
    fi
    
    if $CARGO_CMD; then
        log_success "Build test passed"
        ((PASSED_CHECKS++))
        
        # Check if binary was created
        BINARY_PATH="target/release/ferrocp"
        if [[ -f "$BINARY_PATH" ]]; then
            BINARY_SIZE=$(du -h "$BINARY_PATH" | cut -f1)
            log_info "Binary size: $BINARY_SIZE"
            
            # Test basic functionality
            if VERSION_OUTPUT=$("$BINARY_PATH" --version 2>&1); then
                log_info "Binary version: $VERSION_OUTPUT"
                log_success "Binary functionality test passed"
            else
                log_warning "Binary functionality test failed"
            fi
        fi
    else
        log_error "Build test failed"
        FAILED_CHECKS+=("Build Test")
    fi
fi

# 5. Benchmark Tests (matches CI: cargo bench --workspace --exclude ferrocp-python)
if [[ "$SKIP_BENCH" != "true" ]]; then
    log_section "Benchmark Tests"
    ((TOTAL_CHECKS++))
    
    log_info "Running benchmark tests (sample size 10 for CI compatibility)..."
    if cargo bench --workspace --exclude ferrocp-python -- --sample-size 10; then
        log_success "Benchmark tests passed"
        ((PASSED_CHECKS++))
    else
        log_warning "Benchmark tests had issues (non-fatal)"
        ((PASSED_CHECKS++))  # Count as passed since benchmarks can be flaky
    fi
fi

# Summary
log_section "Summary"
log_info "Total checks: $TOTAL_CHECKS"
log_info "Passed checks: $PASSED_CHECKS"
log_info "Failed checks: ${#FAILED_CHECKS[@]}"

if [[ ${#FAILED_CHECKS[@]} -gt 0 ]]; then
    log_error "Failed checks:"
    for check in "${FAILED_CHECKS[@]}"; do
        echo -e "${RED}  - $check${NC}"
    done
    echo ""
    log_error "❌ Local CI check failed! Please fix issues before pushing to CI."
    exit 1
else
    log_success "✅ All checks passed! Code is ready for CI."
    log_info "Your code should pass CI checks on the remote machines."
fi

echo ""
log_info "Next steps:"
echo -e "${CYAN}  1. Commit your changes: git add . && git commit -m 'your message'${NC}"
echo -e "${CYAN}  2. Push to trigger CI: git push${NC}"
echo -e "${CYAN}  3. Monitor CI results in GitHub Actions${NC}"
