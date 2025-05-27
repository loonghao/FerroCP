//! Zero-copy file operations for py-eacopy
//!
//! This module provides platform-specific zero-copy file operations including
//! copy_file_range (Linux), reflink (BTRFS/XFS), CoW (ReFS), and automatic
//! fallback to optimized copy methods on unsupported platforms.

use crate::error::Result;
use std::path::Path;
use tracing::{debug, warn};

#[cfg(target_os = "linux")]
use std::os::unix::io::AsRawFd;

/// Zero-copy operation result
#[derive(Debug, Clone)]
pub struct ZeroCopyResult {
    /// Number of bytes copied using zero-copy
    pub bytes_copied: u64,
    /// Whether zero-copy was actually used
    pub zerocopy_used: bool,
    /// The method used for zero-copy
    pub method: ZeroCopyMethod,
}

/// Available zero-copy methods
#[derive(Debug, Clone, PartialEq)]
pub enum ZeroCopyMethod {
    /// Linux copy_file_range system call
    CopyFileRange,
    /// BTRFS/XFS reflink
    Reflink,
    /// Windows ReFS Copy-on-Write
    RefsCoW,
    /// Fallback to regular copy
    Fallback,
    /// Not attempted
    None,
}

/// Zero-copy engine for handling platform-specific operations
pub struct ZeroCopyEngine {
    /// Whether zero-copy is enabled
    enabled: bool,
}

impl ZeroCopyEngine {
    /// Create a new zero-copy engine
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    /// Attempt to copy a file using zero-copy methods
    pub async fn copy_file<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        source: P,
        destination: Q,
        file_size: u64,
    ) -> Result<ZeroCopyResult> {
        let source = source.as_ref();
        let destination = destination.as_ref();

        if !self.enabled {
            debug!("Zero-copy disabled, skipping");
            return Ok(ZeroCopyResult {
                bytes_copied: 0,
                zerocopy_used: false,
                method: ZeroCopyMethod::None,
            });
        }

        debug!("Attempting zero-copy for {:?} -> {:?}", source, destination);

        // Try platform-specific zero-copy methods
        #[cfg(target_os = "linux")]
        {
            if let Ok(result) = self.try_copy_file_range(source, destination, file_size).await {
                if result.zerocopy_used {
                    debug!("Successfully used copy_file_range");
                    return Ok(result);
                }
            }

            if let Ok(result) = self.try_reflink(source, destination, file_size).await {
                if result.zerocopy_used {
                    debug!("Successfully used reflink");
                    return Ok(result);
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            if let Ok(result) = self.try_refs_cow(source, destination, file_size).await {
                if result.zerocopy_used {
                    debug!("Successfully used ReFS CoW");
                    return Ok(result);
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            if let Ok(result) = self.try_clonefile(source, destination, file_size).await {
                if result.zerocopy_used {
                    debug!("Successfully used clonefile");
                    return Ok(result);
                }
            }
        }

        warn!("No zero-copy method available, will fallback to regular copy");
        Ok(ZeroCopyResult {
            bytes_copied: 0,
            zerocopy_used: false,
            method: ZeroCopyMethod::Fallback,
        })
    }

    /// Check if zero-copy is supported on the current platform
    pub fn is_supported(&self) -> bool {
        if !self.enabled {
            return false;
        }

        #[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
        return true;

        #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
        return false;
    }

    /// Get available zero-copy methods for the current platform
    pub fn available_methods(&self) -> Vec<ZeroCopyMethod> {
        if !self.enabled {
            return vec![ZeroCopyMethod::None];
        }

        let mut methods = Vec::new();

        #[cfg(target_os = "linux")]
        {
            methods.push(ZeroCopyMethod::CopyFileRange);
            methods.push(ZeroCopyMethod::Reflink);
        }

        #[cfg(target_os = "windows")]
        {
            methods.push(ZeroCopyMethod::RefsCoW);
        }

        #[cfg(target_os = "macos")]
        {
            methods.push(ZeroCopyMethod::Reflink);
        }

        if methods.is_empty() {
            methods.push(ZeroCopyMethod::Fallback);
        }

        methods
    }

    /// Try Linux copy_file_range system call
    #[cfg(target_os = "linux")]
    async fn try_copy_file_range<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        source: P,
        destination: Q,
        file_size: u64,
    ) -> Result<ZeroCopyResult> {
        use tokio::fs::File;

        let source_file = File::open(source).await?;
        let dest_file = File::create(destination).await?;

        let source_fd = source_file.as_raw_fd();
        let dest_fd = dest_file.as_raw_fd();

        // Use copy_file_range system call
        let result = unsafe {
            libc::copy_file_range(
                source_fd,
                std::ptr::null_mut(),
                dest_fd,
                std::ptr::null_mut(),
                file_size as usize,
                0,
            )
        };

        if result > 0 {
            Ok(ZeroCopyResult {
                bytes_copied: result as u64,
                zerocopy_used: true,
                method: ZeroCopyMethod::CopyFileRange,
            })
        } else {
            Ok(ZeroCopyResult {
                bytes_copied: 0,
                zerocopy_used: false,
                method: ZeroCopyMethod::Fallback,
            })
        }
    }

    /// Try reflink operation (BTRFS/XFS)
    #[cfg(target_os = "linux")]
    async fn try_reflink<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        source: P,
        destination: Q,
        file_size: u64,
    ) -> Result<ZeroCopyResult> {
        use tokio::fs::File;

        let source_file = File::open(source).await?;
        let dest_file = File::create(destination).await?;

        let source_fd = source_file.as_raw_fd();
        let dest_fd = dest_file.as_raw_fd();

        // FICLONE ioctl for reflink
        const FICLONE: libc::c_ulong = 0x40049409;

        let result = unsafe { libc::ioctl(dest_fd, FICLONE, source_fd) };

        if result == 0 {
            Ok(ZeroCopyResult {
                bytes_copied: file_size,
                zerocopy_used: true,
                method: ZeroCopyMethod::Reflink,
            })
        } else {
            Ok(ZeroCopyResult {
                bytes_copied: 0,
                zerocopy_used: false,
                method: ZeroCopyMethod::Fallback,
            })
        }
    }

    /// Try Windows ReFS Copy-on-Write
    #[cfg(target_os = "windows")]
    async fn try_refs_cow<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        _source: P,
        _destination: Q,
        _file_size: u64,
    ) -> Result<ZeroCopyResult> {
        // TODO: Implement Windows ReFS CoW using FSCTL_DUPLICATE_EXTENTS_TO_FILE
        warn!("Windows ReFS CoW not yet implemented");
        Ok(ZeroCopyResult {
            bytes_copied: 0,
            zerocopy_used: false,
            method: ZeroCopyMethod::Fallback,
        })
    }

    /// Try macOS clonefile
    #[cfg(target_os = "macos")]
    async fn try_clonefile<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        _source: P,
        _destination: Q,
        _file_size: u64,
    ) -> Result<ZeroCopyResult> {
        // TODO: Implement macOS clonefile system call
        warn!("macOS clonefile not yet implemented");
        Ok(ZeroCopyResult {
            bytes_copied: 0,
            zerocopy_used: false,
            method: ZeroCopyMethod::Fallback,
        })
    }
}

impl Default for ZeroCopyEngine {
    fn default() -> Self {
        Self::new(true)
    }
}
