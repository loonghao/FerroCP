//! Linux-specific zero-copy implementations
//!
//! This module provides Linux-specific zero-copy operations including
//! copy_file_range, sendfile, and filesystem-specific optimizations.

#[cfg(target_os = "linux")]
use crate::methods::ZeroCopyMethod;
#[cfg(target_os = "linux")]
use ferrocp_types::{Error, Result, ZeroCopyMethod as ZeroCopyMethodType};
#[cfg(target_os = "linux")]
use std::fs::File;
#[cfg(target_os = "linux")]
use std::os::unix::io::AsRawFd;
#[cfg(target_os = "linux")]
use std::path::Path;
#[cfg(target_os = "linux")]
use tokio::fs::OpenOptions;
#[cfg(target_os = "linux")]
use tracing::{debug, warn};

#[cfg(target_os = "linux")]
/// Linux zero-copy implementation
#[derive(Debug)]
pub struct LinuxZeroCopy {
    /// Whether copy_file_range is available
    copy_file_range_available: bool,
    /// Whether sendfile is available
    sendfile_available: bool,
    /// Whether io_uring is available
    io_uring_available: bool,
}

#[cfg(target_os = "linux")]
impl LinuxZeroCopy {
    /// Create a new Linux zero-copy implementation
    pub fn new() -> Self {
        Self {
            copy_file_range_available: Self::check_copy_file_range(),
            sendfile_available: Self::check_sendfile(),
            io_uring_available: Self::check_io_uring(),
        }
    }

    /// Check if copy_file_range is available
    fn check_copy_file_range() -> bool {
        // copy_file_range was introduced in Linux 4.5
        // In a real implementation, this would check the kernel version
        // or attempt to use the syscall
        true
    }

    /// Check if sendfile is available
    fn check_sendfile() -> bool {
        // sendfile is available on most Linux systems
        true
    }

    /// Check if io_uring is available
    fn check_io_uring() -> bool {
        // io_uring was introduced in Linux 5.1
        // For compatibility, we'll assume it's not available
        false
    }

    /// Attempt copy_file_range operation
    async fn try_copy_file_range<P: AsRef<Path> + Send + Sync>(
        &self,
        source: P,
        destination: P,
        size: u64,
    ) -> Result<()> {
        if !self.copy_file_range_available {
            return Err(Error::zero_copy("copy_file_range not available"));
        }

        debug!("Attempting copy_file_range for {} bytes", size);

        // Open source file for reading
        let source_file = OpenOptions::new()
            .read(true)
            .open(source.as_ref())
            .await
            .map_err(|e| Error::io(format!("Failed to open source file: {}", e)))?;

        // Open destination file for writing
        let dest_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(destination.as_ref())
            .await
            .map_err(|e| Error::io(format!("Failed to open destination file: {}", e)))?;

        // Convert to std::fs::File for raw fd access
        let source_std = source_file.into_std().await;
        let dest_std = dest_file.into_std().await;

        // Perform copy_file_range in a blocking task
        let result = tokio::task::spawn_blocking(move || {
            Self::copy_file_range_blocking(&source_std, &dest_std, size)
        })
        .await
        .map_err(|e| Error::io(format!("Task join error: {}", e)))?;

        result
    }

    /// Blocking copy_file_range implementation
    fn copy_file_range_blocking(source: &File, dest: &File, mut size: u64) -> Result<()> {
        let source_fd = source.as_raw_fd();
        let dest_fd = dest.as_raw_fd();
        let mut offset_in = 0i64;
        let mut offset_out = 0i64;

        while size > 0 {
            // copy_file_range can copy at most 2GB at a time
            let chunk_size = std::cmp::min(size, 2 * 1024 * 1024 * 1024);

            let copied = unsafe {
                libc::syscall(
                    libc::SYS_copy_file_range,
                    source_fd,
                    &mut offset_in as *mut i64,
                    dest_fd,
                    &mut offset_out as *mut i64,
                    chunk_size,
                    0u32, // flags
                )
            };

            if copied < 0 {
                let errno = unsafe { *libc::__errno_location() };
                return Err(Error::zero_copy(format!(
                    "copy_file_range failed with errno: {}",
                    errno
                )));
            }

            if copied == 0 {
                break; // End of file or no more data to copy
            }

            size -= copied as u64;
            debug!(
                "copy_file_range copied {} bytes, {} remaining",
                copied, size
            );
        }

        Ok(())
    }

    /// Attempt sendfile operation
    async fn try_sendfile<P: AsRef<Path> + Send + Sync>(
        &self,
        source: P,
        destination: P,
        size: u64,
    ) -> Result<()> {
        if !self.sendfile_available {
            return Err(Error::zero_copy("sendfile not available"));
        }

        debug!("Attempting sendfile for {} bytes", size);

        // Open source file for reading
        let source_file = OpenOptions::new()
            .read(true)
            .open(source.as_ref())
            .await
            .map_err(|e| Error::io(format!("Failed to open source file: {}", e)))?;

        // Open destination file for writing
        let dest_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(destination.as_ref())
            .await
            .map_err(|e| Error::io(format!("Failed to open destination file: {}", e)))?;

        // Convert to std::fs::File for raw fd access
        let source_std = source_file.into_std().await;
        let dest_std = dest_file.into_std().await;

        // Perform sendfile in a blocking task
        let result = tokio::task::spawn_blocking(move || {
            Self::sendfile_blocking(&source_std, &dest_std, size)
        })
        .await
        .map_err(|e| Error::io(format!("Task join error: {}", e)))?;

        result
    }

