"""Benchmark tests for skip existing file performance."""

import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

import pytest
import ferrocp
from .utils import create_test_file


class TestSkipExistingPerformance:
    """Test performance of skipping existing files."""
    
    @pytest.mark.benchmark(group="skip_existing")
    def test_skip_existing_single_file(self, benchmark, temp_dir):
        """Benchmark skipping a single existing file."""
        source = temp_dir / "skip_source.dat"
        dest = temp_dir / "skip_dest.dat"
        
        # Create source and destination files
        create_test_file(source, 10 * 1024 * 1024)  # 10MB
        shutil.copy2(str(source), str(dest))
        
        # Make destination slightly newer to ensure it gets skipped
        dest_stat = dest.stat()
        os.utime(dest, (dest_stat.st_atime, dest_stat.st_mtime + 1))
        
        def copy_with_skip():
            eacopy = ferrocp.EACopy()
            return eacopy.copy_file(str(source), str(dest), skip_existing=True)
        
        result = benchmark(copy_with_skip)
        assert result.files_skipped == 1
        assert result.files_copied == 0
    
    @pytest.mark.benchmark(group="skip_existing")
    def test_skip_existing_directory(self, benchmark, temp_dir):
        """Benchmark skipping existing files in a directory."""
        source_dir = temp_dir / "skip_source_dir"
        dest_dir = temp_dir / "skip_dest_dir"
        
        # Create source directory with multiple files
        source_dir.mkdir()
        file_count = 100
        file_size = 1024 * 1024  # 1MB each
        
        for i in range(file_count):
            file_path = source_dir / f"file_{i:03d}.dat"
            create_test_file(file_path, file_size)
        
        # Copy to destination first
        shutil.copytree(str(source_dir), str(dest_dir))
        
        # Make all destination files newer
        for dest_file in dest_dir.rglob("*.dat"):
            dest_stat = dest_file.stat()
            os.utime(dest_file, (dest_stat.st_atime, dest_stat.st_mtime + 1))
        
        def copy_dir_with_skip():
            eacopy = ferrocp.EACopy()
            return eacopy.copy_directory(str(source_dir), str(dest_dir), skip_existing=True)
        
        result = benchmark(copy_dir_with_skip)
        assert result.files_skipped == file_count
        assert result.files_copied == 0


