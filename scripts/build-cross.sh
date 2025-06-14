#!/bin/bash
# Cross-compilation build script for FerroCP
# This script builds FerroCP for multiple platforms using cargo-zigbuild

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

# Build configuration
BINARY_NAME="ferrocp"

# Target configurations with cross-compilation support
declare -A TARGETS=(
    ["x86_64-unknown-linux-gnu"]="Linux x86_64"
    ["aarch64-unknown-linux-gnu"]="Linux ARM64"
    ["x86_64-apple-darwin"]="macOS x86_64"
    ["aarch64-apple-darwin"]="macOS ARM64"
    ["x86_64-pc-windows-gnu"]="Windows x86_64"
)

log_info "Building FerroCP for all targets using cargo-zigbuild..."

# Install cargo-zigbuild if not available
if ! cargo install --list | grep -q "cargo-zigbuild"; then
    log_info "Installing cargo-zigbuild..."
    cargo install --locked cargo-zigbuild
fi

# Install zig if not available
if ! command -v zig >/dev/null 2>&1; then
    log_info "Installing zig via pip..."
    pip install ziglang
fi

# Function to build a single target using cargo-zigbuild
build_target() {
    local target=$1
    local description=$2

    log_info "Building for $description ($target)..."

    # Add Rust target if not already added
    rustup target add "$target" 2>/dev/null || true

    # Use cargo-zigbuild for cross-compilation
    if cargo zigbuild --bin "$BINARY_NAME" --release --target "$target"; then
        # Determine binary extension
        local binary_ext=""
        if [[ "$target" == *"windows"* ]]; then
            binary_ext=".exe"
        fi

        local binary_path="target/$target/release/$BINARY_NAME$binary_ext"

        # Verify binary exists
        if [[ -f "$binary_path" ]]; then
            # Get binary size
            local size=$(du -h "$binary_path" | cut -f1)
            log_success "Built $description: $binary_path ($size)"
        else
            log_error "Binary not found at $binary_path"
            return 1
        fi
    else
        log_error "Failed to build $description"
        return 1
    fi
}

# Build all targets
failed_targets=()
successful_targets=()

for target in "${!TARGETS[@]}"; do
    description="${TARGETS[$target]}"

    if build_target "$target" "$description"; then
        successful_targets+=("$description")
    else
        failed_targets+=("$description")
    fi

    echo # Add spacing between builds
done

# Summary
echo "=================================="
log_info "Build Summary"
echo "=================================="

if [[ ${#successful_targets[@]} -gt 0 ]]; then
    log_success "Successfully built ${#successful_targets[@]} targets:"
    for target in "${successful_targets[@]}"; do
        echo "  ✓ $target"
    done
fi

if [[ ${#failed_targets[@]} -gt 0 ]]; then
    log_error "Failed to build ${#failed_targets[@]} targets:"
    for target in "${failed_targets[@]}"; do
        echo "  ✗ $target"
    done
    echo
    log_error "Some builds failed. Check the output above for details."
    exit 1
fi

log_success "All targets built successfully!"

# List all built binaries
echo
log_info "Built binaries:"
find target -name "$BINARY_NAME" -o -name "$BINARY_NAME.exe" | while read -r binary; do
    if [[ -f "$binary" ]]; then
        size=$(du -h "$binary" | cut -f1)
        echo "  $binary ($size)"
    fi
done
