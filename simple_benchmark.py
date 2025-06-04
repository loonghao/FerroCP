#!/usr/bin/env python3
"""Simple benchmark test for FerroCP."""

import shutil
import tempfile
import time
from pathlib import Path

import ferrocp


def benchmark_file_copy(source_file, iterations=5):
    """Benchmark file copy operations."""
    print(f"\n📊 Benchmarking: {source_file.name}")
    print(f"File size: {source_file.stat().st_size:,} bytes")
    
    ferrocp_times = []
    shutil_times = []
    
    for i in range(iterations):
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)
            
            # Test FerroCP
            dest_ferrocp = temp_path / f"ferrocp_dest_{i}.dat"
            start_time = time.perf_counter()
            result = ferrocp.copyfile(str(source_file), str(dest_ferrocp))
            ferrocp_time = time.perf_counter() - start_time
            ferrocp_times.append(ferrocp_time)
            
            # Test shutil
            dest_shutil = temp_path / f"shutil_dest_{i}.dat"
            start_time = time.perf_counter()
            shutil.copyfile(str(source_file), str(dest_shutil))
            shutil_time = time.perf_counter() - start_time
            shutil_times.append(shutil_time)
            
            print(f"  Iteration {i+1}: FerroCP={ferrocp_time:.4f}s, shutil={shutil_time:.4f}s")
    
    # Calculate averages
    avg_ferrocp = sum(ferrocp_times) / len(ferrocp_times)
    avg_shutil = sum(shutil_times) / len(shutil_times)
    
    # Calculate speed
    file_size_mb = source_file.stat().st_size / (1024 * 1024)
    ferrocp_speed = file_size_mb / avg_ferrocp
    shutil_speed = file_size_mb / avg_shutil
    
    print(f"\n📈 Results for {source_file.name}:")
    print(f"  FerroCP: {avg_ferrocp:.4f}s avg, {ferrocp_speed:.2f} MB/s")
    print(f"  shutil:  {avg_shutil:.4f}s avg, {shutil_speed:.2f} MB/s")
    
    if avg_ferrocp < avg_shutil:
        speedup = avg_shutil / avg_ferrocp
        print(f"  🚀 FerroCP is {speedup:.2f}x faster!")
    else:
        slowdown = avg_ferrocp / avg_shutil
        print(f"  🐌 FerroCP is {slowdown:.2f}x slower")
    
    return {
        'file': source_file.name,
        'size_bytes': source_file.stat().st_size,
        'ferrocp_avg': avg_ferrocp,
        'shutil_avg': avg_shutil,
        'ferrocp_speed_mbps': ferrocp_speed,
        'shutil_speed_mbps': shutil_speed,
        'speedup': avg_shutil / avg_ferrocp
    }


def main():
    """Run benchmarks on test files."""
    print("🚀 Starting FerroCP Performance Benchmarks")
    
    test_data_dir = Path("benchmarks/data/test_files")
    if not test_data_dir.exists():
        print("❌ Test data directory not found. Please run generate_test_data.py first.")
        return 1
    
    # Test files to benchmark
    test_files = [
        "test_1kb_mixed.dat",
        "test_1mb_mixed.dat", 
        "test_10mb_mixed.dat",
        # "test_100mb_mixed.dat",  # Skip large file for quick test
    ]
    
    results = []
    
    for test_file in test_files:
        file_path = test_data_dir / test_file
        if file_path.exists():
            try:
                result = benchmark_file_copy(file_path)
                results.append(result)
            except Exception as e:
                print(f"❌ Error benchmarking {test_file}: {e}")
        else:
            print(f"⚠️  Test file not found: {test_file}")
    
    # Summary
    print("\n" + "="*60)
    print("📊 BENCHMARK SUMMARY")
    print("="*60)
    
    for result in results:
        print(f"{result['file']:20} | {result['speedup']:6.2f}x | "
              f"FerroCP: {result['ferrocp_speed_mbps']:7.1f} MB/s | "
              f"shutil: {result['shutil_speed_mbps']:7.1f} MB/s")
    
    # Overall performance
    if results:
        avg_speedup = sum(r['speedup'] for r in results) / len(results)
        print(f"\nAverage speedup: {avg_speedup:.2f}x")
        
        if avg_speedup > 1.0:
            print("🎉 FerroCP shows performance improvements!")
        else:
            print("📝 FerroCP performance needs optimization")
    
    return 0


if __name__ == "__main__":
    exit(main())
