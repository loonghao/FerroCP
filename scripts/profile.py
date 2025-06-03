#!/usr/bin/env python3
"""Performance profiling script for ferrocp."""

import argparse
import subprocess
import tempfile
import time
from pathlib import Path



def create_test_file(path: Path, size: int):
    """Create a test file with specified size."""
    data = bytearray()
    for i in range(size):
        if i % 1000 < 100:
            data.append(0)  # Compressible zeros
        elif i % 1000 < 200:
            data.append(255)  # Compressible ones
        else:
            data.append(i % 256)  # Semi-random
    
    path.write_bytes(data)


def profile_with_py_spy(script_path: Path, output_path: Path, duration: int = 30):
    """Profile using py-spy."""
    print(f"Profiling with py-spy for {duration} seconds...")
    
    cmd = [
        "py-spy", "record",
        "-o", str(output_path),
        "-d", str(duration),
        "-f", "speedscope",
        "--", "python", str(script_path)
    ]
    
    try:
        subprocess.run(cmd, check=True)
        print(f"Profile saved to {output_path}")
    except subprocess.CalledProcessError as e:
        print(f"py-spy profiling failed: {e}")
    except FileNotFoundError:
        print("py-spy not found. Install with: pip install py-spy")


def profile_with_cprofile(script_path: Path, output_path: Path):
    """Profile using cProfile."""
    print("Profiling with cProfile...")
    
    cmd = [
        "python", "-m", "cProfile",
        "-o", str(output_path),
        str(script_path)
    ]
    
    try:
        subprocess.run(cmd, check=True)
        print(f"Profile saved to {output_path}")
        
        # Also generate text output
        text_output = output_path.with_suffix(".txt")
        with open(text_output, "w") as f:
            subprocess.run([
                "python", "-c",
                f"import pstats; pstats.Stats('{output_path}').sort_stats('cumulative').print_stats(50)"
            ], stdout=f, check=True)
        print(f"Text profile saved to {text_output}")
        
    except subprocess.CalledProcessError as e:
        print(f"cProfile profiling failed: {e}")


def memory_profile(script_path: Path, output_path: Path):
    """Profile memory usage."""
    print("Profiling memory usage...")
    
    cmd = [
        "python", "-m", "memory_profiler",
        str(script_path)
    ]
    
    try:
        with open(output_path, "w") as f:
            subprocess.run(cmd, stdout=f, check=True)
        print(f"Memory profile saved to {output_path}")
    except subprocess.CalledProcessError as e:
        print(f"Memory profiling failed: {e}")
    except FileNotFoundError:
        print("memory_profiler not found. Install with: pip install memory-profiler")


def create_benchmark_script(temp_dir: Path, test_type: str) -> Path:
    """Create a benchmark script for profiling."""
    script_path = temp_dir / "benchmark_script.py"
    
    if test_type == "file_copy":
        script_content = f'''
import ferrocp
from pathlib import Path

# Create test file
source = Path("{temp_dir}") / "source.dat"
dest = Path("{temp_dir}") / "dest.dat"

# Generate 50MB test file
data = bytearray()
size = 50 * 1024 * 1024
for i in range(size):
    if i % 1000 < 100:
        data.append(0)
    elif i % 1000 < 200:
        data.append(255)
    else:
        data.append(i % 256)

source.write_bytes(data)

# Perform multiple copies for profiling
for i in range(10):
    if dest.exists():
        dest.unlink()
    ferrocp.copy(str(source), str(dest))
    print(f"Copy {{i+1}}/10 completed")
'''
    
    elif test_type == "directory_copy":
        script_content = f'''
import ferrocp
from pathlib import Path
import shutil

# Create test directory
source_dir = Path("{temp_dir}") / "source_dir"
dest_dir = Path("{temp_dir}") / "dest_dir"

source_dir.mkdir(exist_ok=True)

# Create test files
for i in range(100):
    file_path = source_dir / f"file_{{i:03d}}.dat"
    data = b"test data " * 1000  # ~9KB per file
    file_path.write_bytes(data)

# Perform multiple directory copies
for i in range(5):
    if dest_dir.exists():
        shutil.rmtree(dest_dir)
    ferrocp.copytree(str(source_dir), str(dest_dir))
    print(f"Directory copy {{i+1}}/5 completed")
'''
    
    elif test_type == "compression":
        script_content = f'''
import ferrocp
from pathlib import Path

# Create test file
source = Path("{temp_dir}") / "source.dat"
dest = Path("{temp_dir}") / "dest.dat"

# Generate compressible test file
data = b"A" * 1000 + b"B" * 1000 + b"\\x00" * 8000  # Highly compressible
full_data = data * 5000  # ~50MB

source.write_bytes(full_data)

# Test different compression levels
for level in [0, 1, 3, 6, 9]:
    if dest.exists():
        dest.unlink()

    engine = ferrocp.CopyEngine()
    options = ferrocp.CopyOptions()
    options.compression_level = level
    options.enable_compression = level > 0
    engine.copy_file(str(source), str(dest), options)
    print(f"Compression level {{level}} completed")
'''
    
    else:
        raise ValueError(f"Unknown test type: {test_type}")
    
    script_path.write_text(script_content)
    return script_path


def main():
    """Main function."""
    parser = argparse.ArgumentParser(description="Profile ferrocp performance")
    parser.add_argument("--test-type", choices=["file_copy", "directory_copy", "compression"],
                       default="file_copy", help="Type of test to profile")
    parser.add_argument("--profiler", choices=["py-spy", "cprofile", "memory", "all"],
                       default="all", help="Profiler to use")
    parser.add_argument("--output-dir", type=Path, default="benchmarks/results",
                       help="Output directory for profiles")
    parser.add_argument("--duration", type=int, default=30,
                       help="Duration for py-spy profiling (seconds)")
    
    args = parser.parse_args()
    
    # Create output directory
    args.output_dir.mkdir(parents=True, exist_ok=True)
    
    # Create temporary directory for test files
    with tempfile.TemporaryDirectory() as temp_dir:
        temp_path = Path(temp_dir)
        
        # Create benchmark script
        script_path = create_benchmark_script(temp_path, args.test_type)
        
        # Generate output filenames
        timestamp = int(time.time())
        base_name = f"{args.test_type}_{timestamp}"
        
        if args.profiler in ["py-spy", "all"]:
            output_path = args.output_dir / f"{base_name}_pyspy.speedscope"
            profile_with_py_spy(script_path, output_path, args.duration)
        
        if args.profiler in ["cprofile", "all"]:
            output_path = args.output_dir / f"{base_name}_cprofile.prof"
            profile_with_cprofile(script_path, output_path)
        
        if args.profiler in ["memory", "all"]:
            output_path = args.output_dir / f"{base_name}_memory.txt"
            memory_profile(script_path, output_path)
    
    print("Profiling complete!")
    print(f"Results saved in {args.output_dir}")
    print("\nTo view results:")
    print("- py-spy: Open .speedscope files at https://www.speedscope.app/")
    print("- cProfile: Use snakeviz or other profile viewers")
    print("- Memory: View .txt files directly")


if __name__ == "__main__":
    main()
