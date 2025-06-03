"""Nox configuration for ferrocp development tasks.

This file defines development tasks using nox sessions that work with
the new dependency-groups format and uv package manager.
"""

import os
import shutil
import subprocess
import sys
from pathlib import Path

import nox

# Configure nox to use uv for faster dependency resolution
nox.options.default_venv_backend = "uv"

# Python versions to test against
PYTHON_VERSIONS = ["3.9", "3.10", "3.11", "3.12"]
DEFAULT_PYTHON = "3.11"


def install_with_groups(session, *groups):
    """Install dependencies using uv with dependency groups."""
    if groups:
        for group in groups:
            session.run("uv", "sync", "--group", group, external=True)
    else:
        session.run("uv", "sync", external=True)


def check_build_environment(session):
    """Check and setup build environment for maturin.

    This function checks linker availability, sets environment variables,
    and configures fallback options based on the logic from
    scripts/build-python-wheels.sh.
    """
    session.log("Checking build environment...")

    # Check if basic tools are available
    tools_to_check = ["rustc", "cargo"]
    missing_tools = []

    for tool in tools_to_check:
        if not shutil.which(tool):
            missing_tools.append(tool)

    if missing_tools:
        session.error(f"Missing required tools: {', '.join(missing_tools)}. Please install Rust toolchain.")
        return False

    # Check linker availability
    session.log("Checking linker availability...")

    linker_found = False
    preferred_linker = None

    # Check for available linkers in order of preference
    linkers = [
        ("lld", "lld"),
        ("clang", "clang"),
        ("ld", "ld")
    ]

    for linker_name, command in linkers:
        if shutil.which(command):
            session.log(f"Found {linker_name}: {shutil.which(command)}")
            if not linker_found:
                preferred_linker = (linker_name, command)
                linker_found = True

    if not linker_found:
        session.log("⚠️  No linker found. This may cause build failures.")
        session.log("💡 Try installing build tools:")
        session.log("   - Linux: sudo apt-get install build-essential binutils")
        session.log("   - macOS: xcode-select --install")
        session.log("   - Windows: Install Visual Studio Build Tools")
        return False

    # Set environment variables for stable builds
    session.log("Setting up build environment...")

    # Basic environment variables
    env_vars = {
        "CARGO_NET_GIT_FETCH_WITH_CLI": "true",
        "RUSTFLAGS": "-C opt-level=3"
    }

    # Configure linker based on what's available
    if preferred_linker:
        linker_name, linker_cmd = preferred_linker

        if linker_name == "clang":
            # Use clang as both compiler and linker (most reliable)
            env_vars.update({
                "CC": "clang",
                "CXX": "clang++",
                "RUSTFLAGS": env_vars["RUSTFLAGS"] + " -C linker=clang"
            })
            session.log(f"✅ Using clang as compiler and linker")

        elif linker_name == "lld":
            # Use lld linker (fast and reliable)
            env_vars["RUSTFLAGS"] += " -C link-arg=-fuse-ld=lld"
            session.log(f"✅ Using lld linker")

        else:
            # Use system default linker
            session.log(f"✅ Using system linker: {linker_cmd}")

    # Apply environment variables to the session
    for key, value in env_vars.items():
        os.environ[key] = value
        session.log(f"Set {key}={value}")

    session.log("✅ Build environment check completed successfully")
    return True


