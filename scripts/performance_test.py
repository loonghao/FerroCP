#!/usr/bin/env python3
"""
FerroCP Performance Testing Script

This script demonstrates how to use FerroCP's JSON output for automated performance testing.
"""

import json
import subprocess
import sys
import time
from pathlib import Path
from typing import Dict, List, Any
import tempfile
import os

def run_ferrocp_copy(source: str, destination: str, **kwargs) -> Dict[str, Any]:
    """Run FerroCP copy command and return JSON results."""
    # Try to find ferrocp executable
    ferrocp_path = None
    possible_paths = [
        "./target/debug/ferrocp.exe",
        "./target/release/ferrocp.exe",
        "ferrocp.exe",
        "ferrocp"
    ]

    for path in possible_paths:
        try:
            subprocess.run([path, "--version"], capture_output=True, check=True)
            ferrocp_path = path
            break
        except (subprocess.CalledProcessError, FileNotFoundError):
            continue

    if not ferrocp_path:
        raise FileNotFoundError("Could not find ferrocp executable")

    cmd = [
        ferrocp_path,
        "copy",
        source,
        destination,
        "--json"
    ]
    
    # Add additional options
    for key, value in kwargs.items():
        if value is True:
            cmd.append(f"--{key.replace('_', '-')}")
        elif value is not False and value is not None:
            cmd.extend([f"--{key.replace('_', '-')}", str(value)])
    
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, check=True)
        return json.loads(result.stdout)
    except subprocess.CalledProcessError as e:
        print(f"Error running FerroCP: {e}")
        print(f"Stderr: {e.stderr}")
        sys.exit(1)
    except json.JSONDecodeError as e:
        print(f"Error parsing JSON output: {e}")
        print(f"Raw output: {result.stdout}")
        sys.exit(1)

def create_test_files(base_path: Path, num_files: int = 10, file_size: int = 1024) -> None:
    """Create test files for performance testing."""
    base_path.mkdir(parents=True, exist_ok=True)
    
    for i in range(num_files):
        file_path = base_path / f"test_file_{i:03d}.txt"
        with open(file_path, 'wb') as f:
            f.write(b'A' * file_size)
    
    print(f"Created {num_files} test files of {file_size} bytes each in {base_path}")

def analyze_performance(results: List[Dict[str, Any]]) -> None:
    """Analyze performance results and generate report."""
    print("\n" + "="*60)
    print("PERFORMANCE ANALYSIS REPORT")
    print("="*60)
    
    for i, result in enumerate(results, 1):
        metadata = result['metadata']
        copy_stats = result['copy_stats']
        performance = result['performance_analysis']
        device_result = result['result']
        
        print(f"\nTest {i}: {metadata['source_path']} -> {metadata['destination_path']}")
        print(f"  Files copied: {copy_stats['files_copied']}")
        print(f"  Bytes copied: {copy_stats['bytes_copied']:,}")
        print(f"  Duration: {copy_stats['duration_seconds']:.3f}s")
        print(f"  Actual speed: {copy_stats['actual_transfer_rate_mbps']:.2f} MB/s")
        print(f"  Expected speed: {performance['expected_speed_mbps']:.0f} MB/s")
        print(f"  Efficiency: {copy_stats['performance_efficiency_percent']:.1f}%")
        print(f"  Performance rating: {device_result['performance_rating']}")
        print(f"  Bottleneck: {performance['bottleneck']['device']} ({performance['bottleneck']['limiting_speed_mbps']:.0f} MB/s)")
        
        if copy_stats['errors'] > 0:
            print(f"  ⚠️  Errors: {copy_stats['errors']}")
        
        if copy_stats['performance_efficiency_percent'] < 50:
            print(f"  ⚠️  Low efficiency detected!")
        elif copy_stats['performance_efficiency_percent'] > 90:
            print(f"  ✅ Excellent performance!")

