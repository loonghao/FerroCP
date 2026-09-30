"""Unit tests for the tuning options that are validated when a copy runs.

``buffer_size``, ``num_threads`` and ``compression_level`` are plain attributes
on ``CopyOptions``, so ``test_copy_options.py`` cannot reject them: the object
accepts any value and the copy refuses the ones it cannot honour. That is the
behaviour documented in ``docs/COPY_SEMANTICS.md`` §8, and these tests pin it.

Each test therefore has to submit a real copy. They are kept small - one file of
a few bytes - because what is under test is the rejection, not the transfer.
"""

# Import built-in modules
import asyncio
from pathlib import Path

# Import third-party modules
import pytest

# Import local modules
import ferrocp

# ``buffer_size`` must be a power of two between 4 KiB and 64 MiB.
MIN_BUFFER = 4 * 1024
MAX_BUFFER = 64 * 1024 * 1024


async def _copy_file(source: Path, destination: Path, options: object) -> object:
    """Await a copy from inside a running loop.

    pyo3-async-runtimes binds the future to the loop that is current when the
    binding is called, so the awaitable must be created inside the loop.
    """
    return await ferrocp.copy_file(str(source), str(destination), options)


def copy(source: Path, destination: Path, options: object = None) -> object:
    """Copy a file and return the result."""
    return asyncio.run(_copy_file(source, destination, options))


@pytest.fixture
def source(tmp_path: Path) -> Path:
    """Create a small file worth copying."""
    path = tmp_path / "source.txt"
    path.write_text("tuning payload")
    return path


def test_a_valid_buffer_size_is_honoured(source: Path, tmp_path: Path) -> None:
    """The one tuning knob the contract says is applied, is applied."""
    destination = tmp_path / "out.txt"

    result = copy(source, destination, ferrocp.CopyOptions(buffer_size=MIN_BUFFER))

    assert destination.read_text() == "tuning payload"
    assert result.success


def test_every_power_of_two_in_range_is_honoured(source: Path, tmp_path: Path) -> None:
    """Sweep the accepted range so no size silently falls off the list."""
    size = MIN_BUFFER
    while size <= MAX_BUFFER:
        destination = tmp_path / f"out-{size}.txt"
        result = copy(source, destination, ferrocp.CopyOptions(buffer_size=size))
        assert result.success, f"buffer_size={size} was rejected"
        assert destination.read_text() == "tuning payload"
        size *= 2


@pytest.mark.parametrize("size", [0, 1, 4095, 100_000, MAX_BUFFER * 2])
def test_an_unusable_buffer_size_is_rejected(source: Path, tmp_path: Path, size: int) -> None:
    """An unusable size raises instead of being clamped or rounded."""
    destination = tmp_path / "out.txt"

    with pytest.raises(ValueError, match="buffer_size"):
        copy(source, destination, ferrocp.CopyOptions(buffer_size=size))

    assert not destination.exists(), "a rejected option must not touch the disk"


def test_a_non_power_of_two_buffer_size_is_rejected(source: Path, tmp_path: Path) -> None:
    """Off-by-one sizes are the classic silent-rounding bug."""
    with pytest.raises(ValueError, match="buffer_size"):
        copy(source, tmp_path / "out.txt", ferrocp.CopyOptions(buffer_size=(MIN_BUFFER * 2) + 1))


@pytest.mark.parametrize("threads", [1, 2, 8, 64])
def test_num_threads_is_rejected_unless_zero(source: Path, tmp_path: Path, threads: int) -> None:
    """The engine sizes its own pool, so a count would be ignored not applied."""
    destination = tmp_path / "out.txt"

    with pytest.raises(ValueError, match="num_threads"):
        copy(source, destination, ferrocp.CopyOptions(num_threads=threads))

    assert not destination.exists()


def test_num_threads_zero_is_accepted(source: Path, tmp_path: Path) -> None:
    """Zero means auto-detect, which is what actually happens."""
    destination = tmp_path / "out.txt"

    result = copy(source, destination, ferrocp.CopyOptions(num_threads=0))

    assert result.success
    assert destination.read_text() == "tuning payload"


@pytest.mark.parametrize("level", [1, 3, 9, 22])
def test_compression_level_is_rejected_unless_zero(source: Path, tmp_path: Path, level: int) -> None:
    """The I/O layer has no compressor, so no level can be applied."""
    destination = tmp_path / "out.txt"

    with pytest.raises(ValueError, match="compression_level"):
        copy(source, destination, ferrocp.CopyOptions(compression_level=level))

    assert not destination.exists()


def test_compression_level_above_the_valid_range_is_rejected(source: Path, tmp_path: Path) -> None:
    """Out-of-range levels are rejected too, not silently truncated."""
    with pytest.raises(ValueError):
        copy(source, tmp_path / "out.txt", ferrocp.CopyOptions(compression_level=99))


def test_rejection_wins_over_a_copy_that_would_succeed(source: Path, tmp_path: Path) -> None:
    """Tuning is checked before the copy, not after it has already happened."""
    destination = tmp_path / "out.txt"

    with pytest.raises(ValueError):
        copy(source, destination, ferrocp.CopyOptions(num_threads=8))

    assert not destination.exists(), "validation must run before any I/O"
