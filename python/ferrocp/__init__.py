"""
FerroCP - High-performance file copying library with zero-copy optimization.

This library provides fast, efficient file copying capabilities with support for:
- Zero-copy optimization for maximum performance
- Asynchronous operations with cancellation support
- Network file transfers with resume capability
- Directory synchronization with conflict resolution
- Progress reporting and monitoring
- Cross-platform compatibility

Basic Usage (shutil-compatible):
    >>> import ferrocp
    >>>
    >>> # Drop-in replacement for shutil
    >>> ferrocp.copy("source.txt", "destination.txt")  # Like shutil.copy
    >>> ferrocp.copytree("src_dir", "dest_dir")        # Like shutil.copytree
    >>> ferrocp.move("old_path", "new_path")           # Like shutil.move
    >>>
    >>> # Or use the explicit API
    >>> ferrocp.copy_file("source.txt", "destination.txt")
    >>>
    >>> # Copy with options
    >>> options = ferrocp.CopyOptions(verify=True, preserve_timestamps=True)
    >>> ferrocp.copy_file("source.txt", "dest.txt", options=options)
    >>>
    >>> # Async copy with progress
    >>> async def copy_with_progress():
    ...     def progress_callback(progress):
    ...         print(f"Progress: {progress.percentage:.1f}%")
    ...
    ...     await ferrocp.copy_file_async(
    ...         "large_file.bin",
    ...         "backup.bin",
    ...         progress_callback=progress_callback
    ...     )

Advanced Usage:
    >>> # Create a copy engine for multiple operations
    >>> engine = ferrocp.CopyEngine()
    >>>
    >>> # Copy directory with exclusions
    >>> options = ferrocp.CopyOptions(
    ...     recursive=True,
    ...     exclude_patterns=["*.tmp", "*.log"]
    ... )
    >>> engine.copy_directory("src_dir", "dest_dir", options)
    >>>
    >>> # Network transfer
    >>> client = ferrocp.NetworkClient("192.168.1.100:8080")
    >>> client.transfer_file("local.txt", "remote.txt")
    >>>
    >>> # Directory synchronization
    >>> sync_engine = ferrocp.SyncEngine()
    >>> sync_options = ferrocp.SyncOptions(
    ...     bidirectional=True,
    ...     delete_orphaned=True
    ... )
    >>> sync_engine.sync_directories("dir1", "dir2", sync_options)
"""

from ._ferrocp import (
    # Core classes
    CopyEngine,
    CopyOptions,
    CopyResult,
    Progress,

    # Sync classes
    SyncEngine,
    SyncOptions,

    # Network classes
    NetworkClient,
    NetworkConfig,

    # Async classes
    AsyncOperation,
    AsyncManager,

    # Convenience functions
    copy_file,
    copy_directory,
    quick_copy,
    copy_with_verification,
    copy_with_compression,
    copy_file_async,
    create_async_manager,
    sync_directories,
    get_version,

    # Exceptions
    FerrocpError,
    IoError,
    NetworkError,
    SyncError,
    ConfigError,
)

# Re-export with Python-friendly names
from ._ferrocp import (
    CopyEngine as Engine,
    CopyOptions as Options,
    CopyResult as Result,
    Progress as ProgressInfo,
    SyncEngine as Synchronizer,
    SyncOptions as SyncConfig,
    NetworkClient as Client,
    NetworkConfig as ClientConfig,
)

# Version information
__version__ = get_version()
__eacopy_version__ = get_version()  # Backward compatibility
__author__ = "FerroCP Team"
__email__ = "team@ferrocp.dev"
__license__ = "MIT OR Apache-2.0"

# Shutil-compatible API aliases for easy migration
copy = copy_file  # shutil.copy equivalent
copy2 = copy_file  # shutil.copy2 equivalent (preserves metadata by default)
copytree = copy_directory  # shutil.copytree equivalent

