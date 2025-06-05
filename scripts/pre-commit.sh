#!/bin/bash
# Pre-commit hook for FerroCP project
# This script runs before each commit to ensure code quality

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

# Get the project root
PROJECT_ROOT=$(git rev-parse --show-toplevel)
cd "$PROJECT_ROOT"

log_info "Running pre-commit checks for FerroCP..."

# 1. Check code formatting
log_info "Checking code formatting..."
if cargo fmt --all -- --check; then
    log_success "Code formatting is correct"
else
    log_error "Code formatting issues found!"
    log_info "Run 'cargo fmt --all' to fix formatting issues"
    exit 1
fi

# 2. Check for basic compilation errors
log_info "Checking compilation..."
if cargo check --workspace --exclude ferrocp-python; then
    log_success "Code compiles successfully"
else
    log_error "Compilation errors found!"
    exit 1
fi

# 3. Run clippy for basic linting (non-blocking)
log_info "Running clippy checks..."
if cargo clippy --workspace --exclude ferrocp-python --all-targets --all-features -- -D warnings; then
    log_success "Clippy checks passed"
else
    log_warning "Clippy warnings found (non-blocking)"
fi

# 4. Check for common issues
log_info "Checking for common issues..."

# Check for TODO/FIXME comments in staged files
STAGED_FILES=$(git diff --cached --name-only --diff-filter=ACM | grep -E '\.(rs|toml|md)$' || true)
if [ -n "$STAGED_FILES" ]; then
    TODO_COUNT=$(echo "$STAGED_FILES" | xargs grep -l "TODO\|FIXME" 2>/dev/null | wc -l || echo "0")
    if [ "$TODO_COUNT" -gt 0 ]; then
        log_warning "Found $TODO_COUNT file(s) with TODO/FIXME comments"
        echo "$STAGED_FILES" | xargs grep -n "TODO\|FIXME" 2>/dev/null || true
    fi
fi

# Check for debug prints in Rust files
if [ -n "$STAGED_FILES" ]; then
    DEBUG_COUNT=$(echo "$STAGED_FILES" | grep '\.rs$' | xargs grep -l "println!\|dbg!\|eprintln!" 2>/dev/null | wc -l || echo "0")
    if [ "$DEBUG_COUNT" -gt 0 ]; then
        log_warning "Found $DEBUG_COUNT Rust file(s) with debug prints"
        echo "$STAGED_FILES" | grep '\.rs$' | xargs grep -n "println!\|dbg!\|eprintln!" 2>/dev/null || true
    fi
fi

log_success "Pre-commit checks completed successfully!"
log_info "You can now commit your changes"

exit 0
