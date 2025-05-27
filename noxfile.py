# Import built-in modules
import os
import platform
import sys

# Import third-party modules
import nox


@nox.session
def lint(session):
    """Run linting checks."""
    session.install("ruff", "mypy", "isort")
    session.run("mypy", "--install-types", "--non-interactive")
    session.run("ruff", "check", ".")
    session.run("ruff", "format", "--check", ".")
    session.run("isort", "--check-only", ".")
    session.run("mypy", "src/eacopy", "--strict")


@nox.session
def lint_fix(session):
    """Fix linting issues."""
    session.install("ruff", "mypy", "isort")
    session.run("ruff", "check", "--fix", ".")
    session.run("ruff", "format", ".")
    session.run("isort", ".")


@nox.session
def pytest(session):
    """Run tests."""
    session.install("pytest", "pytest-cov")
    session.install("-e", ".")
    session.run("pytest", "tests/", "--cov=py_eacopy", "--cov-report=xml:coverage.xml", "--cov-report=term-missing")


@nox.session
def benchmark(session):
    """Run performance benchmarks."""
    session.install("pytest-benchmark", "memory-profiler", "psutil")
    session.install("-e", ".")
    session.run("pytest", "benchmarks/", "--benchmark-only", "--benchmark-sort=mean",
                "--benchmark-json=benchmarks/results/benchmark.json")


@nox.session
def benchmark_compare(session):
    """Run comparison benchmarks against standard tools."""
    session.install("pytest-benchmark", "memory-profiler", "psutil")
    session.install("-e", ".")
    session.run("pytest", "benchmarks/test_comparison.py", "--benchmark-only", "--benchmark-sort=mean")


@nox.session
def profile(session):
    """Run performance profiling."""
    session.install("py-spy", "memory-profiler")
    session.install("-e", ".")
    session.log("Use: py-spy record -o profile.svg -- python your_script.py")
    session.log("Or run: python scripts/profile.py --help")


@nox.session
def codspeed(session):
    """Run CodSpeed benchmarks locally."""
    session.install("pytest-codspeed")
    session.install("-e", ".")
    session.run("pytest", "benchmarks/test_codspeed.py", "--codspeed")


@nox.session
def codspeed_all(session):
    """Run all benchmarks with CodSpeed locally."""
    session.install("pytest-codspeed", "memory-profiler", "psutil")
    session.install("-e", ".")
    session.run("pytest", "benchmarks/", "--codspeed", "-k", "not comparison")


@nox.session
def docs(session):
    """Build documentation."""
    session.install("-e", ".[docs]")
    session.chdir("docs")
    session.run("make", "html", external=True)


@nox.session
def docs_serve(session):
    """Build and serve documentation with live reloading."""
    session.install("-e", ".[docs]")
    session.install("sphinx-autobuild")
    session.run("sphinx-autobuild", "docs", "docs/_build/html", "--open-browser")


@nox.session
def build_wheels(session):
    """Build wheels for the current platform using cibuildwheel.

    This session builds wheels for the current Python version and platform
    using cibuildwheel. Configuration is read from .cibuildwheel.toml.
    """
    # Install cibuildwheel and dependencies
    session.log("Installing cibuildwheel and dependencies...")
    session.install(
        "cibuildwheel",
        "wheel",
        "setuptools>=42.0.0",
        "setuptools_scm>=8.0.0",
        "scikit-build-core>=0.5.0",
        "pybind11>=2.10.0",
        "cmake>=3.15.0",
        "ninja",
    )

    # Determine current platform for cibuildwheel
    system = platform.system().lower()
    if system == "linux":
        current_platform = "linux"
    elif system == "darwin":
        current_platform = "macos"
    elif system == "windows":
        current_platform = "windows"
    else:
        session.error(f"Unsupported platform: {system}")

    # Set environment variables
    env = {
        "CIBW_BUILD_VERBOSITY": "3",
        "CIBW_BUILD": f"cp{sys.version_info.major}{sys.version_info.minor}-*",
    }

    # Create output directory if it doesn't exist
    os.makedirs("wheelhouse", exist_ok=True)

    # On Windows, we need special handling
    if current_platform == "windows":
        session.log("Building wheels on Windows...")
        try:
            # Try using cibuildwheel first
            session.run(
                "python",
                "-m",
                "cibuildwheel",
                "--platform",
                current_platform,
                "--output-dir",
                "wheelhouse",
                env=env,
            )
        except Exception as e:
            session.log(f"cibuildwheel failed: {e}")
            session.log("Falling back to pip wheel...")

            # Try direct pip wheel as a fallback
            session.run(
                "pip",
                "wheel",
                ".",
                "-w",
                "wheelhouse",
                "--no-deps",
                "-v",
            )
    else:
        # On other platforms, use cibuildwheel
        session.log(f"Building wheels on {current_platform}...")
        try:
            session.run(
                "python",
                "-m",
                "cibuildwheel",
                "--platform",
                current_platform,
                "--output-dir",
                "wheelhouse",
                env=env,
            )
        except Exception as e:
            session.log(f"cibuildwheel failed: {e}")
            session.log("Falling back to pip wheel...")

            # Try direct pip wheel as a fallback
            session.run(
                "pip",
                "wheel",
                ".",
                "-w",
                "wheelhouse",
                "--no-deps",
                "-v",
            )

    # List the built wheels
    session.log("Built wheels:")
    for wheel in os.listdir("wheelhouse"):
        if wheel.endswith(".whl"):
            session.log(f"  - {wheel}")


@nox.session
def verify_wheels(session):
    """Verify the built wheels."""
    session.install("wheel", "setuptools")

    # Check if wheelhouse directory exists
    if not os.path.exists("wheelhouse"):
        session.error("No wheelhouse directory found. Run build_wheels first.")

    # List and verify wheels
    wheels = [f for f in os.listdir("wheelhouse") if f.endswith(".whl")]
    if not wheels:
        session.error("No wheels found in wheelhouse directory.")

    session.log(f"Found {len(wheels)} wheels:")
    for wheel in wheels:
        session.log(f"  - {wheel}")

        # Try to install the wheel
        try:
            with session.chdir("wheelhouse"):
                session.run("pip", "install", wheel, "--force-reinstall")
            session.log(f"Successfully installed {wheel}")
        except Exception as e:
            session.error(f"Failed to install {wheel}: {e}")

    # Try to import the package
    session.run("python", "-c", "import eacopy; print(f'eacopy version: {eacopy.__version__}')")
    session.log("All wheels verified successfully!")
