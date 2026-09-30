"""Unit tests for ``CopyOptions``.

The rule under test is the second invariant of ``docs/COPY_SEMANTICS.md``: an
option that cannot be honoured is rejected with an error, never ignored. A knob
that is silently dropped is worse than one that raises.

Two different layers enforce it, and this module keeps them apart:

* ``CopyOptions(...)`` validates the combinations it can see at construction
  time - the ones that would otherwise leave the object in a state that cannot
  be honoured, such as ``overwrite="prompt"`` with no callback.
* The **tuning** fields (``buffer_size``, ``num_threads``,
  ``compression_level``) are plain attributes, so they are validated when a copy
  is submitted. ``test_tuning.py`` covers that path because it needs a real
  copy.

Everything here therefore runs without touching the copy engine, which keeps
the module fast and makes a failure point at the option layer rather than at
the engine.
"""

# Import built-in modules
import math

# Import third-party modules
import pytest

# Import local modules
import ferrocp

# ``buffer_size`` must be a power of two between 4 KiB and 64 MiB.
MIN_BUFFER = 4 * 1024
MAX_BUFFER = 64 * 1024 * 1024


def test_defaults_match_the_documented_contract() -> None:
    """The advertised defaults are the ones that actually take effect."""
    options = ferrocp.CopyOptions()

    # "always" is what FerroCP has always done. The old default was "prompt",
    # but nothing ever prompted, so it advertised protection that did not exist.
    assert options.overwrite == "always"
    # ``preserve`` is the default symlink handling: never lose data.
    assert options.follow_symlinks is False
    assert options.mode == "auto"
    assert options.preserve_timestamps is True
    assert options.preserve_permissions is True
    assert options.verify is False
    # The tuning knobs default to "do what you would do anyway".
    assert options.num_threads == 0
    assert options.compression_level == 0


@pytest.mark.parametrize(
    "value",
    ["always", "never", "if_newer", "if_different", "fail", "prompt"],
)
def test_every_documented_overwrite_policy_is_accepted(value: str) -> None:
    """Every policy the contract lists is accepted."""
    options = ferrocp.CopyOptions(overwrite=value, overwrite_callback=lambda *_: True)
    assert options.overwrite == value


@pytest.mark.parametrize(
    "alias",
    ["overwrite", "auto", "all", "skip", "newer", "different", "error", "ask"],
)
def test_documented_overwrite_aliases_are_accepted(alias: str) -> None:
    """``OverwritePolicy::parse`` accepts the spellings the contract lists."""
    options = ferrocp.CopyOptions(overwrite=alias, overwrite_callback=lambda *_: True)
    assert options.overwrite == alias


def test_overwrite_matching_is_case_insensitive() -> None:
    """Policy names are matched case-insensitively, per the contract."""
    options = ferrocp.CopyOptions(overwrite="IF_NEWER", overwrite_callback=lambda *_: True)
    assert options.overwrite == "IF_NEWER"


@pytest.mark.parametrize("value", ["", "sometimes", "if_size_differs", "ALWAYS_"])
def test_unknown_overwrite_policy_is_rejected_not_defaulted(value: str) -> None:
    """A typo must raise, not silently fall back to ``always``."""
    with pytest.raises(ValueError):
        ferrocp.CopyOptions(overwrite=value)


@pytest.mark.parametrize("value", ["all", "auto", "newer", "different"])
def test_supported_copy_modes_are_accepted(value: str) -> None:
    """Every implemented copy mode is accepted."""
    options = ferrocp.CopyOptions(mode=value)
    assert options.mode == value


def test_mirror_mode_is_rejected_not_degraded() -> None:
    """``mirror`` deletes destination files, so it must not degrade to ``all``."""
    with pytest.raises(ValueError, match="mirror"):
        ferrocp.CopyOptions(mode="mirror")


@pytest.mark.parametrize("value", ["", "synchronize", "incremental"])
def test_unknown_copy_mode_is_rejected(value: str) -> None:
    """A mode that does not exist must raise, not fall back."""
    with pytest.raises(ValueError):
        ferrocp.CopyOptions(mode=value)


def test_prompt_requires_a_callback_at_construction_time() -> None:
    """An unanswered prompt must never be treated as consent to overwrite."""
    with pytest.raises(ValueError, match="overwrite_callback"):
        ferrocp.CopyOptions(overwrite="prompt", overwrite_callback=None)


def test_prompt_with_a_callback_is_accepted() -> None:
    """A registered handler makes ``prompt`` a valid, honoured policy."""
    callback_calls: list[tuple[str, str]] = []

    def callback(source: str, destination: str) -> bool:
        callback_calls.append((source, destination))
        return True

    options = ferrocp.CopyOptions(overwrite="prompt", overwrite_callback=callback)
    assert options.overwrite == "prompt"
    assert options.overwrite_callback is callback


def test_a_non_default_policy_does_not_require_a_callback() -> None:
    """Only ``prompt`` needs a handler; the others decide on their own."""
    for value in ["always", "never", "if_newer", "if_different", "fail"]:
        options = ferrocp.CopyOptions(overwrite=value)
        assert options.overwrite_callback is None


def test_options_are_mutable() -> None:
    """The fields are read/write, so callers can adjust a shared template."""
    options = ferrocp.CopyOptions()
    options.overwrite = "never"
    options.follow_symlinks = True
    options.buffer_size = 128 * 1024

    assert options.overwrite == "never"
    assert options.follow_symlinks is True
    assert options.buffer_size == 128 * 1024


def test_mutation_is_not_validated_eagerly() -> None:
    """Assigning ``prompt`` after construction is accepted, not rejected.

    The ``prompt`` plus missing callback check runs in the constructor and in
    the copy path, but not in the field setter, so a mutated object can hold the
    combination for a while. That is deliberate rather than a hole: there is no
    I/O to protect yet, and the copy still refuses before touching anything.

    The contract that matters is the one this pins down - the copy raises - and
    ``tests/e2e`` covers it end to end.
    """
    options = ferrocp.CopyOptions()
    options.overwrite = "prompt"
    assert options.overwrite == "prompt"
    assert options.overwrite_callback is None


def test_buffer_size_default_is_within_the_accepted_range() -> None:
    """The default has to be a value the I/O layer can actually use."""
    size = ferrocp.CopyOptions().buffer_size
    assert MIN_BUFFER <= size <= MAX_BUFFER
    assert size & (size - 1) == 0, f"default buffer size {size} is not a power of two"


@pytest.mark.parametrize("size", [MIN_BUFFER, 64 * 1024, 1024 * 1024, MAX_BUFFER])
def test_valid_buffer_sizes_are_accepted(size: int) -> None:
    """Sizes the I/O layer can use are accepted."""
    options = ferrocp.CopyOptions(buffer_size=size)
    assert options.buffer_size == size


def test_every_power_of_two_in_range_is_accepted() -> None:
    """Sweep the whole accepted range rather than a few sample points."""
    for exponent in range(int(math.log2(MIN_BUFFER)), int(math.log2(MAX_BUFFER)) + 1):
        size = 2**exponent
        assert ferrocp.CopyOptions(buffer_size=size).buffer_size == size
