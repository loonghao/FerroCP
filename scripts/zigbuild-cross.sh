#!/bin/bash
# FerroCP Cross-Compilation Script using cargo-zigbuild
# Optimized cross-compilation for VFX Platform targets

set -euo pipefail

# Default values
TARGET="all"
RELEASE=false
VERBOSE=false
CLEAN=false

# VFX Platform supported targets
VFX_TARGETS=(
    "x86_64-unknown-linux-gnu"
    "aarch64-unknown-linux-gnu" 
    "x86_64-apple-darwin"
    "aarch64-apple-darwin"
    "x86_64-pc-windows-msvc"
)

# Color output functions
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

log_info() { echo -e "${CYAN}ℹ️  $1${NC}"; }
log_success() { echo -e "${GREEN}✅ $1${NC}"; }
log_warning() { echo -e "${YELLOW}⚠️  $1${NC}"; }
log_error() { echo -e "${RED}❌ $1${NC}"; }

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -t|--target)
            TARGET="$2"
            shift 2
            ;;
        -r|--release)
            RELEASE=true
            shift
            ;;
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        -c|--clean)
            CLEAN=true
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo "Options:"
            echo "  -t, --target TARGET    Target to build (default: all)"
            echo "  -r, --release          Build in release mode"
            echo "  -v, --verbose          Verbose output"
            echo "  -c, --clean            Clean before building"
            echo "  -h, --help             Show this help"
            echo ""
            echo "Available targets: ${VFX_TARGETS[*]}, all"
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."
    
    # Check cargo
    if ! command -v cargo &> /dev/null; then
        log_error "Cargo not found. Please install Rust toolchain."
        exit 1
    fi
    
    # Check if cargo-zigbuild is installed
    if ! cargo install --list | grep -q "cargo-zigbuild"; then
        log_info "Installing cargo-zigbuild..."
        cargo install --locked cargo-zigbuild
    fi
    
    # Check zig installation
    if ! command -v zig &> /dev/null; then
        log_info "Installing zig via pip..."
        pip install ziglang
    fi
    
    log_success "Prerequisites check completed"
}

# Install Rust targets
install_rust_targets() {
    local targets=("$@")
    
    log_info "Installing Rust targets..."
    for target in "${targets[@]}"; do
        log_info "Adding target: $target"
        rustup target add "$target" || log_warning "Failed to add target: $target"
    done
}

# Build for specific target
build_target() {
    local target_triple="$1"
    local is_release="$2"
    local is_verbose="$3"
    
    local build_args=("zigbuild" "--bin" "ferrocp" "--target" "$target_triple")
    
    if [[ "$is_release" == "true" ]]; then
        build_args+=("--release")
    fi
    
    if [[ "$is_verbose" == "true" ]]; then
        build_args+=("--verbose")
    fi
    
    log_info "Building for target: $target_triple"
    log_info "Command: cargo ${build_args[*]}"
    
    local start_time=$(date +%s.%N)
    
    if cargo "${build_args[@]}"; then
        local end_time=$(date +%s.%N)
        local elapsed=$(echo "$end_time - $start_time" | bc -l)
        local elapsed_formatted=$(printf "%.2f" "$elapsed")
        
        log_success "Build completed for $target_triple in ${elapsed_formatted}s"
        
        # Show binary info
        local profile_dir
        if [[ "$is_release" == "true" ]]; then
            profile_dir="release"
        else
            profile_dir="debug"
        fi
        
        local binary_path="target/$target_triple/$profile_dir/ferrocp"
        if [[ "$target_triple" == *"windows"* ]]; then
            binary_path+=".exe"
        fi
        
        if [[ -f "$binary_path" ]]; then
            local size_kb=$(du -k "$binary_path" | cut -f1)
            log_info "Binary size: ${size_kb} KB"
        fi
        
        return 0
    else
        local end_time=$(date +%s.%N)
        local elapsed=$(echo "$end_time - $start_time" | bc -l)
        local elapsed_formatted=$(printf "%.2f" "$elapsed")
        
        log_error "Build failed for $target_triple after ${elapsed_formatted}s"
        return 1
    fi
}

# Main execution
main() {
    echo -e "${MAGENTA}🚀 FerroCP Cross-Compilation with cargo-zigbuild${NC}"
    log_info "Target: $TARGET | Release: $RELEASE | Verbose: $VERBOSE"
    
    # Clean if requested
    if [[ "$CLEAN" == "true" ]]; then
        log_info "Cleaning previous builds..."
        cargo clean
    fi
    
    # Check prerequisites
    check_prerequisites
    
    # Determine targets to build
    local targets_to_build=()
    if [[ "$TARGET" == "all" ]]; then
        targets_to_build=("${VFX_TARGETS[@]}")
    else
        # Check if target is valid
        local valid_target=false
        for vfx_target in "${VFX_TARGETS[@]}"; do
            if [[ "$TARGET" == "$vfx_target" ]]; then
                valid_target=true
                break
            fi
        done
        
        if [[ "$valid_target" == "false" ]]; then
            log_error "Invalid target: $TARGET. Valid targets: ${VFX_TARGETS[*]}, all"
            exit 1
        fi
        
        targets_to_build=("$TARGET")
    fi
    
    # Install required targets
    install_rust_targets "${targets_to_build[@]}"
    
    # Build targets
    local failed_builds=()
    local successful_builds=()
    
    for target in "${targets_to_build[@]}"; do
        if build_target "$target" "$RELEASE" "$VERBOSE"; then
            successful_builds+=("$target")
        else
            failed_builds+=("$target")
        fi
    done
    
    # Summary
    echo -e "\n${MAGENTA}📊 Build Summary:${NC}"
    log_success "Successful builds (${#successful_builds[@]}): ${successful_builds[*]}"
    
    if [[ ${#failed_builds[@]} -gt 0 ]]; then
        log_error "Failed builds (${#failed_builds[@]}): ${failed_builds[*]}"
        exit 1
    else
        log_success "All builds completed successfully! 🎉"
    fi
}

# Run main function
main "$@"
