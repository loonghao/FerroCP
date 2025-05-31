#!/bin/bash
# Cross-platform Clippy Check Script
# This script runs clippy with platform-aware configurations

set -e

# Parse command line arguments
FIX=false
STRICT=false
TARGET="all"

while [[ $# -gt 0 ]]; do
    case $1 in
        --fix)
            FIX=true
            shift
            ;;
        --strict)
            STRICT=true
            shift
            ;;
        --target)
            TARGET="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

echo "🔍 Running cross-platform Clippy check..."

# Clean build artifacts to avoid version conflicts
echo "🧹 Cleaning build artifacts..."
cargo clean

# Basic clippy check with cross-platform friendly settings
CLIPPY_ARGS=(
    "clippy"
    "--workspace"
    "--all-targets"
    "--"
    "-A" "clippy::cargo_common_metadata"
    "-A" "clippy::module_name_repetitions"
    "-A" "clippy::missing_errors_doc"
    "-A" "clippy::missing_panics_doc"
    "-A" "clippy::too_many_arguments"
    "-A" "clippy::too_many_lines"
    "-A" "clippy::similar_names"
)

if [ "$FIX" = true ]; then
    CLIPPY_ARGS+=("--fix")
    echo "🔧 Running with --fix flag"
fi

if [ "$STRICT" != true ]; then
    # Add more lenient settings for cross-platform compatibility
    CLIPPY_ARGS+=(
        "-A" "clippy::redundant_pub_crate"
        "-A" "clippy::wildcard_imports"
        "-A" "clippy::single_match_else"
    )
    echo "📋 Running in lenient mode (cross-platform friendly)"
else
    echo "⚡ Running in strict mode"
fi

echo "🚀 Executing: cargo ${CLIPPY_ARGS[*]}"

if cargo "${CLIPPY_ARGS[@]}"; then
    echo "✅ Clippy check passed!"
    exit 0
else
    EXIT_CODE=$?
    echo "❌ Clippy check failed with exit code: $EXIT_CODE"
    echo "💡 Try running with --fix flag to auto-fix issues"
    echo "💡 Or use lenient mode for cross-platform compatibility"
    exit $EXIT_CODE
fi
