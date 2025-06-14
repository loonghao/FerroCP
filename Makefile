# Makefile for ferrocp development and benchmarking

.PHONY: help install test benchmark profile clean docs build rust-lint rust-fix format format-check setup pre-commit

# Default target
help:
	@echo "Available targets:"
	@echo "  setup            - Set up development environment and git hooks"
	@echo "  install          - Install development dependencies"
	@echo "  format           - Format all code"
	@echo "  format-check     - Check code formatting"
	@echo "  test             - Run unit tests"
	@echo "  lint             - Run code quality checks"
	@echo "  rust-lint        - Run Rust code quality checks"
	@echo "  rust-fix         - Fix Rust code issues automatically"
	@echo "  pre-commit       - Run all pre-commit checks"
	@echo "  benchmark        - Run all performance benchmarks"
	@echo "  benchmark-quick  - Run quick benchmarks (small files only)"
	@echo "  benchmark-compare - Run comparison benchmarks vs standard tools"
	@echo "  profile          - Run performance profiling"
	@echo "  build            - Build project with maturin"
	@echo "  build-pgo        - Build with Profile-Guided Optimization"
	@echo "  build-wheels     - Build wheels for distribution"
	@echo "  build-cross      - Cross-compile for all VFX Platform targets using zigbuild"
	@echo "  build-cross-release - Cross-compile release builds using zigbuild"
	@echo "  flamegraph       - Generate flamegraph (Rust benchmarks)"
	@echo "  docs             - Build documentation"
	@echo "  clean            - Clean build artifacts"

# Setup development environment
setup:
	@echo "Setting up FerroCP development environment..."
	@chmod +x scripts/setup-hooks.sh scripts/pre-commit.sh scripts/format-check.ps1
	@./scripts/setup-hooks.sh

# Code formatting
format:
	@echo "Formatting all code..."
	@cargo fmt --all

format-check:
	@echo "Checking code formatting..."
	@cargo fmt --all -- --check

# Install development dependencies
install:
	uv sync --group all

# Run unit tests
test:
	uv run nox -s test

# Run unit tests (Rust only, fast)
test-rust:
	@echo "Running Rust tests..."
	@BLAKE3_NO_ASM=1 cargo test --workspace --exclude ferrocp-python

# Run linting
lint:
	uv run nox -s lint

# Fix linting issues
lint-fix:
	uv run nox -s lint_fix

# Rust code quality
rust-lint:
	cargo clippy --workspace --exclude ferrocp-python --all-targets --all-features -- -D warnings

rust-fix:
	cargo fmt --all
	cargo fix --workspace --allow-dirty --allow-staged
	cargo clippy --workspace --exclude ferrocp-python --all-targets --all-features -- -D warnings

# Pre-commit checks
pre-commit: format-check rust-lint test-rust
	@echo "All pre-commit checks passed!"

# Run all benchmarks
benchmark:
	uv run nox -s benchmark

# Run quick benchmarks (for development)
benchmark-quick:
	uv run nox -s benchmark -- -k "small_file or medium_file" --benchmark-disable-gc

# Run comparison benchmarks
benchmark-compare:
	uv run nox -s benchmark_compare

# Run CodSpeed benchmarks locally
codspeed:
	uv run nox -s codspeed

# Run all CodSpeed benchmarks
codspeed-all:
	uv run nox -s codspeed_all

# Run performance profiling
profile:
	uv run nox -s profile

# Generate flamegraph (requires flamegraph tool)
flamegraph:
	cargo flamegraph --bench copy_benchmarks

# Run Rust benchmarks
bench-rust:
	cargo bench

# Build project with maturin
build:
	uv run nox -s build

# Build with Profile-Guided Optimization
build-pgo:
	uv run nox -s build_pgo

# Build wheels for distribution
build-wheels:
	uv run nox -s build_wheels

# Verify build works correctly
verify-build:
	uv run nox -s verify_build

# Cross-compilation with cargo-zigbuild
build-cross:
	@echo "Cross-compiling for all VFX Platform targets..."
	@powershell -ExecutionPolicy Bypass -File scripts/zigbuild-cross.ps1

build-cross-release:
	@echo "Cross-compiling release builds for all VFX Platform targets..."
	@powershell -ExecutionPolicy Bypass -File scripts/zigbuild-cross.ps1 -Release

build-cross-target:
	@echo "Cross-compiling for specific target: $(TARGET)"
	@powershell -ExecutionPolicy Bypass -File scripts/zigbuild-cross.ps1 -Target $(TARGET)

# Generate test data
generate-test-data:
	uv run python benchmarks/data/generate_test_data.py --output-dir benchmarks/data/test_files --directories

# Build documentation
docs:
	uv run nox -s docs

# Serve documentation with live reload
docs-serve:
	uv run nox -s docs_serve

# Clean build artifacts
clean:
	rm -rf build/
	rm -rf dist/
	rm -rf wheelhouse/
	rm -rf target/
	rm -rf .nox/
	rm -rf benchmarks/results/*.json
	rm -rf benchmarks/results/*.svg
	rm -rf benchmarks/results/*.prof
	find . -name "*.pyc" -delete
	find . -name "__pycache__" -delete

# Performance regression testing
benchmark-regression:
	@echo "Running baseline benchmark..."
	uv run nox -s benchmark -- --benchmark-save=baseline
	@echo "Run 'make benchmark-compare-baseline' after making changes"

benchmark-compare-baseline:
	uv run nox -s benchmark -- --benchmark-compare=baseline

# Comprehensive performance analysis
analyze-performance: generate-test-data benchmark profile
	@echo "Performance analysis complete!"
	@echo "Check benchmarks/results/ for detailed results"

# CI-style benchmark (faster, less comprehensive)
benchmark-ci:
	uv run nox -s benchmark -- --benchmark-disable-gc --benchmark-min-rounds=3

# Development workflow shortcuts
dev-setup: install build
	@echo "Development environment ready!"

dev-test: lint test
	@echo "Code quality and tests passed!"

dev-benchmark: build benchmark-quick
	@echo "Quick performance check complete!"
