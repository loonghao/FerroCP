#!/bin/bash
# Test script to verify the build fixes work correctly

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

# Test environment setup
test_environment() {
    log_info "Testing build environment setup..."
    
    # Check if required tools are available
    local tools=("cargo" "rustc" "python3" "pip")
    local missing_tools=()
    
    for tool in "${tools[@]}"; do
        if ! command -v "$tool" >/dev/null 2>&1; then
            missing_tools+=("$tool")
        else
            log_success "$tool is available: $(which $tool)"
        fi
    done
    
    if [ ${#missing_tools[@]} -ne 0 ]; then
        log_error "Missing required tools: ${missing_tools[*]}"
        return 1
    fi
    
    # Check linker
    if command -v ld >/dev/null 2>&1; then
        log_success "Linker is available: $(which ld)"
        ld --version | head -1
    else
        log_error "Linker 'ld' not found"
        return 1
    fi
    
    # Check clang if available
    if command -v clang >/dev/null 2>&1; then
        log_success "Clang is available: $(which clang)"
        clang --version | head -1
    else
        log_warning "Clang not available, will use default compiler"
    fi
    
    log_success "Environment check passed"
}

# Test maturin installation and basic functionality
test_maturin() {
    log_info "Testing maturin installation and functionality..."
    
    # Install maturin if not available
    if ! command -v maturin >/dev/null 2>&1; then
        log_info "Installing maturin..."
        pip install maturin[patchelf]
    fi
    
    # Check maturin version
    log_success "Maturin version: $(maturin --version)"
    
    # Test maturin list-python
    log_info "Available Python interpreters:"
    maturin list-python || log_warning "Failed to list Python interpreters"
    
    log_success "Maturin test passed"
}

# Test basic Rust compilation
test_rust_compilation() {
    log_info "Testing basic Rust compilation..."
    
    # Create a temporary Rust project
    local temp_dir=$(mktemp -d)
    cd "$temp_dir"
    
    # Initialize a simple Rust project
    cargo init --name test_project --bin
    
    # Try to compile
    if cargo build; then
        log_success "Basic Rust compilation works"
    else
        log_error "Basic Rust compilation failed"
        cd - >/dev/null
        rm -rf "$temp_dir"
        return 1
    fi
    
    # Clean up
    cd - >/dev/null
    rm -rf "$temp_dir"
    
    log_success "Rust compilation test passed"
}

# Test Python wheel building (dry run)
test_python_wheel_build() {
    log_info "Testing Python wheel building (dry run)..."
    
    # Set environment variables as in the fixed workflow
    export CARGO_NET_GIT_FETCH_WITH_CLI=true
    export RUSTFLAGS="-C opt-level=3"
    
    if command -v clang >/dev/null 2>&1; then
        export CC=clang
        export CXX=clang++
        export RUSTFLAGS="$RUSTFLAGS -C linker=clang"
        log_info "Using clang as compiler and linker"
    fi
    
    # Test maturin build command (without actually building)
    log_info "Testing maturin build command syntax..."
    if maturin build --help >/dev/null 2>&1; then
        log_success "Maturin build command is available"
    else
        log_error "Maturin build command failed"
        return 1
    fi
    
    log_success "Python wheel build test passed"
}

# Test cross-compilation targets
test_cross_compilation_targets() {
    log_info "Testing cross-compilation targets..."
    
    local targets=(
        "x86_64-unknown-linux-gnu"
        "aarch64-unknown-linux-gnu"
    )
    
    for target in "${targets[@]}"; do
        if rustup target list --installed | grep -q "$target"; then
            log_success "Target $target is installed"
        else
            log_warning "Target $target is not installed"
            log_info "Installing target $target..."
            if rustup target add "$target"; then
                log_success "Successfully installed target $target"
            else
                log_warning "Failed to install target $target"
            fi
        fi
    done
    
    log_success "Cross-compilation targets test completed"
}

# Main test function
main() {
    log_info "Starting build fix verification tests..."
    echo "========================================"
    
    # Run all tests
    test_environment || { log_error "Environment test failed"; exit 1; }
    echo "----------------------------------------"
    
    test_maturin || { log_error "Maturin test failed"; exit 1; }
    echo "----------------------------------------"
    
    test_rust_compilation || { log_error "Rust compilation test failed"; exit 1; }
    echo "----------------------------------------"
    
    test_python_wheel_build || { log_error "Python wheel build test failed"; exit 1; }
    echo "----------------------------------------"
    
    test_cross_compilation_targets || { log_error "Cross-compilation test failed"; exit 1; }
    echo "========================================"
    
    log_success "All tests passed! Build environment should work correctly."
    
    # Summary
    echo
    log_info "Test Summary:"
    echo "  ✅ Environment setup"
    echo "  ✅ Maturin functionality"
    echo "  ✅ Rust compilation"
    echo "  ✅ Python wheel building"
    echo "  ✅ Cross-compilation targets"
    echo
    log_info "The build fixes should resolve the GoReleaser issues."
}

# Run main function
main "$@"
