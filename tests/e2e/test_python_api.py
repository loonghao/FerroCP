"""End-to-end tests that drive the installed Python extension module.

These go through the same public API a user calls - ``ferrocp.copy_file``,
``ferrocp.copy_directory`` and the ``CopyOptions`` that shape them - against the
real filesystem. The results are verified the way a user would check them: read
the bytes back, walk the tree, stat the files.

What distinguishes this from ``tests/test_copy_options.py`` and
``tests/test_tuning.py`` is that those assert *which values are accepted*; these
assert *what a copy actually did* to the filesystem.
"""

# Import built-in modules
import asyncio
import os
from pathlib import Path

# Import third-party modules
import pytest

# Import local modules
from .conftest import skip_if_no_symlink


def copy_file(source: Path, destination: Path, options: object = None):
    """Copy one file through the public API."""
    import ferrocp

    async def _copy():
        if options is None:
            return await ferrocp.copy_file(str(source), str(destination))
        return await ferrocp.copy_file(str(source), str(destination), options)

    return asyncio.run(_copy())


def copy_directory(source: Path, destination: Path, options: object = None):
    """Copy a directory tree through the public API."""
    import ferrocp

    async def _copy():
        if options is None:
            return await ferrocp.copy_directory(str(source), str(destination))
        return await ferrocp.copy_directory(str(source), str(destination), options)

    return asyncio.run(_copy())


def build_tree(root: Path) -> None:
    """A small tree with enough structure to prove a copy reproduced it."""
    root.mkdir(parents=True, exist_ok=True)
    (root / "top.txt").write_text("top")
    (root / "nested").mkdir()
    (root / "nested" / "middle.txt").write_text("middle")
    (root / "nested" / "deep").mkdir()
    (root / "nested" / "deep" / "leaf.bin").write_bytes(bytes(range(256)))
    (root / "empty").mkdir()


# --------------------------------------------------------------------------- #
# Happy paths
# --------------------------------------------------------------------------- #


def test_copy_a_single_file(workspace: Path) -> None:
    source = workspace / "source.txt"
    destination = workspace / "destination.txt"
    source.write_text("python payload")

    result = copy_file(source, destination)

    assert result.success
    assert destination.read_text() == "python payload"
    assert result.bytes_copied == len("python payload")
    assert result.files_copied == 1


def test_copy_a_directory_tree(workspace: Path) -> None:
    source = workspace / "source"
    destination = workspace / "destination"
    build_tree(source)

    result = copy_directory(source, destination)

    assert result.success, result.error_message
    assert (destination / "top.txt").read_text() == "top"
    assert (destination / "nested" / "middle.txt").read_text() == "middle"
    assert (destination / "nested" / "deep" / "leaf.bin").read_bytes() == bytes(range(256))
    assert (destination / "empty").is_dir(), "empty directories must be reproduced"


def test_copy_a_large_file(workspace: Path) -> None:
    source = workspace / "large.bin"
    destination = workspace / "large-copy.bin"
    payload = bytes(range(256)) * (32 * 1024)  # 8 MiB
    source.write_bytes(payload)

    result = copy_file(source, destination)

    assert result.success
    assert destination.read_bytes() == payload
    assert result.bytes_copied == len(payload)


def test_copy_an_empty_file(workspace: Path) -> None:
    source = workspace / "empty.bin"
    destination = workspace / "empty-copy.bin"
    source.write_bytes(b"")

    result = copy_file(source, destination)

    assert result.success
    assert destination.exists(), "an empty source must still produce a destination"
    assert destination.stat().st_size == 0


def test_a_deeply_nested_tree_is_reproduced(workspace: Path) -> None:
    source = workspace / "source"
    destination = workspace / "destination"
    # Two-character components keep the absolute path inside the 260 character
    # MAX_PATH limit that still applies to Win32 paths on a CI runner.
    deepest = source
    for level in range(32):
        deepest = deepest / f"{level:02}"
    deepest.mkdir(parents=True)
    (deepest / "leaf.txt").write_text("leaf")

    result = copy_directory(source, destination)

    assert result.success, result.error_message
    copied_leaf = destination / deepest.relative_to(source) / "leaf.txt"
    assert copied_leaf.read_text() == "leaf", f"missing {copied_leaf}"