def safe_maturin_build(session, *args, **kwargs):
    """Build with maturin using environment checks and fallback strategies.

    This function wraps maturin build calls with proper environment setup
    and error handling, following the project's established patterns.
    """
    # Extract custom env from kwargs if provided
    custom_env = kwargs.pop('env', None)

    # Check build environment first (but skip if custom env is provided for PGO)
    if not custom_env and not check_build_environment(session):
        session.log("⚠️  Build environment check failed, attempting build anyway...")

    try:
        # Attempt the build with current environment
        session.log("Building project with maturin...")
        if custom_env:
            session.run("maturin", *args, env=custom_env, **kwargs)
        else:
            session.run("maturin", *args, **kwargs)
        session.log("✅ Maturin build completed successfully")

    except Exception as e:
        session.log(f"❌ Maturin build failed: {e}")
        session.log("🔄 Attempting fallback build strategy...")

        # Fallback strategy: try with minimal flags
        try:
            # Reset environment to minimal settings
            fallback_env = {
                "CARGO_NET_GIT_FETCH_WITH_CLI": "true",
                "RUSTFLAGS": "-C opt-level=1"  # Lower optimization for compatibility
            }

            # If there was a custom env, merge it with fallback
            if custom_env:
                fallback_env.update(custom_env)

            for key, value in fallback_env.items():
                os.environ[key] = value
                session.log(f"Fallback: Set {key}={value}")

            session.log("Retrying maturin build with fallback settings...")
            session.run("maturin", *args, env=fallback_env, **kwargs)
            session.log("✅ Fallback build completed successfully")

        except Exception as fallback_error:
            session.log(f"❌ Fallback build also failed: {fallback_error}")
            session.log("💡 Troubleshooting suggestions:")
            session.log("   1. Check that Rust toolchain is properly installed")
            session.log("   2. Ensure build tools are available (gcc, clang, or MSVC)")
            session.log("   3. Try running: rustup update")
            session.log("   4. Check for system-specific build requirements")
            raise


@nox.session(python=DEFAULT_PYTHON)
def lint(session):
    """Run linting checks using ruff and mypy."""
    install_with_groups(session, "linting")

    session.log("Running ruff checks...")
    session.run("ruff", "check", ".")

    session.log("Running ruff format check...")
    session.run("ruff", "format", "--check", ".")

    session.log("Running mypy type checking...")
    session.run("mypy", "--install-types", "--non-interactive")
    session.run("mypy", "python/ferrocp", "--strict")


@nox.session(python=DEFAULT_PYTHON)
def lint_fix(session):
    """Fix linting issues automatically."""
    install_with_groups(session, "linting")

    session.log("Fixing ruff issues...")
    session.run("ruff", "check", "--fix", ".")

    session.log("Formatting code with ruff...")
    session.run("ruff", "format", ".")

    session.log("Fixing import order with isort...")
    session.run("isort", ".")


@nox.session(python=PYTHON_VERSIONS)
def test(session):
    """Run tests with pytest."""
    install_with_groups(session, "testing")

    # Build the project first with environment checks
    safe_maturin_build(session, "develop", "--release")

    session.log("Running tests...")
    session.run(
        "pytest",
        "tests/",
        "--cov=ferrocp",
        "--cov-report=xml:coverage.xml",
        "--cov-report=term-missing",
        "--cov-report=html:htmlcov",
    )


@nox.session(python=DEFAULT_PYTHON)
def benchmark(session):
    """Run performance benchmarks."""
    install_with_groups(session, "testing")

    # Build the project first with environment checks
    safe_maturin_build(session, "develop", "--release")

    # Ensure results directory exists
    os.makedirs("benchmarks/results", exist_ok=True)

    session.log("Running benchmarks...")
    session.run(
        "pytest",
        "benchmarks/",
        "--benchmark-only",
        "--benchmark-sort=mean",
        "--benchmark-json=benchmarks/results/benchmark.json",
    )


@nox.session(python=DEFAULT_PYTHON)
def benchmark_compare(session):
    """Run comparison benchmarks against standard tools."""
    install_with_groups(session, "testing")

    # Build the project first with environment checks
    safe_maturin_build(session, "develop", "--release")

    session.log("Running comparison benchmarks...")
    session.run(
        "pytest",
        "benchmarks/test_comparison.py",
        "--benchmark-only",
        "--benchmark-sort=mean",
    )


