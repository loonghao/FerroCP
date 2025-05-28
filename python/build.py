#!/usr/bin/env python3
"""
Build script for FerroCP Python package.

This script automates the build process including:
- Compiling the Rust extension
- Copying the compiled library
- Building the Python wheel
- Running tests
"""

import os
import sys
import shutil
import subprocess
import platform
from pathlib import Path


def run_command(cmd, cwd=None, check=True):
    """Run a command and return the result."""
    print(f"Running: {' '.join(cmd)}")
    result = subprocess.run(cmd, cwd=cwd, check=check, capture_output=True, text=True)
    if result.stdout:
        print(result.stdout)
    if result.stderr:
        print(result.stderr, file=sys.stderr)
    return result


def get_project_root():
    """Get the project root directory."""
    return Path(__file__).parent.parent


def get_python_dir():
    """Get the Python package directory."""
    return Path(__file__).parent


def get_target_dir():
    """Get the Rust target directory."""
    return get_project_root() / "target"


def get_library_extension():
    """Get the appropriate library extension for the platform."""
    system = platform.system().lower()
    if system == "windows":
        return ".dll"
    elif system == "darwin":
        return ".dylib"
    else:
        return ".so"


def get_python_extension():
    """Get the appropriate Python extension for the platform."""
    system = platform.system().lower()
    if system == "windows":
        return ".pyd"
    else:
        return ".so"


def build_rust_extension(release=True):
    """Build the Rust extension."""
    print("Building Rust extension...")
    
    project_root = get_project_root()
    
    cmd = ["cargo", "build", "-p", "ferrocp-python"]
    if release:
        cmd.append("--release")
    
    run_command(cmd, cwd=project_root)
    
    # Determine source and destination paths
    build_type = "release" if release else "debug"
    target_dir = get_target_dir() / build_type
    
    lib_name = f"ferrocp_python{get_library_extension()}"
    source_lib = target_dir / lib_name
    
    python_dir = get_python_dir()
    dest_lib = python_dir / "ferrocp" / f"_ferrocp{get_python_extension()}"
    
    if not source_lib.exists():
        raise FileNotFoundError(f"Built library not found: {source_lib}")
    
    print(f"Copying {source_lib} to {dest_lib}")
    shutil.copy2(source_lib, dest_lib)
    
    return dest_lib


def install_dependencies():
    """Install Python dependencies."""
    print("Installing Python dependencies...")
    
    python_dir = get_python_dir()
    
    # Install build dependencies
    run_command([sys.executable, "-m", "pip", "install", "maturin", "wheel", "setuptools"])
    
    # Install package in development mode
    run_command([sys.executable, "-m", "pip", "install", "-e", "."], cwd=python_dir)


def run_tests():
    """Run the test suite."""
    print("Running tests...")
    
    python_dir = get_python_dir()
    
    # Install test dependencies
    run_command([sys.executable, "-m", "pip", "install", "pytest", "pytest-asyncio"])
    
    # Run tests
    test_dir = python_dir / "tests"
    if test_dir.exists():
        run_command([sys.executable, "-m", "pytest", "tests/"], cwd=python_dir)
    else:
        print("No tests directory found, skipping tests")


def build_wheel():
    """Build a wheel package."""
    print("Building wheel package...")
    
    python_dir = get_python_dir()
    
    # Clean previous builds
    dist_dir = python_dir / "dist"
    if dist_dir.exists():
        shutil.rmtree(dist_dir)
    
    build_dir = python_dir / "build"
    if build_dir.exists():
        shutil.rmtree(build_dir)
    
    # Build wheel
    run_command([sys.executable, "setup.py", "bdist_wheel"], cwd=python_dir)
    
    # List built wheels
    if dist_dir.exists():
        wheels = list(dist_dir.glob("*.whl"))
        if wheels:
            print(f"Built wheels:")
            for wheel in wheels:
                print(f"  {wheel}")
        else:
            print("No wheels found in dist directory")


def create_setup_py():
    """Create a minimal setup.py for wheel building."""
    python_dir = get_python_dir()
    setup_py = python_dir / "setup.py"
    
    if not setup_py.exists():
        setup_content = '''
from setuptools import setup, find_packages

# Read version from pyproject.toml or use a default
try:
    import tomllib
    with open("pyproject.toml", "rb") as f:
        pyproject = tomllib.load(f)
    version = pyproject["project"]["version"]
except:
    version = "0.2.0"

setup(
    name="ferrocp",
    version=version,
    packages=find_packages(),
    package_data={
        "ferrocp": ["*.pyd", "*.so", "*.dylib", "py.typed", "*.pyi"],
    },
    include_package_data=True,
    zip_safe=False,
)
'''
        setup_py.write_text(setup_content.strip())
        print(f"Created {setup_py}")


def clean():
    """Clean build artifacts."""
    print("Cleaning build artifacts...")
    
    python_dir = get_python_dir()
    project_root = get_project_root()
    
    # Clean Python build artifacts
    for pattern in ["build", "dist", "*.egg-info", "__pycache__"]:
        for path in python_dir.rglob(pattern):
            if path.is_dir():
                shutil.rmtree(path)
                print(f"Removed {path}")
            elif path.is_file():
                path.unlink()
                print(f"Removed {path}")
    
    # Clean Rust build artifacts
    target_dir = get_target_dir()
    if target_dir.exists():
        shutil.rmtree(target_dir)
        print(f"Removed {target_dir}")
    
    # Remove compiled extension
    ferrocp_dir = python_dir / "ferrocp"
    for ext in [".pyd", ".so", ".dylib"]:
        lib_file = ferrocp_dir / f"_ferrocp{ext}"
        if lib_file.exists():
            lib_file.unlink()
            print(f"Removed {lib_file}")


def main():
    """Main build function."""
    import argparse
    
    parser = argparse.ArgumentParser(description="Build FerroCP Python package")
    parser.add_argument("--clean", action="store_true", help="Clean build artifacts")
    parser.add_argument("--debug", action="store_true", help="Build in debug mode")
    parser.add_argument("--no-tests", action="store_true", help="Skip running tests")
    parser.add_argument("--wheel", action="store_true", help="Build wheel package")
    parser.add_argument("--install-deps", action="store_true", help="Install dependencies")
    
    args = parser.parse_args()
    
    try:
        if args.clean:
            clean()
            return
        
        if args.install_deps:
            install_dependencies()
        
        # Create setup.py if needed for wheel building
        if args.wheel:
            create_setup_py()
        
        # Build the Rust extension
        build_rust_extension(release=not args.debug)
        
        # Run tests unless skipped
        if not args.no_tests:
            try:
                run_tests()
            except subprocess.CalledProcessError:
                print("Tests failed, but continuing with build...")
        
        # Build wheel if requested
        if args.wheel:
            build_wheel()
        
        print("\nBuild completed successfully!")
        
    except Exception as e:
        print(f"Build failed: {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()
