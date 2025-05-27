"""Python bindings for EACopy, a high-performance file copy tool."""

# Import built-in modules
import os
import sys
from typing import Optional, Union

# Import local modules
from .__version__ import __version__

# Import Rust bindings
from ._py_eacopy_binding import (
    EACopy,
    copy,
    copy2,
    copyfile,
    copytree,
    copy_with_server,
    __eacopy_version__,
)

from .config import Config

# Initialize global configuration
config = Config()

__all__ = [
    "__version__",
    "__eacopy_version__",
    "config",
    "copy",
    "copy2",
    "copyfile",
    "copytree",
    "copy_with_server",
    "Config",
    "EACopy",
]