@nox.session(python=DEFAULT_PYTHON)
def profile(session):
    """Run performance profiling tools."""
    install_with_groups(session, "testing")

    # Build the project first with environment checks
    safe_maturin_build(session, "develop", "--release")

    session.log("Performance profiling tools available:")
    session.log("  py-spy record -o profile.svg -- python your_script.py")
    session.log("  memory_profiler: python -m memory_profiler your_script.py")
    session.log("  cProfile: python -m cProfile -o profile.prof your_script.py")


@nox.session(python=DEFAULT_PYTHON)
def codspeed(session):
    """Run CodSpeed benchmarks locally."""
    install_with_groups(session, "testing")

    # Build the project first with environment checks
    safe_maturin_build(session, "develop", "--release")

    # Generate test data if needed
    os.makedirs("benchmarks/data/test_files", exist_ok=True)
    session.run("python", "benchmarks/data/generate_test_data.py",
                "--output-dir", "benchmarks/data/test_files")

    session.log("Running CodSpeed benchmarks...")
    session.run("pytest", "benchmarks/test_codspeed.py", "--codspeed")


@nox.session(python=DEFAULT_PYTHON)
def codspeed_all(session):
    """Run all benchmarks with CodSpeed locally."""
    install_with_groups(session, "testing")

    # Build the project first with environment checks
    safe_maturin_build(session, "develop", "--release")

    # Generate test data if needed
    os.makedirs("benchmarks/data/test_files", exist_ok=True)
    session.run("python", "benchmarks/data/generate_test_data.py",
                "--output-dir", "benchmarks/data/test_files")

    session.log("Running all CodSpeed benchmarks...")
    session.run("pytest", "benchmarks/", "--codspeed", "-k", "not comparison")


@nox.session(python=DEFAULT_PYTHON)
def rust_coverage(session):
    """Run Rust code coverage analysis using cargo-tarpaulin (fallback to basic test coverage)."""
    session.log("Attempting to run Rust coverage analysis...")

    # Ensure coverage directory exists
    os.makedirs("coverage", exist_ok=True)

    # Try cargo-tarpaulin first (if available and working)
    try:
        session.log("Trying cargo-tarpaulin...")
        session.run("cargo", "tarpaulin", "--version", external=True)

        session.log("Running cargo-tarpaulin...")
        session.run(
            "cargo", "tarpaulin",
            "--workspace",
            "--all-features",
            "--out", "xml", "html", "lcov",
            "--output-dir", "coverage/",
            "--timeout", "120",
            "--skip-clean",
            external=True
        )

        session.log("Tarpaulin coverage report generated successfully")

    except Exception as e:
        session.log(f"Tarpaulin failed: {e}")
        session.log("Falling back to basic test coverage analysis...")

        # Use our custom coverage script
        session.run("python", "scripts/generate_rust_coverage.py", external=True)
        session.log("Basic coverage report generated using fallback script")

    # Display coverage summary
    coverage_xml = Path("coverage/cobertura.xml")
    coverage_html = Path("coverage/index.html")

    if coverage_xml.exists():
        session.log("Coverage report files:")
        session.log(f"  - XML: {coverage_xml}")
        if coverage_html.exists():
            session.log(f"  - HTML: {coverage_html}")
        if Path("coverage/lcov.info").exists():
            session.log(f"  - LCOV: coverage/lcov.info")
    else:
        session.log("Warning: No coverage files generated")


