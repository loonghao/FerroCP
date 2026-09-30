"""End-to-end tests for the platform matrix in `docs/COPY_SEMANTICS.md` §5.

The matrix is a promise about what each platform preserves. Every row is
asserted here, on whatever platform the suite is running, so a platform that
drifts from its documented behaviour fails the suite rather than quietly
differing.

The matrix holds for every engine, so these tests do not try to select one:
they copy through the CLI and the Python API and check the result either way.
"""

# Import built-in modules
import asyncio
import os
import platform
import stat
import sys
from pathlib import Path

# Import third-party modules
import pytest

IS_WINDOWS = os.name == "nt"
IS_UNIX = os.name == "posix"


def copy_with_cli(run_cli, source: Path, destination: Path) -> None:
    result = run_cli("copy", str(source), str(destination))
    assert result.returncode == 0, result.output


def copy_with_python(source: Path, destination: Path, options: object = None) -> None:
    import ferrocp

    async def _copy():
        if options is None:
            return await ferrocp.copy_file(str(source), str(destination))
        return await ferrocp.copy_file(str(source), str(destination), options)

    asyncio.run(_copy())


# --------------------------------------------------------------------------- #
# Timestamps - preserved on every platform
# --------------------------------------------------------------------------- #


@pytest.mark.parametrize("driver", ["cli", "python"])
def test_timestamps_are_preserved(run_cli, workspace: Path, driver: str) -> None:
    """Matrix row: mtime is preserved on Linux, macOS and Windows alike."""
    source = workspace / "source.txt"
    destination = workspace / "destination.txt"
    source.write_text("payload")
    os.utime(source, (1_000_000, 1_234_567))

    if driver == "cli":
        copy_with_cli(run_cli, source, destination)
    else:
        copy_with_python(source, destination)

    assert int(destination.stat().st_mtime) == 1_234_567


# --------------------------------------------------------------------------- #
# Permissions - the one row that genuinely differs
# --------------------------------------------------------------------------- #


@pytest.mark.skipif(not IS_UNIX, reason="Unix-only row of the platform matrix")
def test_unix_preserves_the_full_mode(run_cli, workspace: Path) -> None:
    """Matrix row: Linux / macOS preserve the full mode."""
    if os.geteuid() == 0:
        pytest.skip("root ignores mode bits, so this row cannot be falsified")

    source = workspace / "source.txt"
    destination = workspace / "destination.txt"
    source.write_text("payload")
    os.chmod(source, 0o750)

    copy_with_cli(run_cli, source, destination)

    assert stat.S_IMODE(destination.stat().st_mode) == 0o750


@pytest.mark.skipif(not IS_WINDOWS, reason="Windows-only row of the platform matrix")
def test_windows_preserves_the_read_only_attribute(run_cli, workspace: Path) -> None:
    """Matrix row: Windows preserves only the read-only attribute.

    The full POSIX mode does not exist there, and the matrix says so rather
    than leaving the difference for callers to discover.
    """
    source = workspace / "source.txt"
    destination = workspace / "destination.txt"
    source.write_text("payload")
    os.chmod(source, stat.S_IWRITE)  # clear read-only

    copy_with_cli(run_cli, source, destination)

    # Readable and writable, i.e. the attribute was carried across, and nothing
    # claims a POSIX mode that Windows does not have.
    assert os.access(destination, os.W_OK)


@pytest.mark.skipif(not IS_WINDOWS, reason="Windows-only row of the platform matrix")
@pytest.mark.xfail(
    strict=True,
    reason=(
        "docs/COPY_SEMANTICS.md"
        " §5 promises a read-only destination is cleared to writable when the "
        "source is writable. It is not: the writer opens the destination and "
        "fails with Access denied (os error 5) before the attribute is "
        "touched, so the documented recovery never runs."
    ),
)
def test_windows_clears_a_stale_read_only_destination(run_cli, workspace: Path) -> None:
    """The read-only attribute must be carried from the source, both ways.

    `docs/COPY_SEMANTICS.md` §5: "A read-only destination is cleared to writable
    when the source is writable." So a writable source over a read-only
    destination must end up writable.
    """
    source = workspace / "source.txt"
    destination = workspace / "destination.txt"
    source.write_text("new content")
    destination.write_text("old content")
    os.chmod(destination, stat.S_IREAD)  # read-only

    result = run_cli("copy", str(source), str(destination))

    assert result.returncode == 0, result.output
    assert destination.read_text() == "new content"
    assert os.access(destination, os.W_OK), "the read-only attribute was not cleared"


