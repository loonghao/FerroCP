"""Comparison benchmarks against standard tools."""

import shutil
import subprocess
import sys

import pytest
import py_eacopy
from .utils import create_test_file, create_test_directory


class TestVsStandardLibrary:
    """Compare py-eacopy against Python standard library."""
    
    @pytest.mark.benchmark(group="vs_shutil")
    @pytest.mark.parametrize("file_size", [1024, 1024*1024, 10*1024*1024])
    def test_vs_shutil_copy(self, benchmark, temp_dir, file_size):
        """Compare against shutil.copy."""
        source = temp_dir / "shutil_test_source.dat"
        dest = temp_dir / "shutil_test_dest.dat"
        create_test_file(source, file_size)
        
        def py_eacopy_copy():
            if dest.exists():
                dest.unlink()
            return py_eacopy.copy(str(source), str(dest))
        
        def shutil_copy():
            dest_shutil = temp_dir / "shutil_dest.dat"
            if dest_shutil.exists():
                dest_shutil.unlink()
            return shutil.copy(str(source), str(dest_shutil))
        
        # Benchmark py-eacopy
        py_eacopy_result = benchmark.pedantic(py_eacopy_copy, iterations=5, rounds=3)
        
        # Benchmark shutil for comparison (not timed by pytest-benchmark)
        import time
        times = []
        for _ in range(5):
            start = time.time()
            shutil_copy()
            times.append(time.time() - start)
        
        shutil_avg_time = sum(times) / len(times)
        py_eacopy_time = benchmark.stats.stats.mean
        
        speedup = shutil_avg_time / py_eacopy_time if py_eacopy_time > 0 else 0
        benchmark.extra_info.update({
            "shutil_time": f"{shutil_avg_time:.4f}s",
            "speedup": f"{speedup:.2f}x",
            "file_size": f"{file_size // 1024}KB" if file_size < 1024*1024 else f"{file_size // (1024*1024)}MB"
        })
    
    @pytest.mark.benchmark(group="vs_shutil_tree")
    def test_vs_shutil_copytree(self, benchmark, temp_dir):
        """Compare directory copying against shutil.copytree."""
        source_dir = temp_dir / "source_tree"
        dest_dir = temp_dir / "dest_tree"
        dest_shutil = temp_dir / "dest_shutil_tree"
        
        # Create test directory
        create_test_directory(source_dir, num_files=20, file_size=1024)
        
        def py_eacopy_copytree():
            if dest_dir.exists():
                shutil.rmtree(dest_dir)
            return py_eacopy.copytree(str(source_dir), str(dest_dir))
        
        def shutil_copytree():
            if dest_shutil.exists():
                shutil.rmtree(dest_shutil)
            return shutil.copytree(str(source_dir), str(dest_shutil))
        
        # Benchmark py-eacopy
        py_eacopy_result = benchmark.pedantic(py_eacopy_copytree, iterations=3, rounds=2)
        
        # Benchmark shutil for comparison
        import time
        times = []
        for _ in range(3):
            start = time.time()
            shutil_copytree()
            times.append(time.time() - start)
        
        shutil_avg_time = sum(times) / len(times)
        py_eacopy_time = benchmark.stats.stats.mean
        
        speedup = shutil_avg_time / py_eacopy_time if py_eacopy_time > 0 else 0
        benchmark.extra_info.update({
            "shutil_time": f"{shutil_avg_time:.4f}s",
            "speedup": f"{speedup:.2f}x"
        })