def test_paths_with_spaces_and_non_ascii(workspace: Path) -> None:
    import ferrocp

    source_dir = workspace / "源 directory"
    destination_dir = workspace / "целевая directory"
    source_dir.mkdir()
    (source_dir / "file with spaces.txt").write_text("spaces")
    (source_dir / "中文文件.txt").write_text("cjk")

    result = copy_directory(source_dir, destination_dir)

    assert result.success, result.error_message
    assert (destination_dir / "file with spaces.txt").read_text() == "spaces"
    assert (destination_dir / "中文文件.txt").read_text() == "cjk"
    assert isinstance(result, ferrocp.CopyResult)


def test_timestamps_are_preserved_by_default(workspace: Path) -> None:
    source = workspace / "source.txt"
    destination = workspace / "destination.txt"
    source.write_text("payload")
    os.utime(source, (1_000_000, 1_234_567))

    result = copy_file(source, destination)

    assert result.success
    assert int(destination.stat().st_mtime) == 1_234_567


def test_metadata_is_not_preserved_when_both_flags_are_off(workspace: Path) -> None:
    """Turning both metadata flags off must reach the I/O layer."""
    import ferrocp

    source = workspace / "source.txt"
    destination = workspace / "destination.txt"
    source.write_text("payload")
    os.utime(source, (1_000_000, 1_234_567))
    options = ferrocp.CopyOptions(preserve_timestamps=False, preserve_permissions=False)

    result = copy_file(source, destination, options)

    assert result.success
    assert int(destination.stat().st_mtime) != 1_234_567


@pytest.mark.xfail(
    strict=True,
    reason=(
        "preserve_timestamps=False is ignored when preserve_permissions=True: "
        "apply_copy_options collapses both into the single "
        "CopyRequest.preserve_metadata field, so either flag being true "
        "preserves both. The underlying helper, "
        "ferrocp_io::metadata::preserve_metadata, keeps them apart - the gap is "
        "CopyRequest having one field for two independent flags."
    ),
)
def test_timestamps_can_be_disabled_independently_of_permissions(
    workspace: Path,
) -> None:
    """Each metadata flag has to be independent of the other.

    Turning timestamps off while still asking for permissions currently keeps
    the timestamps anyway. The Rust helper both flags reach already treats them
    independently - it is tested that way in ``ferrocp-io`` - so this is a gap
    in the Python binding's plumbing, not in the I/O layer.
    """
    import ferrocp

    source = workspace / "source.txt"
    destination = workspace / "destination.txt"
    source.write_text("payload")
    os.utime(source, (1_000_000, 1_234_567))
    options = ferrocp.CopyOptions(preserve_timestamps=False, preserve_permissions=True)

    result = copy_file(source, destination, options)

    assert result.success
    assert int(destination.stat().st_mtime) != 1_234_567, (
        "timestamps were preserved despite preserve_timestamps=False"
    )


# --------------------------------------------------------------------------- #
# Overwrite policies, end to end
# --------------------------------------------------------------------------- #


def test_never_policy_leaves_the_destination_alone(workspace: Path) -> None:
    import ferrocp

    source = workspace / "source.txt"
    destination = workspace / "destination.txt"
    source.write_text("new content")
    destination.write_text("original content")

    result = copy_file(source, destination, ferrocp.CopyOptions(overwrite="never"))

    assert result.success
    assert destination.read_text() == "original content"
    assert result.files_copied == 0


def test_always_policy_replaces_the_destination(workspace: Path) -> None:
    import ferrocp

    source = workspace / "source.txt"
    destination = workspace / "destination.txt"
    source.write_text("new content")
    destination.write_text("original content")

    result = copy_file(source, destination, ferrocp.CopyOptions(overwrite="always"))

    assert result.success
    assert destination.read_text() == "new content"


