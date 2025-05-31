#!/bin/bash
# Build Python wheels for all platforms using maturin

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

# Configuration
PYTHON_VERSIONS=("3.9" "3.10" "3.11" "3.12")
PLATFORMS=("linux" "macos" "windows")

log_info "Building Python wheels for FerroCP..."

# Create dist directory
mkdir -p dist

# Install maturin if not available
if ! command -v maturin >/dev/null 2>&1; then
    log_info "Installing maturin..."
    pip install maturin
fi

# Install cibuildwheel for cross-platform builds
if ! command -v cibuildwheel >/dev/null 2>&1; then
    log_info "Installing cibuildwheel..."
    pip install cibuildwheel
fi

# Function to check and fix linker issues
check_linker() {
    log_info "Checking linker availability..."

    # Check if ld is available
    if ! command -v ld >/dev/null 2>&1; then
        log_warning "Linker 'ld' not found, attempting to fix..."

        # Try to install binutils if on Linux
        if [[ "$OSTYPE" == "linux-gnu"* ]]; then
            if command -v apt-get >/dev/null 2>&1; then
                sudo apt-get update && sudo apt-get install -y binutils
            elif command -v yum >/dev/null 2>&1; then
                sudo yum install -y binutils
            fi
        fi

        # Verify again
        if ! command -v ld >/dev/null 2>&1; then
            log_error "Failed to install linker. Please install binutils manually."
            return 1
        fi
    fi

    log_success "Linker check passed: $(which ld)"
    ld --version | head -1
}

# Function to build wheels using maturin
build_with_maturin() {
    log_info "Building wheels with maturin..."

    # Check linker first
    check_linker || return 1

    # Set environment variables for stable builds
    export CARGO_NET_GIT_FETCH_WITH_CLI=true
    export RUSTFLAGS="-C opt-level=3"

    # Use clang as linker if available (more reliable)
    if command -v clang >/dev/null 2>&1; then
        export CC=clang
        export CXX=clang++
        export RUSTFLAGS="$RUSTFLAGS -C linker=clang"
        log_info "Using clang as compiler and linker"
    fi

    # Build wheels for current platform only (avoid cross-compilation issues)
    maturin build --release --out dist --interpreter python3

    # Build source distribution
    maturin sdist --out dist

    log_success "Maturin build completed"
}

# Function to build wheels using cibuildwheel (for cross-platform)
build_with_cibuildwheel() {
    log_info "Building cross-platform wheels with cibuildwheel..."
    
    # Set environment variables for cibuildwheel
    export CIBW_BUILD="cp39-* cp310-* cp311-* cp312-*"
    export CIBW_SKIP="*-win32 *-manylinux_i686 *-musllinux_*"
    export CIBW_ARCHS_LINUX="x86_64 aarch64"
    export CIBW_ARCHS_MACOS="x86_64 arm64"
    export CIBW_ARCHS_WINDOWS="AMD64"
    
    # Build command for cibuildwheel
    export CIBW_BUILD_FRONTEND="build"
    export CIBW_BEFORE_BUILD="pip install maturin"
    export CIBW_BUILD_VERBOSITY=1
    
    # Platform-specific settings
    case "$(uname -s)" in
        Linux)
            log_info "Building Linux wheels..."
            export CIBW_ENVIRONMENT_LINUX="PATH=/opt/python/cp39-cp39/bin:/opt/python/cp310-cp310/bin:/opt/python/cp311-cp311/bin:/opt/python/cp312-cp312/bin:$PATH"
            ;;
        Darwin)
            log_info "Building macOS wheels..."
            export CIBW_ENVIRONMENT_MACOS="MACOSX_DEPLOYMENT_TARGET=10.15"
            ;;
        MINGW*|MSYS*|CYGWIN*)
            log_info "Building Windows wheels..."
            ;;
    esac
    
    # Run cibuildwheel
    cibuildwheel --output-dir dist
    
    log_success "cibuildwheel build completed"
}

