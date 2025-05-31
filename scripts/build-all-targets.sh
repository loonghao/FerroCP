#!/bin/bash
# Build script for all FerroCP targets used by GoReleaser

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
RUSTFLAGS="-C opt-level=3 -C lto=fat -C codegen-units=1"

# Target configurations
declare -A TARGETS=(
    ["x86_64-unknown-linux-gnu"]="Linux x86_64"
    ["aarch64-unknown-linux-gnu"]="Linux ARM64"
    ["x86_64-apple-darwin"]="macOS x86_64"
    ["aarch64-apple-darwin"]="macOS ARM64"
    ["x86_64-pc-windows-gnu"]="Windows x86_64"
)

log_info "Building FerroCP for all targets..."
log_info "Using target-specific Rust flags for optimization"

# Note: RUSTFLAGS are set per-target to avoid conflicts

# Function to build a single target
build_target() {
    local target=$1
    local description=$2
    
    log_info "Building for $description ($target)..."
    
    # Platform-specific setup
    case "$target" in
        "aarch64-unknown-linux-gnu")
            # Linux ARM64 cross-compilation setup
            if command -v aarch64-linux-gnu-gcc >/dev/null 2>&1; then
                export CC_aarch64_unknown_linux_gnu="aarch64-linux-gnu-gcc"
                export CXX_aarch64_unknown_linux_gnu="aarch64-linux-gnu-g++"
                export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER="aarch64-linux-gnu-gcc"
                export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUSTFLAGS="-C opt-level=3 -C lto=fat"
            else
                log_warning "aarch64-linux-gnu-gcc not found, cross-compilation may fail"
            fi
            ;;
        "x86_64-unknown-linux-gnu")
            # Linux x86_64 - native or cross-compile
            export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS="-C opt-level=3 -C lto=fat"
            if [[ "$(uname -s)" != "Linux" ]]; then
                log_info "Cross-compiling for Linux from $(uname -s)"
                if command -v x86_64-linux-gnu-gcc >/dev/null 2>&1; then
                    export CC="x86_64-linux-gnu-gcc"
                    export CXX="x86_64-linux-gnu-g++"
                fi
            fi
            ;;
        "x86_64-apple-darwin")
            # macOS x86_64 setup
            export CARGO_TARGET_X86_64_APPLE_DARWIN_RUSTFLAGS="-C opt-level=3 -C lto=fat"
            if [[ "$(uname -s)" == "Darwin" ]]; then
                export MACOSX_DEPLOYMENT_TARGET="10.15"
            fi
            ;;
        "aarch64-apple-darwin")
            # macOS ARM64 setup
            export CARGO_TARGET_AARCH64_APPLE_DARWIN_RUSTFLAGS="-C opt-level=3 -C lto=fat"
            if [[ "$(uname -s)" == "Darwin" ]]; then
                export MACOSX_DEPLOYMENT_TARGET="11.0"
            fi
            ;;
        "x86_64-pc-windows-gnu")
            # Windows cross-compilation setup
            if command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1; then
                export CC_x86_64_pc_windows_gnu="x86_64-w64-mingw32-gcc"
                export CXX_x86_64_pc_windows_gnu="x86_64-w64-mingw32-g++"
                export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER="x86_64-w64-mingw32-gcc"
                # Disable problematic linker flags for Windows cross-compilation
                export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUSTFLAGS="-C opt-level=3 -C lto=fat"
            else
                log_warning "x86_64-w64-mingw32-gcc not found, Windows cross-compilation may fail"
            fi
            ;;
    esac
    
    # Build the target
    if cargo build --bin "$BINARY_NAME" --release --target "$target"; then
        # Determine binary extension
        local binary_ext=""
        if [[ "$target" == *"windows"* ]]; then
            binary_ext=".exe"
        fi
        
        local binary_path="target/$target/release/$BINARY_NAME$binary_ext"
        
        # Verify binary exists
        if [[ -f "$binary_path" ]]; then
            # Strip binary if possible (Unix systems only)
            if [[ -z "$binary_ext" ]] && command -v strip >/dev/null 2>&1; then
                strip "$binary_path" 2>/dev/null || log_warning "Failed to strip $binary_path"
            fi
            
            # Get binary size
            local size=$(du -h "$binary_path" | cut -f1)
            log_success "Built $description: $binary_path ($size)"
            
            # Test binary if possible
            if [[ "$target" == "$(rustc -vV | grep host | cut -d' ' -f2)" ]] || [[ "$(uname -s)" == "Darwin" && "$target" == *"apple-darwin" ]]; then
                if "$binary_path" --version >/dev/null 2>&1; then
                    log_success "Binary test passed for $description"
                else
                    log_warning "Binary test failed for $description"
                fi
            fi
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
