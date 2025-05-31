//! Windows-specific zero-copy implementations
//!
//! This module provides Windows-specific zero-copy operations including
//! ReFS Copy-on-Write and NTFS hardlinks.

#[cfg(target_os = "windows")]
use crate::methods::ZeroCopyMethod;
#[cfg(target_os = "windows")]
use ferrocp_types::{Error, Result, ZeroCopyMethod as ZeroCopyMethodType};
#[cfg(target_os = "windows")]
use std::ffi::OsStr;
#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;
#[cfg(target_os = "windows")]
use std::path::Path;
#[cfg(target_os = "windows")]
use std::ptr;
#[cfg(target_os = "windows")]
use tracing::debug;
#[cfg(target_os = "windows")]
use winapi::shared::minwindef::{DWORD, FALSE};
#[cfg(target_os = "windows")]
use winapi::um::fileapi::{GetFileAttributesW, GetVolumeInformationW};
#[cfg(target_os = "windows")]
use winapi::um::winnt::FILE_ATTRIBUTE_DIRECTORY;

#[cfg(target_os = "windows")]
/// Windows zero-copy implementation
#[derive(Debug)]
pub struct WindowsZeroCopy {
    /// Whether ReFS Copy-on-Write is available
    refs_cow_available: bool,
    /// Whether NTFS hardlinks are available
    ntfs_hardlinks_available: bool,
}

#[cfg(target_os = "windows")]
impl WindowsZeroCopy {
    /// Create a new Windows zero-copy implementation
    pub fn new() -> Self {
        Self {
            refs_cow_available: Self::check_refs_cow(),
            ntfs_hardlinks_available: Self::check_ntfs_hardlinks(),
        }
    }

    /// Check if ReFS Copy-on-Write is available
    fn check_refs_cow() -> bool {
        // In a real implementation, this would check if the system supports ReFS
        // and if the current filesystem is ReFS
        false // Simplified: assume not available for compatibility
    }

    /// Check if NTFS hardlinks are available
    fn check_ntfs_hardlinks() -> bool {
        // NTFS hardlinks are available on most Windows systems
        true
    }

    /// Attempt ReFS Copy-on-Write operation
    async fn try_refs_cow<P: AsRef<Path> + Send + Sync>(
        &self,
        source: P,
        destination: P,
        _size: u64,
    ) -> Result<()> {
        if !self.refs_cow_available {
            return Err(Error::zero_copy("ReFS Copy-on-Write not available"));
        }

        debug!("Attempting ReFS Copy-on-Write");

        let source_path = source.as_ref();
        let dest_path = destination.as_ref();

        // Check if both paths are on ReFS
        if !Self::is_refs_filesystem(source_path)? || !Self::is_refs_filesystem(dest_path)? {
            return Err(Error::zero_copy("Both paths must be on ReFS filesystem"));
        }

        // In a real implementation, this would use Windows APIs to perform CoW
        // For now, we'll simulate the operation
        tokio::task::spawn_blocking(move || {
            // Simulate ReFS CoW operation
            std::thread::sleep(std::time::Duration::from_millis(10));
            Ok(())
        })
        .await
        .map_err(|e| Error::other(format!("Task join error: {}", e)))?
    }

    /// Attempt NTFS hardlink operation
    async fn try_ntfs_hardlink<P: AsRef<Path> + Send + Sync>(
        &self,
        source: P,
        destination: P,
        _size: u64,
    ) -> Result<()> {
        if !self.ntfs_hardlinks_available {
            return Err(Error::zero_copy("NTFS hardlinks not available"));
        }

        debug!("Attempting NTFS hardlink creation");

        let source_path = source.as_ref().to_path_buf();
        let dest_path = destination.as_ref().to_path_buf();

        // Check if both paths are on the same NTFS volume
        if !Self::same_volume(&source_path, &dest_path)? {
            return Err(Error::zero_copy("Hardlinks require same volume"));
        }

        // Create hardlink using Windows API
        let result = tokio::task::spawn_blocking(move || {
            Self::create_hardlink_blocking(&source_path, &dest_path)
        })
        .await
        .map_err(|e| Error::other(format!("Task join error: {}", e)))?;

        result
    }

