"""
FerroCP - High-performance file copying library with zero-copy optimization.

This library provides fast, efficient file copying capabilities with support for:
- Zero-copy optimization for maximum performance
- Asynchronous operations with cancellation support
- Network file transfers with resume capability
- Directory synchronization with conflict resolution
- Progress reporting and monitoring
- Cross-platform compatibility

Basic Usage:
    >>> import ferrocp
    >>>
    >>> # Simple file copy
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
__author__ = "FerroCP Team"
__email__ = "team@ferrocp.dev"
__license__ = "MIT OR Apache-2.0"

# Public API
__all__ = [
    # Core functionality
    "copy_file",
    "copy_directory",
    "quick_copy",
    "copy_with_verification",
    "copy_with_compression",
    "sync_directories",

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

    # Metadata
    "__version__",
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
