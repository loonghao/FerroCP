# Makefile for py-eacopy development and benchmarking

.PHONY: help install test benchmark profile clean docs

# Default target
help:
	@echo "Available targets:"
	@echo "  install          - Install development dependencies"
	@echo "  test             - Run unit tests"
	@echo "  benchmark        - Run all performance benchmarks"
	@echo "  benchmark-quick  - Run quick benchmarks (small files only)"
	@echo "  benchmark-compare - Run comparison benchmarks vs standard tools"
	@echo "  profile          - Run performance profiling"
	@echo "  flamegraph       - Generate flamegraph (Rust benchmarks)"
	@echo "  docs             - Build documentation"
	@echo "  clean            - Clean build artifacts"
	@echo "  build            - Build wheels"

# Install development dependencies
install:
	uv pip install -e ".[dev,test,benchmark,docs]"

# Run unit tests
test:
	uvx nox -s pytest

# Run all benchmarks
benchmark:
	uvx nox -s benchmark

# Run quick benchmarks (for development)
benchmark-quick:
	uvx nox -s benchmark -- -k "small_file or medium_file" --benchmark-disable-gc

# Run comparison benchmarks
benchmark-compare:
	uvx nox -s benchmark_compare

# Run CodSpeed benchmarks locally
codspeed:
	uvx nox -s codspeed

# Run all CodSpeed benchmarks
codspeed-all:
	uvx nox -s codspeed_all

# Run performance profiling
profile:
	python scripts/profile.py --test-type file_copy --profiler all

# Generate flamegraph (requires flamegraph tool)
flamegraph:
	cargo flamegraph --bench copy_benchmarks

# Run Rust benchmarks
bench-rust:
	cargo bench

# Generate test data
generate-test-data:
	python benchmarks/data/generate_test_data.py --output-dir benchmarks/data/test_files --directories

# Build documentation
docs:
	uvx nox -s docs

# Serve documentation with live reload
docs-serve:
	uvx nox -s docs_serve

# Build wheels
build:
	uvx nox -s build_wheels

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
	uvx nox -s benchmark -- --benchmark-save=baseline
	@echo "Run 'make benchmark-compare-baseline' after making changes"

benchmark-compare-baseline:
	uvx nox -s benchmark -- --benchmark-compare=baseline

# Memory profiling
profile-memory:
	python scripts/profile.py --test-type file_copy --profiler memory

# CPU profiling with py-spy
profile-cpu:
	python scripts/profile.py --test-type file_copy --profiler py-spy

# Comprehensive performance analysis
analyze-performance: generate-test-data benchmark profile
	@echo "Performance analysis complete!"
	@echo "Check benchmarks/results/ for detailed results"

# CI-style benchmark (faster, less comprehensive)
benchmark-ci:
	uvx nox -s benchmark -- --benchmark-disable-gc --benchmark-min-rounds=3