    /// Blocking sendfile implementation
    fn sendfile_blocking(source: &File, dest: &File, mut size: u64) -> Result<()> {
        let source_fd = source.as_raw_fd();
        let dest_fd = dest.as_raw_fd();
        let mut offset = 0i64;

        while size > 0 {
            // sendfile can send at most 2GB at a time
            let chunk_size = std::cmp::min(size, 2 * 1024 * 1024 * 1024) as usize;

            let sent =
                unsafe { libc::sendfile(dest_fd, source_fd, &mut offset as *mut i64, chunk_size) };

            if sent < 0 {
                let errno = unsafe { *libc::__errno_location() };
                return Err(Error::zero_copy(format!(
                    "sendfile failed with errno: {}",
                    errno
                )));
            }

            if sent == 0 {
                break; // End of file or no more data to send
            }

            size -= sent as u64;
            debug!("sendfile sent {} bytes, {} remaining", sent, size);
        }

        Ok(())
    }

    /// Check if paths are on the same filesystem
    fn same_filesystem<P: AsRef<Path>>(source: P, destination: P) -> bool {
        // In a real implementation, this would check the device IDs
        // For now, we'll do a simple check
        let source_path = source.as_ref();
        let dest_path = destination.as_ref();

        // Get the parent directories
        let source_parent = source_path.parent().unwrap_or(source_path);
        let dest_parent = dest_path.parent().unwrap_or(dest_path);

        // Simple heuristic: if they start with the same mount point, assume same filesystem
        source_parent == dest_parent
    }

    /// Check if filesystem supports reflink (BTRFS/XFS)
    fn supports_reflink<P: AsRef<Path>>(_path: P) -> bool {
        // In a real implementation, this would check the filesystem type
        // For now, we'll assume no reflink support for compatibility
        false
    }
}

#[cfg(target_os = "linux")]
#[async_trait::async_trait]
impl ZeroCopyMethod for LinuxZeroCopy {
    fn is_zero_copy_available(&self, source: &Path, destination: &Path) -> bool {
        // Check if any zero-copy method is available
        if !self.copy_file_range_available && !self.sendfile_available {
            return false;
        }

        // Check if paths are on the same filesystem (required for copy_file_range)
        if self.copy_file_range_available && Self::same_filesystem(source, destination) {
            return true;
        }

        // sendfile can work across filesystems in some cases
        if self.sendfile_available {
            return true;
        }

        false
    }

    async fn try_zero_copy(
        &self,
        source: &Path,
        destination: &Path,
        size: u64,
        method: ZeroCopyMethodType,
    ) -> Result<()> {
        match method {
            ZeroCopyMethodType::CopyFileRange => {
                self.try_copy_file_range(source, destination, size).await
            }
            ZeroCopyMethodType::SendFile => self.try_sendfile(source, destination, size).await,
            ZeroCopyMethodType::IoUring => {
                // io_uring implementation would go here
                Err(Error::zero_copy("io_uring not implemented"))
            }
            _ => Err(Error::zero_copy(format!(
                "Unsupported zero-copy method on Linux: {:?}",
                method
            ))),
        }
    }

    fn preferred_method(&self) -> ZeroCopyMethodType {
        if self.copy_file_range_available {
            ZeroCopyMethodType::CopyFileRange
        } else if self.sendfile_available {
            ZeroCopyMethodType::SendFile
        } else {
            ZeroCopyMethodType::Fallback
        }
    }

    fn supported_methods(&self) -> Vec<ZeroCopyMethodType> {
        let mut methods = Vec::new();

        if self.copy_file_range_available {
            methods.push(ZeroCopyMethodType::CopyFileRange);
        }
        if self.sendfile_available {
            methods.push(ZeroCopyMethodType::SendFile);
        }
        if self.io_uring_available {
            methods.push(ZeroCopyMethodType::IoUring);
        }

        methods.push(ZeroCopyMethodType::Fallback);
        methods
    }
}

#[cfg(target_os = "linux")]
impl Default for LinuxZeroCopy {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests {
    use super::*;

    #[test]
    fn test_linux_zero_copy_creation() {
        let linux_zc = LinuxZeroCopy::new();
        assert!(linux_zc.copy_file_range_available || linux_zc.sendfile_available);
    }

    #[test]
    fn test_supported_methods() {
        let linux_zc = LinuxZeroCopy::new();
        let methods = linux_zc.supported_methods();
        assert!(!methods.is_empty());
        assert!(methods.contains(&ZeroCopyMethodType::Fallback));
    }

    #[test]
    fn test_preferred_method() {
        let linux_zc = LinuxZeroCopy::new();
        let preferred = linux_zc.preferred_method();

        // Should prefer copy_file_range if available
        if linux_zc.copy_file_range_available {
            assert_eq!(preferred, ZeroCopyMethodType::CopyFileRange);
        } else if linux_zc.sendfile_available {
            assert_eq!(preferred, ZeroCopyMethodType::SendFile);
        } else {
            assert_eq!(preferred, ZeroCopyMethodType::Fallback);
        }
    }

    #[test]
    fn test_same_filesystem_check() {
        // Test with same directory
        assert!(LinuxZeroCopy::same_filesystem("/tmp/source", "/tmp/dest"));

        // Test with different directories (simplified check)
        assert!(!LinuxZeroCopy::same_filesystem("/tmp/source", "/home/dest"));
    }

    #[tokio::test]
    async fn test_zero_copy_availability() {
        let linux_zc = LinuxZeroCopy::new();

        // Test availability check
        let available = linux_zc.is_zero_copy_available("/tmp/source", "/tmp/dest");
        // Should be available if any method is supported
        assert_eq!(
            available,
            linux_zc.copy_file_range_available || linux_zc.sendfile_available
        );
    }
}
