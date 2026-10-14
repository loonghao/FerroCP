"""Contract tests for ``EACopy.copy_with_server`` and its ``server`` argument.

Two contracts are covered here:

1. ``server`` is documented as "host name or IP address", and the
   ``ferrocp copy-with-server --server HOST`` CLI asks users for a host name.
   The address therefore has to be resolved, not only parsed as an IP literal.
   Failures must name the offending value instead of a generic
   "Invalid server address".

2. The transfer runs on the shared Tokio runtime while the calling Python
   thread is blocked. That thread must not hold the GIL for the duration, or
   every other Python thread stalls for the whole transfer.

The transfer itself is not yet implemented by ``ferrocp_network`` (it stops at
"Connection creation not implemented yet"), so these tests assert on the
*address handling* and the *GIL behaviour* around that boundary, not on bytes
moving. They stop being meaningful the moment the transfer path starts
connecting: at that point a live server is needed and this module should grow
one.
"""

import threading
import time
from pathlib import Path

import pytest

import ferrocp

# Raised by ferrocp_network once the address is accepted and the transfer proper
# is attempted. Reaching it proves the address was resolved successfully.
_TRANSFER_NOT_IMPLEMENTED = "Connection creation not implemented yet"


def _engine() -> object:
    return ferrocp.EACopy(thread_count=0, buffer_size=64 * 1024)


def test_hostname_is_resolved_not_rejected(tmp_path: Path) -> None:
    """A host name must reach the transfer stage instead of failing to parse."""
    with pytest.raises(Exception) as excinfo:
        _engine().copy_with_server(
            str(tmp_path / "src.txt"),
            str(tmp_path / "dst.txt"),
            "localhost",
            8080,
        )

    message = str(excinfo.value)
    assert _TRANSFER_NOT_IMPLEMENTED in message, (
        "a resolvable host name should get past address resolution, got: " + message
    )


def test_hostname_with_explicit_port_overrides_port_argument(tmp_path: Path) -> None:
    """An explicit `:port` on `server` wins over the separate `port` argument."""
    with pytest.raises(Exception) as excinfo:
        _engine().copy_with_server(
            str(tmp_path / "src.txt"),
            str(tmp_path / "dst.txt"),
            "localhost:9999",
            8080,
        )

    assert _TRANSFER_NOT_IMPLEMENTED in str(excinfo.value)


def test_ipv4_literal_still_works(tmp_path: Path) -> None:
    """An IP literal keeps working; resolving names must not break it."""
    with pytest.raises(Exception) as excinfo:
        _engine().copy_with_server(
            str(tmp_path / "src.txt"),
            str(tmp_path / "dst.txt"),
            "127.0.0.1",
            8080,
        )

    assert _TRANSFER_NOT_IMPLEMENTED in str(excinfo.value)


def test_ipv6_literal_is_accepted(tmp_path: Path) -> None:
    """Both the bracketed and bare IPv6 forms resolve to a socket address."""
    for server in ("[::1]", "::1"):
        with pytest.raises(Exception) as excinfo:
            _engine().copy_with_server(
                str(tmp_path / "src.txt"),
                str(tmp_path / "dst.txt"),
                server,
                8080,
            )
        assert _TRANSFER_NOT_IMPLEMENTED in str(excinfo.value), server


def test_unresolvable_host_error_names_the_host(tmp_path: Path) -> None:
    """The error must identify the bad value, not just say 'Invalid server address'."""
    with pytest.raises(ValueError) as excinfo:
        _engine().copy_with_server(
            str(tmp_path / "src.txt"),
            str(tmp_path / "dst.txt"),
            "no-such-host.invalid",
            8080,
        )

    message = str(excinfo.value)
    assert "no-such-host.invalid" in message
    assert "host name" in message, "the error should state which forms are accepted, got: " + message


def test_empty_server_is_rejected_clearly(tmp_path: Path) -> None:
    """A blank `server` is rejected with a message that says it is blank."""
    with pytest.raises(ValueError) as excinfo:
        _engine().copy_with_server(
            str(tmp_path / "src.txt"),
            str(tmp_path / "dst.txt"),
            "   ",
            8080,
        )

    assert "empty" in str(excinfo.value)


def test_out_of_range_port_is_reported_with_the_address(tmp_path: Path) -> None:
    """A port beyond u16 is reported against the address it came from."""
    with pytest.raises(ValueError) as excinfo:
        _engine().copy_with_server(
            str(tmp_path / "src.txt"),
            str(tmp_path / "dst.txt"),
            "127.0.0.1:99999",
            8080,
        )

    assert "127.0.0.1:99999" in str(excinfo.value)


def test_copy_with_server_releases_the_gil(tmp_path: Path) -> None:
    """Another Python thread must make progress while a transfer is in flight.

    ``copy_with_server`` blocks the calling thread on the Tokio runtime. If it
    held the GIL across that wait, the worker thread below could not run and the
    counter would stay at its starting value.
    """
    src = tmp_path / "src.txt"
    src.write_text("payload")
    dst = tmp_path / "dst.txt"

    progress = {"ticks": 0}
    stop = threading.Event()

    def spin() -> None:
        while not stop.is_set():
            progress["ticks"] += 1
            time.sleep(0.001)

    worker = threading.Thread(target=spin, daemon=True)
    worker.start()
    try:
        # Give the worker a moment to reach its loop before the transfer starts.
        time.sleep(0.05)
        before = progress["ticks"]

        with pytest.raises(Exception):
            _engine().copy_with_server(str(src), str(dst), "localhost", 8080)

        after = progress["ticks"]
    finally:
        stop.set()
        worker.join(timeout=5)

    assert before > 0, "worker thread never started; the test proves nothing"
    assert after > before, (
        f"no Python thread progress during the transfer ({before} -> {after}): the GIL was held for the whole transfer"
    )
