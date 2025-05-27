"""Test version information."""

# Import local modules
import ferrocp


def test_version():
    """Test that version is a string."""
    assert isinstance(ferrocp.__version__, str)
    assert ferrocp.__version__ != ""


def test_eacopy_version():
    """Test that EACopy version is a string."""
    assert isinstance(ferrocp.__eacopy_version__, str)
    assert ferrocp.__eacopy_version__ != ""
