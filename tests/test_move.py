"""Regression tests for ferrocp.move().

`move()` used to call the async `copy_file`/`copy_directory` helpers without
awaiting them and then deleted the source, which silently lost data. It now
fails fast instead. These tests pin the contract that matters most: **the
source must survive** whatever `move()` does.
"""

# Import built-in modules
from pathlib import Path

# Import third-party modules
import pytest

# Import local modules
import ferrocp


def test_move_file_raises_not_implemented(tmp_path: Path) -> None:
    """move() on a file refuses to run instead of pretending to succeed."""
    src = tmp_path / "source.txt"
    src.write_text("payload")

    with pytest.raises(NotImplementedError):
        ferrocp.move(str(src), str(tmp_path / "dest.txt"))


def test_move_file_leaves_source_and_creates_no_destination(tmp_path: Path) -> None:
    """move() on a file must not delete the source or leave a partial copy."""
    src = tmp_path / "source.txt"
    src.write_text("payload")
    dst = tmp_path / "dest.txt"

    with pytest.raises(NotImplementedError):
        ferrocp.move(str(src), str(dst))

    # The regression this guards: the source must still be there.
    assert src.exists(), "move() deleted the source without copying it"
    assert src.read_text() == "payload"
    assert not dst.exists(), "move() produced a destination it did not complete"


def test_move_directory_leaves_source_and_creates_no_destination(tmp_path: Path) -> None:
    """move() on a directory must not delete the source tree."""
    src = tmp_path / "src_dir"
    src.mkdir()
    (src / "nested").mkdir()
    (src / "nested" / "file.txt").write_text("nested payload")
    dst = tmp_path / "dest_dir"

    with pytest.raises(NotImplementedError):
        ferrocp.move(str(src), str(dst))

    # The regression this guards: the whole source tree must still be there.
    assert src.exists(), "move() deleted the source directory without copying it"
    assert (src / "nested" / "file.txt").read_text() == "nested payload"
    assert not dst.exists(), "move() produced a destination it did not complete"


def test_move_accepts_path_objects_and_keeps_source(tmp_path: Path) -> None:
    """move() keeps working with pathlib inputs and still refuses to run."""
    src = tmp_path / "source.txt"
    src.write_text("payload")

    with pytest.raises(NotImplementedError):
        ferrocp.move(src, tmp_path / "dest.txt")

    assert src.exists()


def test_move_into_existing_directory_keeps_source(tmp_path: Path) -> None:
    """move() into an existing directory must not delete the source."""
    src = tmp_path / "source.txt"
    src.write_text("payload")
    target_dir = tmp_path / "target"
    target_dir.mkdir()

    with pytest.raises(NotImplementedError):
        ferrocp.move(str(src), str(target_dir))

    assert src.exists()
    assert not (target_dir / "source.txt").exists()


def test_move_honours_custom_copy_function_signature(tmp_path: Path) -> None:
    """A caller-supplied copy_function still cannot make move() proceed."""
    src = tmp_path / "source.txt"
    src.write_text("payload")

    async def failing_copy(source: str, destination: str) -> None:
        raise RuntimeError("copy_function must not be reached")

    with pytest.raises(NotImplementedError):
        ferrocp.move(str(src), str(tmp_path / "dest.txt"), failing_copy)

    assert src.exists()


def test_move_does_not_emit_unawaited_coroutine_warning(tmp_path: Path, recwarn) -> None:
    """move() must no longer create a coroutine that is never awaited."""
    src = tmp_path / "source.txt"
    src.write_text("payload")

    with pytest.raises(NotImplementedError):
        ferrocp.move(str(src), str(tmp_path / "dest.txt"))

    messages = [str(warning.message) for warning in recwarn]
    assert not any("never awaited" in message for message in messages), messages
