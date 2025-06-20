"""Core performance benchmarks for ferrocp."""

import shutil

import pytest
import ferrocp
from .utils import PerformanceMonitor, create_test_file


class TestFileCopyPerformance:
    """Test file copy performance with different configurations."""
    
    @pytest.mark.benchmark(group="file_copy_sizes")
    def test_copy_small_file(self, benchmark, temp_dir):
        """Benchmark copying small files (1KB)."""
        source = temp_dir / "small_source.dat"
        dest = temp_dir / "small_dest.dat"
        create_test_file(source, 1024)
        
        def copy_file():
            if dest.exists():
                dest.unlink()
            return ferrocp.copy_file(str(source), str(dest))
        
        result = benchmark(copy_file)
        assert dest.exists()
        assert dest.stat().st_size == 1024
    
    @pytest.mark.benchmark(group="file_copy_sizes")
    def test_copy_medium_file(self, benchmark, temp_dir):
        """Benchmark copying medium files (1MB)."""
        source = temp_dir / "medium_source.dat"
        dest = temp_dir / "medium_dest.dat"
        create_test_file(source, 1024 * 1024)
        
        def copy_file():
            if dest.exists():
                dest.unlink()
            return ferrocp.copy(str(source), str(dest))
        
        result = benchmark(copy_file)
        assert dest.exists()
        assert dest.stat().st_size == 1024 * 1024
    
    @pytest.mark.benchmark(group="file_copy_sizes")
    def test_copy_large_file(self, benchmark, temp_dir):
        """Benchmark copying large files (10MB)."""
        source = temp_dir / "large_source.dat"
        dest = temp_dir / "large_dest.dat"
        create_test_file(source, 10 * 1024 * 1024)
        
        def copy_file():
            if dest.exists():
                dest.unlink()
            return ferrocp.copy(str(source), str(dest))
        
        result = benchmark(copy_file)
        assert dest.exists()
        assert dest.stat().st_size == 10 * 1024 * 1024
    
    @pytest.mark.benchmark(group="file_copy_sizes")
    @pytest.mark.slow
    def test_copy_huge_file(self, benchmark, temp_dir):
        """Benchmark copying huge files (100MB)."""
        source = temp_dir / "huge_source.dat"
        dest = temp_dir / "huge_dest.dat"
        create_test_file(source, 100 * 1024 * 1024)
        
        def copy_file():
            if dest.exists():
                dest.unlink()
            return ferrocp.copy(str(source), str(dest))
        
        result = benchmark(copy_file)
        assert dest.exists()
        assert dest.stat().st_size == 100 * 1024 * 1024


class TestCopyEnginePerformance:
    """Test CopyEngine class performance with different configurations."""
    
    @pytest.mark.benchmark(group="thread_counts")
    @pytest.mark.parametrize("thread_count", [1, 2, 4, 8])
    def test_thread_count_performance(self, benchmark, temp_dir, thread_count):
        """Benchmark different thread counts."""
        source = temp_dir / "thread_test_source.dat"
        dest = temp_dir / "thread_test_dest.dat"
        create_test_file(source, 10 * 1024 * 1024)  # 10MB
        
        def copy_with_threads():
            if dest.exists():
                dest.unlink()
            engine = ferrocp.CopyEngine()
            options = ferrocp.CopyOptions()
            options.num_threads = thread_count
            return engine.copy_file(str(source), str(dest), options)
        
        result = benchmark(copy_with_threads)
        assert dest.exists()
    
    @pytest.mark.benchmark(group="compression_levels")
    @pytest.mark.parametrize("compression_level", [0, 1, 3, 6, 9])
    def test_compression_performance(self, benchmark, temp_dir, compression_level):
        """Benchmark different compression levels."""
        source = temp_dir / "compression_test_source.dat"
        dest = temp_dir / "compression_test_dest.dat"
        create_test_file(source, 1024 * 1024)  # 1MB
        
        def copy_with_compression():
            if dest.exists():
                dest.unlink()
            engine = ferrocp.CopyEngine()
            options = ferrocp.CopyOptions()
            options.compression_level = compression_level
            options.enable_compression = compression_level > 0
            return engine.copy_file(str(source), str(dest), options)
        
        result = benchmark(copy_with_compression)
        assert dest.exists()
    
    @pytest.mark.benchmark(group="buffer_sizes")
    @pytest.mark.parametrize("buffer_size", [4*1024, 64*1024, 1024*1024, 8*1024*1024])
    def test_buffer_size_performance(self, benchmark, temp_dir, buffer_size):
        """Benchmark different buffer sizes."""
        source = temp_dir / "buffer_test_source.dat"
        dest = temp_dir / "buffer_test_dest.dat"
        create_test_file(source, 10 * 1024 * 1024)  # 10MB
        
        def copy_with_buffer():
            if dest.exists():
                dest.unlink()
            engine = ferrocp.CopyEngine()
            options = ferrocp.CopyOptions()
            options.buffer_size = buffer_size
            return engine.copy_file(str(source), str(dest), options)
        
        result = benchmark(copy_with_buffer)
        assert dest.exists()