# Function to build wheels using maturin with specific targets
build_cross_platform_maturin() {
    log_info "Building cross-platform wheels with maturin..."
    
    # Define target mappings
    declare -A TARGETS=(
        ["linux-x86_64"]="x86_64-unknown-linux-gnu"
        ["linux-aarch64"]="aarch64-unknown-linux-gnu"
        ["macos-x86_64"]="x86_64-apple-darwin"
        ["macos-aarch64"]="aarch64-apple-darwin"
        ["windows-x86_64"]="x86_64-pc-windows-gnu"
    )
    
    # Build for each target
    for platform_arch in "${!TARGETS[@]}"; do
        target="${TARGETS[$platform_arch]}"
        log_info "Building wheel for $platform_arch ($target)..."
        
        # Check if target is installed
        if rustup target list --installed | grep -q "$target"; then
            # Build wheel for specific target
            if maturin build --release --target "$target" --out dist; then
                log_success "Built wheel for $platform_arch"
            else
                log_warning "Failed to build wheel for $platform_arch"
            fi
        else
            log_warning "Target $target not installed, skipping $platform_arch"
        fi
    done
}

# Function to verify wheels
verify_wheels() {
    log_info "Verifying built wheels..."
    
    if [ ! -d "dist" ] || [ -z "$(ls -A dist 2>/dev/null)" ]; then
        log_error "No wheels found in dist directory"
        return 1
    fi
    
    log_info "Built wheels:"
    ls -la dist/
    
    # Check wheel contents
    for wheel in dist/*.whl; do
        if [ -f "$wheel" ]; then
            log_info "Checking wheel: $(basename "$wheel")"
            python -m zipfile -l "$wheel" | head -20
            echo "..."
        fi
    done
    
    # Verify source distribution
    for sdist in dist/*.tar.gz; do
        if [ -f "$sdist" ]; then
            log_info "Found source distribution: $(basename "$sdist")"
        fi
    done
    
    log_success "Wheel verification completed"
}

# Function to test wheels
test_wheels() {
    log_info "Testing wheels..."
    
    # Create a temporary virtual environment for testing
    python -m venv test_env
    source test_env/bin/activate 2>/dev/null || source test_env/Scripts/activate
    
    # Test each wheel
    for wheel in dist/*.whl; do
        if [ -f "$wheel" ]; then
            log_info "Testing wheel: $(basename "$wheel")"
            
            # Install the wheel
            pip install "$wheel" --force-reinstall
            
            # Test import
            if python -c "import ferrocp; print(f'ferrocp version: {ferrocp.__version__}')"; then
                log_success "Wheel test passed: $(basename "$wheel")"
            else
                log_warning "Wheel test failed: $(basename "$wheel")"
            fi
            
            # Uninstall for next test
            pip uninstall ferrocp -y
        fi
    done
    
    # Clean up
    deactivate
    rm -rf test_env
    
    log_success "Wheel testing completed"
}

# Main execution
main() {
    log_info "Starting Python wheel build process..."
    
    # Choose build method based on environment
    if command -v cibuildwheel >/dev/null 2>&1 && [ "${USE_CIBUILDWHEEL:-false}" = "true" ]; then
        build_with_cibuildwheel
    elif [ "${CROSS_COMPILE:-false}" = "true" ]; then
        build_cross_platform_maturin
    else
        build_with_maturin
    fi
    
    # Verify and test wheels
    verify_wheels
    
    # Test wheels if requested
    if [ "${TEST_WHEELS:-false}" = "true" ]; then
        test_wheels
    fi
    
    log_success "Python wheel build process completed successfully!"
    
    # Summary
    echo
    log_info "Build Summary:"
    echo "  Wheels built: $(find dist -name "*.whl" | wc -l)"
    echo "  Source distributions: $(find dist -name "*.tar.gz" | wc -l)"
    echo "  Total artifacts: $(find dist -type f | wc -l)"
    echo
    log_info "Artifacts ready for PyPI upload:"
    find dist -type f -name "*.whl" -o -name "*.tar.gz" | sort
}

# Run main function
main "$@"
