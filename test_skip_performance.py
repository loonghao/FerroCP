#!/usr/bin/env python3
"""Simple test script to verify skip existing file performance optimization."""

import os
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

# Add the project root to Python path
sys.path.insert(0, str(Path(__file__).parent))

try:
    import ferrocp
except ImportError:
    print("ferrocp not found. Please build the project first with: uvx nox -s build")
    sys.exit(1)


def create_test_file(path: Path, size: int):
    """Create a test file with specified size."""
    with open(path, 'wb') as f:
        f.write(b'A' * size)


def test_skip_performance():
    """Test skip existing file performance."""
    print("Testing skip existing file performance optimization...")
    
    with tempfile.TemporaryDirectory() as temp_dir:
        temp_path = Path(temp_dir)
        
        # Test 1: Single file skip performance
        print("\n1. Testing single file skip performance:")
        source_file = temp_path / "source.dat"
        dest_file = temp_path / "dest.dat"
        
        # Create source file (10MB)
        file_size = 10 * 1024 * 1024
        create_test_file(source_file, file_size)
        
        # Copy file first time
        start_time = time.time()
        eacopy = ferrocp.EACopy()
        stats1 = eacopy.copy_file(str(source_file), str(dest_file))
        first_copy_time = time.time() - start_time
        
        print(f"   First copy: {first_copy_time:.4f}s, {stats1.files_copied} files copied")
        
        # Make destination newer to ensure it gets skipped
        dest_stat = dest_file.stat()
        os.utime(dest_file, (dest_stat.st_atime, dest_stat.st_mtime + 1))
        
        # Test skip performance
        start_time = time.time()
        stats2 = eacopy.copy_file(str(source_file), str(dest_file), skip_existing=True)
        skip_time = time.time() - start_time
        
        print(f"   Skip copy: {skip_time:.4f}s, {stats2.files_skipped} files skipped")
        print(f"   Skip speedup: {first_copy_time / skip_time:.1f}x faster")
        
        # Test 2: Directory with multiple files
        print("\n2. Testing directory skip performance:")
        source_dir = temp_path / "source_dir"
        dest_dir = temp_path / "dest_dir"
        source_dir.mkdir()
        
        # Create multiple files
        file_count = 100
        small_file_size = 100 * 1024  # 100KB each
        
        for i in range(file_count):
            file_path = source_dir / f"file_{i:03d}.dat"
            create_test_file(file_path, small_file_size)
        
        # First copy
        start_time = time.time()
        stats3 = eacopy.copy_directory(str(source_dir), str(dest_dir))
        first_dir_copy_time = time.time() - start_time
        
        print(f"   First directory copy: {first_dir_copy_time:.4f}s, {stats3.files_copied} files copied")
        
        # Make all destination files newer
        for dest_file in dest_dir.rglob("*.dat"):
            dest_stat = dest_file.stat()
            os.utime(dest_file, (dest_stat.st_atime, dest_stat.st_mtime + 1))
        
        # Test directory skip performance
        start_time = time.time()
        stats4 = eacopy.copy_directory(str(source_dir), str(dest_dir), skip_existing=True)
        dir_skip_time = time.time() - start_time
        
        print(f"   Skip directory copy: {dir_skip_time:.4f}s, {stats4.files_skipped} files skipped")
        print(f"   Directory skip speedup: {first_dir_copy_time / dir_skip_time:.1f}x faster")
        
        # Test 3: Compare with robocopy (Windows only)
        if sys.platform == "win32":
            print("\n3. Comparing with robocopy (Windows):")
            
            # Test robocopy skip performance
            try:
                start_time = time.time()
                cmd = [
                    "robocopy",
                    str(source_dir),
                    str(dest_dir),
                    "/E",   # Copy subdirectories including empty ones
                    "/XO",  # Exclude older files (skip if dest is newer)
                    "/NFL", "/NDL", "/NJH", "/NJS", "/NC", "/NS"  # Minimal output
                ]
                
                result = subprocess.run(cmd, capture_output=True, text=True)
                robocopy_time = time.time() - start_time
                
                if result.returncode in [0, 1]:  # Success codes for robocopy
                    print(f"   Robocopy skip time: {robocopy_time:.4f}s")
                    print(f"   ferrocp vs robocopy: {robocopy_time / dir_skip_time:.1f}x")
                    if dir_skip_time < robocopy_time:
                        print("   ✅ ferrocp is faster than robocopy!")
                    else:
                        print("   ⚠️  robocopy is still faster, but ferrocp is optimized")
                else:
                    print(f"   Robocopy failed with return code: {result.returncode}")
                    
            except FileNotFoundError:
                print("   Robocopy not found (not available on this system)")
            except Exception as e:
                print(f"   Robocopy test failed: {e}")
        
        print("\n✅ Skip performance test completed!")
        print("\nOptimizations implemented:")
        print("  - Single metadata call instead of exists() + metadata()")
        print("  - Fast size comparison before time comparison")
        print("  - Parallel skip checking for directories")
        print("  - Batch processing of file operations")


if __name__ == "__main__":
    test_skip_performance()