class TestDirectoryCopyPerformance:
    """Test directory copy performance."""
    
    @pytest.mark.benchmark(group="directory_copy")
    @pytest.mark.parametrize("num_files", [10, 50, 100])
    def test_directory_copy_performance(self, benchmark, temp_dir, num_files):
        """Benchmark directory copying with different file counts."""
        from .utils import create_test_directory
        
        source_dir = temp_dir / "source_dir"
        dest_dir = temp_dir / "dest_dir"
        
        # Create test directory
        create_test_directory(source_dir, num_files=num_files, file_size=1024)
        
        def copy_directory():
            if dest_dir.exists():
                shutil.rmtree(dest_dir)
            return ferrocp.copytree(str(source_dir), str(dest_dir))
        
        result = benchmark(copy_directory)
        assert dest_dir.exists()
        assert len(list(dest_dir.rglob("*.dat"))) >= num_files


class TestMemoryUsage:
    """Test memory usage during copy operations."""
    
    @pytest.mark.benchmark(group="memory_usage")
    def test_memory_usage_large_file(self, benchmark, temp_dir):
        """Monitor memory usage during large file copy."""
        source = temp_dir / "memory_test_source.dat"
        dest = temp_dir / "memory_test_dest.dat"
        create_test_file(source, 50 * 1024 * 1024)  # 50MB
        
        monitor = PerformanceMonitor()
        
        def copy_with_monitoring():
            if dest.exists():
                dest.unlink()
            monitor.start()
            result = ferrocp.copy(str(source), str(dest))
            metrics = monitor.stop()
            
            # Store metrics for analysis
            benchmark.extra_info.update({
                "memory_rss_mb": metrics["memory_rss_mb"],
                "memory_vms_mb": metrics["memory_vms_mb"],
                "read_bytes": metrics.get("read_bytes", 0),
                "write_bytes": metrics.get("write_bytes", 0),
            })
            
            return result
        
        result = benchmark(copy_with_monitoring)
        assert dest.exists()


class TestZeroCopyPerformance:
    """Test zero-copy performance when available."""
    
    @pytest.mark.benchmark(group="zerocopy")
    def test_zerocopy_vs_regular(self, benchmark, temp_dir):
        """Compare zero-copy vs regular copy performance."""
        source = temp_dir / "zerocopy_test_source.dat"
        dest = temp_dir / "zerocopy_test_dest.dat"
        create_test_file(source, 10 * 1024 * 1024)  # 10MB
        
        def copy_with_zerocopy():
            if dest.exists():
                dest.unlink()
            engine = ferrocp.CopyEngine()
            options = ferrocp.CopyOptions()
            options.enable_zerocopy = True
            return engine.copy_file(str(source), str(dest), options)
        
        result = benchmark(copy_with_zerocopy)
        assert dest.exists()
        
        # Check if zero-copy was actually used
        if hasattr(result, 'zerocopy_used') and result.zerocopy_used > 0:
            zerocopy_rate = result.zerocopy_bytes / result.bytes_copied * 100
            benchmark.extra_info["zerocopy_rate"] = f"{zerocopy_rate:.1f}%"
