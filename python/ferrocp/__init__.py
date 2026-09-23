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
    >>> # ferrocp.move() is not implemented yet and raises NotImplementedError
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

# Import built-in modules
from pathlib import Path
from typing import Awaitable, Callable, Optional, Union

from ._ferrocp import (
    AsyncManager,
    # Async classes
    AsyncOperation,
    ConfigError,
    # Core classes
    CopyEngine,
    CopyOptions,
    CopyResult,
    # Exceptions
    FerrocpError,
    IoError,
    # Network classes
    NetworkClient,
    NetworkConfig,
    NetworkError,
    Progress,
    # Sync classes
    SyncEngine,
    SyncError,
    SyncOptions,
    copy_directory,
    # Convenience functions
    copy_file,
    copy_file_async,
    copy_with_compression,
    copy_with_verification,
    create_async_manager,
    get_version,
    quick_copy,
    sync_directories,
)

# Re-export with Python-friendly names
from ._ferrocp import (
    CopyEngine as Engine,
)
from ._ferrocp import (
    CopyOptions as Options,
)
from ._ferrocp import (
    CopyResult as Result,
)
from ._ferrocp import (
    NetworkClient as Client,
)
from ._ferrocp import (
    NetworkConfig as ClientConfig,
)
from ._ferrocp import (
    Progress as ProgressInfo,
)
from ._ferrocp import (
    SyncEngine as Synchronizer,
)
from ._ferrocp import (
    SyncOptions as SyncConfig,
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


def move(
    src: Union[str, Path],
    dst: Union[str, Path],
    copy_function: Callable[..., Awaitable[CopyResult]] = copy_file,
) -> str:
    """
    Move a file or directory tree to another location.

    Not implemented: this always raises NotImplementedError.

    A move must finish copying before it removes the source, but the FerroCP
    copy helpers cannot do that today. They are awaitables backed by an engine
    that never starts its scheduler dispatch loop, so the submitted copy task
    is never executed and the await only resolves when the executor's 3600
    second timeout fires, returning ``Err(Timeout waiting for task ...)``
    after an hour (see
    ``crates/ferrocp-python/tests/copy_completes.rs``).

    The previous implementation skipped the await, deleted the source and
    silently lost data; awaiting it would instead block for an hour and then
    fail. Failing fast is the only safe behaviour until the engine dispatch
    path is fixed.

    Args:
        src: Source path
        dst: Destination path
        copy_function: Function to use for copying (default: copy_file)

    Returns:
        The destination path

    Raises:
        NotImplementedError: Always. Use shutil.move() in the meantime.
    """
    raise NotImplementedError(
        "ferrocp.move() is not implemented and would otherwise lose data: "
        "the FerroCP copy helpers are backed by an engine whose dispatch loop "
        "is never started, so a copy only ends when the executor's one-hour "
        "timeout fires. Use shutil.move() instead."
    )


# Backward compatibility class for EACopy
class EACopy:
    """
    Backward compatibility wrapper for the old EACopy API.

    This class provides compatibility with existing code that uses the EACopy interface.
    """

    def __init__(
        self,
        thread_count: int = 4,
        buffer_size: int = 64 * 1024,
        compression_level: int = 0,
        verify_integrity: bool = False,
    ) -> None:
        """Initialize EACopy with configuration options."""
        self.engine = CopyEngine()
        self.default_options = CopyOptions()
        self.default_options.num_threads = thread_count
        self.default_options.buffer_size = buffer_size
        self.default_options.compression_level = compression_level
        self.default_options.enable_compression = compression_level > 0
        self.default_options.verify = verify_integrity

    def copy_file(
        self,
        source: str,
        destination: str,
        options: Optional[CopyOptions] = None,
    ) -> Awaitable[CopyResult]:
        """Copy a single file."""
        copy_options = options or self.default_options
        result = self.engine.copy_file(source, destination, copy_options)
        return result

    def copy_with_server(
        self,
        source: str,
        destination: str,
        server: str,
        port: int = 8080,
    ) -> CopyResult:
        """Copy file using network transfer through a server.

        Args:
            source: Source file path
            destination: Destination file path
            server: Server hostname or IP address
            port: Server port (default: 8080)

        Returns:
            Copy result with network transfer statistics

        Note:
            This uses the ferrocp-network crate for efficient network transfer.
        """
        # Use the network engine for remote transfers
        from ._ferrocp import NetworkCopyEngine

        network_engine = NetworkCopyEngine()
        result = network_engine.copy_with_server(source, destination, server, port)
        return result
