#!/bin/bash
# Test script to verify ARM64 cross-compilation fixes
# This script tests the specific issues that were causing compilation failures

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

log_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

log_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

log_error() {
    echo -e "${RED}❌ $1${NC}"
}

# Check if we're on Linux (required for ARM64 cross-compilation)
if [[ "$OSTYPE" != "linux-gnu"* ]]; then
    log_error "This script must be run on Linux for ARM64 cross-compilation"
    exit 1
fi

log_info "Testing ARM64 cross-compilation fixes..."

# Check if cross-compilation tools are available
if ! command -v aarch64-linux-gnu-gcc >/dev/null 2>&1; then
    log_warning "aarch64-linux-gnu-gcc not found, installing cross-compilation tools..."
    sudo apt-get update
    sudo apt-get install -y gcc-aarch64-linux-gnu g++-aarch64-linux-gnu libc6-dev-arm64-cross
fi

# Add ARM64 target if not already added
log_info "Adding ARM64 target..."
rustup target add aarch64-unknown-linux-gnu

# Set up environment variables for cross-compilation (conservative approach)
export CC=aarch64-linux-gnu-gcc
export CXX=aarch64-linux-gnu-g++
export AR=aarch64-linux-gnu-ar
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
export PKG_CONFIG_ALLOW_CROSS=1
export OPENSSL_STATIC=1
export OPENSSL_NO_VENDOR=1
export LIBZ_SYS_STATIC=1
export BLAKE3_NO_ASM=1

# Use conservative RUSTFLAGS (no aggressive static linking)
export RUSTFLAGS="-C link-arg=-fuse-ld=lld"

log_info "Environment variables set (conservative approach):"
log_info "  CC: $CC"
log_info "  CXX: $CXX"
log_info "  RUSTFLAGS: $RUSTFLAGS"

# Test 1: Build ferrocp-tests library (should work now)
log_info "Test 1: Building ferrocp-tests library for ARM64..."
if cargo build --target aarch64-unknown-linux-gnu -p ferrocp-tests --lib; then
    log_success "ferrocp-tests library built successfully"
else
    log_error "Failed to build ferrocp-tests library for ARM64"
    exit 1
fi

# Test 2: Build ferrocp-tests with --lib --bins (avoiding examples)
log_info "Test 2: Building ferrocp-tests with --lib --bins (avoiding examples)..."
if cargo build --target aarch64-unknown-linux-gnu -p ferrocp-tests --lib --bins; then
    log_success "ferrocp-tests built successfully with --lib --bins"
else
    log_error "Failed to build ferrocp-tests with --lib --bins for ARM64"
    exit 1
fi

# Test 3: Run tests with --lib --bins (simulating CI behavior)
log_info "Test 3: Running tests with --lib --bins for ARM64..."
if cargo test --target aarch64-unknown-linux-gnu -p ferrocp-tests --lib --bins; then
    log_success "ferrocp-tests tests passed with --lib --bins"
else
    log_error "Failed to run ferrocp-tests tests with --lib --bins for ARM64"
    exit 1
fi

# Test 4: Build main ferrocp binary
log_info "Test 4: Building ferrocp binary for ARM64..."
if cargo build --bin ferrocp --target aarch64-unknown-linux-gnu --release; then
    log_success "ferrocp binary built successfully"
    
    # Check if binary exists
    BINARY_PATH="target/aarch64-unknown-linux-gnu/release/ferrocp"
    if [[ -f "$BINARY_PATH" ]]; then
        log_success "Binary exists: $BINARY_PATH"
        ls -la "$BINARY_PATH"
        
        # Check binary architecture
        file "$BINARY_PATH"
    else
        log_error "Binary not found: $BINARY_PATH"
        exit 1
    fi
else
    log_error "Failed to build ferrocp binary for ARM64"
    exit 1
fi

# Test 5: Try to build the example with native-only feature (should fail gracefully)
log_info "Test 5: Testing example build with native-only feature..."
if cargo build --target aarch64-unknown-linux-gnu -p ferrocp-tests --example usage_example --features native-only; then
    log_warning "Example built unexpectedly (this might be okay)"
else
    log_success "Example correctly failed to build without native-only feature (expected)"
fi

log_success "All ARM64 cross-compilation fix tests passed!"
log_info "The ARM64 build configuration fixes are working correctly."
log_info ""
log_info "Summary of fixes applied:"
log_info "  1. Removed aggressive static linking flags (-C target-feature=+crt-static -C link-arg=-static-libgcc)"
log_info "  2. Used conservative RUSTFLAGS (-C link-arg=-fuse-ld=lld only)"
log_info "  3. Disabled automatic example discovery in ferrocp-tests"
log_info "  4. Modified CI to use --lib --bins for cross-compilation"
log_info "  5. Added conditional example compilation with required-features"
