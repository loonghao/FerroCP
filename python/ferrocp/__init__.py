"""FerroCP - High-performance cross-platform file copying tool written in Rust."""

# Import built-in modules
import os
import sys
from typing import Optional, Union

# Import local modules
from .__version__ import __version__

# Import Rust bindings
from ._ferrocp_binding import (
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
    "Config",
    "EACopy",
    "__eacopy_version__",
    "__version__",
    "config",
    "copy",
    "copy2",
    "copy_with_server",
    "copyfile",
    "copytree",
]
