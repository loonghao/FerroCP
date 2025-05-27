"""Test version information."""

# Import local modules
import py_eacopy


def test_version():
    """Test that version is a string."""
    assert isinstance(py_eacopy.__version__, str)
    assert py_eacopy.__version__ != ""


def test_eacopy_version():
    """Test that EACopy version is a string."""
    assert isinstance(py_eacopy.__eacopy_version__, str)
    assert py_eacopy.__eacopy_version__ != ""