# Public API
__all__ = [
    # Core functionality
    "copy_file",
    "copy_directory",
    "quick_copy",
    "copy_with_verification",
    "copy_with_compression",
    "sync_directories",

    # Shutil-compatible aliases
    "copy",
    "copy2",
    "copytree",
    "move",

    # Async functionality
    "copy_file_async",
    "create_async_manager",

    # Classes
    "CopyEngine",
    "CopyOptions",
    "CopyResult",
    "Progress",
    "SyncEngine",
    "SyncOptions",
    "NetworkClient",
    "NetworkConfig",
    "AsyncOperation",
    "AsyncManager",

    # Aliases
    "Engine",
    "Options",
    "Result",
    "ProgressInfo",
    "Synchronizer",
    "SyncConfig",
    "Client",
    "ClientConfig",

    # Exceptions
    "FerrocpError",
    "IoError",
    "NetworkError",
    "SyncError",
    "ConfigError",

    # Utilities
    "get_version",

    # Backward compatibility
    "EACopy",

    # Metadata
    "__version__",
    "__eacopy_version__",
    "__author__",
    "__email__",
    "__license__",
]


def configure_logging(level: str = "INFO") -> None:
    """
    Configure logging for FerroCP operations.

    Args:
        level: Logging level ("DEBUG", "INFO", "WARNING", "ERROR")
    """
    import logging

    logger = logging.getLogger("ferrocp")
    handler = logging.StreamHandler()
    formatter = logging.Formatter(
        "%(asctime)s - %(name)s - %(levelname)s - %(message)s"
    )
    handler.setFormatter(formatter)
    logger.addHandler(handler)
    logger.setLevel(getattr(logging, level.upper()))


def get_features() -> dict:
    """
    Get information about available features.

    Returns:
        Dictionary containing feature availability information
    """
    engine = CopyEngine()
    return engine.get_features()


def get_statistics() -> dict:
    """
    Get global statistics for FerroCP operations.

    Returns:
        Dictionary containing operation statistics
    """
    engine = CopyEngine()
    return engine.get_statistics()


def move(src, dst, copy_function=copy_file):
    """
    Move a file or directory tree to another location.

    This is similar to shutil.move() but uses FerroCP for better performance.

    Args:
        src: Source path
        dst: Destination path
        copy_function: Function to use for copying (default: copy_file)

    Returns:
        The destination path
    """
    import os
    from pathlib import Path

    src_path = Path(src)
    dst_path = Path(dst)

    # If destination is a directory, move source into it
    if dst_path.is_dir():
        dst_path = dst_path / src_path.name

    # Copy the file/directory
    if src_path.is_dir():
        copy_directory(str(src_path), str(dst_path))
    else:
        copy_file(str(src_path), str(dst_path))

    # Remove the source
    if src_path.is_dir():
        import shutil
        shutil.rmtree(src_path)
    else:
        src_path.unlink()

    return str(dst_path)


# Backward compatibility class for EACopy
class EACopy:
    """
    Backward compatibility wrapper for the old EACopy API.

    This class provides compatibility with existing code that uses the EACopy interface.
    """

    def __init__(self, thread_count=4, buffer_size=64*1024, compression_level=0, verify_integrity=False):
        """Initialize EACopy with configuration options."""
        self.engine = CopyEngine()
        self.default_options = CopyOptions()
        self.default_options.num_threads = thread_count
        self.default_options.buffer_size = buffer_size
        self.default_options.compression_level = compression_level
        self.default_options.enable_compression = compression_level > 0
        self.default_options.verify = verify_integrity

    def copy_file(self, source, destination, options=None):
        """Copy a single file."""
        copy_options = options or self.default_options
        result = self.engine.copy_file(source, destination, copy_options)
        return result

    def copy_with_server(self, source, destination, server, port=8080):
        """Copy file using a server (mock implementation for compatibility)."""
        # For backward compatibility, just do a regular copy
        # In a real implementation, this would use network transfer
        result = self.copy_file(source, destination)
        return result