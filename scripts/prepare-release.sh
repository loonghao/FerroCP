#!/bin/bash
# Release preparation script for FerroCP

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if version is provided
if [ $# -eq 0 ]; then
    print_error "Usage: $0 <version>"
    print_error "Example: $0 0.2.0"
    exit 1
fi

VERSION=$1
print_status "Preparing release for version $VERSION"

# Validate version format (semantic versioning)
if ! [[ $VERSION =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9]+)?$ ]]; then
    print_error "Invalid version format. Use semantic versioning (e.g., 1.0.0, 1.0.0-beta.1)"
    exit 1
fi

# Check if we're on main branch
CURRENT_BRANCH=$(git branch --show-current)
if [ "$CURRENT_BRANCH" != "main" ]; then
    print_warning "You are not on the main branch (current: $CURRENT_BRANCH)"
    read -p "Continue anyway? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Check for uncommitted changes
if ! git diff-index --quiet HEAD --; then
    print_error "You have uncommitted changes. Please commit or stash them first."
    exit 1
fi

# Update version in Cargo.toml
print_status "Updating version in Cargo.toml"
sed -i.bak "s/^version = \".*\"/version = \"$VERSION\"/" Cargo.toml
rm Cargo.toml.bak

# Update version in all crate Cargo.toml files
print_status "Updating version in all crate Cargo.toml files"
find crates -name "Cargo.toml" -exec sed -i.bak "s/^version = \".*\"/version = \"$VERSION\"/" {} \;
find crates -name "Cargo.toml.bak" -delete

# Update Cargo.lock
print_status "Updating Cargo.lock"
cargo update --workspace

# Run tests to ensure everything works
print_status "Running tests to verify the build"
if ! cargo test --workspace --exclude ferrocp-python; then
    print_error "Tests failed. Please fix the issues before releasing."
    exit 1
fi

# Build release binaries
print_status "Building release binaries"
if ! cargo build --release; then
    print_error "Release build failed. Please fix the issues before releasing."
    exit 1
fi

# Build PGO-optimized binary (optional)
read -p "Build PGO-optimized binary? (y/N): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    print_status "Building PGO-optimized binary..."
    if command -v powershell &> /dev/null; then
        # Use PowerShell script on Windows
        if ! powershell -ExecutionPolicy Bypass -File scripts/pgo-build.ps1; then
            print_warning "PGO build failed, continuing with regular release build"
        else
            print_success "PGO-optimized binary built successfully"
        fi
    else
        # Manual PGO build for Unix-like systems
        print_status "Running manual PGO build process..."

        # Step 1: Build with profile generation
        print_status "Step 1: Building instrumented binary..."
        mkdir -p pgo-data
        if ! RUSTFLAGS="-Cprofile-generate=./pgo-data" cargo build --profile release-pgo --bin ferrocp; then
            print_warning "PGO instrumented build failed, continuing with regular release build"
        else
            # Step 2: Collect profile data
            print_status "Step 2: Collecting profile data..."
            INSTRUMENTED_BINARY="target/release-pgo/ferrocp"

            # Create test data and run representative workloads
            TEMP_DIR=$(mktemp -d)
            SOURCE_DIR="$TEMP_DIR/source"
            DEST_DIR="$TEMP_DIR/dest"
            mkdir -p "$SOURCE_DIR" "$DEST_DIR"

            # Create test files of various sizes
            for size in 1024 16384 1048576 16777216; do
                for i in {1..5}; do
                    dd if=/dev/zero of="$SOURCE_DIR/test_${size}_${i}.dat" bs=$size count=1 2>/dev/null
                done
            done

            # Run copy operations to collect profile data
            for file in "$SOURCE_DIR"/*; do
                "$INSTRUMENTED_BINARY" copy "$file" "$DEST_DIR/$(basename "$file")" || true
            done

            # Clean up test data
            rm -rf "$TEMP_DIR"

            # Step 3: Merge profile data
            print_status "Step 3: Merging profile data..."
            if command -v llvm-profdata &> /dev/null; then
                if ! llvm-profdata merge -o merged.profdata pgo-data/*.profraw; then
                    print_warning "Failed to merge profile data"
                else
                    # Step 4: Build with profile use
                    print_status "Step 4: Building PGO-optimized binary..."
                    if ! RUSTFLAGS="-Cprofile-use=./merged.profdata" cargo build --profile release-pgo --bin ferrocp; then
                        print_warning "PGO optimized build failed"
                    else
                        # Copy PGO binary to release location
                        cp target/release-pgo/ferrocp target/release/ferrocp-pgo
                        print_success "PGO-optimized binary created: target/release/ferrocp-pgo"
                    fi
                fi
            else
                print_warning "llvm-profdata not found. Install with: rustup component add llvm-tools-preview"
            fi
        fi
    fi
fi

# Generate documentation
print_status "Generating documentation"
cargo doc --workspace --no-deps

# Update CHANGELOG.md
print_status "Please update CHANGELOG.md with the changes for version $VERSION"
print_warning "After updating CHANGELOG.md, run the following commands:"
echo
echo "  git add ."
echo "  git commit -m \"Release $VERSION\""
echo "  git tag $VERSION"
echo "  git push origin main"
echo "  git push origin $VERSION"
echo
print_success "Release preparation completed for version $VERSION"
print_status "Don't forget to update CHANGELOG.md before committing!"
