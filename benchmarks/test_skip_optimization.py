"""Performance tests for skip existing file optimizations."""

import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

import pytest
import ferrocp
from .utils import create_test_file


class TestSkipOptimizationPerformance:
    """Test the performance improvements of skip existing optimizations."""
    
    @pytest.mark.benchmark(group="skip_optimization")
    def test_single_file_skip_performance(self, benchmark, temp_dir):
        """Benchmark single file skip performance with optimized implementation."""
        source = temp_dir / "opt_source.dat"
        dest = temp_dir / "opt_dest.dat"
        
        # Create source and destination files
        create_test_file(source, 50 * 1024 * 1024)  # 50MB
        shutil.copy2(str(source), str(dest))
        
        # Make destination newer
        dest_stat = dest.stat()
        os.utime(dest, (dest_stat.st_atime, dest_stat.st_mtime + 10))
        
        def optimized_skip():
            eacopy = ferrocp.EACopy()
            return eacopy.copy_file(str(source), str(dest), skip_existing=True)
        
        result = benchmark(optimized_skip)
        assert result.files_skipped == 1
        assert result.files_copied == 0
        
        # Verify the optimization worked by checking timing
        benchmark.extra_info.update({
            "file_size": "50MB",
            "optimization": "single_metadata_call"
        })
    
    @pytest.mark.benchmark(group="skip_optimization")
    def test_batch_skip_performance(self, benchmark, temp_dir):
        """Benchmark batch skip performance for multiple files."""
        source_dir = temp_dir / "batch_opt_source"
        dest_dir = temp_dir / "batch_opt_dest"
        
        # Create source directory with many files
        source_dir.mkdir()
        file_count = 500  # Moderate number for reliable benchmarking
        file_size = 100 * 1024  # 100KB each
        
        for i in range(file_count):
            file_path = source_dir / f"batch_file_{i:04d}.dat"
            create_test_file(file_path, file_size)
        
        # Copy to destination first
        shutil.copytree(str(source_dir), str(dest_dir))
        
        # Make all destination files newer
        for dest_file in dest_dir.rglob("*.dat"):
            dest_stat = dest_file.stat()
            os.utime(dest_file, (dest_stat.st_atime, dest_stat.st_mtime + 5))
        
        def batch_skip():
            eacopy = ferrocp.EACopy()
            return eacopy.copy_directory(str(source_dir), str(dest_dir), skip_existing=True)
        
        result = benchmark(batch_skip)
        assert result.files_skipped == file_count
        assert result.files_copied == 0
        
        benchmark.extra_info.update({
            "files_processed": file_count,
            "optimization": "batch_parallel_skip"
        })
    
    @pytest.mark.benchmark(group="skip_optimization")
    def test_mixed_skip_copy_performance(self, benchmark, temp_dir):
        """Benchmark performance with mixed skip and copy operations."""
        source_dir = temp_dir / "mixed_opt_source"
        dest_dir = temp_dir / "mixed_opt_dest"
        
        # Create source directory
        source_dir.mkdir()
        dest_dir.mkdir()
        
        file_count = 200
        file_size = 500 * 1024  # 500KB each
        
        # Create all source files
        for i in range(file_count):
            file_path = source_dir / f"mixed_file_{i:04d}.dat"
            create_test_file(file_path, file_size)
        
        # Copy only half the files to destination (so half will be skipped, half copied)
        for i in range(0, file_count, 2):  # Every other file
            src_file = source_dir / f"mixed_file_{i:04d}.dat"
            dest_file = dest_dir / f"mixed_file_{i:04d}.dat"
            shutil.copy2(str(src_file), str(dest_file))
            
            # Make destination files newer
            dest_stat = dest_file.stat()
            os.utime(dest_file, (dest_stat.st_atime, dest_stat.st_mtime + 3))
        
        def mixed_operation():
            eacopy = ferrocp.EACopy()
            return eacopy.copy_directory(str(source_dir), str(dest_dir), skip_existing=True)
        
        result = benchmark(mixed_operation)
        expected_skipped = file_count // 2
        expected_copied = file_count - expected_skipped
        
        assert result.files_skipped == expected_skipped
        assert result.files_copied == expected_copied
        
        benchmark.extra_info.update({
            "files_skipped": expected_skipped,
            "files_copied": expected_copied,
            "optimization": "mixed_batch_processing"
        })