@pytest.mark.skipif(sys.platform != "win32", reason="Windows-specific tests")
class TestVsRobocopy:
    """Compare against Windows robocopy (Windows only)."""
    
    @pytest.mark.benchmark(group="vs_robocopy")
    def test_vs_robocopy_file(self, benchmark, temp_dir):
        """Compare file copying against robocopy."""
        source = temp_dir / "robocopy_test_source.dat"
        dest_dir = temp_dir / "robocopy_dest"
        dest_dir.mkdir()
        
        create_test_file(source, 10 * 1024 * 1024)  # 10MB
        
        def py_eacopy_copy():
            dest = dest_dir / "py_eacopy_dest.dat"
            if dest.exists():
                dest.unlink()
            return py_eacopy.copy(str(source), str(dest))
        
        def robocopy_copy():
            dest_robocopy = dest_dir / "robocopy_dest.dat"
            if dest_robocopy.exists():
                dest_robocopy.unlink()
            
            cmd = [
                "robocopy",
                str(source.parent),
                str(dest_dir),
                source.name,
                "/NFL", "/NDL", "/NJH", "/NJS", "/NC", "/NS"  # Minimal output
            ]
            
            result = subprocess.run(cmd, capture_output=True, text=True)
            # Robocopy returns 1 for successful copy
            if result.returncode not in [0, 1]:
                raise RuntimeError(f"Robocopy failed: {result.stderr}")
            
            # Rename to expected name
            robocopy_output = dest_dir / source.name
            if robocopy_output.exists():
                robocopy_output.rename(dest_robocopy)
        
        # Benchmark py-eacopy
        py_eacopy_result = benchmark.pedantic(py_eacopy_copy, iterations=3, rounds=2)
        
        # Benchmark robocopy for comparison
        try:
            import time
            times = []
            for _ in range(3):
                start = time.time()
                robocopy_copy()
                times.append(time.time() - start)
            
            robocopy_avg_time = sum(times) / len(times)
            py_eacopy_time = benchmark.stats.stats.mean
            
            speedup = robocopy_avg_time / py_eacopy_time if py_eacopy_time > 0 else 0
            benchmark.extra_info.update({
                "robocopy_time": f"{robocopy_avg_time:.4f}s",
                "speedup": f"{speedup:.2f}x"
            })
        except Exception as e:
            benchmark.extra_info["robocopy_error"] = str(e)
    
    @pytest.mark.benchmark(group="vs_robocopy_tree")
    def test_vs_robocopy_directory(self, benchmark, temp_dir):
        """Compare directory copying against robocopy."""
        source_dir = temp_dir / "robocopy_source_tree"
        dest_dir = temp_dir / "robocopy_dest_tree"
        dest_robocopy = temp_dir / "robocopy_dest_robocopy"
        
        # Create test directory
        create_test_directory(source_dir, num_files=15, file_size=1024)
        
        def py_eacopy_copytree():
            if dest_dir.exists():
                shutil.rmtree(dest_dir)
            return py_eacopy.copytree(str(source_dir), str(dest_dir))
        
        def robocopy_copytree():
            if dest_robocopy.exists():
                shutil.rmtree(dest_robocopy)
            
            cmd = [
                "robocopy",
                str(source_dir),
                str(dest_robocopy),
                "/E",  # Copy subdirectories including empty ones
                "/NFL", "/NDL", "/NJH", "/NJS", "/NC", "/NS"  # Minimal output
            ]
            
            result = subprocess.run(cmd, capture_output=True, text=True)
            # Robocopy returns 1 for successful copy
            if result.returncode not in [0, 1]:
                raise RuntimeError(f"Robocopy failed: {result.stderr}")
        
        # Benchmark py-eacopy
        py_eacopy_result = benchmark.pedantic(py_eacopy_copytree, iterations=3, rounds=2)
        
        # Benchmark robocopy for comparison
        try:
            import time
            times = []
            for _ in range(3):
                start = time.time()
                robocopy_copytree()
                times.append(time.time() - start)
            
            robocopy_avg_time = sum(times) / len(times)
            py_eacopy_time = benchmark.stats.stats.mean
            
            speedup = robocopy_avg_time / py_eacopy_time if py_eacopy_time > 0 else 0
            benchmark.extra_info.update({
                "robocopy_time": f"{robocopy_avg_time:.4f}s",
                "speedup": f"{speedup:.2f}x"
            })
        except Exception as e:
            benchmark.extra_info["robocopy_error"] = str(e)


class TestThroughputComparison:
    """Compare throughput across different tools and configurations."""
    
    @pytest.mark.benchmark(group="throughput_comparison")
    @pytest.mark.slow
    def test_throughput_large_file(self, benchmark, temp_dir):
        """Compare throughput for large file copying."""
        source = temp_dir / "throughput_test_source.dat"
        dest = temp_dir / "throughput_test_dest.dat"
        file_size = 100 * 1024 * 1024  # 100MB
        
        create_test_file(source, file_size)
        
        def copy_and_measure():
            if dest.exists():
                dest.unlink()
            
            import time
            start = time.time()
            result = py_eacopy.copy(str(source), str(dest))
            duration = time.time() - start
            
            throughput_mbps = (file_size / duration) / (1024 * 1024)
            
            benchmark.extra_info.update({
                "file_size_mb": file_size // (1024 * 1024),
                "throughput_mbps": f"{throughput_mbps:.2f}",
                "duration": f"{duration:.3f}s"
            })
            
            return result
        
        result = benchmark(copy_and_measure)
        assert dest.exists()
        assert dest.stat().st_size == file_size
