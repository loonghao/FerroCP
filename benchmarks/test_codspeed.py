"""CodSpeed optimized benchmarks for ferrocp.

These benchmarks are specifically designed for CodSpeed continuous performance monitoring.
They focus on the most critical performance paths and provide stable, reproducible results.
"""

import asyncio
import shutil
import tempfile
import uuid
from pathlib import Path

import pytest
import ferrocp


def run_async_safely(coro):
    """Run an async coroutine safely, handling existing event loops."""
    try:
        # Check if there's already a running event loop
        loop = asyncio.get_running_loop()
        # Create a new thread with its own event loop
        import threading
        import queue

        result_queue = queue.Queue()
        exception_queue = queue.Queue()

        def run_in_thread():
            try:
                # Create a new event loop for this thread
                new_loop = asyncio.new_event_loop()
                asyncio.set_event_loop(new_loop)
                try:
                    result = new_loop.run_until_complete(coro)
                    result_queue.put(result)
                finally:
                    new_loop.close()
            except Exception as e:
                exception_queue.put(e)

        thread = threading.Thread(target=run_in_thread)
        thread.start()
        thread.join()

        if not exception_queue.empty():
            raise exception_queue.get()

        return result_queue.get()

    except RuntimeError:
        # No event loop running, safe to use asyncio.run()
        return asyncio.run(coro)


# Test data sizes for different benchmark categories
SMALL_FILE_SIZE = 1024  # 1KB
MEDIUM_FILE_SIZE = 1024 * 1024  # 1MB
LARGE_FILE_SIZE = 10 * 1024 * 1024  # 10MB


def create_test_data(size: int, pattern: str = "mixed") -> bytes:
    """Create test data with specified pattern for consistent benchmarking."""
    if pattern == "zeros":
        return b"\x00" * size
    elif pattern == "random":
        # Use deterministic "random" data for reproducible benchmarks
        return bytes((i * 37 + 42) % 256 for i in range(size))
    elif pattern == "mixed":
        # Mix of compressible and incompressible data
        data = bytearray()
        for i in range(size):
            if i % 1000 < 100:
                data.append(0)  # Compressible zeros
            elif i % 1000 < 200:
                data.append(255)  # Compressible ones
            else:
                data.append((i * 37 + 42) % 256)  # Deterministic pseudo-random
        return bytes(data)
    else:
        return b"A" * size


def get_unique_filename(base_name: str) -> str:
    """Generate a unique filename to avoid conflicts."""
    return f"{base_name}_{uuid.uuid4().hex[:8]}.dat"


@pytest.fixture
def temp_dir():
    """Create a temporary directory for tests."""
    with tempfile.TemporaryDirectory() as temp_dir:
        yield Path(temp_dir)


@pytest.fixture
def small_test_file(temp_dir):
    """Create a small test file."""
    file_path = temp_dir / get_unique_filename("small_test")
    file_path.write_bytes(create_test_data(SMALL_FILE_SIZE))
    return file_path


@pytest.fixture
def medium_test_file(temp_dir):
    """Create a medium test file."""
    file_path = temp_dir / get_unique_filename("medium_test")
    file_path.write_bytes(create_test_data(MEDIUM_FILE_SIZE))
    return file_path


@pytest.fixture
def large_test_file(temp_dir):
    """Create a large test file."""
    file_path = temp_dir / get_unique_filename("large_test")
    file_path.write_bytes(create_test_data(LARGE_FILE_SIZE))
    return file_path


# Core file copy benchmarks
@pytest.mark.benchmark
def test_copy_small_file_codspeed(small_test_file, temp_dir):
    """Benchmark small file copying (1KB) - Core performance metric."""
    dest = temp_dir / get_unique_filename("small_dest")
    run_async_safely(ferrocp.copy_file(str(small_test_file), str(dest)))
    assert dest.exists()
    assert dest.stat().st_size == SMALL_FILE_SIZE


@pytest.mark.benchmark
def test_copy_medium_file_codspeed(medium_test_file, temp_dir):
    """Benchmark medium file copying (1MB) - Core performance metric."""
    dest = temp_dir / get_unique_filename("medium_dest")
    run_async_safely(ferrocp.copy_file(str(medium_test_file), str(dest)))
    assert dest.exists()
    assert dest.stat().st_size == MEDIUM_FILE_SIZE


@pytest.mark.benchmark
def test_copy_large_file_codspeed(large_test_file, temp_dir):
    """Benchmark large file copying (10MB) - Core performance metric."""
    dest = temp_dir / get_unique_filename("large_dest")
    run_async_safely(ferrocp.copy_file(str(large_test_file), str(dest)))
    assert dest.exists()
    assert dest.stat().st_size == LARGE_FILE_SIZE


# Comparison benchmarks (for regression detection)
@pytest.mark.benchmark
def test_shutil_copy_comparison(medium_test_file, temp_dir):
    """Benchmark shutil.copy for comparison baseline."""
    dest = temp_dir / get_unique_filename("shutil_dest")
    shutil.copy(str(medium_test_file), str(dest))
    assert dest.exists()
    assert dest.stat().st_size == MEDIUM_FILE_SIZE


# Compression benchmark
@pytest.mark.benchmark
def test_copy_with_compression(medium_test_file, temp_dir):
    """Benchmark file copying with compression."""
    dest = temp_dir / get_unique_filename("compressed_dest")
    # Use the copy function with compression options
    options = ferrocp.CopyOptions()
    options.compression_level = 3
    options.enable_compression = True
    run_async_safely(ferrocp.copy_file(str(medium_test_file), str(dest), options=options))
    assert dest.exists()


# Threading benchmark
@pytest.mark.benchmark
def test_copy_multi_thread(large_test_file, temp_dir):
    """Benchmark multi-threaded file copying."""
    dest = temp_dir / get_unique_filename("multi_thread_dest")
    # Use the copy function with threading options
    options = ferrocp.CopyOptions()
    options.num_threads = 4
    run_async_safely(ferrocp.copy_file(str(large_test_file), str(dest), options=options))
    assert dest.exists()
