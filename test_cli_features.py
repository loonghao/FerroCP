#!/usr/bin/env python3
"""Test script for new CLI features: progress bar, timing, and mirror mode."""

import os
import shutil
import subprocess
import tempfile
import time
from pathlib import Path


def create_test_file(path: Path, size: int):
    """Create a test file with specified size."""
    with open(path, 'wb') as f:
        f.write(b'A' * size)


def run_eacopy_command(args, cwd=None):
    """Run eacopy command and return result."""
    cmd = ["target/release/eacopy.exe"] + args
    print(f"Running: {' '.join(cmd)}")
    
    start_time = time.time()
    result = subprocess.run(cmd, capture_output=True, text=True, cwd=cwd)
    end_time = time.time()
    
    print(f"Command completed in {end_time - start_time:.3f}s")
    print(f"Return code: {result.returncode}")
    if result.stdout:
        print("STDOUT:")
        print(result.stdout)
    if result.stderr:
        print("STDERR:")
        print(result.stderr)
    print("-" * 50)
    
    return result


def test_progress_bar_and_timing():
    """Test progress bar and timing output."""
    print("🧪 Testing Progress Bar and Timing Output")
    print("=" * 60)
    
    with tempfile.TemporaryDirectory() as temp_dir:
        temp_path = Path(temp_dir)
        
        # Create test directory with multiple files
        source_dir = temp_path / "source"
        dest_dir = temp_path / "dest"
        source_dir.mkdir()
        
        print("Creating test files...")
        for i in range(20):
            file_path = source_dir / f"file_{i:02d}.dat"
            create_test_file(file_path, 1024 * 1024)  # 1MB each
        
        print(f"Created 20 files (20MB total) in {source_dir}")
        
        # Test 1: Normal copy with progress bar
        print("\n1. Testing normal copy with progress bar:")
        result = run_eacopy_command([
            "copy", str(source_dir), str(dest_dir)
        ], cwd="c:/github/ferrocp")
        
        # Test 2: Copy with skip existing (should show skipped files)
        print("\n2. Testing copy with skip existing:")
        result = run_eacopy_command([
            "copy", str(source_dir), str(dest_dir), "--skip-existing"
        ], cwd="c:/github/ferrocp")
        
        # Test 3: Mirror mode
        print("\n3. Testing mirror mode:")
        result = run_eacopy_command([
            "copy", str(source_dir), str(dest_dir), "--mirror"
        ], cwd="c:/github/ferrocp")
        
        # Test 4: Quiet mode (should not show progress)
        print("\n4. Testing quiet mode:")
        result = run_eacopy_command([
            "-q", "copy", str(source_dir), str(dest_dir / "quiet_test")
        ], cwd="c:/github/ferrocp")


def test_multithreading():
    """Test default multithreading."""
    print("\n🧪 Testing Default Multithreading")
    print("=" * 60)
    
    with tempfile.TemporaryDirectory() as temp_dir:
        temp_path = Path(temp_dir)
        
        # Create larger test directory
        source_dir = temp_path / "mt_source"
        dest_dir = temp_path / "mt_dest"
        source_dir.mkdir()
        
        print("Creating larger test files for multithreading test...")
        for i in range(50):
            file_path = source_dir / f"large_file_{i:02d}.dat"
            create_test_file(file_path, 2 * 1024 * 1024)  # 2MB each
        
        print(f"Created 50 files (100MB total) in {source_dir}")
        
        # Test with default threading (should auto-detect CPU cores)
        print("\n1. Testing with default threading (auto-detect):")
        result = run_eacopy_command([
            "copy", str(source_dir), str(dest_dir), "--verbose"
        ], cwd="c:/github/ferrocp")
        
        # Test with explicit thread count
        print("\n2. Testing with explicit thread count (8 threads):")
        result = run_eacopy_command([
            "-t", "8", "copy", str(source_dir), str(dest_dir / "threaded"), "--verbose"
        ], cwd="c:/github/ferrocp")