def benchmark_different_scenarios() -> List[Dict[str, Any]]:
    """Run benchmark tests with different scenarios."""
    results = []
    
    with tempfile.TemporaryDirectory() as temp_dir:
        temp_path = Path(temp_dir)
        
        # Test 1: Small files
        print("Running Test 1: Small files (10 files, 1KB each)")
        source1 = temp_path / "small_files"
        dest1 = temp_path / "dest_small"
        create_test_files(source1, num_files=10, file_size=1024)
        result1 = run_ferrocp_copy(str(source1), str(dest1))
        results.append(result1)
        
        # Test 2: Medium files
        print("Running Test 2: Medium files (5 files, 1MB each)")
        source2 = temp_path / "medium_files"
        dest2 = temp_path / "dest_medium"
        create_test_files(source2, num_files=5, file_size=1024*1024)
        result2 = run_ferrocp_copy(str(source2), str(dest2))
        results.append(result2)
        
        # Test 3: Large files
        print("Running Test 3: Large files (2 files, 10MB each)")
        source3 = temp_path / "large_files"
        dest3 = temp_path / "dest_large"
        create_test_files(source3, num_files=2, file_size=10*1024*1024)
        result3 = run_ferrocp_copy(str(source3), str(dest3))
        results.append(result3)
        
        # Test 4: With compression
        print("Running Test 4: With compression (5 files, 1MB each)")
        source4 = temp_path / "compress_test"
        dest4 = temp_path / "dest_compress"
        create_test_files(source4, num_files=5, file_size=1024*1024)
        result4 = run_ferrocp_copy(str(source4), str(dest4), compress=True)
        results.append(result4)
    
    return results

def export_results_to_csv(results: List[Dict[str, Any]], filename: str = "ferrocp_benchmark.csv") -> None:
    """Export results to CSV for further analysis."""
    import csv
    
    with open(filename, 'w', newline='') as csvfile:
        fieldnames = [
            'test_name', 'files_copied', 'bytes_copied', 'duration_seconds',
            'actual_transfer_rate_mbps', 'expected_speed_mbps', 'efficiency_percent',
            'performance_rating', 'bottleneck_device', 'errors', 'source_device_type',
            'destination_device_type', 'compression_used'
        ]
        
        writer = csv.DictWriter(csvfile, fieldnames=fieldnames)
        writer.writeheader()
        
        for i, result in enumerate(results, 1):
            metadata = result['metadata']
            copy_stats = result['copy_stats']
            performance = result['performance_analysis']
            device_result = result['result']
            
            # Determine if compression was used (simple heuristic)
            compression_used = 'compress' in metadata.get('source_path', '').lower()
            
            writer.writerow({
                'test_name': f"Test_{i}",
                'files_copied': copy_stats['files_copied'],
                'bytes_copied': copy_stats['bytes_copied'],
                'duration_seconds': copy_stats['duration_seconds'],
                'actual_transfer_rate_mbps': copy_stats['actual_transfer_rate_mbps'],
                'expected_speed_mbps': performance['expected_speed_mbps'],
                'efficiency_percent': copy_stats['performance_efficiency_percent'],
                'performance_rating': device_result['performance_rating'],
                'bottleneck_device': performance['bottleneck']['device'],
                'errors': copy_stats['errors'],
                'source_device_type': result['source_device']['device_type'],
                'destination_device_type': result['destination_device']['device_type'],
                'compression_used': compression_used
            })
    
    print(f"\n📊 Results exported to {filename}")

def main():
    """Main function to run performance tests."""
    print("FerroCP Performance Testing Suite")
    print("=" * 40)
    
    # Check if ferrocp is available
    try:
        # This will be checked in run_ferrocp_copy function
        pass
    except Exception as e:
        print(f"Error: {e}")
        sys.exit(1)
    
    # Run benchmark tests
    print("Starting benchmark tests...")
    start_time = time.time()
    
    results = benchmark_different_scenarios()
    
    end_time = time.time()
    total_time = end_time - start_time
    
    # Analyze results
    analyze_performance(results)
    
    # Export to CSV
    export_results_to_csv(results)
    
    print(f"\n⏱️  Total benchmark time: {total_time:.2f} seconds")
    print(f"📈 Completed {len(results)} performance tests")
    
    # Summary statistics
    efficiencies = [r['copy_stats']['performance_efficiency_percent'] for r in results]
    avg_efficiency = sum(efficiencies) / len(efficiencies)
    max_efficiency = max(efficiencies)
    min_efficiency = min(efficiencies)
    
    print(f"\n📊 SUMMARY STATISTICS:")
    print(f"  Average efficiency: {avg_efficiency:.1f}%")
    print(f"  Best efficiency: {max_efficiency:.1f}%")
    print(f"  Worst efficiency: {min_efficiency:.1f}%")

if __name__ == "__main__":
    main()
