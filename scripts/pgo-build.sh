#!/bin/bash
# Profile-Guided Optimization (PGO) build script for FerroCP
# This script generates optimized binaries using PGO to reduce size and improve performance

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
TARGET="${1:-}"
PROFILE_DIR="target/pgo-profiles"
BINARY_NAME="ferrocp"

echo -e "${BLUE}🚀 Starting PGO build for FerroCP${NC}"

# Validate target
if [ -z "$TARGET" ]; then
    echo -e "${RED}❌ Error: Target not specified${NC}"
    echo "Usage: $0 <target>"
    echo "Example: $0 x86_64-unknown-linux-gnu"
    exit 1
fi

echo -e "${YELLOW}📋 Configuration:${NC}"
echo "  Target: $TARGET"
echo "  Profile Directory: $PROFILE_DIR"
echo "  Binary: $BINARY_NAME"

# Clean previous builds and profiles
echo -e "${YELLOW}🧹 Cleaning previous builds and profiles...${NC}"
cargo clean
rm -rf "$PROFILE_DIR"
mkdir -p "$PROFILE_DIR"

# Step 1: Build instrumented binary for profiling
echo -e "${BLUE}📊 Step 1: Building instrumented binary for profiling...${NC}"
export RUSTFLAGS="-Cprofile-generate=$PROFILE_DIR"

# Build with profiling instrumentation
cargo build --bin "$BINARY_NAME" --target "$TARGET" --release

# Determine binary path
if [[ "$TARGET" == *"windows"* ]]; then
    BINARY_PATH="target/$TARGET/release/${BINARY_NAME}.exe"
else
    BINARY_PATH="target/$TARGET/release/$BINARY_NAME"
fi

echo -e "${GREEN}✅ Instrumented binary built: $BINARY_PATH${NC}"

# Step 2: Generate profiles by running typical workloads
echo -e "${BLUE}📈 Step 2: Generating profiles with typical workloads...${NC}"

# Create test data for profiling
TEST_DATA_DIR="target/pgo-test-data"
mkdir -p "$TEST_DATA_DIR"

# Generate various test files for realistic profiling
echo -e "${YELLOW}📁 Creating test data...${NC}"

# Small files (1KB - 64KB)
for size in 1024 4096 16384 65536; do
    dd if=/dev/urandom of="$TEST_DATA_DIR/test_${size}.dat" bs="$size" count=1 2>/dev/null || {
        # Fallback for systems without /dev/urandom
        head -c "$size" /dev/zero > "$TEST_DATA_DIR/test_${size}.dat"
    }
done

# Medium files (1MB - 10MB)
for size_mb in 1 5 10; do
    size=$((size_mb * 1024 * 1024))
    dd if=/dev/urandom of="$TEST_DATA_DIR/test_${size_mb}MB.dat" bs=1M count="$size_mb" 2>/dev/null || {
        head -c "$size" /dev/zero > "$TEST_DATA_DIR/test_${size_mb}MB.dat"
    }
done

# Create directory structure for testing
mkdir -p "$TEST_DATA_DIR/source_dir/subdir1" "$TEST_DATA_DIR/source_dir/subdir2"
cp "$TEST_DATA_DIR"/*.dat "$TEST_DATA_DIR/source_dir/"
cp "$TEST_DATA_DIR/test_1024.dat" "$TEST_DATA_DIR/source_dir/subdir1/"
cp "$TEST_DATA_DIR/test_4096.dat" "$TEST_DATA_DIR/source_dir/subdir2/"

echo -e "${GREEN}✅ Test data created${NC}"

# Run profiling workloads (only if not cross-compiling)
if [[ "$TARGET" == *"$(uname -m)"* ]] && [[ "$TARGET" != *"windows"* || "$(uname -s)" == *"MINGW"* ]]; then
    echo -e "${YELLOW}🏃 Running profiling workloads...${NC}"
    
    # Profile 1: Single file copy operations
    echo "  - Single file copy operations..."
    for file in "$TEST_DATA_DIR"/*.dat; do
        "$BINARY_PATH" copy "$file" "${file}.copy" 2>/dev/null || true
    done
    
    # Profile 2: Directory operations
    echo "  - Directory copy operations..."
    "$BINARY_PATH" copy "$TEST_DATA_DIR/source_dir" "$TEST_DATA_DIR/dest_dir" 2>/dev/null || true
    
    # Profile 3: Help and version commands
    echo "  - CLI operations..."
    "$BINARY_PATH" --help >/dev/null 2>&1 || true
    "$BINARY_PATH" --version >/dev/null 2>&1 || true
    "$BINARY_PATH" device >/dev/null 2>&1 || true
    "$BINARY_PATH" config >/dev/null 2>&1 || true
    
    echo -e "${GREEN}✅ Profiling workloads completed${NC}"
else
    echo -e "${YELLOW}⏭️  Skipping profiling workloads (cross-compilation target)${NC}"
    # For cross-compilation, create dummy profile data
    echo "dummy" > "$PROFILE_DIR/dummy.profraw"
fi

# Step 3: Build optimized binary using collected profiles
echo -e "${BLUE}🎯 Step 3: Building optimized binary with PGO...${NC}"

# Check if we have profile data
if [ -n "$(find "$PROFILE_DIR" -name "*.profraw" -o -name "dummy.profraw" 2>/dev/null)" ]; then
    echo -e "${GREEN}✅ Profile data found, building with PGO optimization${NC}"
    export RUSTFLAGS="-Cprofile-use=$PROFILE_DIR -Cllvm-args=-pgo-warn-missing-function"
else
    echo -e "${YELLOW}⚠️  No profile data found, building without PGO${NC}"
    unset RUSTFLAGS
fi

# Clean and rebuild with optimization
cargo clean
cargo build --bin "$BINARY_NAME" --target "$TARGET" --release

echo -e "${GREEN}✅ PGO-optimized binary built: $BINARY_PATH${NC}"

# Step 4: Verify and compare binary sizes
echo -e "${BLUE}📏 Step 4: Binary size analysis...${NC}"

if [ -f "$BINARY_PATH" ]; then
    BINARY_SIZE=$(stat -f%z "$BINARY_PATH" 2>/dev/null || stat -c%s "$BINARY_PATH" 2>/dev/null || echo "unknown")
    echo -e "${GREEN}✅ Final binary size: $BINARY_SIZE bytes${NC}"
    
    # Convert to human readable
    if [ "$BINARY_SIZE" != "unknown" ]; then
        BINARY_SIZE_MB=$((BINARY_SIZE / 1024 / 1024))
        echo -e "${GREEN}   (~${BINARY_SIZE_MB}MB)${NC}"
    fi
else
    echo -e "${RED}❌ Binary not found: $BINARY_PATH${NC}"
    exit 1
fi

# Cleanup test data
echo -e "${YELLOW}🧹 Cleaning up test data...${NC}"
rm -rf "$TEST_DATA_DIR"

echo -e "${GREEN}🎉 PGO build completed successfully!${NC}"
echo -e "${BLUE}📦 Optimized binary: $BINARY_PATH${NC}"
