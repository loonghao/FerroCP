"""Utility functions for benchmarks."""

import shutil
import subprocess
import time
from pathlib import Path
from typing import Dict, List

import psutil


class PerformanceMonitor:
    """Monitor system performance during benchmarks."""
    
    def __init__(self):
        self.process = psutil.Process()
        self.start_time = None
        self.start_cpu = None
        self.start_memory = None
        self.start_io = None
    
    def start(self):
        """Start monitoring."""
        self.start_time = time.time()
        self.start_cpu = self.process.cpu_percent()
        self.start_memory = self.process.memory_info()
        try:
            self.start_io = self.process.io_counters()
        except (psutil.AccessDenied, AttributeError):
            self.start_io = None
    
    def stop(self) -> Dict[str, float]:
        """Stop monitoring and return metrics."""
        end_time = time.time()
        end_cpu = self.process.cpu_percent()
        end_memory = self.process.memory_info()
        
        metrics = {
            "duration": end_time - self.start_time,
            "cpu_percent": end_cpu,
            "memory_rss_mb": end_memory.rss / 1024 / 1024,
            "memory_vms_mb": end_memory.vms / 1024 / 1024,
        }
        
        if self.start_io:
            try:
                end_io = self.process.io_counters()
                metrics.update({
                    "read_bytes": end_io.read_bytes - self.start_io.read_bytes,
                    "write_bytes": end_io.write_bytes - self.start_io.write_bytes,
                    "read_count": end_io.read_count - self.start_io.read_count,
                    "write_count": end_io.write_count - self.start_io.write_count,
                })
            except (psutil.AccessDenied, AttributeError):
                pass
        
        return metrics


def generate_test_data(size: int, pattern: str = "mixed") -> bytes:
    """Generate test data with different patterns.
    
    Args:
        size: Size of data to generate in bytes
        pattern: Type of pattern ("zeros", "ones", "random", "mixed")
    
    Returns:
        Generated test data

    """
    if pattern == "zeros":
        return b"\x00" * size
    elif pattern == "ones":
        return b"\xff" * size
    elif pattern == "random":
        import random
        return bytes(random.randint(0, 255) for _ in range(size))
    elif pattern == "mixed":
        # Mix of compressible and incompressible data
        data = bytearray()
        for i in range(size):
            if i % 1000 < 100:
                data.append(0)  # Compressible zeros
            elif i % 1000 < 200:
                data.append(255)  # Compressible ones
            else:
                data.append(i % 256)  # Semi-random
        return bytes(data)
    else:
        raise ValueError(f"Unknown pattern: {pattern}")


def create_test_file(path: Path, size: int, pattern: str = "mixed") -> Path:
    """Create a test file with specified size and pattern."""
    data = generate_test_data(size, pattern)
    path.write_bytes(data)
    return path


def create_test_directory(
    path: Path, 
    num_files: int = 10, 
    file_size: int = 1024,
    num_subdirs: int = 3,
    pattern: str = "mixed"
) -> Path:
    """Create a test directory structure.
    
    Args:
        path: Directory path to create
        num_files: Number of files per directory
        file_size: Size of each file in bytes
        num_subdirs: Number of subdirectories
        pattern: Data pattern for files
    
    Returns:
        Path to created directory

    """
    path.mkdir(parents=True, exist_ok=True)
    
    # Create files in root directory
    for i in range(num_files):
        file_path = path / f"file_{i:03d}.dat"
        create_test_file(file_path, file_size, pattern)
    
    # Create subdirectories with files
    for i in range(num_subdirs):
        subdir = path / f"subdir_{i:02d}"
        subdir.mkdir()
        
        for j in range(num_files):
            file_path = subdir / f"file_{j:03d}.dat"
            create_test_file(file_path, file_size, pattern)
    
    return path


def measure_copy_performance(
    copy_func, 
    source: Path, 
    dest: Path, 
    iterations: int = 1
) -> Dict[str, float]:
    """Measure copy performance with detailed metrics.
    
    Args:
        copy_func: Function to perform the copy
        source: Source path
        dest: Destination path
        iterations: Number of iterations to run
    
    Returns:
        Performance metrics

    """
    total_size = get_path_size(source)
    times = []
    
    monitor = PerformanceMonitor()
    
    for _ in range(iterations):
        # Clean up destination
        if dest.exists():
            if dest.is_dir():
                shutil.rmtree(dest)
            else:
                dest.unlink()
        
        # Measure copy time
        monitor.start()
        start_time = time.time()
        
        copy_func(source, dest)
        
        end_time = time.time()
        metrics = monitor.stop()
        
        times.append(end_time - start_time)
    
    avg_time = sum(times) / len(times)
    throughput_mbps = (total_size / avg_time) / (1024 * 1024) if avg_time > 0 else 0
    
    return {
        "avg_time": avg_time,
        "min_time": min(times),
        "max_time": max(times),
        "throughput_mbps": throughput_mbps,
        "total_size_bytes": total_size,
        **metrics
    }


def get_path_size(path: Path) -> int:
    """Get total size of a path (file or directory)."""
    if path.is_file():
        return path.stat().st_size
    elif path.is_dir():
        total = 0
        for item in path.rglob("*"):
            if item.is_file():
                total += item.stat().st_size
        return total
    else:
        return 0


def run_command_benchmark(command: List[str], iterations: int = 3) -> Dict[str, float]:
    """Benchmark a command line tool.
    
    Args:
        command: Command to run as list of strings
        iterations: Number of iterations
    
    Returns:
        Performance metrics

    """
    times = []
    
    for _ in range(iterations):
        start_time = time.time()
        result = subprocess.run(command, capture_output=True, text=True)
        end_time = time.time()
        
        if result.returncode != 0:
            raise RuntimeError(f"Command failed: {result.stderr}")
        
        times.append(end_time - start_time)
    
    return {
        "avg_time": sum(times) / len(times),
        "min_time": min(times),
        "max_time": max(times),
    }


def format_size(size_bytes: int) -> str:
    """Format size in human readable format."""
    for unit in ['B', 'KB', 'MB', 'GB', 'TB']:
        if size_bytes < 1024.0:
            return f"{size_bytes:.1f} {unit}"
        size_bytes /= 1024.0
    return f"{size_bytes:.1f} PB"


def format_throughput(bytes_per_second: float) -> str:
    """Format throughput in human readable format."""
    return f"{format_size(int(bytes_per_second))}/s"