def test_robocopy_comparison():
    """Compare with robocopy for mirror functionality."""
    print("\n🧪 Testing vs Robocopy Mirror Mode")
    print("=" * 60)
    
    with tempfile.TemporaryDirectory() as temp_dir:
        temp_path = Path(temp_dir)
        
        # Create test directory
        source_dir = temp_path / "robocopy_source"
        eacopy_dest = temp_path / "eacopy_dest"
        robocopy_dest = temp_path / "robocopy_dest"
        source_dir.mkdir()
        
        print("Creating test files for robocopy comparison...")
        for i in range(30):
            file_path = source_dir / f"compare_file_{i:02d}.dat"
            create_test_file(file_path, 512 * 1024)  # 512KB each
        
        print(f"Created 30 files (15MB total) in {source_dir}")
        
        # Test eacopy mirror mode
        print("\n1. Testing eacopy mirror mode:")
        result = run_eacopy_command([
            "copy", str(source_dir), str(eacopy_dest), "--mirror"
        ], cwd="c:/github/ferrocp")
        
        # Test robocopy /MIR for comparison
        print("\n2. Testing robocopy /MIR for comparison:")
        try:
            start_time = time.time()
            robocopy_result = subprocess.run([
                "robocopy", str(source_dir), str(robocopy_dest), "/MIR", "/NFL", "/NDL"
            ], capture_output=True, text=True)
            end_time = time.time()
            
            print(f"Robocopy completed in {end_time - start_time:.3f}s")
            print(f"Return code: {robocopy_result.returncode}")
            if robocopy_result.stdout:
                print("STDOUT:")
                print(robocopy_result.stdout)
        except FileNotFoundError:
            print("Robocopy not found (not available on this system)")
        
        # Compare results
        print("\n3. Comparing results:")
        eacopy_files = list(eacopy_dest.rglob("*")) if eacopy_dest.exists() else []
        robocopy_files = list(robocopy_dest.rglob("*")) if robocopy_dest.exists() else []
        
        print(f"EACopy copied: {len([f for f in eacopy_files if f.is_file()])} files")
        print(f"Robocopy copied: {len([f for f in robocopy_files if f.is_file()])} files")


def test_help_and_version():
    """Test help and version commands."""
    print("\n🧪 Testing Help and Version")
    print("=" * 60)
    
    print("1. Testing version command:")
    result = run_eacopy_command(["version"], cwd="c:/github/ferrocp")
    
    print("2. Testing help command:")
    result = run_eacopy_command(["--help"], cwd="c:/github/ferrocp")
    
    print("3. Testing copy help:")
    result = run_eacopy_command(["copy", "--help"], cwd="c:/github/ferrocp")


def main():
    """Run all tests."""
    print("🚀 Testing New EACopy CLI Features")
    print("=" * 60)
    print("Features being tested:")
    print("  ✅ Progress bar (like Rust cargo)")
    print("  ✅ Copy timing and statistics")
    print("  ✅ Mirror mode (robocopy /MIR equivalent)")
    print("  ✅ Default multithreading")
    print("  ✅ Quiet mode support")
    print("=" * 60)
    
    # Check if eacopy binary exists
    eacopy_path = Path("c:/github/ferrocp/target/release/eacopy.exe")
    if not eacopy_path.exists():
        print(f"❌ EACopy binary not found at {eacopy_path}")
        print("Please build it first with: cargo build --bin eacopy --release")
        return
    
    try:
        test_help_and_version()
        test_progress_bar_and_timing()
        test_multithreading()
        test_robocopy_comparison()
        
        print("\n🎉 All tests completed!")
        print("=" * 60)
        print("Summary of new features:")
        print("  📊 Modern progress bar with file names and speed")
        print("  ⏱️  Detailed timing and statistics output")
        print("  🔄 Mirror mode equivalent to robocopy /MIR")
        print("  🧵 Default multithreading (auto-detect CPU cores)")
        print("  🤫 Quiet mode for scripting")
        
    except KeyboardInterrupt:
        print("\n❌ Tests interrupted by user")
    except Exception as e:
        print(f"\n❌ Test failed with error: {e}")


if __name__ == "__main__":
    main()