@pytest.mark.skipif(not IS_WINDOWS, reason="Windows-only row of the platform matrix")
def test_windows_marks_a_read_only_destination_read_only(
    run_cli, workspace: Path
) -> None:
    """The other half of the same row: a read-only source stays read-only."""
    source = workspace / "source.txt"
    destination = workspace / "destination.txt"
    source.write_text("payload")
    os.chmod(source, stat.S_IREAD)

    result = run_cli("copy", str(source), str(destination))

    assert result.returncode == 0, result.output
    assert not os.access(destination, os.W_OK), (
        "the source's read-only attribute was not carried across"
    )


# --------------------------------------------------------------------------- #
# ACLs and ownership - not preserved anywhere
# --------------------------------------------------------------------------- #


@pytest.mark.skipif(not IS_WINDOWS, reason="Windows-only row of the platform matrix")
def test_windows_reports_a_read_only_destination_instead_of_failing_obscurely(
    run_cli, workspace: Path
) -> None:
    """Whatever happens, a read-only destination must not be a silent no-op.

    Either the documented clearing happens (and the test above asserts it) or
    the copy fails - but it must not report success while leaving the old
    content in place. This is the assertion that survives both worlds.
    """
    source = workspace / "source.txt"
    destination = workspace / "destination.txt"
    source.write_text("new content")
    destination.write_text("old content")
    os.chmod(destination, stat.S_IREAD)

    result = run_cli("copy", str(source), str(destination))

    failed = result.returncode != 0
    copied = destination.read_text() == "new content"
    assert failed or copied, (
        "the copy reported success but left the old destination in place"
    )


def test_ownership_and_acls_are_not_preserved_but_the_copy_still_succeeds(
    run_cli, workspace: Path
) -> None:
    """Matrix row: ACLs and ownership are not copied on any platform.

    The assertion is that the copy still works and the content still arrives -
    the matrix documents the limitation, it does not promise a failure.
    """
    source = workspace / "source.txt"
    destination = workspace / "destination.txt"
    source.write_text("payload")

    copy_with_cli(run_cli, source, destination)

    assert destination.read_text() == "payload"


# --------------------------------------------------------------------------- #
# Overwrite policies - identical on every platform
# --------------------------------------------------------------------------- #


@pytest.mark.parametrize("driver", ["cli", "python"])
def test_overwrite_never_is_identical_everywhere(
    run_cli, workspace: Path, driver: str
) -> None:
    """Matrix row: overwrite policies behave identically on all platforms."""
    import ferrocp

    source = workspace / "source.txt"
    destination = workspace / "destination.txt"
    source.write_text("new content")
    destination.write_text("original content")

    if driver == "cli":
        result = run_cli("copy", str(source), str(destination), "--overwrite", "never")
        assert result.returncode == 0, result.output
    else:
        copy_with_python(source, destination, ferrocp.CopyOptions(overwrite="never"))

    assert destination.read_text() == "original content"


# --------------------------------------------------------------------------- #
# Special files - skipped and counted, Unix only
# --------------------------------------------------------------------------- #


@pytest.mark.skipif(not IS_UNIX, reason="FIFOs and sockets are Unix concepts")
def test_special_files_are_skipped_and_counted(run_cli, workspace: Path) -> None:
    """Matrix row: sockets and FIFOs are skipped and counted in files_skipped."""
    source = workspace / "source"
    source.mkdir()
    (source / "regular.txt").write_text("payload")
    os.mkfifo(source / "pipe")

    destination = workspace / "destination"
    result = run_cli("copy", str(source), str(destination))

    assert result.returncode == 0, result.output
    assert (destination / "regular.txt").read_text() == "payload"
    assert not (destination / "pipe").exists()


# --------------------------------------------------------------------------- #
# Make the platform visible in the report
# --------------------------------------------------------------------------- #


def test_platform_is_reported(record_property) -> None:
    """Record which platform the matrix rows were asserted against."""
    record_property("platform", platform.platform())
    record_property("python", sys.version.split()[0])
    assert sys.platform