def test_fail_policy_refuses_an_existing_destination(workspace: Path) -> None:
    import ferrocp

    source = workspace / "source.txt"
    destination = workspace / "destination.txt"
    source.write_text("new content")
    destination.write_text("original content")

    with pytest.raises(Exception):
        copy_file(source, destination, ferrocp.CopyOptions(overwrite="fail"))

    assert destination.read_text() == "original content", (
        "the fail policy must not modify the destination"
    )


def test_if_newer_skips_when_the_destination_is_newer(workspace: Path) -> None:
    import ferrocp

    source = workspace / "source.txt"
    destination = workspace / "destination.txt"
    source.write_text("old source")
    destination.write_text("newer destination")
    os.utime(source, (1_000_000, 1_000_000))
    os.utime(destination, (1_000_000, 2_000_000))

    result = copy_file(source, destination, ferrocp.CopyOptions(overwrite="if_newer"))

    assert result.success
    assert destination.read_text() == "newer destination"


def test_if_newer_copies_when_the_source_is_newer(workspace: Path) -> None:
    import ferrocp

    source = workspace / "source.txt"
    destination = workspace / "destination.txt"
    source.write_text("newer source")
    destination.write_text("old destination")
    os.utime(source, (1_000_000, 2_000_000))
    os.utime(destination, (1_000_000, 1_000_000))

    result = copy_file(source, destination, ferrocp.CopyOptions(overwrite="if_newer"))

    assert result.success
    assert destination.read_text() == "newer source"


# --------------------------------------------------------------------------- #
# Failure paths
# --------------------------------------------------------------------------- #


def test_a_missing_source_raises_and_creates_nothing(workspace: Path) -> None:
    destination = workspace / "out.txt"

    with pytest.raises(Exception):
        copy_file(workspace / "nope.txt", destination)

    assert not destination.exists()


def test_a_rejected_option_writes_nothing(workspace: Path) -> None:
    import ferrocp

    source = workspace / "source.txt"
    destination = workspace / "out.txt"
    source.write_text("payload")

    with pytest.raises(ValueError):
        copy_file(source, destination, ferrocp.CopyOptions(num_threads=8))

    assert not destination.exists(), "validation must run before any I/O"


# --------------------------------------------------------------------------- #
# Symbolic links
# --------------------------------------------------------------------------- #


def test_symlinks_are_recreated_by_default(workspace: Path, symlink_supported: bool) -> None:
    """`preserve` is the default: the link itself is recreated."""
    skip_if_no_symlink(symlink_supported)

    source = workspace / "source"
    destination = workspace / "destination"
    source.mkdir()
    target = source / "target.txt"
    target.write_text("target payload")
    (source / "link.txt").symlink_to(target)

    result = copy_directory(source, destination)

    assert result.success, result.error_message
    link = destination / "link.txt"
    assert link.is_symlink(), "the link was dereferenced instead of recreated"
    assert link.read_text() == "target payload"


def test_follow_symlinks_copies_the_content(
    workspace: Path, symlink_supported: bool
) -> None:
    skip_if_no_symlink(symlink_supported)

    import ferrocp

    source = workspace / "source"
    destination = workspace / "destination"
    source.mkdir()
    target = source / "target.txt"
    target.write_text("target payload")
    (source / "link.txt").symlink_to(target)

    options = ferrocp.CopyOptions(follow_symlinks=True)
    result = copy_directory(source, destination)

    assert result.success
    # Sanity-check the default before asserting the other mode.
    assert (destination / "link.txt").is_symlink()

    destination2 = workspace / "destination-follow"
    result = copy_directory(source, destination2, options)
    assert result.success, result.error_message
    assert not (destination2 / "link.txt").is_symlink(), (
        "follow mode must write a regular file"
    )
    assert (destination2 / "link.txt").read_text() == "target payload"
