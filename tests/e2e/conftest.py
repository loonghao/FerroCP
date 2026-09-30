"""Shared fixtures for the FerroCP end-to-end suite.

The suite drives the two artifacts a user actually installs:

* the **CLI binary** built from this working tree, and
* the **Python extension module** installed into the current interpreter.

Both hit the real filesystem. Nothing is mocked, and there is no fake
filesystem anywhere in this directory: the point of an end-to-end test is that
it fails when real I/O breaks, which a mock filesystem cannot tell you.

Locating the binary
-------------------

``FERROCP_E2E_CLI`` wins if it is set, so CI can point at an artifact it built
itself. Otherwise the session builds ``--bin ferrocp`` once with Cargo. If
neither works the tests fail with an explanation rather than being skipped:
a suite that quietly passes because its subject was missing is worse than a
loud failure.
"""

# Import built-in modules
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Callable, Optional

# Import third-party modules
import pytest

# The repository root: this file lives at <root>/tests/e2e/conftest.py.
PROJECT_ROOT = Path(__file__).resolve().parents[2]


def _binary_name() -> str:
    return "ferrocp.exe" if os.name == "nt" else "ferrocp"


def _discover_built_binary() -> Optional[Path]:
    """Find an already built CLI binary in the Cargo target directory."""
    for profile in ("debug", "release"):
        candidate = PROJECT_ROOT / "target" / profile / _binary_name()
        if candidate.is_file():
            return candidate
    return None


def _build_binary() -> Path:
    """Build the CLI binary once for the whole session."""
    subprocess.run(
        ["cargo", "build", "--bin", "ferrocp"],
        cwd=PROJECT_ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    binary = _discover_built_binary()
    if binary is None:
        raise RuntimeError(
            f"cargo build --bin ferrocp succeeded but no binary was found under {PROJECT_ROOT / 'target'}"
        )
    return binary


@pytest.fixture(scope="session")
def cli_binary() -> Path:
    """Path to the CLI binary under test."""
    override = os.environ.get("FERROCP_E2E_CLI")
    if override:
        binary = Path(override)
        if not binary.is_file():
            pytest.fail(f"FERROCP_E2E_CLI points at {binary}, which is not a file")
        return binary

    if shutil.which("cargo") is None:
        pytest.fail(
            "no CLI binary is available: set FERROCP_E2E_CLI to a built "
            "`ferrocp` executable, or install Cargo so the suite can build one"
        )

    return _build_binary()


class CliResult:
    """The outcome of one CLI invocation."""

    def __init__(self, returncode: int, stdout: str, stderr: str) -> None:
        self.returncode = returncode
        self.stdout = stdout
        self.stderr = stderr

    @property
    def output(self) -> str:
        """Everything the process printed, in the order it was produced."""
        return self.stdout + self.stderr

    def json(self) -> dict:
        """Parse stdout as the JSON document ``--json`` produces.

        Fails with the raw output attached, because a truncated or prefixed
        document is otherwise very hard to diagnose from a CI log.
        """
        try:
            return json.loads(self.stdout)  # type: ignore[no-any-return]
        except json.JSONDecodeError as error:
            raise AssertionError(
                f"--json did not emit parseable JSON: {error}\n--- stdout ---\n{self.stdout}"
            ) from error


@pytest.fixture
def run_cli(cli_binary: Path):
    """Run the CLI binary against the real filesystem.

    Each call is a separate process, exactly like a user typing the command.
    A generous timeout is enforced so a copy that never completes fails this
    test instead of hanging the job until CI kills it.
    """

    def _run(*args: str, timeout: int = 120) -> CliResult:
        proc = subprocess.run(
            [str(cli_binary), *args],
            capture_output=True,
            text=True,
            timeout=timeout,
        )
        return CliResult(proc.returncode, proc.stdout, proc.stderr)

    return _run


@pytest.fixture
def workspace(tmp_path: Path) -> Path:
    """Create an empty scratch directory for one test.

    Deliberately empty: tests create the `source` and `destination` names they
    need, so a fixture that pre-created them would collide with the ones that
    build a directory called `source`.
    """
    return tmp_path


# --------------------------------------------------------------------------- #
# Platform capability probes
# --------------------------------------------------------------------------- #


def _symlink_works(directory: Path) -> bool:
    """Whether this process can create a symbolic link here."""
    target = directory / "probe-target.txt"
    link = directory / "probe-link.txt"
    target.write_text("probe")
    try:
        link.symlink_to(target)
        return link.is_symlink()
    except (OSError, NotImplementedError, ValueError):
        return False
    finally:
        for path in (link, target):
            try:
                path.unlink()
            except OSError:
                pass


@pytest.fixture(scope="session")
def symlink_supported(tmp_path_factory: pytest.TempPathFactory) -> bool:
    """Whether symlinks can be created in this environment.

    Windows needs Developer Mode or an elevated process. The suite does not
    skip symlink tests when this is unavailable: `docs/COPY_SEMANTICS.md` §5
    specifies what must happen instead (the failure is reported and counted in
    `errors`), and that behaviour is asserted by
    ``test_symlinks.py::test_symlink_creation_failure_is_reported_not_dropped``.
    """
    directory = tmp_path_factory.mktemp("symlink-probe")
    return _symlink_works(directory)


@pytest.fixture
def python_api():
    """Return the installed ``ferrocp`` extension module.

    Imported here rather than at module scope so a missing build produces a
    clear failure for the tests that need it.
    """
    try:
        import ferrocp
    except ImportError as error:  # pragma: no cover - environment problem
        pytest.fail(
            f"the ferrocp extension module is not importable: {error}. "
            "Build it with `maturin develop` before running the e2e suite."
        )
    return ferrocp


@pytest.fixture
def run_async():
    """Drive an awaitable to completion, creating it inside the event loop.

    pyo3-async-runtimes binds the future to the loop that is current when the
    binding is called, so building the awaitable outside the loop fails with
    "no running event loop".
    """
    import asyncio

    def _run(awaitable_factory):
        async def _wrapper():
            return await awaitable_factory()

        return asyncio.run(_wrapper())

    return _run


def skip_if_no_symlink(symlink_supported: bool) -> None:
    """Record why a symlink test cannot assert the happy path here."""
    if not symlink_supported:
        pytest.skip(
            "this environment cannot create symbolic links (on Windows this "
            "needs Developer Mode or an elevated process); "
            "test_symlink_creation_failure_is_reported_not_dropped covers the "
            "documented behaviour for this case instead"
        )
