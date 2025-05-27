"""Pytest configuration for benchmarks."""

import tempfile
from pathlib import Path
from collections.abc import Generator

import pytest


@pytest.fixture(scope="session")
def benchmark_data_dir() -> Generator[Path, None, None]:
    """Create a temporary directory for benchmark data."""
    with tempfile.TemporaryDirectory(prefix="py_eacopy_bench_") as temp_dir:
        yield Path(temp_dir)


@pytest.fixture
def temp_dir() -> Generator[Path, None, None]:
    """Create a temporary directory for individual tests."""
    with tempfile.TemporaryDirectory() as temp_dir:
        yield Path(temp_dir)


@pytest.fixture(scope="session")
def test_files(benchmark_data_dir: Path) -> dict[str, Path]:
    """Generate test files of various sizes."""
    files = {}
    
    # Generate files of different sizes
    sizes = {
        "small": 1024,          # 1KB
        "medium": 1024 * 1024,  # 1MB
        "large": 10 * 1024 * 1024,  # 10MB
        "huge": 100 * 1024 * 1024,  # 100MB
    }
    
    for name, size in sizes.items():
        file_path = benchmark_data_dir / f"test_{name}.dat"
        
        # Generate pseudo-random data for better compression testing
        data = bytearray()
        for i in range(size):
            # Mix of patterns to test compression
            if i % 1000 < 100:
                data.append(0)  # Compressible zeros
            elif i % 1000 < 200:
                data.append(255)  # Compressible ones
            else:
                data.append(i % 256)  # Semi-random data
        
        file_path.write_bytes(data)
        files[name] = file_path
    
    return files


@pytest.fixture(scope="session")
def test_directory(benchmark_data_dir: Path) -> Path:
    """Generate a test directory structure."""
    test_dir = benchmark_data_dir / "test_directory"
    test_dir.mkdir()
    
    # Create nested directory structure
    for i in range(5):
        sub_dir = test_dir / f"subdir_{i}"
        sub_dir.mkdir()
        
        # Create files in each subdirectory
        for j in range(10):
            file_path = sub_dir / f"file_{j}.txt"
            content = f"Test content for file {j} in directory {i}\n" * 100
            file_path.write_text(content)
    
    return test_dir


def pytest_configure(config):
    """Configure pytest for benchmarks."""
    config.addinivalue_line(
        "markers", "benchmark: mark test as a benchmark"
    )
    config.addinivalue_line(
        "markers", "slow: mark test as slow running"
    )


def pytest_collection_modifyitems(config, items):
    """Modify test collection to handle benchmark markers."""
    for item in items:
        # Add benchmark marker to all tests in benchmarks directory
        if "benchmarks" in str(item.fspath):
            item.add_marker(pytest.mark.benchmark)
            
        # Add slow marker to tests that might take a long time
        if any(keyword in item.name.lower() for keyword in ["large", "huge", "directory"]):
            item.add_marker(pytest.mark.slow)
