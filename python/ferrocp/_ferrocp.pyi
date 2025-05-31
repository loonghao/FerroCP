"""Type stubs for FerroCP Rust bindings."""

from typing import Any, Callable, Dict, List, Optional, Union
import asyncio

# Type aliases
ProgressCallback = Callable[["Progress"], None]
PathLike = Union[str, bytes]

class CopyOptions:
    """Configuration options for copy operations."""
    
    def __init__(
        self,
        *,
        verify: bool = False,
        preserve_timestamps: bool = True,
        preserve_permissions: bool = True,
        enable_compression: bool = False,
        buffer_size: int = 65536,
        max_retries: int = 3,
        retry_delay: float = 1.0,
        overwrite: bool = True,
        recursive: bool = False,
    ) -> None: ...
    
    @property
    def verify(self) -> bool: ...
    @verify.setter
    def verify(self, value: bool) -> None: ...
    
    @property
    def preserve_timestamps(self) -> bool: ...
    @preserve_timestamps.setter
    def preserve_timestamps(self, value: bool) -> None: ...
    
    @property
    def preserve_permissions(self) -> bool: ...
    @preserve_permissions.setter
    def preserve_permissions(self, value: bool) -> None: ...
    
    @property
    def enable_compression(self) -> bool: ...
    @enable_compression.setter
    def enable_compression(self, value: bool) -> None: ...
    
    @property
    def buffer_size(self) -> int: ...
    @buffer_size.setter
    def buffer_size(self, value: int) -> None: ...
    
    @property
    def max_retries(self) -> int: ...
    @max_retries.setter
    def max_retries(self, value: int) -> None: ...
    
    @property
    def retry_delay(self) -> float: ...
    @retry_delay.setter
    def retry_delay(self, value: float) -> None: ...
    
    @property
    def overwrite(self) -> bool: ...
    @overwrite.setter
    def overwrite(self, value: bool) -> None: ...
    
    @property
    def recursive(self) -> bool: ...
    @recursive.setter
    def recursive(self, value: bool) -> None: ...

class CopyResult:
    """Result of a copy operation."""
    
    @property
    def success(self) -> bool: ...
    
    @property
    def bytes_copied(self) -> int: ...
    
    @property
    def files_copied(self) -> int: ...
    
    @property
    def duration_ms(self) -> int: ...
    
    @property
    def error_message(self) -> Optional[str]: ...
    
    @property
    def throughput_mbps(self) -> float: ...

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
    def speed_mbps(self) -> float: ...
    
    @property
    def eta_seconds(self) -> Optional[float]: ...

class CopyEngine:
    """High-performance copy engine."""
    
    def __init__(self) -> None: ...
    
    def copy_file(
        self,
        source: PathLike,
        destination: PathLike,
        options: Optional[CopyOptions] = None,
        progress_callback: Optional[ProgressCallback] = None,
    ) -> asyncio.Future[CopyResult]: ...
    
    def copy_directory(
        self,
        source: PathLike,
        destination: PathLike,
        options: Optional[CopyOptions] = None,
        progress_callback: Optional[ProgressCallback] = None,
    ) -> asyncio.Future[CopyResult]: ...
    
    def copy_file_async(
        self,
        source: PathLike,
        destination: PathLike,
        options: Optional[CopyOptions] = None,
    ) -> asyncio.Future["AsyncOperation"]: ...
    
    def get_statistics(self) -> Dict[str, Any]: ...
    
    def get_features(self) -> Dict[str, bool]: ...
    
    def is_busy(self) -> bool: ...
    
    def get_async_manager(self) -> "AsyncManager": ...

class AsyncOperation:
    """Handle for an asynchronous operation."""
    
    @property
    def id(self) -> str: ...
    
    def is_running(self) -> asyncio.Future[bool]: ...
    
    def cancel(self) -> asyncio.Future[bool]: ...
    
    def get_progress(self) -> asyncio.Future[Optional[float]]: ...
    
    def wait(self) -> asyncio.Future[bool]: ...

class AsyncManager:
    """Manager for asynchronous operations."""
    
    def __init__(self) -> None: ...
    
    def get_active_operations(self) -> asyncio.Future[List[str]]: ...
    
    def get_operation(self, operation_id: str) -> asyncio.Future[Optional[str]]: ...
    
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

# Exceptions
class FerrocpError(Exception):
    """Base exception for FerroCP errors."""
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
    options: Optional[CopyOptions] = None,
    progress_callback: Optional[ProgressCallback] = None,
) -> asyncio.Future[CopyResult]: ...

def copy_directory(
    source: PathLike,
    destination: PathLike,
    options: Optional[CopyOptions] = None,
    progress_callback: Optional[ProgressCallback] = None,
) -> asyncio.Future[CopyResult]: ...

def quick_copy(source: PathLike, destination: PathLike) -> asyncio.Future[CopyResult]: ...

def copy_with_verification(
    source: PathLike,
    destination: PathLike,
    progress_callback: Optional[ProgressCallback] = None,
) -> asyncio.Future[CopyResult]: ...

def copy_with_compression(
    source: PathLike,
    destination: PathLike,
    progress_callback: Optional[ProgressCallback] = None,
) -> asyncio.Future[CopyResult]: ...

def copy_file_async(
    source: PathLike,
    destination: PathLike,
    options: Optional[CopyOptions] = None,
) -> asyncio.Future[AsyncOperation]: ...

def create_async_manager() -> AsyncManager: ...

def sync_directories(
    source: PathLike,
    destination: PathLike,
    options: Optional[SyncOptions] = None,
) -> asyncio.Future[CopyResult]: ...

def get_version() -> str: ...
