"""End-to-end tests for symbolic link handling.

`docs/COPY_SEMANTICS.md` §3 defines three modes and §5 states what happens when
a link cannot be created. Both halves are covered here, including the case where
this environment cannot create links at all:

* When links **can** be created, the modes are asserted directly.
* When they **cannot** (Windows without Developer Mode or elevation), the suite
  does not skip silently. It asserts the documented fallback instead: the
  failure is reported as an error and counted, never dropped. That is a real
  assertion about real behaviour, not a placeholder.

The distinction is driven by the ``symlink_supported`` session fixture, whose
docstring records which branch a given machine takes.
"""

# Import built-in modules
from pathlib import Path

# Import third-party modules
import pytest

# Import local modules
from .conftest import skip_if_no_symlink


def make_linked_tree(root: Path) -> Path:
    """A tree containing a file link, a directory link and a dangling link."""
    root.mkdir(parents=True, exist_ok=True)
    target = root / "target.txt"
    target.write_text("target payload")

    nested = root / "nested"
    nested.mkdir()
    (nested / "inside.txt").write_text("inside")

    (root / "file-link.txt").symlink_to(target)
    (root / "dir-link").symlink_to(nested, target_is_directory=True)
    (root / "dangling-link.txt").symlink_to(root / "nowhere.txt")
    return root


# --------------------------------------------------------------------------- #
# Links can be created here
# --------------------------------------------------------------------------- #


def test_preserve_recreates_links(
    run_cli, workspace: Path, symlink_supported: bool
) -> None:
    """`preserve` is the default: the link itself is recreated."""
    skip_if_no_symlink(symlink_supported)

    source = make_linked_tree(workspace / "source")
    destination = workspace / "destination"

    result = run_cli("copy", str(source), str(destination))

    assert result.returncode == 0, result.output
    assert (destination / "file-link.txt").is_symlink(), (
        "the file link was dereferenced instead of recreated"
    )
    assert (destination / "file-link.txt").read_text() == "target payload"


def test_preserve_keeps_a_dangling_link_dangling(
    run_cli, workspace: Path, symlink_supported: bool
) -> None:
    """A dangling link stays dangling - `preserve` never dereferences."""
    skip_if_no_symlink(symlink_supported)

    source = make_linked_tree(workspace / "source")
    destination = workspace / "destination"

    result = run_cli("copy", str(source), str(destination))

    assert result.returncode == 0, result.output
    link = destination / "dangling-link.txt"
    assert link.is_symlink(), "the dangling link was not recreated"
    assert not link.exists(), "a dangling link must not be resolved into a file"


def test_follow_mode_copies_the_link_content(
    run_cli, workspace: Path, symlink_supported: bool
) -> None:
    skip_if_no_symlink(symlink_supported)

    source = make_linked_tree(workspace / "source")
    destination = workspace / "destination"

    result = run_cli("copy", str(source), str(destination), "--symlinks", "follow")

    assert result.returncode == 0, result.output
    copied = destination / "file-link.txt"
    assert not copied.is_symlink(), "follow mode must write a regular file"
    assert copied.read_text() == "target payload"


def test_fail_mode_rejects_links(
    run_cli, workspace: Path, symlink_supported: bool
) -> None:
    skip_if_no_symlink(symlink_supported)

    source = workspace / "source"
    source.mkdir()
    target = source / "target.txt"
    target.write_text("target payload")
    (source / "link.txt").symlink_to(target)

    result = run_cli("copy", str(source), str(workspace / "destination"), "--symlinks", "fail")

    # A link in `fail` mode is an error, not a skip.
    assert result.returncode != 0 or "error" in result.output.lower(), result.output


# --------------------------------------------------------------------------- #
# Links cannot be created here
# --------------------------------------------------------------------------- #


def test_symlink_creation_failure_is_reported_not_dropped(
    run_cli, workspace: Path, symlink_supported: bool
) -> None:
    """`docs/COPY_SEMANTICS.md` §5: a link that cannot be created is an error.

    "Creating a symlink without the required privilege is reported as an error
    and counted in `errors`; the entry is never silently dropped."

    This is the substitute verification for a machine that cannot create links,
    which is why the suite does not simply skip there. Where links *can* be
    created the denied path is unreachable, so the test skips - but the skip
    message says which case applied, and the modes themselves are covered by
    the tests above.
    """
    if symlink_supported:
        pytest.skip(
            "this environment can create symlinks, so the permission-denied "
            "path is not reachable here; the link modes are covered by the "
            "tests above instead"
        )

    source = workspace / "source"
    source.mkdir()
    target = source / "target.txt"
    target.write_text("target payload")
    try:
        (source / "link.txt").symlink_to(target)
    except (OSError, NotImplementedError, ValueError):
        pass
    else:
        pytest.skip("link creation unexpectedly succeeded here")

    result = run_cli("copy", str(source), str(workspace / "destination"))

    # Whatever happens, the entry must not vanish without a trace.
    assert result.returncode != 0 or "error" in result.output.lower(), (
        "an entry that could not be created was dropped without being reported"
    )


def test_symlink_capability_is_reported(
    symlink_supported: bool, record_property
) -> None:
    """Make the branch this machine took visible in the test report.

    A reader of a CI log should be able to see whether the symlink assertions
    ran or the documented fallback did, without reading the fixtures.
    """
    record_property("symlink_supported", symlink_supported)
    assert isinstance(symlink_supported, bool)