@pytest.mark.skipif(sys.platform != "win32", reason="Windows-specific robocopy comparison")
class TestOptimizationVsRobocopy:
    """Compare optimized skip performance against robocopy."""
    
    @pytest.mark.benchmark(group="optimization_vs_robocopy")
    def test_large_directory_skip_vs_robocopy(self, benchmark, temp_dir):
        """Compare large directory skip performance against robocopy."""
        source_dir = temp_dir / "large_opt_source"
        dest_dir = temp_dir / "large_opt_dest"
        
        # Create a larger test case
        source_dir.mkdir()
        file_count = 1000
        file_size = 1024 * 1024  # 1MB each
        
        for i in range(file_count):
            file_path = source_dir / f"large_file_{i:04d}.dat"
            create_test_file(file_path, file_size)
        
        # Copy to destination
        shutil.copytree(str(source_dir), str(dest_dir))
        
        # Make all destination files newer
        for dest_file in dest_dir.rglob("*.dat"):
            dest_stat = dest_file.stat()
            os.utime(dest_file, (dest_stat.st_atime, dest_stat.st_mtime + 10))
        
        def ferrocp_optimized_skip():
            eacopy = ferrocp.EACopy()
            return eacopy.copy_directory(str(source_dir), str(dest_dir), skip_existing=True)
        
        def robocopy_skip():
            cmd = [
                "robocopy",
                str(source_dir),
                str(dest_dir),
                "/E",   # Copy subdirectories including empty ones
                "/XO",  # Exclude older files (skip if dest is newer)
                "/NFL", "/NDL", "/NJH", "/NJS", "/NC", "/NS"  # Minimal output
            ]
            
            result = subprocess.run(cmd, capture_output=True, text=True)
            if result.returncode not in [0, 1]:
                raise RuntimeError(f"Robocopy failed: {result.stderr}")
        
        # Benchmark optimized ferrocp
        ferrocp_result = benchmark.pedantic(ferrocp_optimized_skip, iterations=3, rounds=2)
        
        # Benchmark robocopy for comparison
        try:
            times = []
            for _ in range(3):
                start = time.time()
                robocopy_skip()
                times.append(time.time() - start)
            
            robocopy_avg_time = sum(times) / len(times)
            ferrocp_time = benchmark.stats.stats.mean
            
            speedup = robocopy_avg_time / ferrocp_time if ferrocp_time > 0 else 0
            benchmark.extra_info.update({
                "robocopy_time": f"{robocopy_avg_time:.4f}s",
                "speedup": f"{speedup:.2f}x",
                "files_processed": file_count,
                "optimization": "batch_parallel_metadata"
            })
        except Exception as e:
            benchmark.extra_info["robocopy_error"] = str(e)


class TestSkipOptimizationScaling:
    """Test how skip optimizations scale with different file counts and sizes."""
    
    @pytest.mark.benchmark(group="skip_scaling")
    @pytest.mark.parametrize("file_count", [100, 500, 1000, 2000])
    def test_skip_scaling_by_file_count(self, benchmark, temp_dir, file_count):
        """Test skip performance scaling with different file counts."""
        source_dir = temp_dir / f"scale_source_{file_count}"
        dest_dir = temp_dir / f"scale_dest_{file_count}"
        
        source_dir.mkdir()
        file_size = 10 * 1024  # 10KB each (small files for scaling test)
        
        for i in range(file_count):
            file_path = source_dir / f"scale_file_{i:05d}.txt"
            create_test_file(file_path, file_size)
        
        # Copy to destination
        shutil.copytree(str(source_dir), str(dest_dir))
        
        # Make all destination files newer
        for dest_file in dest_dir.rglob("*.txt"):
            dest_stat = dest_file.stat()
            os.utime(dest_file, (dest_stat.st_atime, dest_stat.st_mtime + 2))
        
        def skip_operation():
            eacopy = ferrocp.EACopy()
            return eacopy.copy_directory(str(source_dir), str(dest_dir), skip_existing=True)
        
        result = benchmark(skip_operation)
        assert result.files_skipped == file_count
        assert result.files_copied == 0
        
        # Calculate files per second
        files_per_second = file_count / benchmark.stats.stats.mean if benchmark.stats.stats.mean > 0 else 0
        
        benchmark.extra_info.update({
            "file_count": file_count,
            "files_per_second": f"{files_per_second:.1f}",
            "optimization": "parallel_batch_skip"
        })
    
    @pytest.mark.benchmark(group="skip_scaling")
    @pytest.mark.parametrize("file_size", [1024, 10*1024, 100*1024, 1024*1024])
    def test_skip_scaling_by_file_size(self, benchmark, temp_dir, file_size):
        """Test skip performance scaling with different file sizes."""
        source_dir = temp_dir / f"size_source_{file_size}"
        dest_dir = temp_dir / f"size_dest_{file_size}"
        
        source_dir.mkdir()
        file_count = 100  # Fixed count, varying size
        
        for i in range(file_count):
            file_path = source_dir / f"size_file_{i:03d}.dat"
            create_test_file(file_path, file_size)
        
        # Copy to destination
        shutil.copytree(str(source_dir), str(dest_dir))
        
        # Make all destination files newer
        for dest_file in dest_dir.rglob("*.dat"):
            dest_stat = dest_file.stat()
            os.utime(dest_file, (dest_stat.st_atime, dest_stat.st_mtime + 1))
        
        def skip_by_size():
            eacopy = ferrocp.EACopy()
            return eacopy.copy_directory(str(source_dir), str(dest_dir), skip_existing=True)
        
        result = benchmark(skip_by_size)
        assert result.files_skipped == file_count
        assert result.files_copied == 0
        
        # Calculate throughput
        total_bytes = file_count * file_size
        throughput_mb = (total_bytes / (1024 * 1024)) / benchmark.stats.stats.mean if benchmark.stats.stats.mean > 0 else 0
        
        benchmark.extra_info.update({
            "file_size": f"{file_size // 1024}KB" if file_size < 1024*1024 else f"{file_size // (1024*1024)}MB",
            "throughput_mb_s": f"{throughput_mb:.2f}",
            "optimization": "size_optimized_skip"
        })