@nox.session(python=DEFAULT_PYTHON)
def coverage_all(session):
    """Run comprehensive coverage analysis for both Python and Rust code."""
    session.log("Running comprehensive coverage analysis...")

    # Run Python coverage first
    session.log("=== Running Python Coverage ===")
    install_with_groups(session, "testing")

    # Build the project first with environment checks
    safe_maturin_build(session, "develop", "--release")

    session.log("Running Python tests with coverage...")
    session.run(
        "pytest",
        "tests/",
        "--cov=ferrocp",
        "--cov-report=xml:coverage/python-coverage.xml",
        "--cov-report=term-missing",
        "--cov-report=html:coverage/python-htmlcov",
    )

    # Run Rust coverage
    session.log("=== Running Rust Coverage ===")

    # Call the rust_coverage session
    session.run("nox", "-s", "rust_coverage", external=True)

    # Generate combined coverage summary
    session.log("=== Coverage Summary ===")
    session.log("Coverage reports generated:")
    session.log("  Python:")
    session.log("    - XML: coverage/python-coverage.xml")
    session.log("    - HTML: coverage/python-htmlcov/")
    session.log("  Rust:")
    session.log("    - XML: coverage/cobertura.xml")
    session.log("    - HTML: coverage/tarpaulin-report.html")
    session.log("    - LCOV: coverage/lcov.info")


@nox.session(python=DEFAULT_PYTHON)
def docs(session):
    """Build documentation."""
    install_with_groups(session, "docs")

    # Build the project first for API documentation with environment checks
    safe_maturin_build(session, "develop", "--release")

    session.log("Building documentation...")
    with session.chdir("docs"):
        session.run("make", "html", external=True)


@nox.session(python=DEFAULT_PYTHON)
def docs_serve(session):
    """Build and serve documentation with live reloading."""
    install_with_groups(session, "docs")

    # Build the project first for API documentation with environment checks
    safe_maturin_build(session, "develop", "--release")

    session.log("Starting documentation server with live reloading...")
    session.run("sphinx-autobuild", "docs", "docs/_build/html", "--open-browser")


@nox.session(python=DEFAULT_PYTHON)
def docs_only(session):
    """Build documentation without building the project (for CI environments)."""
    install_with_groups(session, "docs")

    session.log("Building documentation without project compilation...")
    session.log("This session is optimized for CI environments and does not require Rust compilation.")

    with session.chdir("docs"):
        session.run("make", "html", external=True)

    session.log("Documentation build completed successfully!")


@nox.session(python=DEFAULT_PYTHON)
def build(session):
    """Build the project using maturin."""
    install_with_groups(session, "build")

    # Build with environment checks
    safe_maturin_build(session, "build", "--release")

    # List built wheels
    dist_dir = Path("target/wheels")
    if dist_dir.exists():
        wheels = list(dist_dir.glob("*.whl"))
        session.log(f"Built {len(wheels)} wheels:")
        for wheel in wheels:
            session.log(f"  - {wheel.name}")
    else:
        session.log("No wheels found in target/wheels")


