"""Unit tests for the FerroCP error model.

``docs/ERROR_MODEL.md`` is the contract: which exception type each failure maps
to, what context an error carries, and which exceptions derive from which.

Where the implementation currently does **not** match the contract, the test
says so with ``pytest.mark.xfail(strict=True)`` rather than being written to
pass. A strict xfail fails the suite once the gap is closed, so the test cannot
quietly rot into a description of the wrong behaviour - and it cannot be read
as an endorsement of it either.

The async helpers use ``asyncio.run`` directly instead of adding a pytest
async plugin: each test gets its own event loop, and the dependency list stays
as it is.
"""

# Import built-in modules
import asyncio
import os
from pathlib import Path

# Import third-party modules
import pytest

# Import local modules
import ferrocp


async def _copy_file(source: Path, destination: Path, options: object) -> object:
    """Await a copy from inside a running loop.

    The coroutine has to be created inside the loop, not outside it: pyo3-async-
    runtimes binds the future to the loop that is current when the binding is
    called, so building the awaitable first and running it afterwards fails
    with "no running event loop".
    """
    if options is None:
        return await ferrocp.copy_file(str(source), str(destination))
    return await ferrocp.copy_file(str(source), str(destination), options)


def copy(source: Path, destination: Path, options: object = None) -> object:
    """Copy a file the way a caller would and return the result."""
    return asyncio.run(_copy_file(source, destination, options))


# --------------------------------------------------------------------------- #
# Exception hierarchy
# --------------------------------------------------------------------------- #


def test_ferrocp_error_is_the_base_of_the_ferrocp_family() -> None:
    """Every FerroCP-specific failure derives from FerrocpError."""
    assert issubclass(ferrocp.FerrocpError, Exception)
    for name in ["IoError", "ConfigError", "NetworkError", "SyncError"]:
        error_type = getattr(ferrocp, name)
        assert issubclass(error_type, ferrocp.FerrocpError), name


def test_catching_ferrocp_error_catches_the_whole_family() -> None:
    """``except FerrocpError`` must catch every FerroCP-specific failure."""
    for name in ["IoError", "ConfigError", "NetworkError", "SyncError"]:
        error_type = getattr(ferrocp, name)
        try:
            raise error_type("boom")
        except ferrocp.FerrocpError:
            pass
        else:  # pragma: no cover - only reached if the hierarchy is broken
            pytest.fail(f"{name} is not catchable as FerrocpError")


# --------------------------------------------------------------------------- #
# Reporting failures
# --------------------------------------------------------------------------- #


def test_a_failed_copy_raises_instead_of_returning_an_empty_success(
    tmp_path: Path,
) -> None:
    """Principle 1: never swallow an error.

    A copy that cannot run must raise. Returning a successful-looking result
    with zero files copied would be indistinguishable from a real success.
    """
    with pytest.raises(Exception) as excinfo:
        copy(tmp_path / "does-not-exist.txt", tmp_path / "out.txt")

    assert str(excinfo.value), "the error must carry a message"


def test_a_failed_copy_creates_no_destination(tmp_path: Path) -> None:
    """A failure must not leave a destination that looks like a success."""
    destination = tmp_path / "out.txt"

    with pytest.raises(Exception):
        copy(tmp_path / "does-not-exist.txt", destination)

    assert not destination.exists()


def test_the_error_message_names_the_path_that_failed(tmp_path: Path) -> None:
    """Contract: an error says what was being attempted and on which path."""
    missing = tmp_path / "missing-file.txt"

    with pytest.raises(Exception) as excinfo:
        copy(missing, tmp_path / "out.txt")

    assert "missing-file.txt" in str(excinfo.value), f"the error does not name the path: {excinfo.value}"


# --------------------------------------------------------------------------- #
# Classification: documented contract vs. current behaviour
# --------------------------------------------------------------------------- #

# ERROR_MODEL.md §4 and COPY_SEMANTICS.md §7 both promise that a missing source
# surfaces as the builtin ``FileNotFoundError``. It does not yet: the executor
# turns a failed copy into an unstructured ``CopyResult::failure`` message, so
# the ``io::ErrorKind`` is lost before the PyO3 boundary can map it.
#
# This is the deferred finding recorded against the error-model work as
# "f8_kind_gap": several ``Error::Io`` sites pass ``kind: None`` even though the
# originating ``io::Error`` is in scope. These xfails are the executable record
# of the gap; closing it makes them fail until they are flipped to real tests.
_MISSING_KIND_REASON = (
    "ERROR_MODEL.md promises FileNotFoundError for a missing source; the io "
    "error kind is lost at CopyResult::failure (finding f8_kind_gap)"
)


@pytest.mark.xfail(strict=True, reason=_MISSING_KIND_REASON)
def test_missing_source_raises_file_not_found(tmp_path: Path) -> None:
    """A missing source should surface as the builtin FileNotFoundError."""
    with pytest.raises(FileNotFoundError):
        copy(tmp_path / "does-not-exist.txt", tmp_path / "out.txt")


@pytest.mark.xfail(strict=True, reason=_MISSING_KIND_REASON)
def test_missing_source_is_catchable_as_os_error(tmp_path: Path) -> None:
    """``except OSError`` is the idiom the contract promises will work."""
    with pytest.raises(OSError):
        copy(tmp_path / "does-not-exist.txt", tmp_path / "out.txt")


@pytest.mark.xfail(strict=True, reason=_MISSING_KIND_REASON)
def test_unreadable_source_raises_permission_error(tmp_path: Path) -> None:
    """Only meaningful where the permission can actually be withdrawn."""
    if os.name != "posix" or os.geteuid() == 0:
        pytest.skip(
            "Windows attributes and root cannot express an unreadable file, so "
            "the permission-denied classification cannot be exercised here"
        )

    source = tmp_path / "secret.txt"
    source.write_text("classified")
    os.chmod(source, 0o000)
    try:
        with pytest.raises(PermissionError):
            copy(source, tmp_path / "copy.txt")
    finally:
        os.chmod(source, 0o644)


# --------------------------------------------------------------------------- #
# Configuration errors
# --------------------------------------------------------------------------- #


def test_an_option_that_cannot_be_honoured_is_rejected(tmp_path: Path) -> None:
    """Rule 2 of the contract: rejected, never ignored."""
    source = tmp_path / "source.txt"
    source.write_text("payload")

    with pytest.raises(ValueError, match="mirror"):
        copy(source, tmp_path / "out.txt", ferrocp.CopyOptions(mode="mirror"))


def test_a_rejected_option_writes_nothing(tmp_path: Path) -> None:
    """Validation happens before any I/O, so nothing is left behind."""
    source = tmp_path / "source.txt"
    destination = tmp_path / "out.txt"
    source.write_text("payload")

    with pytest.raises(ValueError):
        copy(source, destination, ferrocp.CopyOptions(mode="mirror"))

    assert not destination.exists()
