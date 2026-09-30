"""End-to-end tests that drive the real CLI binary.

Each test runs ``ferrocp`` as a separate process and then verifies the result
against the real filesystem: file contents, directory structure, metadata, and
the exit status. The build artifact, the CLI and the filesystem are all the real
ones - see ``conftest.py``.
"""

# Import built-in modules
import json
import os
from pathlib import Path

# Import third-party modules
import pytest


def build_tree(root: Path) -> None:
    """Build a small tree with enough structure to prove a copy reproduced it."""
    root.mkdir(parents=True, exist_ok=True)
    (root / "top.txt").write_text("top")
    (root / "nested").mkdir()
    (root / "nested" / "middle.txt").write_text("middle")
    (root / "nested" / "deep").mkdir()
    (root / "nested" / "deep" / "leaf.bin").write_bytes(bytes(range(256)))
    (root / "empty").mkdir()


def test_copy_a_single_file(run_cli, workspace: Path) -> None:
    """Copy one file and verify its contents."""
    source = workspace / "source.txt"
    destination = workspace / "destination.txt"
    source.write_text("cli payload")

    result = run_cli("copy", str(source), str(destination))

    assert result.returncode == 0, result.output
    assert destination.read_text() == "cli payload"


def test_copy_a_directory_tree(run_cli, workspace: Path) -> None:
    """Copy a tree and verify every level was reproduced."""
    source = workspace / "source"
    destination = workspace / "destination"
    build_tree(source)

    result = run_cli("copy", str(source), str(destination))

    assert result.returncode == 0, result.output
    assert (destination / "top.txt").read_text() == "top"
    assert (destination / "nested" / "middle.txt").read_text() == "middle"
    assert (destination / "nested" / "deep" / "leaf.bin").read_bytes() == bytes(range(256))
    # An empty source directory has to appear on the other side.
    assert (destination / "empty").is_dir()


def test_copy_a_large_file(run_cli, workspace: Path) -> None:
    """A file big enough to cross the buffered engine's chunk boundary."""
    source = workspace / "large.bin"
    destination = workspace / "large-copy.bin"
    payload = bytes(range(256)) * (32 * 1024)  # 8 MiB
    source.write_bytes(payload)

    result = run_cli("copy", str(source), str(destination))

    assert result.returncode == 0, result.output
    assert destination.read_bytes() == payload


def test_copy_an_empty_file(run_cli, workspace: Path) -> None:
    """Copy a zero-byte file and verify a destination is produced."""
    source = workspace / "empty.bin"
    destination = workspace / "empty-copy.bin"
    source.write_bytes(b"")

    result = run_cli("copy", str(source), str(destination))

    assert result.returncode == 0, result.output
    assert destination.exists(), "an empty source must still produce a destination"
    assert destination.stat().st_size == 0


def test_paths_with_spaces_and_non_ascii(run_cli, workspace: Path) -> None:
    """Copy paths containing spaces, accents, CJK and Cyrillic."""
    source_dir = workspace / "源 directory"
    destination_dir = workspace / "целевая directory"
    source_dir.mkdir()
    (source_dir / "file with spaces.txt").write_text("spaces")
    (source_dir / "中文文件.txt").write_text("cjk")
    (source_dir / "café-ünïcode.txt").write_text("accents")

    result = run_cli("copy", str(source_dir), str(destination_dir))

    assert result.returncode == 0, result.output
    assert (destination_dir / "file with spaces.txt").read_text() == "spaces"
    assert (destination_dir / "中文文件.txt").read_text() == "cjk"
    assert (destination_dir / "café-ünïcode.txt").read_text() == "accents"


def test_timestamps_are_preserved_by_default(run_cli, workspace: Path) -> None:
    """The platform matrix promises preserved timestamps on every platform."""
    source = workspace / "source.txt"
    destination = workspace / "destination.txt"
    source.write_text("payload")
    os.utime(source, (1_000_000, 1_234_567))

    result = run_cli("copy", str(source), str(destination))

    assert result.returncode == 0, result.output
    assert int(destination.stat().st_mtime) == 1_234_567


# --------------------------------------------------------------------------- #
# Failure paths: exit codes and error messages
# --------------------------------------------------------------------------- #


def test_a_missing_source_exits_non_zero(run_cli, workspace: Path) -> None:
    """A failed copy must not look like a successful one to a calling script."""
    result = run_cli("copy", str(workspace / "nope.txt"), str(workspace / "out.txt"))

    assert result.returncode != 0, "a failed copy exited 0; a script cannot tell it apart from a success"
    assert not (workspace / "out.txt").exists()


def test_a_failed_copy_names_the_path(run_cli, workspace: Path) -> None:
    """ERROR_MODEL.md: an error says what was attempted and on which path."""
    missing = workspace / "missing-file.txt"

    result = run_cli("copy", str(missing), str(workspace / "out.txt"))

    assert result.returncode != 0
    assert "missing-file.txt" in result.output, result.output


def test_an_invalid_overwrite_value_is_rejected(run_cli, workspace: Path) -> None:
    """Reject an unknown --overwrite value before any I/O happens."""
    source = workspace / "source.txt"
    source.write_text("payload")

    result = run_cli("copy", str(source), str(workspace / "out.txt"), "--overwrite", "bogus")

    assert result.returncode != 0
    assert "overwrite" in result.output.lower(), result.output
    assert not (workspace / "out.txt").exists(), "validation must run before any I/O"