@nox.session(python=DEFAULT_PYTHON)
def build_pgo(session):
    """Build the project with Profile-Guided Optimization (PGO)."""
    install_with_groups(session, "build", "testing")

    session.log("Building PGO-optimized wheel...")

    # Create a temporary directory for PGO data
    pgo_dir = Path("pgo_data")
    pgo_dir.mkdir(exist_ok=True)

    try:
        # Step 1: Build with profile generation
        session.log("Step 1: Building with profile generation...")
        env = {"RUSTFLAGS": f"-Cprofile-generate={pgo_dir.absolute()}"}
        safe_maturin_build(session, "build", "--release", env=env)

        # Step 2: Install and run benchmarks to collect profile data
        session.log("Step 2: Collecting profile data...")
        wheels = list(Path("target/wheels").glob("*.whl"))
        if wheels:
            session.run("pip", "install", str(wheels[-1]), "--force-reinstall")

            # Run some basic operations to collect profile data
            session.run("python", "-c", """
import ferrocp
import tempfile
from pathlib import Path

# Create test data and run copy operations
with tempfile.TemporaryDirectory() as temp_dir:
    temp_path = Path(temp_dir)
    source_dir = temp_path / 'source'
    dest_dir = temp_path / 'dest'
    source_dir.mkdir()
    dest_dir.mkdir()

    # Create test files
    for i in range(50):
        test_file = source_dir / f'test_{i}.txt'
        test_file.write_text(f'Test content {i}' * 100)

    # Run copy operations
    engine = ferrocp.CopyEngine()
    options = ferrocp.CopyOptions()
    for i in range(20):
        try:
            engine.copy_file(
                str(source_dir / f'test_{i}.txt'),
                str(dest_dir / f'test_{i}.txt'),
                options
            )
        except Exception:
            continue

print('Profile data collection completed')
""")

        # Step 3: Merge profile data and rebuild
        session.log("Step 3: Merging profile data and rebuilding...")

        # Use LLVM profdata tool
        try:
            # Try to find llvm-profdata
            profdata_cmd = "llvm-profdata"

            # Merge profile data
            merged_profile = pgo_dir / "merged.profdata"
            session.run(
                profdata_cmd, "merge",
                "-o", str(merged_profile),
                *[str(f) for f in pgo_dir.glob("*.profraw")],
                external=True
            )

            # Step 4: Build with profile use
            session.log("Step 4: Building optimized version...")
            env = {"RUSTFLAGS": f"-Cprofile-use={merged_profile.absolute()}"}
            safe_maturin_build(session, "build", "--release", env=env)

        except Exception as e:
            session.log(f"PGO optimization failed: {e}")
            session.log("Falling back to regular build...")
            safe_maturin_build(session, "build", "--release")

    finally:
        # Clean up PGO data
        import shutil
        if pgo_dir.exists():
            shutil.rmtree(pgo_dir)

    # List built wheels
    dist_dir = Path("target/wheels")
    if dist_dir.exists():
        wheels = list(dist_dir.glob("*.whl"))
        session.log(f"Built {len(wheels)} PGO wheels:")
        for wheel in wheels:
            session.log(f"  - {wheel.name}")


@nox.session(python=DEFAULT_PYTHON)
def build_wheels(session):
    """Build wheels using cibuildwheel for multiple platforms."""
    install_with_groups(session, "build")

    session.log("Building wheels with cibuildwheel...")
    session.run("cibuildwheel", "--output-dir", "wheelhouse")

    # List built wheels
    wheelhouse = Path("wheelhouse")
    if wheelhouse.exists():
        wheels = list(wheelhouse.glob("*.whl"))
        session.log(f"Built {len(wheels)} wheels:")
        for wheel in wheels:
            session.log(f"  - {wheel.name}")


@nox.session(python=DEFAULT_PYTHON)
def verify_build(session):
    """Verify the built wheels work correctly."""
    # Look for wheels in both possible locations
    wheel_dirs = [Path("target/wheels"), Path("wheelhouse"), Path("dist")]
    wheels = []

    for wheel_dir in wheel_dirs:
        if wheel_dir.exists():
            wheels.extend(wheel_dir.glob("*.whl"))

    if not wheels:
        session.error("No wheels found. Run 'build' or 'build_wheels' first.")

    # Test the most recent wheel
    latest_wheel = max(wheels, key=lambda p: p.stat().st_mtime)
    session.log(f"Testing wheel: {latest_wheel.name}")

    # Install the wheel
    session.run("pip", "install", str(latest_wheel), "--force-reinstall")

    # Test basic functionality
    session.run("python", "-c", """
import ferrocp
import tempfile
from pathlib import Path

print(f'ferrocp imported successfully')

# Test basic functionality
with tempfile.TemporaryDirectory() as temp_dir:
    temp_path = Path(temp_dir)
    source_file = temp_path / 'test.txt'
    dest_file = temp_path / 'test_copy.txt'

    source_file.write_text('Hello, World!')

    engine = ferrocp.CopyEngine()
    options = ferrocp.CopyOptions()
    engine.copy_file(str(source_file), str(dest_file), options)

    if dest_file.exists() and dest_file.read_text() == 'Hello, World!':
        print('✅ Basic copy functionality works!')
    else:
        raise RuntimeError('❌ Basic copy functionality failed!')
""")

    session.log("✅ Wheel verification completed successfully!")
