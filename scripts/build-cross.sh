#!/bin/bash
# Cross-compilation build script for FerroCP
# This script is used by GoReleaser to build Rust binaries for multiple platforms

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

# Parse command line arguments
TARGET=""
OUTPUT_DIR=""
BINARY_NAME="ferrocp"

while [[ $# -gt 0 ]]; do
    case $1 in
        --target)
            TARGET="$2"
            shift 2
            ;;
        --output-dir)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        --binary-name)
            BINARY_NAME="$2"
            shift 2
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Validate required arguments
if [[ -z "$TARGET" ]]; then
    log_error "Target is required. Use --target <target-triple>"
    exit 1
fi

if [[ -z "$OUTPUT_DIR" ]]; then
    OUTPUT_DIR="target/$TARGET/release"
fi

log_info "Building FerroCP for target: $TARGET"
log_info "Output directory: $OUTPUT_DIR"
log_info "Binary name: $BINARY_NAME"

# Map GoReleaser OS/ARCH to Rust target triples
case "$TARGET" in
    "linux-amd64")
        RUST_TARGET="x86_64-unknown-linux-gnu"
        BINARY_EXT=""
        ;;
    "linux-arm64")
        RUST_TARGET="aarch64-unknown-linux-gnu"
        BINARY_EXT=""
        ;;
    "darwin-amd64")
        RUST_TARGET="x86_64-apple-darwin"
        BINARY_EXT=""
        ;;
    "darwin-arm64")
        RUST_TARGET="aarch64-apple-darwin"
        BINARY_EXT=""
        ;;
    "windows-amd64")
        RUST_TARGET="x86_64-pc-windows-gnu"
        BINARY_EXT=".exe"
        ;;
    *)
        log_error "Unsupported target: $TARGET"
        exit 1
        ;;
esac

log_info "Rust target: $RUST_TARGET"

# Install target if not already installed
log_info "Ensuring Rust target $RUST_TARGET is installed..."
rustup target add "$RUST_TARGET"

# Set up cross-compilation environment
export RUSTFLAGS="-C target-cpu=native -C opt-level=3 -C lto=fat -C codegen-units=1"

# Platform-specific setup
case "$RUST_TARGET" in
    "x86_64-unknown-linux-gnu")
        # Linux x86_64 - native or cross-compile
        if [[ "$(uname -s)" != "Linux" ]]; then
            log_info "Cross-compiling for Linux from $(uname -s)"
            export CC="x86_64-linux-gnu-gcc"
            export CXX="x86_64-linux-gnu-g++"
        fi
        ;;
    "aarch64-unknown-linux-gnu")
        # Linux ARM64 - cross-compile
        log_info "Cross-compiling for Linux ARM64"
        export CC="aarch64-linux-gnu-gcc"
        export CXX="aarch64-linux-gnu-g++"
        export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER="aarch64-linux-gnu-gcc"
        ;;
    "x86_64-apple-darwin")
        # macOS x86_64
        if [[ "$(uname -s)" == "Darwin" ]]; then
            log_info "Building natively for macOS x86_64"
            export MACOSX_DEPLOYMENT_TARGET="10.15"
        else
            log_warning "Cross-compiling for macOS from $(uname -s) - may require additional setup"
        fi
        ;;
    "aarch64-apple-darwin")
        # macOS ARM64 (Apple Silicon)
        if [[ "$(uname -s)" == "Darwin" ]]; then
            log_info "Building for macOS ARM64"
            export MACOSX_DEPLOYMENT_TARGET="11.0"
        else
            log_warning "Cross-compiling for macOS ARM64 from $(uname -s) - may require additional setup"
        fi
        ;;
    "x86_64-pc-windows-gnu")
        # Windows x86_64
        log_info "Cross-compiling for Windows x86_64"
        export CC="x86_64-w64-mingw32-gcc"
        export CXX="x86_64-w64-mingw32-g++"
        export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER="x86_64-w64-mingw32-gcc"
        ;;
esac

# Build the binary
log_info "Building binary..."
cargo build \
    --bin ferrocp \
    --release \
    --target "$RUST_TARGET" \
    --features "default"

# Check if build was successful
BINARY_PATH="target/$RUST_TARGET/release/$BINARY_NAME$BINARY_EXT"
if [[ ! -f "$BINARY_PATH" ]]; then
    log_error "Build failed: binary not found at $BINARY_PATH"
    exit 1
fi

# Create output directory if it doesn't exist
mkdir -p "$OUTPUT_DIR"

# Copy binary to output directory
OUTPUT_PATH="$OUTPUT_DIR/$BINARY_NAME$BINARY_EXT"
cp "$BINARY_PATH" "$OUTPUT_PATH"

# Make binary executable (Unix systems)
if [[ -z "$BINARY_EXT" ]]; then
    chmod +x "$OUTPUT_PATH"
fi

# Strip binary to reduce size (if strip is available and it's not Windows)
if [[ -z "$BINARY_EXT" ]] && command -v strip >/dev/null 2>&1; then
    log_info "Stripping debug symbols..."
    strip "$OUTPUT_PATH" || log_warning "Failed to strip binary"
fi

# Get binary info
log_info "Binary information:"
ls -lh "$OUTPUT_PATH"

if command -v file >/dev/null 2>&1; then
    file "$OUTPUT_PATH"
fi

# Test the binary
log_info "Testing binary..."
if "$OUTPUT_PATH" --version >/dev/null 2>&1; then
    log_success "Binary test passed"
else
    log_warning "Binary test failed - binary may not be functional"
fi

log_success "Build completed successfully: $OUTPUT_PATH"
