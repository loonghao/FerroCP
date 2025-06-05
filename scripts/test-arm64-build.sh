#!/bin/bash
# Test script for ARM64 cross-compilation
# This script tests the ARM64 build configuration locally

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

log_info "Testing ARM64 cross-compilation setup..."

# Check if cross-compilation tools are available
if ! command -v aarch64-linux-gnu-gcc >/dev/null 2>&1; then
    log_warning "aarch64-linux-gnu-gcc not found, installing cross-compilation tools..."
    sudo apt-get update
    sudo apt-get install -y gcc-aarch64-linux-gnu g++-aarch64-linux-gnu libc6-dev-arm64-cross
fi

# Add ARM64 target if not already added
log_info "Adding ARM64 target..."
rustup target add aarch64-unknown-linux-gnu

# Set up environment variables for cross-compilation
export CC=aarch64-linux-gnu-gcc
export CXX=aarch64-linux-gnu-g++
export AR=aarch64-linux-gnu-ar
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
export PKG_CONFIG_ALLOW_CROSS=1
export OPENSSL_STATIC=1
export OPENSSL_NO_VENDOR=1
export LIBZ_SYS_STATIC=1
export BLAKE3_NO_ASM=1

log_info "Environment variables set:"
log_info "  CC: $CC"
log_info "  CXX: $CXX"
log_info "  AR: $AR"
log_info "  CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER: $CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER"

# Test building the main binary
log_info "Building ferrocp binary for ARM64..."
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

# Test building a simple test (not the problematic example)
log_info "Testing simple ARM64 build..."
if cargo build --target aarch64-unknown-linux-gnu --lib -p ferrocp-types; then
    log_success "ferrocp-types library built successfully for ARM64"
else
    log_error "Failed to build ferrocp-types library for ARM64"
    exit 1
fi

# Test building ferrocp-tests without examples
log_info "Testing ferrocp-tests build (excluding examples)..."
if cargo build --target aarch64-unknown-linux-gnu -p ferrocp-tests --lib; then
    log_success "ferrocp-tests library built successfully for ARM64"
else
    log_error "Failed to build ferrocp-tests library for ARM64"
    exit 1
fi

log_success "All ARM64 cross-compilation tests passed!"
log_info "The ARM64 build configuration is working correctly."