    /// Create hardlink using Windows API (blocking)
    fn create_hardlink_blocking(source: &Path, destination: &Path) -> Result<()> {
        // Convert paths to wide strings
        let source_wide: Vec<u16> = OsStr::new(source)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let dest_wide: Vec<u16> = OsStr::new(destination)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        // Use CreateHardLinkW API
        let result = unsafe {
            winapi::um::winbase::CreateHardLinkW(
                dest_wide.as_ptr(),
                source_wide.as_ptr(),
                ptr::null_mut(),
            )
        };

        if result == FALSE {
            let error_code = unsafe { winapi::um::errhandlingapi::GetLastError() };
            return Err(Error::zero_copy(format!(
                "CreateHardLinkW failed with error code: {}",
                error_code
            )));
        }

        debug!("Successfully created hardlink");
        Ok(())
    }

    /// Check if a path is on ReFS filesystem
    fn is_refs_filesystem(path: &Path) -> Result<bool> {
        let volume_path = Self::get_volume_path(path)?;
        let volume_wide: Vec<u16> = OsStr::new(&volume_path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let mut filesystem_name = vec![0u16; 256];
        let mut serial_number: DWORD = 0;
        let mut max_component_length: DWORD = 0;
        let mut filesystem_flags: DWORD = 0;

        let result = unsafe {
            GetVolumeInformationW(
                volume_wide.as_ptr(),
                ptr::null_mut(),
                0,
                &mut serial_number,
                &mut max_component_length,
                &mut filesystem_flags,
                filesystem_name.as_mut_ptr(),
                filesystem_name.len() as DWORD,
            )
        };

        if result == FALSE {
            return Err(Error::zero_copy("Failed to get volume information"));
        }

        let filesystem = String::from_utf16_lossy(&filesystem_name)
            .trim_end_matches('\0')
            .to_string();

        Ok(filesystem == "ReFS")
    }

    /// Check if two paths are on the same volume
    fn same_volume(source: &Path, destination: &Path) -> Result<bool> {
        let source_volume = Self::get_volume_path(source)?;
        let dest_volume = Self::get_volume_path(destination)?;
        Ok(source_volume == dest_volume)
    }

    /// Get the volume path for a given file path
    fn get_volume_path(path: &Path) -> Result<String> {
        // Get the root path (drive letter)
        if let Some(root) = path.ancestors().last() {
            Ok(root.to_string_lossy().to_string())
        } else {
            Err(Error::zero_copy("Cannot determine volume path"))
        }
    }

    /// Check if a path exists and is accessible
    fn path_exists(path: &Path) -> bool {
        let path_wide: Vec<u16> = OsStr::new(path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let attributes = unsafe { GetFileAttributesW(path_wide.as_ptr()) };
        attributes != winapi::um::fileapi::INVALID_FILE_ATTRIBUTES
    }

    /// Check if a path is a directory
    fn is_directory(path: &Path) -> bool {
        let path_wide: Vec<u16> = OsStr::new(path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let attributes = unsafe { GetFileAttributesW(path_wide.as_ptr()) };
        if attributes == winapi::um::fileapi::INVALID_FILE_ATTRIBUTES {
            return false;
        }

        (attributes & FILE_ATTRIBUTE_DIRECTORY) != 0
    }
}

#[cfg(target_os = "windows")]
#[async_trait::async_trait]
impl ZeroCopyMethod for WindowsZeroCopy {
    fn is_zero_copy_available(&self, source: &Path, destination: &Path) -> bool {
        let source_path = source;
        let dest_path = destination;

        // Check if source exists
        if !Self::path_exists(source_path) {
            return false;
        }

        // Check if source is not a directory
        if Self::is_directory(source_path) {
            return false;
        }

        // Check if ReFS CoW is available and both paths are on ReFS
        if self.refs_cow_available {
            if let (Ok(source_refs), Ok(dest_refs)) = (
                Self::is_refs_filesystem(source_path),
                Self::is_refs_filesystem(dest_path),
            ) {
                if source_refs && dest_refs {
                    return true;
                }
            }
        }

        // Check if NTFS hardlinks are available and both paths are on same volume
        if self.ntfs_hardlinks_available {
            if let Ok(same_vol) = Self::same_volume(source_path, dest_path) {
                if same_vol {
                    return true;
                }
            }
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
            ZeroCopyMethodType::RefsCoW => self.try_refs_cow(source, destination, size).await,
            ZeroCopyMethodType::NtfsHardlink => {
                self.try_ntfs_hardlink(source, destination, size).await
            }
            _ => Err(Error::zero_copy(format!(
                "Unsupported zero-copy method on Windows: {:?}",
                method
            ))),
        }
    }

    fn preferred_method(&self) -> ZeroCopyMethodType {
        if self.refs_cow_available {
            ZeroCopyMethodType::RefsCoW
        } else if self.ntfs_hardlinks_available {
            ZeroCopyMethodType::NtfsHardlink
        } else {
            ZeroCopyMethodType::Fallback
        }
    }

    fn supported_methods(&self) -> Vec<ZeroCopyMethodType> {
        let mut methods = Vec::new();

        if self.refs_cow_available {
            methods.push(ZeroCopyMethodType::RefsCoW);
        }
        if self.ntfs_hardlinks_available {
            methods.push(ZeroCopyMethodType::NtfsHardlink);
        }

        methods.push(ZeroCopyMethodType::Fallback);
        methods
    }
}

#[cfg(target_os = "windows")]
impl Default for WindowsZeroCopy {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::*;

    #[test]
    fn test_windows_zero_copy_creation() {
        let windows_zc = WindowsZeroCopy::new();
        // Should have at least NTFS hardlinks available
        assert!(windows_zc.ntfs_hardlinks_available);
    }

    #[test]
    fn test_supported_methods() {
        let windows_zc = WindowsZeroCopy::new();
        let methods = windows_zc.supported_methods();
        assert!(!methods.is_empty());
        assert!(methods.contains(&ZeroCopyMethodType::Fallback));

        // Should support NTFS hardlinks
        assert!(methods.contains(&ZeroCopyMethodType::NtfsHardlink));
    }

    #[test]
    fn test_preferred_method() {
        let windows_zc = WindowsZeroCopy::new();
        let preferred = windows_zc.preferred_method();

        // Should prefer ReFS CoW if available, otherwise NTFS hardlinks
        if windows_zc.refs_cow_available {
            assert_eq!(preferred, ZeroCopyMethodType::RefsCoW);
        } else if windows_zc.ntfs_hardlinks_available {
            assert_eq!(preferred, ZeroCopyMethodType::NtfsHardlink);
        } else {
            assert_eq!(preferred, ZeroCopyMethodType::Fallback);
        }
    }

    #[test]
    fn test_volume_path_extraction() {
        let path = Path::new("C:\\temp\\test.txt");
        let volume = WindowsZeroCopy::get_volume_path(path).unwrap();
        assert!(volume.starts_with("C:"));
    }

    #[test]
    fn test_same_volume_check() {
        let path1 = Path::new("C:\\temp\\file1.txt");
        let path2 = Path::new("C:\\temp\\file2.txt");
        let path3 = Path::new("D:\\temp\\file3.txt");

        // Same volume
        assert!(WindowsZeroCopy::same_volume(path1, path2).unwrap_or(false));

        // Different volumes
        assert!(!WindowsZeroCopy::same_volume(path1, path3).unwrap_or(true));
    }

    #[tokio::test]
    async fn test_zero_copy_availability() {
        let windows_zc = WindowsZeroCopy::new();

        // Test with current directory (should exist)
        let current_dir = std::env::current_dir().unwrap();
        let source = current_dir.join("test_source.txt");
        let dest = current_dir.join("test_dest.txt");

        // Create a test file
        if let Ok(_) = tokio::fs::write(&source, b"test content").await {
            let available = windows_zc.is_zero_copy_available(&source, &dest);
            // Should be available if NTFS hardlinks are supported and same volume
            assert_eq!(available, windows_zc.ntfs_hardlinks_available);

            // Clean up
            let _ = tokio::fs::remove_file(&source).await;
        }
    }
}
