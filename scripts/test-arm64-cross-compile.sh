#!/bin/bash
# Test script for ARM64 cross-compilation setup
# This script simulates the CI environment for testing ARM64 cross-compilation

set -e

echo "🔧 Testing ARM64 Cross-Compilation Setup"
echo "========================================"

# Check if running on Ubuntu
if ! grep -q "Ubuntu" /etc/os-release 2>/dev/null; then
    echo "⚠️  This script is designed for Ubuntu. Skipping ARM64 setup test."
    exit 0
fi

# Check if we're on x86_64
if [ "$(uname -m)" != "x86_64" ]; then
    echo "⚠️  This script is designed for x86_64 hosts. Current architecture: $(uname -m)"
    exit 0
fi

echo "📋 Current system information:"
echo "OS: $(lsb_release -d | cut -f2)"
echo "Architecture: $(uname -m)"
echo "Kernel: $(uname -r)"

# Test 1: Check if dpkg supports multi-arch
echo ""
echo "🧪 Test 1: Multi-arch support"
if dpkg --print-foreign-architectures | grep -q arm64; then
    echo "✅ ARM64 architecture already configured"
else
    echo "📋 ARM64 architecture not configured (this is expected)"
fi

# Test 2: Check apt sources configuration
echo ""
echo "🧪 Test 2: APT sources configuration"
echo "Current apt sources:"
grep -E "(ubuntu|ports)" /etc/apt/sources.list | head -5

# Test 3: Check if cross-compilation tools are available
echo ""
echo "🧪 Test 3: Cross-compilation tools availability"
if command -v aarch64-linux-gnu-gcc >/dev/null 2>&1; then
    echo "✅ aarch64-linux-gnu-gcc is available"
    aarch64-linux-gnu-gcc --version | head -1
else
    echo "📋 aarch64-linux-gnu-gcc not installed (this is expected)"
fi

# Test 4: Check Rust cross-compilation target
echo ""
echo "🧪 Test 4: Rust cross-compilation target"
if command -v rustc >/dev/null 2>&1; then
    echo "📋 Rust toolchain information:"
    rustc --version
    
    if rustup target list --installed | grep -q aarch64-unknown-linux-gnu; then
        echo "✅ aarch64-unknown-linux-gnu target is installed"
    else
        echo "📋 aarch64-unknown-linux-gnu target not installed"
        echo "   To install: rustup target add aarch64-unknown-linux-gnu"
    fi
else
    echo "📋 Rust not installed"
fi

# Test 5: Simulate the CI setup process (dry run)
echo ""
echo "🧪 Test 5: Simulate CI setup (dry run)"
echo "This would be the commands run in CI:"
echo ""
echo "sudo dpkg --add-architecture arm64"
echo "sudo sed -i '/^deb.*security\.ubuntu\.com.*arm64/d' /etc/apt/sources.list"
echo "echo 'deb [arch=arm64] http://ports.ubuntu.com/ubuntu-ports jammy main restricted universe multiverse' | sudo tee -a /etc/apt/sources.list"
echo "sudo apt-get update"
echo "sudo apt-get install -y gcc-aarch64-linux-gnu g++-aarch64-linux-gnu libc6-dev-arm64-cross"

echo ""
echo "✅ ARM64 cross-compilation test completed!"
echo ""
echo "📋 To actually set up ARM64 cross-compilation:"
echo "   1. Run the commands shown in Test 5"
echo "   2. Install Rust target: rustup target add aarch64-unknown-linux-gnu"
echo "   3. Set environment variables as shown in CI configuration"
