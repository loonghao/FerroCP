"""Type stubs for FerroCP Rust bindings."""

import asyncio
from typing import Any, Callable

# Type aliases
ProgressCallback = Callable[["Progress"], None]
PathLike = str | bytes

class CopyOptions:
    """Configuration options for copy operations."""

    def __init__(
        self,
        *,
        mode: str = "auto",
        overwrite: str = "always",
        overwrite_callback: Callable[[str, str], bool] | None = None,
        preserve_timestamps: bool = True,
        preserve_permissions: bool = True,
        follow_symlinks: bool = False,
        enable_compression: bool = False,
        compression_level: int = 6,
        buffer_size: int = 65536,
        num_threads: int = 0,
        verify: bool = False,
    ) -> None: ...
    @property
    def mode(self) -> str: ...
    @mode.setter
    def mode(self, value: str) -> None: ...
    @property
    def overwrite(self) -> str: ...
    @overwrite.setter
    def overwrite(self, value: str) -> None: ...
    @property
    def preserve_timestamps(self) -> bool: ...
    @preserve_timestamps.setter
    def preserve_timestamps(self, value: bool) -> None: ...
    @property
    def preserve_permissions(self) -> bool: ...
    @preserve_permissions.setter
    def preserve_permissions(self, value: bool) -> None: ...
    @property
    def follow_symlinks(self) -> bool: ...
    @follow_symlinks.setter
    def follow_symlinks(self, value: bool) -> None: ...
    @property
    def enable_compression(self) -> bool: ...
    @enable_compression.setter
    def enable_compression(self, value: bool) -> None: ...
    @property
    def compression_level(self) -> int: ...
    @compression_level.setter
    def compression_level(self, value: int) -> None: ...
    @property
    def buffer_size(self) -> int: ...
    @buffer_size.setter
    def buffer_size(self, value: int) -> None: ...
    @property
    def num_threads(self) -> int: ...
    @num_threads.setter
    def num_threads(self, value: int) -> None: ...
    @property
    def verify(self) -> bool: ...
    @verify.setter
    def verify(self, value: bool) -> None: ...
    @property
    def overwrite_callback(self) -> Callable[[str, str], bool] | None: ...
    @overwrite_callback.setter
    def overwrite_callback(self, value: Callable[[str, str], bool] | None) -> None: ...

class CopyResult:
    """Result of a copy operation."""

    @property
    def success(self) -> bool: ...
    @property
    def bytes_copied(self) -> int: ...
    @property
    def files_copied(self) -> int: ...
    @property
    def duration_seconds(self) -> float: ...
    @property
    def transfer_rate(self) -> float: ...
    @property
    def error_message(self) -> str | None: ...

class Progress:
    """Progress information for copy operations."""

    @property
    def bytes_copied(self) -> int: ...
    @property
    def total_bytes(self) -> int: ...
    @property
    def files_copied(self) -> int: ...
    @property
    def total_files(self) -> int: ...
    @property
    def percentage(self) -> float: ...
    @property
    def transfer_rate(self) -> float: ...
    @property
    def eta_seconds(self) -> float | None: ...

class CopyEngine:
    """High-performance copy engine."""

    def __init__(self) -> None: ...
    def copy_file(
        self,
        source: PathLike,
        destination: PathLike,
        options: CopyOptions | None = None,
        progress_callback: ProgressCallback | None = None,
    ) -> asyncio.Future[CopyResult]: ...
    def copy_directory(
        self,
        source: PathLike,
        destination: PathLike,
        options: CopyOptions | None = None,
        progress_callback: ProgressCallback | None = None,
    ) -> asyncio.Future[CopyResult]: ...
    def copy_file_async(
        self,
        source: PathLike,
        destination: PathLike,
        options: CopyOptions | None = None,
    ) -> asyncio.Future[AsyncOperation]: ...
    def get_statistics(self) -> dict[str, Any]: ...
    def get_features(self) -> dict[str, bool]: ...
    def is_busy(self) -> bool: ...
    def get_async_manager(self) -> AsyncManager: ...

class AsyncOperation:
    """Handle for an asynchronous operation."""

    @property
    def id(self) -> str: ...
    def is_running(self) -> asyncio.Future[bool]: ...
    def cancel(self) -> asyncio.Future[bool]: ...
    def get_progress(self) -> asyncio.Future[float | None]: ...
    def wait(self) -> asyncio.Future[bool]: ...

class AsyncManager:
    """Manager for asynchronous operations."""

    def __init__(self) -> None: ...
    def get_active_operations(self) -> asyncio.Future[list[str]]: ...
    def get_operation(self, operation_id: str) -> asyncio.Future[str | None]: ...
    def cancel_all(self) -> asyncio.Future[int]: ...

class SyncOptions:
    """Options for directory synchronization."""

    def __init__(
        self,
        *,
        bidirectional: bool = False,
        delete_orphaned: bool = False,
        preserve_newer: bool = True,
        dry_run: bool = False,
    ) -> None: ...

class SyncEngine:
    """Directory synchronization engine."""

    def __init__(self) -> None: ...

class NetworkConfig:
    """Network configuration for remote transfers."""

    def __init__(
        self,
        *,
        host: str,
        port: int = 8080,
        timeout: float = 30.0,
        max_connections: int = 4,
    ) -> None: ...

class NetworkClient:
    """Client for network file transfers."""

    def __init__(self, config: NetworkConfig) -> None: ...

class NetworkCopyEngine:
    """Copy engine that moves files through a FerroCP server."""

    def __init__(self) -> None: ...
    def copy_with_server(
        self,
        source: str,
        destination: str,
        server: str,
        port: int = 8080,
    ) -> CopyResult: ...

# Exceptions
class FerrocpError(Exception):
    """Base exception for FerroCP errors."""

    pass

class IoError(FerrocpError):
    """I/O error raised for filesystem level failures."""

    pass

class CopyError(FerrocpError):
    """Error during copy operation."""

    pass

class NetworkError(FerrocpError):
    """Network-related error."""

    pass

class SyncError(FerrocpError):
    """Synchronization error."""

    pass

class ConfigError(FerrocpError):
    """Configuration error."""

    pass

# Convenience functions
def copy_file(
    source: PathLike,
    destination: PathLike,
    options: CopyOptions | None = None,
    progress_callback: ProgressCallback | None = None,
) -> asyncio.Future[CopyResult]: ...
def copy_directory(
    source: PathLike,
    destination: PathLike,
    options: CopyOptions | None = None,
    progress_callback: ProgressCallback | None = None,
) -> asyncio.Future[CopyResult]: ...
def quick_copy(
    source: PathLike, destination: PathLike
) -> asyncio.Future[CopyResult]: ...
def copy_with_verification(
    source: PathLike,
    destination: PathLike,
    progress_callback: ProgressCallback | None = None,
) -> asyncio.Future[CopyResult]: ...
def copy_with_compression(
    source: PathLike,
    destination: PathLike,
    progress_callback: ProgressCallback | None = None,
) -> asyncio.Future[CopyResult]: ...
def copy_file_async(
    source: PathLike,
    destination: PathLike,
    options: CopyOptions | None = None,
) -> asyncio.Future[AsyncOperation]: ...
def create_async_manager() -> AsyncManager: ...
def sync_directories(
    source: PathLike,
    destination: PathLike,
    options: SyncOptions | None = None,
) -> asyncio.Future[CopyResult]: ...
def get_version() -> str: ...
