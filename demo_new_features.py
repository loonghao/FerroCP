#!/usr/bin/env python3
"""Demo script to showcase new EACopy CLI features."""

import os
import subprocess
import tempfile
import time
from pathlib import Path


def create_test_file(path: Path, size: int):
    """Create a test file with specified size."""
    with open(path, 'wb') as f:
        f.write(b'A' * size)


def run_demo():
    """Run a demo of the new features."""
    print("🚀 EACopy New Features Demo")
    print("=" * 50)
    
    with tempfile.TemporaryDirectory() as temp_dir:
        temp_path = Path(temp_dir)
        
        # Create demo files
        source_dir = temp_path / "demo_source"
        dest_dir = temp_path / "demo_dest"
        source_dir.mkdir()
        
        print("📁 Creating demo files...")
        for i in range(10):
            file_path = source_dir / f"demo_file_{i:02d}.dat"
            create_test_file(file_path, 2 * 1024 * 1024)  # 2MB each
        
        print(f"✅ Created 10 files (20MB total)")
        print()
        
        # Demo 1: Show help
        print("1️⃣ Help and Version:")
        print("-" * 30)
        subprocess.run([
            "target/release/eacopy.exe", "--help"
        ], cwd="c:/github/ferrocp")
        print()
        
        # Demo 2: Normal copy with progress
        print("2️⃣ Copy with Progress Bar:")
        print("-" * 30)
        subprocess.run([
            "target/release/eacopy.exe", "copy", 
            str(source_dir), str(dest_dir)
        ], cwd="c:/github/ferrocp")
        print()
        
        # Demo 3: Skip existing files
        print("3️⃣ Skip Existing Files:")
        print("-" * 30)
        subprocess.run([
            "target/release/eacopy.exe", "copy", 
            str(source_dir), str(dest_dir), "--skip-existing"
        ], cwd="c:/github/ferrocp")
        print()
        
        # Demo 4: Mirror mode
        print("4️⃣ Mirror Mode (robocopy /MIR equivalent):")
        print("-" * 30)
        subprocess.run([
            "target/release/eacopy.exe", "copy", 
            str(source_dir), str(dest_dir), "--mirror"
        ], cwd="c:/github/ferrocp")
        print()
        
        # Demo 5: Quiet mode
        print("5️⃣ Quiet Mode:")
        print("-" * 30)
        subprocess.run([
            "target/release/eacopy.exe", "-q", "copy", 
            str(source_dir), str(dest_dir / "quiet")
        ], cwd="c:/github/ferrocp")
        print("(No output in quiet mode)")
        print()
        
        # Demo 6: Custom threading
        print("6️⃣ Custom Threading:")
        print("-" * 30)
        subprocess.run([
            "target/release/eacopy.exe", "-t", "4", "copy", 
            str(source_dir), str(dest_dir / "threaded")
        ], cwd="c:/github/ferrocp")
        print()
        
        print("🎉 Demo completed!")
        print()
        print("📋 Summary of new features:")
        print("  ✅ Modern progress bar with Unicode characters")
        print("  ✅ Detailed copy statistics and timing")
        print("  ✅ Mirror mode equivalent to robocopy /MIR")
        print("  ✅ Auto-detect CPU cores for default threading")
        print("  ✅ Quiet mode for scripting")
        print("  ✅ Skip existing files optimization")


if __name__ == "__main__":
    # Check if binary exists
    eacopy_path = Path("c:/github/ferrocp/target/release/eacopy.exe")
    if not eacopy_path.exists():
        print("❌ EACopy binary not found!")
        print("Please build it first with: cargo build --bin eacopy --release")
    else:
        run_demo()