@pytest.mark.skipif(sys.platform != "win32", reason="Windows-specific robocopy tests")
class TestSkipExistingVsRobocopy:
    """Compare skip existing performance against robocopy."""
    
    @pytest.mark.benchmark(group="skip_vs_robocopy")
    def test_skip_single_file_vs_robocopy(self, benchmark, temp_dir):
        """Compare single file skip performance against robocopy."""
        source = temp_dir / "robocopy_skip_source.dat"
        dest_dir = temp_dir / "robocopy_skip_dest"
        dest_dir.mkdir()
        dest_file = dest_dir / source.name
        
        # Create source and destination files
        create_test_file(source, 10 * 1024 * 1024)  # 10MB
        shutil.copy2(str(source), str(dest_file))
        
        # Make destination newer
        dest_stat = dest_file.stat()
        os.utime(dest_file, (dest_stat.st_atime, dest_stat.st_mtime + 1))
        
        def ferrocp_skip():
            eacopy = ferrocp.EACopy()
            return eacopy.copy_file(str(source), str(dest_file), skip_existing=True)
        
        def robocopy_skip():
            cmd = [
                "robocopy",
                str(source.parent),
                str(dest_dir),
                source.name,
                "/XO",  # Exclude older files (skip if dest is newer)
                "/NFL", "/NDL", "/NJH", "/NJS", "/NC", "/NS"  # Minimal output
            ]
            
            result = subprocess.run(cmd, capture_output=True, text=True)
            # Robocopy returns 0 when no files are copied (all skipped)
            if result.returncode not in [0, 1]:
                raise RuntimeError(f"Robocopy failed: {result.stderr}")
        
        # Benchmark ferrocp
        ferrocp_result = benchmark.pedantic(ferrocp_skip, iterations=5, rounds=3)
        
        # Benchmark robocopy for comparison
        try:
            times = []
            for _ in range(5):
                start = time.time()
                robocopy_skip()
                times.append(time.time() - start)
            
            robocopy_avg_time = sum(times) / len(times)
            ferrocp_time = benchmark.stats.stats.mean
            
            speedup = robocopy_avg_time / ferrocp_time if ferrocp_time > 0 else 0
            benchmark.extra_info.update({
                "robocopy_time": f"{robocopy_avg_time:.4f}s",
                "speedup": f"{speedup:.2f}x"
            })
        except Exception as e:
            benchmark.extra_info["robocopy_error"] = str(e)
    
    @pytest.mark.benchmark(group="skip_vs_robocopy")
    def test_skip_directory_vs_robocopy(self, benchmark, temp_dir):
        """Compare directory skip performance against robocopy."""
        source_dir = temp_dir / "robocopy_skip_source_dir"
        dest_dir = temp_dir / "robocopy_skip_dest_dir"
        
        # Create source directory with files
        source_dir.mkdir()
        file_count = 50
        file_size = 2 * 1024 * 1024  # 2MB each
        
        for i in range(file_count):
            file_path = source_dir / f"file_{i:03d}.dat"
            create_test_file(file_path, file_size)
        
        # Copy to destination first
        shutil.copytree(str(source_dir), str(dest_dir))
        
        # Make all destination files newer
        for dest_file in dest_dir.rglob("*.dat"):
            dest_stat = dest_file.stat()
            os.utime(dest_file, (dest_stat.st_atime, dest_stat.st_mtime + 1))
        
        def ferrocp_skip_dir():
            eacopy = ferrocp.EACopy()
            return eacopy.copy_directory(str(source_dir), str(dest_dir), skip_existing=True)
        
        def robocopy_skip_dir():
            cmd = [
                "robocopy",
                str(source_dir),
                str(dest_dir),
                "/E",   # Copy subdirectories including empty ones
                "/XO",  # Exclude older files (skip if dest is newer)
                "/NFL", "/NDL", "/NJH", "/NJS", "/NC", "/NS"  # Minimal output
            ]
            
            result = subprocess.run(cmd, capture_output=True, text=True)
            # Robocopy returns 0 when no files are copied (all skipped)
            if result.returncode not in [0, 1]:
                raise RuntimeError(f"Robocopy failed: {result.stderr}")
        
        # Benchmark ferrocp
        ferrocp_result = benchmark.pedantic(ferrocp_skip_dir, iterations=3, rounds=2)
        
        # Benchmark robocopy for comparison
        try:
            times = []
            for _ in range(3):
                start = time.time()
                robocopy_skip_dir()
                times.append(time.time() - start)
            
            robocopy_avg_time = sum(times) / len(times)
            ferrocp_time = benchmark.stats.stats.mean
            
            speedup = robocopy_avg_time / ferrocp_time if ferrocp_time > 0 else 0
            benchmark.extra_info.update({
                "robocopy_time": f"{robocopy_avg_time:.4f}s",
                "speedup": f"{speedup:.2f}x",
                "files_processed": file_count
            })
        except Exception as e:
            benchmark.extra_info["robocopy_error"] = str(e)


class TestSkipExistingOptimizations:
    """Test different optimization strategies for skip existing."""
    
    @pytest.mark.benchmark(group="skip_optimizations")
    def test_metadata_only_check(self, benchmark, temp_dir):
        """Benchmark metadata-only file comparison."""
        source = temp_dir / "metadata_source.dat"
        dest = temp_dir / "metadata_dest.dat"
        
        # Create identical files
        create_test_file(source, 5 * 1024 * 1024)  # 5MB
        shutil.copy2(str(source), str(dest))
        
        def copy_with_metadata_check():
            eacopy = ferrocp.EACopy()
            return eacopy.copy_file(str(source), str(dest), skip_existing=True)
        
        result = benchmark(copy_with_metadata_check)
        assert result.files_skipped == 1
    
    @pytest.mark.benchmark(group="skip_optimizations")
    def test_batch_metadata_check(self, benchmark, temp_dir):
        """Benchmark batch metadata checking for multiple files."""
        source_dir = temp_dir / "batch_source_dir"
        dest_dir = temp_dir / "batch_dest_dir"
        
        # Create many small files
        source_dir.mkdir()
        file_count = 1000
        file_size = 1024  # 1KB each
        
        for i in range(file_count):
            file_path = source_dir / f"small_{i:04d}.txt"
            create_test_file(file_path, file_size)
        
        # Copy to destination
        shutil.copytree(str(source_dir), str(dest_dir))
        
        def copy_batch_with_skip():
            eacopy = ferrocp.EACopy()
            return eacopy.copy_directory(str(source_dir), str(dest_dir), skip_existing=True)
        
        result = benchmark(copy_batch_with_skip)
        assert result.files_skipped == file_count
        assert result.files_copied == 0
