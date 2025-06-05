#!/bin/bash
# Setup script for FerroCP development environment
# This script installs git hooks and sets up the development environment

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

# Check if we're in a git repository
if ! git rev-parse --git-dir > /dev/null 2>&1; then
    log_error "Not in a git repository"
    exit 1
fi

# Get the project root and git directory
PROJECT_ROOT=$(git rev-parse --show-toplevel)
GIT_DIR=$(git rev-parse --git-dir)
HOOKS_DIR="$GIT_DIR/hooks"

cd "$PROJECT_ROOT"

log_info "Setting up FerroCP development environment..."

# 1. Install pre-commit hook
log_info "Installing pre-commit hook..."

if [ ! -d "$HOOKS_DIR" ]; then
    mkdir -p "$HOOKS_DIR"
fi

# Copy and make executable
cp "scripts/pre-commit.sh" "$HOOKS_DIR/pre-commit"
chmod +x "$HOOKS_DIR/pre-commit"

log_success "Pre-commit hook installed"

# 2. Check Rust toolchain
log_info "Checking Rust toolchain..."

if ! command -v rustc &> /dev/null; then
    log_error "Rust is not installed. Please install Rust from https://rustup.rs/"
    exit 1
fi

if ! command -v cargo &> /dev/null; then
    log_error "Cargo is not installed. Please install Rust from https://rustup.rs/"
    exit 1
fi

# Check for required components
log_info "Checking Rust components..."

if ! rustup component list --installed | grep -q "rustfmt"; then
    log_info "Installing rustfmt..."
    rustup component add rustfmt
fi

if ! rustup component list --installed | grep -q "clippy"; then
    log_info "Installing clippy..."
    rustup component add clippy
fi

log_success "Rust toolchain is properly configured"

# 3. Run initial format check
log_info "Running initial format check..."

if cargo fmt --all -- --check; then
    log_success "Code is properly formatted"
else
    log_warning "Code formatting issues found"
    read -p "Do you want to fix formatting issues now? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        cargo fmt --all
        log_success "Code formatting fixed"
    else
        log_warning "Please run 'cargo fmt --all' before committing"
    fi
fi

# 4. Test the setup
log_info "Testing the setup..."

if cargo check --workspace --exclude ferrocp-python; then
    log_success "Project compiles successfully"
else
    log_error "Compilation issues found. Please fix them before continuing."
    exit 1
fi

# 5. Display helpful information
echo ""
log_success "Development environment setup complete!"
echo ""
log_info "Helpful commands:"
echo "  cargo fmt --all              # Format all code"
echo "  cargo clippy --all-targets   # Run linter"
echo "  cargo test --workspace       # Run all tests"
echo "  cargo check --workspace      # Quick compilation check"
echo ""
log_info "The pre-commit hook will automatically:"
echo "  - Check code formatting"
echo "  - Verify compilation"
echo "  - Run clippy checks"
echo "  - Look for common issues"
echo ""
log_warning "If you need to skip the pre-commit hook, use:"
echo "  git commit --no-verify"
echo ""

exit 0
