#!/usr/bin/env python3
"""
Basic usage examples for FerroCP.

This script demonstrates the core functionality of the FerroCP library
including file copying, directory operations, and progress monitoring.
"""

import asyncio
import tempfile
import os
from pathlib import Path

import ferrocp


def create_test_files():
    """Create some test files for demonstration."""
    temp_dir = Path(tempfile.mkdtemp(prefix="ferrocp_demo_"))
    
    # Create source directory
    src_dir = temp_dir / "source"
    src_dir.mkdir()
    
    # Create some test files
    (src_dir / "small.txt").write_text("Hello, FerroCP!")
    (src_dir / "medium.txt").write_text("x" * 1024 * 100)  # 100KB
    
    # Create a subdirectory
    sub_dir = src_dir / "subdir"
    sub_dir.mkdir()
    (sub_dir / "nested.txt").write_text("Nested file content")
    
    # Create destination directory
    dst_dir = temp_dir / "destination"
    dst_dir.mkdir()
    
    return src_dir, dst_dir


def basic_file_copy_example():
    """Demonstrate basic file copying."""
    print("=== Basic File Copy Example ===")
    
    src_dir, dst_dir = create_test_files()
    
    # Simple file copy
    source_file = src_dir / "small.txt"
    dest_file = dst_dir / "copied_small.txt"
    
    print(f"Copying {source_file} to {dest_file}")
    
    # Use the synchronous wrapper (blocks until complete)
    result = asyncio.run(ferrocp.copy_file(str(source_file), str(dest_file)))
    
    print(f"Copy completed: {result.success}")
    print(f"Bytes copied: {result.bytes_copied}")
    print(f"Duration: {result.duration_ms}ms")
    print(f"Throughput: {result.throughput_mbps:.2f} MB/s")
    
    # Verify the file was copied
    if dest_file.exists():
        print("✓ File successfully copied")
        print(f"Content: {dest_file.read_text()}")
    else:
        print("✗ File copy failed")


def copy_with_options_example():
    """Demonstrate copying with custom options."""
    print("\n=== Copy with Options Example ===")
    
    src_dir, dst_dir = create_test_files()
    
    source_file = src_dir / "medium.txt"
    dest_file = dst_dir / "verified_copy.txt"
    
    # Create copy options
    options = ferrocp.CopyOptions(
        verify=True,  # Verify the copy
        preserve_timestamps=True,
        enable_compression=False,  # Disable compression for this example
        buffer_size=32768,  # 32KB buffer
    )
    
    print(f"Copying {source_file} with verification enabled")
    
    result = asyncio.run(ferrocp.copy_file(str(source_file), str(dest_file), options))
    
    print(f"Verified copy completed: {result.success}")
    print(f"Bytes copied: {result.bytes_copied}")
    
    if result.success:
        print("✓ File copied and verified successfully")
    else:
        print(f"✗ Copy failed: {result.error_message}")


def progress_monitoring_example():
    """Demonstrate progress monitoring during copy."""
    print("\n=== Progress Monitoring Example ===")
    
    src_dir, dst_dir = create_test_files()
    
    # Create a larger file for better progress demonstration
    large_file = src_dir / "large.txt"
    large_file.write_text("x" * 1024 * 1024)  # 1MB
    
    dest_file = dst_dir / "large_copy.txt"
    
    def progress_callback(progress):
        """Callback function to handle progress updates."""
        print(f"Progress: {progress.percentage:.1f}% "
              f"({progress.bytes_copied}/{progress.total_bytes} bytes) "
              f"Speed: {progress.speed_mbps:.2f} MB/s")
        
        if progress.eta_seconds is not None:
            print(f"ETA: {progress.eta_seconds:.1f} seconds")
    
    print(f"Copying {large_file} with progress monitoring")
    
    result = asyncio.run(ferrocp.copy_file(
        str(large_file), 
        str(dest_file),
        progress_callback=progress_callback
    ))
    
    print(f"Copy with progress completed: {result.success}")


async def async_copy_example():
    """Demonstrate asynchronous copying with cancellation."""
    print("\n=== Async Copy Example ===")
    
    src_dir, dst_dir = create_test_files()
    
    source_file = src_dir / "medium.txt"
    dest_file = dst_dir / "async_copy.txt"
    
    print(f"Starting async copy of {source_file}")
    
    # Start async copy
    operation = await ferrocp.copy_file_async(str(source_file), str(dest_file))
    
    print(f"Async operation started with ID: {operation.id}")
    
    # Monitor the operation
    while await operation.is_running():
        progress = await operation.get_progress()
        if progress is not None:
            print(f"Async progress: {progress * 100:.1f}%")
        
        await asyncio.sleep(0.1)  # Small delay
    
    # Wait for completion
    success = await operation.wait()
    print(f"Async copy completed: {success}")


def directory_copy_example():
    """Demonstrate directory copying."""
    print("\n=== Directory Copy Example ===")
    
    src_dir, dst_dir = create_test_files()
    
    dest_subdir = dst_dir / "copied_directory"
    
    # Copy entire directory
    options = ferrocp.CopyOptions(recursive=True)
    
    print(f"Copying directory {src_dir} to {dest_subdir}")
    
    result = asyncio.run(ferrocp.copy_directory(
        str(src_dir), 
        str(dest_subdir), 
        options
    ))
    
    print(f"Directory copy completed: {result.success}")
    print(f"Files copied: {result.files_copied}")
    print(f"Total bytes: {result.bytes_copied}")
    
    # List copied files
    if dest_subdir.exists():
        print("Copied files:")
        for file_path in dest_subdir.rglob("*"):
            if file_path.is_file():
                print(f"  {file_path.relative_to(dest_subdir)}")


def engine_features_example():
    """Demonstrate engine features and statistics."""
    print("\n=== Engine Features Example ===")
    
    # Get available features
    features = ferrocp.get_features()
    print("Available features:")
    for feature, available in features.items():
        status = "✓" if available else "✗"
        print(f"  {status} {feature}")
    
    # Get statistics
    stats = ferrocp.get_statistics()
    print("\nEngine statistics:")
    for key, value in stats.items():
        print(f"  {key}: {value}")
    
    # Version information
    print(f"\nFerroCP version: {ferrocp.__version__}")


def main():
    """Run all examples."""
    print("FerroCP Python Library Examples")
    print("=" * 40)
    
    try:
        # Run synchronous examples
        basic_file_copy_example()
        copy_with_options_example()
        progress_monitoring_example()
        directory_copy_example()
        engine_features_example()
        
        # Run async example
        asyncio.run(async_copy_example())
        
        print("\n" + "=" * 40)
        print("All examples completed successfully!")
        
    except Exception as e:
        print(f"\nError running examples: {e}")
        import traceback
        traceback.print_exc()


if __name__ == "__main__":
    main()
