#!/usr/bin/env python3
"""Generate test data for benchmarks."""

import argparse
import os
import random
from pathlib import Path


def generate_file(path: Path, size: int, pattern: str = "mixed"):
    """Generate a test file with specified pattern."""
    print(f"Generating {path} ({size} bytes, pattern: {pattern})")
    
    with open(path, "wb") as f:
        if pattern == "zeros":
            f.write(b"\x00" * size)
        elif pattern == "ones":
            f.write(b"\xff" * size)
        elif pattern == "random":
            # Generate in chunks to avoid memory issues
            chunk_size = 1024 * 1024  # 1MB chunks
            remaining = size
            while remaining > 0:
                current_chunk = min(chunk_size, remaining)
                chunk = bytes(random.randint(0, 255) for _ in range(current_chunk))
                f.write(chunk)
                remaining -= current_chunk
        elif pattern == "mixed":
            # Mix of compressible and incompressible data
            chunk_size = 1024 * 1024  # 1MB chunks
            remaining = size
            while remaining > 0:
                current_chunk = min(chunk_size, remaining)
                chunk = bytearray()
                for i in range(current_chunk):
                    if i % 1000 < 100:
                        chunk.append(0)  # Compressible zeros
                    elif i % 1000 < 200:
                        chunk.append(255)  # Compressible ones
                    else:
                        chunk.append(i % 256)  # Semi-random
                f.write(chunk)
                remaining -= current_chunk
        else:
            raise ValueError(f"Unknown pattern: {pattern}")


def generate_directory(path: Path, num_files: int, file_size: int, num_subdirs: int = 3):
    """Generate a test directory structure."""
    print(f"Generating directory {path} with {num_files} files of {file_size} bytes each")
    
    path.mkdir(parents=True, exist_ok=True)
    
    # Generate files in root directory
    for i in range(num_files):
        file_path = path / f"file_{i:03d}.dat"
        generate_file(file_path, file_size, "mixed")
    
    # Generate subdirectories
    for i in range(num_subdirs):
        subdir = path / f"subdir_{i:02d}"
        subdir.mkdir(exist_ok=True)
        
        for j in range(num_files // 2):  # Fewer files in subdirs
            file_path = subdir / f"file_{j:03d}.dat"
            generate_file(file_path, file_size, "mixed")


def main():
    """Main function."""
    parser = argparse.ArgumentParser(description="Generate test data for benchmarks")
    parser.add_argument("--output-dir", "-o", type=Path, default="test_files",
                       help="Output directory for test files")
    parser.add_argument("--sizes", nargs="+", default=["1KB", "1MB", "10MB", "100MB"],
                       help="File sizes to generate (e.g., 1KB, 1MB, 10MB)")
    parser.add_argument("--patterns", nargs="+", default=["mixed", "zeros", "random"],
                       help="Data patterns to generate")
    parser.add_argument("--directories", action="store_true",
                       help="Generate test directories")
    
    args = parser.parse_args()
    
    # Create output directory
    args.output_dir.mkdir(parents=True, exist_ok=True)
    
    # Parse sizes
    def parse_size(size_str):
        size_str = size_str.upper()
        if size_str.endswith("KB"):
            return int(size_str[:-2]) * 1024
        elif size_str.endswith("MB"):
            return int(size_str[:-2]) * 1024 * 1024
        elif size_str.endswith("GB"):
            return int(size_str[:-2]) * 1024 * 1024 * 1024
        else:
            return int(size_str)
    
    # Generate files
    for size_str in args.sizes:
        size = parse_size(size_str)
        for pattern in args.patterns:
            filename = f"test_{size_str.lower()}_{pattern}.dat"
            file_path = args.output_dir / filename
            
            if not file_path.exists():
                generate_file(file_path, size, pattern)
            else:
                print(f"Skipping {file_path} (already exists)")
    
    # Generate directories if requested
    if args.directories:
        dir_configs = [
            ("small_dir", 10, 1024),      # 10 files, 1KB each
            ("medium_dir", 50, 10240),    # 50 files, 10KB each
            ("large_dir", 100, 102400),   # 100 files, 100KB each
        ]
        
        for dir_name, num_files, file_size in dir_configs:
            dir_path = args.output_dir / dir_name
            if not dir_path.exists():
                generate_directory(dir_path, num_files, file_size)
            else:
                print(f"Skipping {dir_path} (already exists)")
    
    print("Test data generation complete!")


if __name__ == "__main__":
    main()