def test_an_invalid_symlink_mode_is_rejected(run_cli, workspace: Path) -> None:
    """Reject an unknown --symlinks value before any I/O happens."""
    source = workspace / "source.txt"
    source.write_text("payload")

    result = run_cli("copy", str(source), str(workspace / "out.txt"), "--symlinks", "sideways")

    assert result.returncode != 0
    assert "symlink" in result.output.lower(), result.output


def test_mirror_mode_is_rejected_instead_of_degrading(run_cli, workspace: Path) -> None:
    """``mirror`` deletes files, so it must never quietly behave like ``all``."""
    source = workspace / "source.txt"
    source.write_text("payload")

    result = run_cli("copy", str(source), str(workspace / "out.txt"), "--mode", "mirror")

    assert result.returncode != 0
    assert "mirror" in result.output.lower(), result.output


@pytest.mark.parametrize(
    "flag",
    ["--zero-copy", "--compression-level"],
)
def test_unimplemented_tuning_is_rejected(run_cli, workspace: Path, flag: str) -> None:
    """A knob the I/O layer cannot honour is rejected, not ignored."""
    source = workspace / "source.txt"
    source.write_text("payload")
    args = [flag] if flag == "--zero-copy" else [flag, "5"]

    result = run_cli("copy", str(source), str(workspace / "out.txt"), *args)

    assert result.returncode != 0
    assert not (workspace / "out.txt").exists()


def test_unimplemented_subcommands_exit_non_zero(run_cli, workspace: Path) -> None:
    """`sync` and `verify` must not print success for work they did not do."""
    for subcommand in ("sync", "verify"):
        result = run_cli(subcommand, str(workspace))
        assert result.returncode != 0, f"{subcommand} exited 0 without doing any work"


# --------------------------------------------------------------------------- #
# JSON output
# --------------------------------------------------------------------------- #


def test_json_output_describes_a_successful_copy(run_cli, workspace: Path) -> None:
    """Describe a successful copy in the --json document."""
    source = workspace / "source.txt"
    destination = workspace / "destination.txt"
    source.write_text("json payload")

    result = run_cli("copy", str(source), str(destination), "--json")

    assert result.returncode == 0, result.output
    payload = result.json()
    assert payload["metadata"]["operation"] == "copy"
    assert str(source) in payload["metadata"]["source_path"]
    assert payload["copy_stats"]["files_copied"] >= 1


def test_json_output_is_the_only_thing_on_stdout(run_cli, workspace: Path) -> None:
    """`--json` promises a parseable document, so nothing may precede it.

    A warning printed to stdout ahead of the document breaks that promise: a
    script reading the output gets a JSON decoder error instead of the
    statistics. Diagnostics belong on stderr.
    """
    source = workspace / "source.txt"
    destination = workspace / "destination.txt"
    source.write_text("json payload")

    result = run_cli("copy", str(source), str(destination), "--json")

    assert result.returncode == 0, result.output
    assert result.stdout.lstrip().startswith("{"), (
        f"stdout does not start with the JSON document: {result.stdout[:200]!r}"
    )


def test_json_output_reports_a_failure(run_cli, workspace: Path) -> None:
    """A failed copy still emits a document, with the reason recorded."""
    result = run_cli("copy", str(workspace / "nope.txt"), str(workspace / "out.txt"), "--json")

    assert result.returncode != 0
    payload = result.json()
    # A script consuming --json must be able to see the failure without parsing
    # the human-readable output.
    assert payload["result"]["success"] is False
    assert payload["result"]["message"], f"a failed copy emitted no message: {json.dumps(payload['result'])}"
    assert payload["copy_stats"]["files_copied"] == 0


# --------------------------------------------------------------------------- #
# Overwrite policies, end to end
# --------------------------------------------------------------------------- #


def test_never_policy_leaves_the_destination_alone(run_cli, workspace: Path) -> None:
    """Keep an existing destination under --overwrite never."""
    source = workspace / "source.txt"
    destination = workspace / "destination.txt"
    source.write_text("new content")
    destination.write_text("original content")

    result = run_cli("copy", str(source), str(destination), "--overwrite", "never")

    assert result.returncode == 0, result.output
    assert destination.read_text() == "original content"


def test_always_policy_replaces_the_destination(run_cli, workspace: Path) -> None:
    """Replace an existing destination under --overwrite always."""
    source = workspace / "source.txt"
    destination = workspace / "destination.txt"
    source.write_text("new content")
    destination.write_text("original content")

    result = run_cli("copy", str(source), str(destination), "--overwrite", "always")

    assert result.returncode == 0, result.output
    assert destination.read_text() == "new content"


def test_fail_policy_refuses_an_existing_destination(run_cli, workspace: Path) -> None:
    """Refuse to overwrite under --overwrite fail, leaving the file intact."""
    source = workspace / "source.txt"
    destination = workspace / "destination.txt"
    source.write_text("new content")
    destination.write_text("original content")

    result = run_cli("copy", str(source), str(destination), "--overwrite", "fail")

    assert result.returncode != 0, "the fail policy must refuse to overwrite"
    assert destination.read_text() == "original content"
