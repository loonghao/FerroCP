"""Configuration handling for EACopy."""

class Config:
    """Global configuration options for EACopy.
    
    Attributes:
        thread_count: Default number of threads to use for copy operations.
        compression_level: Default compression level (0-9) for network transfers.
        buffer_size: Size of the buffer used for copy operations (in bytes).
        preserve_metadata: Whether to preserve file metadata by default.
        follow_symlinks: Whether to follow symbolic links by default.
        dirs_exist_ok: Whether to allow existing directories by default.
        zerocopy_enabled: Whether zero-copy operations are enabled.
        zerocopy_min_size: Minimum file size for zero-copy operations (bytes).
        auto_detect_device: Whether to automatically detect device types.
        small_file_batch_size: Batch size for small files.
    """

    def __init__(self) -> None:
        """Initialize configuration with default values."""
        # Default thread count
        self.thread_count: int = 4
        
        # Default compression level (0-9)
        self.compression_level: int = 0
        
        # Default buffer size (8MB)
        self.buffer_size: int = 8 * 1024 * 1024
        
        # Default metadata preservation
        self.preserve_metadata: bool = True
        
        # Default symlink handling
        self.follow_symlinks: bool = False
        
        # Default directory handling
        self.dirs_exist_ok: bool = False
        
        # Zero-copy settings
        self.zerocopy_enabled: bool = True
        self.zerocopy_min_size: int = 64 * 1024  # 64KB minimum
        
        # Device detection
        self.auto_detect_device: bool = True
        
        # Batch processing
        self.small_file_batch_size: int = 50

    def __repr__(self) -> str:
        """Return string representation of configuration."""
        return (
            f"Config(thread_count={self.thread_count}, "
            f"compression_level={self.compression_level}, "
            f"buffer_size={self.buffer_size}, "
            f"preserve_metadata={self.preserve_metadata}, "
            f"follow_symlinks={self.follow_symlinks}, "
            f"dirs_exist_ok={self.dirs_exist_ok}, "
            f"zerocopy_enabled={self.zerocopy_enabled}, "
            f"zerocopy_min_size={self.zerocopy_min_size}, "
            f"auto_detect_device={self.auto_detect_device}, "
            f"small_file_batch_size={self.small_file_batch_size})"
        )
